//! Moteur Qwen3-TTS-12Hz-0.6B-CustomVoice via sidecar Python (carte N-071).
//!
//! Le modèle est PyTorch/Transformers (pas ONNX) : l'intégration passe par un
//! processus Python persistant (`resources/qwen_sidecar.py`) chargé une fois,
//! servi en JSONL sur stdin/stdout. La voix officielle par défaut est « Ryan »
//! (carte N-070) ; la fréquence produite est 24 kHz mono (identique au pipeline).
//!
//! Repli : si le sidecar est indisponible (torch/vvenv absent, modèle introuvable,
//! CUDA absent → CPU automatique), le moteur renvoie une erreur claire et
//! l'appelant (repli automatique Kokoro, carte N-071) peut basculer.
//!
//! Variables d'environnement :
//! - `NAVI_QWEN_PYTHON`  : interpréteur Python du venv (défaut : `python` dans le PATH) ;
//! - `NAVI_QWEN_SIDECAR` : chemin du sidecar (défaut : `<src-tauri>/resources/qwen_sidecar.py`) ;
//! - `NAVI_QWEN_VOICE`   : voix par défaut (défaut : `Ryan`) ;
//! - `NAVI_QWEN_STARTUP_TIMEOUT_MS` : délai max de chargement du modèle (défaut : 600 000 ms).

use super::engine::{SynthesisOutput, SynthesisRequest, TtsEngine, TtsLanguage, TtsError};
use base64::Engine as _;
use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex};
use std::time::Duration;

const DEFAULT_SPEAKER: &str = "Ryan";
const DEFAULT_STARTUP_TIMEOUT_MS: u64 = 600_000;

/// Configuration du moteur Qwen.
#[derive(Debug, Clone)]
pub struct QwenConfig {
    /// Interpréteur Python du venv portant torch + qwen_tts.
    pub python: PathBuf,
    /// Script sidecar (JSONL sur stdio).
    pub sidecar: PathBuf,
    /// Voix par défaut (Ryan — voix officielle, carte N-070).
    pub speaker: String,
    /// Délai max de chargement du modèle (le premier run télécharge ~2,3 Go).
    pub startup_timeout: Duration,
}

impl QwenConfig {
    /// Résout la configuration depuis l'environnement (`NAVI_QWEN_*`).
    pub fn from_env() -> Self {
        let python = std::env::var("NAVI_QWEN_PYTHON").unwrap_or_else(|_| "python".to_string());
        let sidecar = std::env::var("NAVI_QWEN_SIDECAR").unwrap_or_else(|_| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("qwen_sidecar.py")
                .to_string_lossy()
                .into_owned()
        });
        let speaker = std::env::var("NAVI_QWEN_VOICE").unwrap_or_else(|_| DEFAULT_SPEAKER.to_string());
        let startup_ms = std::env::var("NAVI_QWEN_STARTUP_TIMEOUT_MS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(DEFAULT_STARTUP_TIMEOUT_MS);
        Self {
            python: PathBuf::from(python),
            sidecar: PathBuf::from(sidecar),
            speaker,
            startup_timeout: Duration::from_millis(startup_ms),
        }
    }

    /// Vrai si le python résolvable existe (chemin explicite) ou est présumé dans le PATH.
    pub fn python_available(&self) -> bool {
        let looks_like_path = self.python.is_absolute() || self.python.components().count() > 1;
        if looks_like_path { self.python.exists() } else { true }
    }
}

/// Requête JSONL envoyée au sidecar.
fn encode_request(id: u64, text: &str, language: &str, speaker: &str) -> String {
    // serde_json échappe les retours à la ligne : la requête tient sur une seule ligne.
    json!({ "id": id, "text": text, "language": language, "speaker": speaker }).to_string() + "\n"
}

/// Réponse JSONL décodée (avant conversion des échantillons).
enum SidecarResponse {
    /// Ligne de disponibilité initiale.
    Ready { ok: bool, device: Option<String>, error: Option<String> },
    /// Réponse à une requête.
    Result { id: Option<u64>, ok: bool, sample_rate: Option<u32>, pcm_f32_base64: Option<String>, error: Option<String> },
}

fn parse_response_line(line: &str) -> Result<SidecarResponse, String> {
    let value: Value = serde_json::from_str(line.trim()).map_err(|e| format!("JSON invalide : {e}"))?;
    if value.get("ready").is_some() {
        let ok = value["ready"].as_bool().unwrap_or(false);
        Ok(SidecarResponse::Ready {
            ok,
            device: value.get("device").and_then(Value::as_str).map(str::to_string),
            error: value.get("error").and_then(Value::as_str).map(str::to_string),
        })
    } else {
        let ok = value.get("ok").and_then(Value::as_bool).unwrap_or(false);
        Ok(SidecarResponse::Result {
            id: value.get("id").and_then(Value::as_u64),
            ok,
            sample_rate: value.get("sample_rate").and_then(Value::as_u64).map(|v| v as u32),
            pcm_f32_base64: value.get("pcm_f32_base64").and_then(Value::as_str).map(str::to_string),
            error: value.get("error").and_then(Value::as_str).map(str::to_string),
        })
    }
}

/// Décode les échantillons f32 mono (little-endian) encodés en base64.
fn decode_pcm_f32(b64: &str) -> Result<Vec<f32>, String> {
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .map_err(|e| format!("base64 invalide : {e}"))?;
    if bytes.len() % 4 != 0 {
        return Err(format!("taille PCM incohérente ({} octets, pas un multiple de 4)", bytes.len()));
    }
    Ok(bytes.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
}

/// Processus sidecar vivant (stdin/stdout + compteur de requêtes).
struct SidecarChild {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    next_id: u64,
}

/// Moteur Qwen3-TTS (Sidecar Python).
pub struct QwenTtsEngine {
    config: QwenConfig,
    child: Mutex<Option<SidecarChild>>,
    /// Passe à faux si le sidecar meurt — le repli Kokoro prend le relais.
    alive: AtomicBool,
}

impl QwenTtsEngine {
    pub fn new(config: QwenConfig) -> Self {
        Self { config, child: Mutex::new(None), alive: AtomicBool::new(false) }
    }

    /// Langue Qwen : « French » | « English » (le sidecar mappe fr* → French).
    fn qwen_language(language: &TtsLanguage) -> &'static str {
        match language {
            TtsLanguage::French => "French",
            TtsLanguage::English => "English",
        }
    }

    /// Charge le modèle (bloque jusqu'à la ligne `ready` ou le délai configuré).
    fn load_inner(&mut self) -> Result<(), TtsError> {
        if !self.config.python_available() {
            return Err(TtsError::Io(format!(
                "python introuvable (NAVI_QWEN_PYTHON ou PATH) : {}",
                self.config.python.display()
            )));
        }
        if !self.config.sidecar.exists() {
            return Err(TtsError::Io(format!("sidecar introuvable : {}", self.config.sidecar.display())));
        }

        let mut child = Command::new(&self.config.python)
            .arg(&self.config.sidecar)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| TtsError::Io(format!("spawn sidecar impossible ({}) : {e}", self.config.python.display())))?;

        let stdin = child.stdin.take().ok_or_else(|| TtsError::Io("stdin du sidecar indisponible".to_string()))?;
        let stdout = child.stdout.take().ok_or_else(|| TtsError::Io("stdout du sidecar indisponible".to_string()))?;

        // Lecture de la ligne de disponibilité depuis un thread — le délai de chargement
        // du modèle peut être long (téléchargement 2,3 Go au premier run) mais reste borné.
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = String::new();
            let result = reader.read_line(&mut line).map(|_| line);
            let _ = tx.send((reader, result));
        });

        let (reader, first) = match rx.recv_timeout(self.config.startup_timeout) {
            Ok(pair) => pair,
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(TtsError::Io(format!(
                    "chargement du modèle dépassé ({} ms) — vérifier NAVI_QWEN_PYTHON et le cache Hugging Face",
                    self.config.startup_timeout.as_millis()
                )));
            }
        };

        let line = first.map_err(|e| {
            let _ = child.kill();
            TtsError::Io(format!("lecture de la ligne ready impossible : {e}"))
        })?;

        match parse_response_line(&line) {
            Ok(SidecarResponse::Ready { ok: true, device, .. }) => {
                eprintln!("[navi] sidecar qwen3-tts prêt ({})", device.as_deref().unwrap_or("?"));
                *self.child.lock().map_err(|_| TtsError::Io("verrou sidecar".to_string()))? =
                    Some(SidecarChild { child, stdin, stdout: reader, next_id: 1 });
                self.alive.store(true, Ordering::SeqCst);
                Ok(())
            }
            Ok(SidecarResponse::Ready { ok: false, error, .. }) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(TtsError::Inference(error.unwrap_or_else(|| "sidecar pas prêt (cause inconnue)".to_string())))
            }
            Ok(SidecarResponse::Result { .. }) | Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                Err(TtsError::Inference(format!("ligne ready inattendue du sidecar : {}", line.trim())))
            }
        }
    }
}

impl Drop for QwenTtsEngine {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.child.lock() {
            if let Some(mut sidecar) = guard.take() {
                let _ = sidecar.child.kill();
                let _ = sidecar.child.wait();
            }
        }
    }
}

impl TtsEngine for QwenTtsEngine {
    fn name(&self) -> &'static str {
        "qwen3-tts-0.6b-customvoice"
    }

    fn supported_languages(&self) -> &'static [TtsLanguage] {
        &[TtsLanguage::French, TtsLanguage::English]
    }

    fn voices(&self) -> Vec<String> {
        vec![self.config.speaker.clone()]
    }

    fn load(&mut self) -> Result<(), TtsError> {
        self.load_inner()
    }

    fn is_loaded(&self) -> bool {
        self.alive.load(Ordering::SeqCst)
    }

    fn synthesize(&self, request: &SynthesisRequest) -> Result<SynthesisOutput, TtsError> {
        if !self.is_loaded() {
            return Err(TtsError::NotLoaded);
        }
        let language = request.language.unwrap_or(TtsLanguage::French);

        let speaker = request.voice.clone().unwrap_or_else(|| self.config.speaker.clone());
        let mut guard = self.child.lock().map_err(|_| TtsError::Io("verrou sidecar".to_string()))?;
        let sidecar = guard.as_mut().ok_or(TtsError::NotLoaded)?;

        let id = sidecar.next_id;
        sidecar.next_id += 1;
        let payload = encode_request(id, &request.text, Self::qwen_language(&language), &speaker);
        sidecar.stdin.write_all(payload.as_bytes()).and_then(|_| sidecar.stdin.flush()).map_err(|e| {
            self.alive.store(false, Ordering::SeqCst);
            TtsError::Inference(format!("écriture vers le sidecar impossible : {e}"))
        })?;

        let mut line = String::new();
        match sidecar.stdout.read_line(&mut line) {
            Ok(0) => {
                self.alive.store(false, Ordering::SeqCst);
                return Err(TtsError::Inference("le sidecar Qwen s'est arrêté (EOF)".to_string()));
            }
            Ok(_) => {}
            Err(e) => {
                self.alive.store(false, Ordering::SeqCst);
                return Err(TtsError::Inference(format!("lecture du sidecar impossible : {e}")));
            }
        }

        match parse_response_line(&line).map_err(TtsError::Inference)? {
            SidecarResponse::Result { id: Some(rid), ok: true, sample_rate: Some(sr), pcm_f32_base64: Some(b64), .. } => {
                if rid != id {
                    return Err(TtsError::Inference(format!("réponse désynchronisée (attendu id {id}, reçu {rid})")));
                }
                let samples = decode_pcm_f32(&b64).map_err(TtsError::Inference)?;
                Ok(SynthesisOutput { samples, sample_rate: sr })
            }
            SidecarResponse::Result { ok: false, error, .. } => {
                Err(TtsError::Inference(error.unwrap_or_else(|| "échec de synthèse côté sidecar".to_string())))
            }
            SidecarResponse::Result { ok: true, .. } => {
                Err(TtsError::Inference("réponse sidecar incomplète (sample_rate/pcm manquant)".to_string()))
            }
            SidecarResponse::Ready { .. } => {
                Err(TtsError::Inference("ligne ready inattendue pendant une requête".to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_request_est_une_seule_ligne_et_echappe_les_retours_ligne() {
        let line = encode_request(7, "ligne 1\nligne 2 \"guillemets\"", "French", "Ryan");
        assert_eq!(line.matches('\n').count(), 1, "la requête doit tenir sur une seule ligne");
        let parsed: Value = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(parsed["id"], 7);
        assert_eq!(parsed["text"], "ligne 1\nligne 2 \"guillemets\"");
        assert_eq!(parsed["language"], "French");
        assert_eq!(parsed["speaker"], "Ryan");
    }

    #[test]
    fn parse_ready_true() {
        match parse_response_line(r#"{"ready": true, "sample_rate": 24000, "device": "cuda:0", "model": "m"}"#).unwrap() {
            SidecarResponse::Ready { ok, device, .. } => {
                assert!(ok);
                assert_eq!(device.as_deref(), Some("cuda:0"));
            }
            _ => panic!("ligne ready attendue"),
        }
    }

    #[test]
    fn parse_ready_false_porte_le_message_derreur() {
        match parse_response_line(r#"{"ready": false, "error": "modèle introuvable"}"#).unwrap() {
            SidecarResponse::Ready { ok, error, .. } => {
                assert!(!ok);
                assert_eq!(error.as_deref(), Some("modèle introuvable"));
            }
            _ => panic!("ligne ready attendue"),
        }
    }

    #[test]
    fn parse_resultat_ok_et_erreur() {
        match parse_response_line(r#"{"id": 3, "ok": true, "sample_rate": 24000, "pcm_f32_base64": "QQAA", "n_samples": 1}"#).unwrap() {
            SidecarResponse::Result { id, ok, sample_rate, pcm_f32_base64, .. } => {
                assert_eq!(id, Some(3));
                assert!(ok);
                assert_eq!(sample_rate, Some(24_000));
                assert_eq!(pcm_f32_base64.as_deref(), Some("QQAA"));
            }
            _ => panic!("résultat attendu"),
        }
        match parse_response_line(r#"{"id": 3, "ok": false, "error": "boom"}"#).unwrap() {
            SidecarResponse::Result { ok, error, .. } => {
                assert!(!ok);
                assert_eq!(error.as_deref(), Some("boom"));
            }
            _ => panic!("résultat attendu"),
        }
    }

    #[test]
    fn decode_pcm_f32_roundtrip() {
        // 2 échantillons f32 : 1.0 et -0.5 (little-endian).
        let bytes: Vec<u8> = [1.0f32.to_le_bytes(), (-0.5f32).to_le_bytes()].concat();
        let b64 = base64::engine::general_purpose::STANDARD.encode(bytes);
        let samples = decode_pcm_f32(&b64).unwrap();
        assert_eq!(samples, vec![1.0, -0.5]);
    }

    #[test]
    fn decode_pcm_f32_rejette_une_taille_incoherente() {
        let b64 = base64::engine::general_purpose::STANDARD.encode([1u8, 2, 3]);
        assert!(decode_pcm_f32(&b64).is_err());
        assert!(decode_pcm_f32("!!!pas du base64!!!").is_err());
    }

    #[test]
    fn langue_qwen_mappee_correctement() {
        assert_eq!(QwenTtsEngine::qwen_language(&TtsLanguage::French), "French");
        assert_eq!(QwenTtsEngine::qwen_language(&TtsLanguage::English), "English");
    }

    #[test]
    fn config_depuis_le_venv_dev_du_poste() {
        // Le venv scratch du poste (voix échantillonnées N-070) est une candidate légitime ;
        // seul le chemin absolu est testé — la présence effective n'est pas garantie en CI.
        let cfg = QwenConfig {
            python: PathBuf::from(r"D:\Git\Projets\Navi\scratch\qwen_test312_cuda\Scripts\python.exe"),
            sidecar: PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources").join("qwen_sidecar.py"),
            speaker: DEFAULT_SPEAKER.to_string(),
            startup_timeout: Duration::from_secs(600),
        };
        assert!(cfg.sidecar.exists(), "le sidecar doit exister dans resources/");
    }
}
