//! Moteurs TTS : Kokoro (ONNX, actuel) et Qwen3-TTS-0.6B-CustomVoice (sidecar Python, carte N-071) — avec repli automatique.

pub mod engine;
pub mod fallback;
pub mod qwen;
pub use engine::{SynthesisOutput, SynthesisRequest, TtsError, TtsLanguage, TTS_SAMPLE_RATE};
pub use engine::{TtsEngine};
pub use fallback::ResilientEngine;
pub use qwen::{QwenConfig, QwenTtsEngine};

use ort::{environment::Environment, session::Session, session::builder::GraphOptimizationLevel, value::Tensor};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::utils;

/// Configuration du moteur TTS
#[derive(Debug, Clone)]
pub struct TtsConfig {
    pub model_path: PathBuf,
    pub voices_path: PathBuf,
    pub vocab_path: PathBuf,
    pub espeak_lib_path: String,
    pub espeak_data_path: String,
    pub default_voice_female: String,
    pub voice_male: String,
    pub default_language: String,
}

/// Chemin du modèle ONNX (nom unique sur la machine).
pub const KOKORO_MODEL_FILE: &str = "kokoro-v1.0.int8.onnx";
/// Nom du fichier de styles de voix.
pub const KOKORO_VOICES_FILE: &str = "voices-v1.0.bin";
/// Nom du vocabulaire.
pub const KOKORO_VOCAB_FILE: &str = "vocab.json";

impl TtsConfig {
    /// Construit une config pour un répertoire de modèles donné (dev : `models/kokoro/`).
    pub fn with_dir(dir: &std::path::Path) -> Self {
        Self {
            model_path: dir.join(KOKORO_MODEL_FILE),
            voices_path: dir.join(KOKORO_VOICES_FILE),
            vocab_path: dir.join(KOKORO_VOCAB_FILE),
            ..Self::default()
        }
    }
}

impl Default for TtsConfig {
    fn default() -> Self {
        let base = crate::utils::resolve_models_dir();
        let phonemizer_dir = base.parent().map(|p| p.join("phonemizer")).filter(|p| p.exists()).unwrap_or_else(|| base.clone());
        let (espeak_lib_path, espeak_data_path) = {
            let dll = phonemizer_dir.join("espeak-ng.dll");
            if dll.exists() {
                (dll.to_string_lossy().to_string(), phonemizer_dir.join("espeak-ng-data").to_string_lossy().to_string())
            } else {
                // Repli : installation système (Program Files)
                (
                    "C:\\Program Files\\eSpeak NG\\libespeak-ng.dll".to_string(),
                    "C:\\Program Files\\eSpeak NG\\espeak-ng-data".to_string(),
                )
            }
        };
        Self {
            model_path: base.join(KOKORO_MODEL_FILE),
            voices_path: base.join(KOKORO_VOICES_FILE),
            vocab_path: base.join(KOKORO_VOCAB_FILE),
            espeak_lib_path,
            espeak_data_path,
            default_voice_female: "af_heart".to_string(),
            voice_male: "am_adam".to_string(),
            default_language: "auto".to_string(),
        }
    }
}

/// Moteur TTS Kokoro
pub struct KokoroEngine {
    config: TtsConfig,
    session: Arc<Mutex<Option<Session>>>,
    vocab: Arc<HashMap<String, i64>>,
    voice_styles: Arc<Mutex<HashMap<String, Vec<f32>>>>,
    phonemizer: Arc<Phonemizer>,
}

struct Phonemizer {
    set_voice: unsafe extern "system" fn(*const i8) -> i32,
    to_phonemes: unsafe extern "system" fn(*mut *const i8, i32, i32) -> *const i8,
    init: unsafe extern "system" fn(i32, i32, *const i8, i32) -> i32,
}

impl KokoroEngine {
    pub fn new(config: TtsConfig) -> Result<Self, Box<dyn std::error::Error>> {
        // Charger vocabulaire
        let vocab_content = std::fs::read_to_string(&config.vocab_path)?;
        let vocab: HashMap<String, i64> = serde_json::from_str(&vocab_content)?;
        
        // Charger styles de voix
        let voices_data = std::fs::read(&config.voices_path)?;
        let voice_styles = Self::parse_voices_bin(&voices_data)?;
        
        // Initialiser espeak-ng
        let phonemizer = Phonemizer::new(&config.espeak_lib_path, &config.espeak_data_path)?;
        
        Ok(Self {
            config,
            session: Arc::new(Mutex::new(None)),
            vocab: Arc::new(vocab),
            voice_styles: Arc::new(Mutex::new(voice_styles)),
            phonemizer: Arc::new(phonemizer),
        })
    }
    
    /// Initialise la session ONNX (lazy)
    fn ensure_session(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut session_lock = self.session.lock().unwrap();
        if session_lock.is_none() {
            let env = ort::environment::init().build()?;
            let session = Session::builder(&env)?
                .with_optimization_level(GraphOptimizationLevel::Level3)?
                .commit_from_file(&self.config.model_path)?;
            *session_lock = Some(session);
        }
        Ok(())
    }
    
    /// Génère l'audio pour un texte
    pub fn synthesize(&self, text: &str, voice: Option<&str>, language: Option<&str>, speed: f32) -> Result<Vec<f32>, Box<dyn std::error::Error>> {
        self.ensure_session()?;
        
        let voice = voice.unwrap_or(if self.config.default_language == "fr" {
            &self.config.default_voice_female
        } else {
            &self.config.voice_male
        });
        
        let language = language.unwrap_or(&self.config.default_language);
        
        // Phonémisation
        let phonemes = self.phonemizer.phonemize(text, language)?;
        
        // Tokenisation
        let tokens = self.tokenize(&phonemes)?;
        if tokens.len() + 2 > 510 {
            return Err(format!(
                "Segment trop long pour Kokoro : {} tokens phonémisés (max 508 + 2 padding) — réduisez la taille de segment",
                tokens.len()
            )
            .into());
        }
        
        // Récupérer style de voix
        let voice_styles = self.voice_styles.lock().unwrap();
        let style = voice_styles.get(voice)
            .ok_or_else(|| format!("Voix non trouvée: {}", voice))?;
        
        // Préparer entrée
        let style_slice = &style[tokens.len() * 256..tokens.len() * 256 + 256];
        let mut ids = vec![0i64];
        ids.extend_from_slice(&tokens);
        ids.push(0);
        
        // Inférence
        let mut session_lock = self.session.lock().unwrap();
        let session = session_lock.as_mut().unwrap();
        
        let input = ort::inputs! {
            "tokens" => Tensor::from_array(([1usize, ids.len()], ids))?,
            "style" => Tensor::from_array(([1usize, 256], style_slice.to_vec()))?,
            "speed" => Tensor::from_array(([1usize], vec![speed]))?,
        };
        
        let outputs = session.run(input)?;
        let (_, audio) = outputs["audio"].try_extract_tensor::<f32>()?;
        
        Ok(audio.to_vec())
    }
    
    /// Tokenisation selon vocabulaire Kokoro
    fn tokenize(&self, phonemes: &str) -> Result<Vec<i64>, Box<dyn std::error::Error>> {
        let filtered: String = phonemes
            .chars()
            .filter(|c| self.vocab.contains_key(&c.to_string()))
            .collect();
        
        let tokens: Vec<i64> = filtered
            .chars()
            .filter_map(|c| self.vocab.get(&c.to_string()).copied())
            .collect();
        
        Ok(tokens)
    }
    
    fn parse_voices_bin(data: &[u8]) -> Result<HashMap<String, Vec<f32>>, Box<dyn std::error::Error>> {
        // voices-v1.0.bin est une archive .npz (zip) d'un .npy par voix, [510, 1, 256] f32
        let reader = std::io::Cursor::new(data);
        let mut archive = zip::ZipArchive::new(reader)?;
        let mut map = HashMap::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let fname = file.name()?.to_string();
            if !fname.ends_with(".npy") {
                continue;
            }
            let name = fname.trim_end_matches(".npy").to_string();
            let npy = npyz::NpyFile::new(&mut file)?;
            let shape = npy.shape().to_vec();
            if shape != [510, 1, 256] {
                return Err(format!("Voix {name}: forme inattendue {shape:?} (attendu [510, 1, 256])").into());
            }
            map.insert(name, npy.into_vec::<f32>()?);
        }
        if map.is_empty() {
            return Err("voices-v1.0.bin : aucune voix trouvée dans l'archive .npz".into());
        }
        Ok(map)
    }
}

impl Phonemizer {
    fn new(lib_path: &str, data_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let lib = unsafe { libloading::Library::new(lib_path)? };
        
        let init: unsafe extern "system" fn(i32, i32, *const i8, i32) -> i32 = unsafe { *lib.get(b"espeak_Initialize")? };
        let set_voice = unsafe { *lib.get(b"espeak_SetVoiceByName")? };
        let to_phonemes = unsafe { *lib.get(b"espeak_TextToPhonemes")? };
        
        let data_c = std::ffi::CString::new(data_path)?;
        let rc = unsafe { init(0x02, 0, data_c.as_ptr(), 0) };
        if rc <= 0 {
            return Err("espeak_Initialize failed".into());
        }
        
        // Garder la lib en vie
        std::mem::forget(lib);
        
        Ok(Self { set_voice, to_phonemes, init })
    }
    
    fn phonemize(&self, text: &str, lang: &str) -> Result<String, Box<dyn std::error::Error>> {
        let voice_id = match lang {
            "fr" | "fr-fr" => "roa\\fr",
            "en" | "en-us" => "gmw\\en-US",
            _ => "gmw\\en-US",
        };
        
        let voice_c = std::ffi::CString::new(voice_id)?;
        unsafe { (self.set_voice)(voice_c.as_ptr()) };
        
        let text_c = std::ffi::CString::new(text)?;
        let mut chunk: *const i8 = text_c.as_ptr();
        let mut result = String::new();
        
        loop {
            let mut p: *const i8 = chunk;
            let r = unsafe { (self.to_phonemes)(&mut p, 1, (b'_' as i32) << 8 | 0x02) };
            if chunk.is_null() { break; }
            chunk = p;
            if !r.is_null() {
                if !result.is_empty() { result.push(' '); }
                result.push_str(unsafe { std::ffi::CStr::from_ptr(r).to_str()? });
            }
        }
        Ok(result)
    }
}
