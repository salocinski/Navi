//! Navi - Lecteur de texte à voix haute 100% local
//!
//! Architecture modulaire :
//! - config: gestion de configuration persistante
//! - capture: capture presse-papiers et texte sélectionné
//! - hotkey: raccourcis globaux système
//! - tts: moteur TTS (Kokoro via ONNX Runtime)
//! - audio: lecture audio (rodio)
//! - state: machine d'états de lecture
//! - text: nettoyage, langue, segmentation en production
//! - pipeline: chaîne lecture (capture → moteur → audio)
//! - ui: zone de notification, panneau, menu
//! - utils: utilitaires communs

pub mod capture;
pub mod config;

static LAST_FOREGROUND: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Suit en arrière-plan la fenêtre active non-Navi — permet à la capture
/// de viser la bonne cible (au lieu de Navi lui-même quand la Steam/Chromium a le focus).
fn track_last_foreground() {
    std::thread::spawn(move || {
        use windows::Win32::System::Threading::GetCurrentProcessId;
        use windows::Win32::UI::WindowsAndMessaging::{
            GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId,
        };
        let own = unsafe { GetCurrentProcessId() };
        let mut cls = [0u16; 256];
        loop {
            let hwnd = unsafe { GetForegroundWindow() };
            if !hwnd.0.is_null() {
                let mut pid: u32 = 0;
                unsafe { GetWindowThreadProcessId(HWND(hwnd.0), Some(&mut pid)) };
                // Ignore les fenêtres Shell (taskbar, overflow, menu, bureau) — mémoire réservée aux fenêtres applicatives.
                let len_class = unsafe { GetClassNameW(hwnd, &mut cls) };
                let class_name = String::from_utf16_lossy(&cls[..len_class as usize]);
                let is_shell_ui = matches!(
                    class_name.as_str(),
                    "NotifyIconOverflowWindow"
                        | "Shell_TrayWnd"
                        | "Shell_SecondaryTrayWnd"
                        | "SysShadow"
                        | "#32768"
                        | "ToolbarWindow32"
                        | "Progman"
                        | "WorkerW"
                );
                if pid != 0 && pid != own && !is_shell_ui {
                    LAST_FOREGROUND.store(hwnd.0 as usize, std::sync::atomic::Ordering::SeqCst);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
    });
}

/// Dernière fenêtre non-Navi mémorisée (0 si aucune).
pub(crate) fn last_foreground() -> Option<*mut core::ffi::c_void> {
    let v = LAST_FOREGROUND.load(std::sync::atomic::Ordering::SeqCst);
    if v == 0 {
        None
    } else {
        Some(v as *mut core::ffi::c_void)
    }
}

/// Oublie la fenêtre mémorisée (appelée quand elle est détruite ou invalide,
/// pour ne pas viser un HWND recyclé).
pub(crate) fn clear_last_foreground() {
    LAST_FOREGROUND.store(0, std::sync::atomic::Ordering::SeqCst);
}

/// Secondes depuis l'epoch de la dernière activité utilisateur (lecture, pause, reprise,
/// arrêt). Initialisé au démarrage ; décrémente à chaque interaction (carte N-075).
static LAST_USER_ACTIVITY_SECS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Marque une interaction utilisateur (lecture, pause, reprise, arrêt).
pub(crate) fn touch_user_activity() {
    LAST_USER_ACTIVITY_SECS.store(now_secs(), std::sync::atomic::Ordering::SeqCst);
}

/// Secondes écoulées depuis la dernière interaction (∞ si aucune).
pub(crate) fn idle_secs() -> Option<u64> {
    let last = LAST_USER_ACTIVITY_SECS.load(std::sync::atomic::Ordering::SeqCst);
    (last != 0).then(|| now_secs().saturating_sub(last))
}
pub mod audio;
pub mod autostart;
pub mod dictation;
pub mod hotkey;
pub mod insert;
pub mod models_store;
pub mod models_update;
pub mod pipeline;
pub mod state;
pub mod stt;
pub mod text;
pub mod tts;
pub mod ui;
pub mod utils;

use serde::Serialize;
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager};
use windows::Win32::Foundation::HWND;

/// Vue partagée entre les commandes Tauri et le thread d'écoute des raccourcis.
/// Chaque champ est clonable (compteurs Arc internes) — construire un `NaviRef`
/// est bon marché et ne prend pas la propriété de l'état global.
#[derive(Clone)]
pub struct NaviRef {
    pub machine: crate::state::PlaybackStateMachine,
    pub engine: Arc<Mutex<Option<crate::tts::KokoroEngine>>>,
    pub audio: Arc<Mutex<Option<crate::audio::AudioHandle>>>,
    /// Garde anti double-chargement (ort n'autorise qu'un seul `Environment` par processus).
    pub loading: Arc<std::sync::atomic::AtomicBool>,
    /// Rapport d'enregistrement des raccourcis (pour l'UI).
    pub hotkey_report: Arc<Mutex<crate::hotkey::RegistrationReport>>,
    /// Dictée vocale (machine d'états + moteur Whisper).
    pub dictation: crate::dictation::Dictation,
}

/// État partagé de Navi (machines à états + moteur Kokoro + audio).
pub struct NaviState {
    machine: crate::state::PlaybackStateMachine,
    engine: Arc<Mutex<Option<crate::tts::KokoroEngine>>>,
    audio: Arc<Mutex<Option<crate::audio::AudioHandle>>>,
    loading: Arc<std::sync::atomic::AtomicBool>,
    hotkey_report: Arc<Mutex<crate::hotkey::RegistrationReport>>,
    dictation: crate::dictation::Dictation,
}

impl NaviState {
    fn new() -> Self {
        Self {
            machine: crate::state::PlaybackStateMachine::new(),
            engine: Arc::new(Mutex::new(None)),
            audio: Arc::new(Mutex::new(None)),
            loading: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            hotkey_report: Arc::new(Mutex::new(crate::hotkey::RegistrationReport::default())),
            dictation: crate::dictation::Dictation::default(),
        }
    }

    fn view(&self) -> NaviRef {
        NaviRef {
            machine: self.machine.clone(),
            engine: self.engine.clone(),
            audio: self.audio.clone(),
            loading: self.loading.clone(),
            hotkey_report: self.hotkey_report.clone(),
            dictation: self.dictation.clone(),
        }
    }
}

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// DTO de statut exposé à l'UI (lecture seule).
#[derive(Serialize)]
struct StatusDto {
    phase: crate::state::PlaybackState,
    current_chunk: usize,
    total_chunks: usize,
    engine_loaded: bool,
    audio_ready: bool,
    /// Message d'échec d'enregistrement des raccourcis, s'il y en a un.
    hotkey_error: Option<String>,
}

/// Configuration courante pour la fenêtre Paramètres (carte N-051).
/// Fichier illisible : valeurs par défaut (journalisé) — l'utilisateur peut ainsi le réparer en enregistrant.
#[tauri::command]
fn get_config() -> crate::config::AppConfig {
    let mut config = crate::config::AppConfig::load().unwrap_or_else(|e| {
        eprintln!("[navi] config illisible, valeurs par défaut : {e}");
        Default::default()
    });
    // Le registre fait foi : l'utilisateur peut retirer Navi du démarrage depuis Windows.
    config.autostart = crate::autostart::is_enabled();
    config
}

/// Voix proposées dans la fenêtre Paramètres (source unique : `config::VOICES_*`).
#[tauri::command]
fn get_voices() -> serde_json::Value {
    serde_json::json!({ "fr": crate::config::VOICES_FR, "en": crate::config::VOICES_EN })
}

/// Gestionnaire de raccourcis partagé : réenregistrement à chaud depuis `set_config`.
pub struct SharedHotkeys(pub Mutex<crate::hotkey::HotkeyManager>);

/// Applique `config` aux raccourcis et met à jour le rapport affiché (`hotkey_error`).
fn apply_hotkeys(
    hotkeys: &SharedHotkeys,
    view: &NaviRef,
    config: &crate::config::AppConfig,
) -> crate::hotkey::RegistrationReport {
    let report = hotkeys
        .0
        .lock()
        .map(|mut m| m.apply(&crate::hotkey::bindings_of(config)))
        .unwrap_or_default();
    if let Ok(mut guard) = view.hotkey_report.lock() {
        *guard = crate::hotkey::RegistrationReport {
            registered: report.registered.clone(),
            failed: report.failed.clone(),
        };
    }
    report
}

/// Valide puis enregistre la configuration reçue de la fenêtre Paramètres.
///
/// Les raccourcis modifiés sont réenregistrés à chaud ; en cas de conflit (OS ou autre
/// application) on revient aux raccourcis précédents et la configuration n'est pas écrite.
/// Commande synchrone : Tauri l'exécute sur le thread principal, celui qui a créé le
/// gestionnaire de raccourcis (exigence de `RegisterHotKey`).
#[tauri::command]
fn set_config(
    mut config: crate::config::AppConfig,
    state: tauri::State<NaviState>,
    hotkeys: tauri::State<SharedHotkeys>,
) -> Result<(), String> {
    config.validate()?;
    let previous = crate::config::AppConfig::load().unwrap_or_default();
    // La date du dernier contrôle appartient à l'application : une page de réglages ouverte avant le contrôle ne l'efface pas.
    config.last_update_check = previous.last_update_check;
    let view = state.view();
    let hotkeys_changed =
        crate::hotkey::bindings_of(&config) != crate::hotkey::bindings_of(&previous);
    let was_autostart = crate::autostart::is_enabled();
    // Retour aux réglages précédents si une étape suivante échoue.
    let rollback = || {
        if hotkeys_changed {
            apply_hotkeys(&hotkeys, &view, &previous);
        }
        if config.autostart != was_autostart {
            let _ = crate::autostart::set_enabled(was_autostart);
        }
    };

    if hotkeys_changed
        && let Some(message) = apply_hotkeys(&hotkeys, &view, &config).failure_message()
    {
        rollback();
        return Err(message);
    }
    if config.autostart != was_autostart
        && let Err(e) = crate::autostart::set_enabled(config.autostart)
    {
        rollback();
        return Err(format!("Démarrage avec Windows : {e}"));
    }
    config.save().map_err(|e| {
        rollback();
        format!("Enregistrement impossible : {e}")
    })
}

#[tauri::command]
fn get_status(state: tauri::State<NaviState>) -> StatusDto {
    let (current_chunk, total_chunks) = state.machine.progress();
    let hotkey_error = state
        .hotkey_report
        .lock()
        .ok()
        .and_then(|r| r.failure_message());
    StatusDto {
        phase: state.machine.state(),
        current_chunk,
        total_chunks,
        engine_loaded: state.engine.lock().map(|e| e.is_some()).unwrap_or(false),
        audio_ready: state.audio.lock().map(|a| a.is_some()).unwrap_or(false),
        hotkey_error,
    }
}

/// Micros disponibles pour la dictée (réglages, N-093).
#[tauri::command]
fn list_microphones() -> Vec<String> {
    crate::audio::micro::list_input_devices()
}

#[derive(serde::Serialize)]
struct DictationModelDto {
    present: bool,
    downloading: bool,
    path: String,
}

/// État du modèle de dictée (présence et taille ; l'empreinte est contrôlée au premier usage).
#[tauri::command]
fn dictation_model_status(state: tauri::State<NaviState>) -> DictationModelDto {
    let path = crate::stt::model::model_path();
    DictationModelDto {
        present: crate::stt::model::is_present(&path),
        downloading: state.dictation.is_downloading(),
        path: path.display().to_string(),
    }
}

/// Télécharge le modèle de dictée en tâche de fond (progression : événements `dictation-state`).
#[tauri::command]
fn download_dictation_model(state: tauri::State<NaviState>) -> Result<(), String> {
    if crate::stt::model::is_present(&crate::stt::model::model_path()) {
        return Ok(());
    }
    state.dictation.start_download();
    Ok(())
}

#[derive(serde::Serialize)]
struct KokoroModelDto {
    present: bool,
    downloading: bool,
    done: u64,
    total: u64,
}

/// État du modèle de lecture (présence et taille ; l'empreinte est contrôlée au chargement).
#[tauri::command]
fn kokoro_model_status() -> KokoroModelDto {
    let settings = crate::config::AppConfig::load().unwrap_or_default();
    let dir = crate::utils::resolve_models_dir_with(settings.models_dir.as_deref());
    let (downloading, done, total) = crate::tts::install::progress();
    KokoroModelDto {
        present: crate::tts::install::is_installed(&dir),
        downloading,
        done,
        total,
    }
}

/// Contrôle à la demande de la disponibilité d'une mise à jour des modèles (aucune installation).
#[tauri::command]
async fn check_model_updates() -> Result<crate::models_update::CheckResult, String> {
    tauri::async_runtime::spawn_blocking(crate::models_update::run_check)
        .await
        .map_err(|e| e.to_string())?
}

/// Dernier résultat de contrôle de la session, s'il y en a un.
#[tauri::command]
fn model_update_status() -> Option<crate::models_update::CheckResult> {
    crate::models_update::last()
}

#[derive(serde::Serialize)]
struct ModelVersionDto {
    id: &'static str,
    version: String,
    has_previous: bool,
    /// `update` ou `rollback` : changement appliqué au prochain démarrage.
    pending: Option<&'static str>,
}

/// Version installée de chaque modèle, retour arrière possible et changement en attente.
#[tauri::command]
fn model_versions() -> Vec<ModelVersionDto> {
    let store = crate::models_store::read();
    crate::models_store::IDS
        .iter()
        .filter_map(|id| {
            let pinned = crate::models_store::pinned(id)?;
            Some(ModelVersionDto {
                id,
                version: store.version(id, &pinned),
                has_previous: store.has_previous(id),
                pending: store.pending(id),
            })
        })
        .collect()
}

#[derive(serde::Serialize)]
struct DownloadProgressDto {
    downloading: bool,
    done: u64,
    total: u64,
}

#[tauri::command]
fn model_update_progress() -> DownloadProgressDto {
    let (downloading, done, total) = crate::models_update::progress();
    DownloadProgressDto {
        downloading,
        done,
        total,
    }
}

/// Télécharge la mise à jour d'un modèle (action explicite) ; appliquée au prochain démarrage.
#[tauri::command]
async fn stage_model_update(id: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || crate::models_update::stage_update(&id))
        .await
        .map_err(|e| e.to_string())?
}

/// Demande le retour à la version précédente d'un modèle ; appliqué au prochain démarrage.
#[tauri::command]
fn stage_model_rollback(id: String) -> Result<(), String> {
    crate::models_store::pinned(&id).ok_or("Modèle inconnu")?;
    crate::models_store::update(|s| s.stage_rollback(&id)).and_then(|r| r)
}

/// Crée le moteur Kokoro si besoin (installation au premier usage, vérification, chargement).
/// Bloquant et à chargement unique : un appel concurrent attend la fin du premier — ort refuse
/// plusieurs `Environment`. N-099 : la lecture passe aussi par ici, le moteur n'existe qu'après.
pub(crate) fn ensure_engine(view: &NaviRef) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    let ready = || -> Result<bool, String> {
        Ok(view
            .engine
            .lock()
            .map_err(|_| "verrou moteur".to_string())?
            .is_some())
    };
    loop {
        if ready()? {
            return Ok(());
        }
        if !view.loading.swap(true, Ordering::SeqCst) {
            break;
        }
        // shortcut: attente par sondage (100 ms), sans délai maximal ; à remplacer par un Condvar si besoin.
        std::thread::sleep(std::time::Duration::from_millis(100));
        if !view.loading.load(Ordering::SeqCst) && !ready()? {
            return Err("Chargement du modèle échoué".to_string());
        }
    }
    let result = (|| -> Result<crate::tts::KokoroEngine, String> {
        let settings = crate::config::AppConfig::load().unwrap_or_default();
        let dir = crate::utils::resolve_models_dir_with(settings.models_dir.as_deref());
        // N-066 : premier usage, les fichiers du modèle sont téléchargés (vérifiés par SHA-256) puis restent locaux.
        if !crate::tts::install::is_installed(&dir) {
            crate::tts::install::install(&dir)?;
        }
        crate::tts::check_models(&dir).map_err(|e| e.to_string())?;
        let config = crate::tts::TtsConfig::with_dir(&dir);
        let mut kokoro = crate::tts::KokoroEngine::new(config).map_err(|e| {
            format!(
                "chargement moteur impossible ({e}). Modèles attendus dans {}",
                dir.display()
            )
        })?;
        crate::tts::TtsEngine::load(&mut kokoro)
            .map_err(|e| format!("modèle Kokoro non chargeable : {e}"))?;
        Ok(kokoro)
    })();
    let outcome = match result {
        Ok(kokoro) => {
            // N-098 : premier chargement réussi, l'ancienne version du modèle devient inutile.
            crate::models_store::confirm_use("kokoro");
            view.engine
                .lock()
                .map_err(|_| "verrou moteur".to_string())
                .map(|mut guard| *guard = Some(kokoro))
        }
        Err(message) => {
            eprintln!("[navi] chargement moteur échoué : {message}");
            view.machine.set_error(message.clone());
            Err(message)
        }
    };
    view.loading.store(false, Ordering::SeqCst);
    outcome
}

/// Charge le modèle Kokoro en tâche de fond (idempotent ; ~11 s au premier appel).
#[tauri::command]
fn load_model(state: tauri::State<NaviState>) -> Result<(), String> {
    let view = state.view();
    std::thread::spawn(move || {
        let _ = ensure_engine(&view);
    });
    Ok(())
}

/// N-100 : supprime le modèle de lecture puis le réinstalle (retour à un état sain).
/// Refusé pendant un chargement ou un téléchargement ; la session ONNX est libérée avant la
/// suppression (fichiers verrouillés sous Windows).
#[tauri::command]
fn reinstall_kokoro(state: tauri::State<NaviState>) -> Result<(), String> {
    use std::sync::atomic::Ordering;
    let view = state.view();
    if view.loading.swap(true, Ordering::SeqCst) {
        return Err("Un chargement du modèle est en cours, réessayez dans un instant".to_string());
    }
    std::thread::spawn(move || {
        let removed = (|| -> Result<(), String> {
            *view.engine.lock().map_err(|_| "verrou moteur".to_string())? = None;
            let settings = crate::config::AppConfig::load().unwrap_or_default();
            let dir = crate::utils::resolve_models_dir_with(settings.models_dir.as_deref());
            crate::tts::install::remove(&dir)
        })();
        view.loading.store(false, Ordering::SeqCst);
        match removed {
            Ok(()) => {
                let _ = ensure_engine(&view);
            }
            Err(message) => {
                eprintln!("[navi] réinstallation du modèle échouée : {message}");
                view.machine.set_error(message);
            }
        }
    });
    Ok(())
}

/// Lance la lecture de la sélection courante : capture → nettoyage → langue → segments → audio.
#[tauri::command]
fn start_reading(state: tauri::State<NaviState>) -> Result<String, String> {
    read_selection_impl(&state.view())
}

pub(crate) fn read_selection_impl(state: &NaviRef) -> Result<String, String> {
    if state.dictation.machine.is_active() {
        return Err("Lecture ignorée : une dictée est en cours".to_string());
    }
    touch_user_activity();
    state.machine.begin_capture()?;

    // 0. Chargement du modèle en parallèle de la capture (carte N-075) :
    //    ~11 s à froid ; la capture (~1 s) se déroule pendant ce temps.
    let loader = {
        let loaded = state
            .engine
            .lock()
            .map_err(|_| "verrou moteur".to_string())?
            .as_ref()
            .map(|e| e.session_loaded())
            .unwrap_or(false);
        if loaded {
            None
        } else {
            let view = state.clone();
            Some(std::thread::spawn(move || {
                // Le chargement est idempotent (ensure_session) ; un préchargement
                // concurrent au démarrage est réutilisé, pas doublonné.
                (|| -> Result<(), String> {
                    ensure_engine(&view)?;
                    let mut guard = view
                        .engine
                        .lock()
                        .map_err(|_| "verrou moteur".to_string())?;
                    let engine = guard
                        .as_mut()
                        .ok_or_else(|| "Moteur TTS non chargé".to_string())?;
                    crate::tts::TtsEngine::load(engine)
                        .map_err(|e| format!("moteur TTS non chargeable : {e}"))
                })()
            }))
        }
    };

    // 1. Capture (snapshot presse-papiers + SendInput Ctrl+C + restauration)
    //    — en parallèle du chargement éventuel.
    let cap = crate::capture::capture_selection();
    if !cap.success {
        let message = cap
            .error
            .clone()
            .unwrap_or_else(|| "Capture échouée".to_string());
        state.machine.set_error(message.clone());
        return Err(message);
    }

    // 2. Attendre la fin du chargement éventuel du moteur (la lecture ne démarre
    //    que moteur prêt — carte N-075).
    if let Some(loader) = loader {
        match loader.join() {
            Ok(Ok(())) => {}
            Ok(Err(message)) => {
                state.machine.set_error(message.clone());
                return Err(message);
            }
            Err(_) => {
                let message = "Chargement du modèle interrompu".to_string();
                state.machine.set_error(message.clone());
                return Err(message);
            }
        }
    }

    // 2bis. Le modèle est là — vrai depuis le boot ou depuis l'étape 2.
    if state
        .engine
        .lock()
        .map_err(|_| "verrou moteur".to_string())?
        .is_none()
    {
        let message = "Modèle Kokoro non chargé — réessayez la lecture".to_string();
        state.machine.set_error(message.clone());
        return Err(message);
    }

    // 3. Nettoyage (HTML/RTF/ANSI/Unicode — médias/markup retirés, erreur claire sinon)
    let format_hint = cap.format_used.to_lowercase();
    let cleaned = match crate::text::clean_captured_text(&cap.text, &format_hint) {
        Ok(text) => text,
        Err(err) => {
            let message = err.to_string();
            state.machine.set_error(message.clone());
            return Err(message);
        }
    };

    // 4. Langue (détection hors moteur ; hint fr en cas de doute)
    let language = Some(match crate::text::detect_language(&cleaned) {
        crate::text::LanguageDetection::English => crate::tts::engine::TtsLanguage::English,
        _ => crate::tts::engine::TtsLanguage::French,
    });

    // 5. Segments
    let segments = crate::text::segmentation::segment_text(&cleaned);
    if segments.is_empty() {
        let message = "Aucun segment à lire".to_string();
        state.machine.set_error(message.clone());
        return Err(message);
    }

    let segment_count = segments.len();
    state.machine.begin_generation_phase()?;
    state.machine.set_text(cleaned, segment_count);

    // Réglages de lecture (fenêtre Paramètres, N-051) : relus à chaque lecture.
    let config = crate::config::AppConfig::load().unwrap_or_else(|e| {
        eprintln!("[navi] config illisible, valeurs par défaut : {e}");
        Default::default()
    });

    // 6. Audio : reset (nouvelle session) ou création (thread audio) si absent
    {
        let mut audio = state.audio.lock().map_err(|_| "verrou audio".to_string())?;
        match audio.as_ref() {
            Some(handle) => handle.reset()?,
            None => {
                let handle = crate::audio::AudioHandle::new()
                    .map_err(|e| format!("Aucun périphérique audio par défaut : {e}"))?;
                *audio = Some(handle);
            }
        }
        // Après reset : le nouveau sink repart au volume par défaut.
        if let Some(handle) = audio.as_ref() {
            handle.set_volume(config.volume)?;
        }
    }

    // 7. Jeton de génération + thread de synthèse
    let token = state.machine.begin_generation();
    state.machine.play()?;

    let machine = state.machine.clone();
    let engine = state.engine.clone();
    let audio = state.audio.clone();
    let params = crate::pipeline::SynthesisParams {
        language,
        speed: config.speech_rate.clamp(0.5, 2.0),
        voice: Some(config.voice_for(language).to_string()),
    };
    std::thread::spawn(move || {
        eprintln!(
            "[navi] thread de synthèse démarré ({} segments)",
            segments.len()
        );
        let outcome = (|| -> Result<usize, String> {
            let engine_guard = engine.lock().map_err(|_| "verrou moteur".to_string())?;
            let kokoro = engine_guard
                .as_ref()
                .ok_or_else(|| "Modèle Kokoro non chargé".to_string())?;
            let sink = audio
                .lock()
                .map_err(|_| "verrou audio".to_string())?
                .as_ref()
                .cloned()
                .ok_or_else(|| "Périphérique audio disparu".to_string())?;
            crate::pipeline::synthesize_segments(
                kokoro as &dyn crate::tts::engine::TtsEngine,
                &segments,
                &sink,
                &|| machine.is_generation_cancelled(token),
                &params,
                &|| {
                    let _ = machine.next_chunk();
                },
            )
        })();
        match outcome {
            Ok(done) => {
                eprintln!("[navi] synthèse terminée — {done} segments envoyés dans la file audio")
            }
            Err(message) => {
                eprintln!("[navi] synthèse échouée : {message}");
                machine.set_error(message);
            }
        }
    });

    Ok(format!("Lecture en cours — {segment_count} segments"))
}

#[tauri::command]
fn pause_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    pause_reading_impl(&state.view())
}

pub(crate) fn pause_reading_impl(state: &NaviRef) -> Result<(), String> {
    touch_user_activity();
    state.machine.pause()?;
    state
        .audio
        .lock()
        .map_err(|_| "verrou audio".to_string())?
        .as_ref()
        .ok_or_else(|| "Audio non prêt".to_string())?
        .pause()
}

#[tauri::command]
fn resume_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    resume_reading_impl(&state.view())
}

pub(crate) fn resume_reading_impl(state: &NaviRef) -> Result<(), String> {
    touch_user_activity();
    state.machine.resume()?;
    state
        .audio
        .lock()
        .map_err(|_| "verrou audio".to_string())?
        .as_ref()
        .ok_or_else(|| "Audio non prêt".to_string())?
        .resume()
}

#[tauri::command]
fn stop_reading(state: tauri::State<NaviState>) -> Result<(), String> {
    stop_reading_impl(&state.view())
}

pub(crate) fn stop_reading_impl(state: &NaviRef) -> Result<(), String> {
    touch_user_activity();
    state.machine.stop();
    if let Ok(audio) = state.audio.lock()
        && let Some(handle) = audio.as_ref()
    {
        let _ = handle.stop();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Instance Navi déjà active : la seconde occurrence se termine ici.
            // Effet au premier plan : afficher/focaliser la fenêtre principale
            // (même effet qu'un clic gauche sur le médaillon).
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .invoke_handler(tauri::generate_handler![
            greet,
            get_status,
            dictation_model_status,
            list_microphones,
            download_dictation_model,
            kokoro_model_status,
            reinstall_kokoro,
            check_model_updates,
            model_update_status,
            model_versions,
            model_update_progress,
            stage_model_update,
            stage_model_rollback,
            get_config,
            get_voices,
            set_config,
            load_model,
            start_reading,
            pause_reading,
            resume_reading,
            stop_reading,
            ui::open_settings,
            ui::hide_panel,
            ui::quit_app,
        ])
        .setup(|app| {
            // N-098 : mises à jour et retours arrière de modèles en attente, avant tout chargement.
            crate::models_store::apply_pending();
            let state = NaviState::new();
            track_last_foreground();
            touch_user_activity();
            let handle = app.handle().clone();
            state.machine.subscribe(move |event| {
                // Événements consommés par l'UI via @tauri-apps/api/event (`navi-state`).
                let _ = handle.emit("navi-state", event);
            });

            // Pastille flottante du cercle animé (carte N-077, révision TW-68) —
            // frameless, toujours-au-dessus, sans focus, non cliquable ; visible de la
            // combinaison (Capture) jusqu'à la fin de l'audio ; masquée sur Stopped/Idle.
            {
                if let Err(err) = crate::ui::flotteur::create_floating_window(app.handle()) {
                    eprintln!("[navi] pastille flottante indisponible : {err}");
                }
            }
            {
                let handle = app.handle().clone();
                state.machine.subscribe(move |event| {
                    use crate::state::{PlaybackState, StateEvent};
                    let StateEvent::PhaseChanged { to, .. } = event else {
                        return;
                    };
                    let Some(window) = handle.get_webview_window(crate::ui::flotteur::LABEL) else {
                        return;
                    };
                    let visible = matches!(
                        to,
                        PlaybackState::Capturing
                            | PlaybackState::Generating
                            | PlaybackState::Playing
                            | PlaybackState::Paused
                            | PlaybackState::Error
                    );
                    if visible {
                        let _ = window.show();
                    } else {
                        let _ = window.hide();
                    }
                });
            }

            // Dictée (carte N-092) : événements pour la pastille + affichage REC / traitement / erreur.
            {
                let handle = app.handle().clone();
                let machine = state.dictation.machine.clone();
                state.dictation.machine.subscribe(move |event| {
                    use crate::dictation::{DictationEvent, DictationState};
                    let _ = handle.emit("dictation-state", &event);
                    let DictationEvent::PhaseChanged { to, .. } = event else {
                        return;
                    };
                    let Some(window) = handle.get_webview_window(crate::ui::flotteur::LABEL) else {
                        return;
                    };
                    if to == DictationState::Idle {
                        let _ = window.hide();
                        return;
                    }
                    let _ = window.show();
                    if to == DictationState::Error {
                        // L'erreur reste visible 3 s puis la pastille se masque.
                        let machine = machine.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(std::time::Duration::from_secs(3));
                            machine.clear_error();
                        });
                    }
                });
            }

            crate::ui::panneau::setup(app.handle());

            // Icône de zone de notification + menu (carte N-047) — feuille DA docs/pistes-interface.html.
            {
                let view = state.view();
                let tray_handle = app.handle().clone();
                if let Err(err) = crate::ui::create_system_tray(&tray_handle, view) {
                    eprintln!("[navi] tray indisponible : {err}");
                }
            }

            // Anneaux d'état de l'icône (carte N-050) — DA maquette C (prêt/génération/lecture/pause/erreur).
            {
                let view = state.view();
                crate::ui::tray_icons::spawn_icon_animator(app.handle().clone(), view);
            }

            // Raccourcis globaux — défauts, écoute via GlobalHotKeyEvent.
            {
                let view = state.view();
                match crate::hotkey::HotkeyManager::new() {
                    Ok(mut manager) => {
                        let config = crate::config::AppConfig::load().unwrap_or_else(|e| {
                            eprintln!("[navi] config illisible, valeurs par défaut : {e}");
                            Default::default()
                        });
                        let report = manager.apply(&crate::hotkey::bindings_of(&config));
                        if let Ok(mut guard) = view.hotkey_report.lock() {
                            *guard = report;
                        }
                        let hotkeys = manager.hotkeys_shared();
                        // Le gestionnaire reste vivant dans l'état Tauri (réenregistrement à chaud, N-052).
                        app.manage(SharedHotkeys(Mutex::new(manager)));
                        let exec = view.clone();
                        let dictation_hold = std::sync::Arc::new(crate::hotkey::hold::HoldGuard::default());
                        std::thread::spawn(move || {
                            use global_hotkey::GlobalHotKeyEvent;
                            for event in GlobalHotKeyEvent::receiver() {
                                let Some(action) = hotkeys.lock().unwrap().get(&event.id).copied() else { continue };
                                match action {
                                    crate::hotkey::HotkeyAction::ReadSelection => {
                                        let phase = exec.machine.state();
                                        if matches!(phase, crate::state::PlaybackState::Capturing | crate::state::PlaybackState::Generating | crate::state::PlaybackState::Playing | crate::state::PlaybackState::Paused) {
                                            eprintln!("[navi] raccourci lecture ignoré — une lecture est déjà en cours (Ctrl+Maj+P pause/reprise, Ctrl+Maj+S arrêt)");
                                        } else if let Err(err) = read_selection_impl(&exec) {
                                            exec.machine.set_error(err.clone());
                                            eprintln!("[navi] raccourci lecture échoué : {err}");
                                        }
                                    }
                                    crate::hotkey::HotkeyAction::PauseResume => {
                                        let outcome = match exec.machine.state() {
                                            crate::state::PlaybackState::Paused => resume_reading_impl(&exec),
                                            _ => pause_reading_impl(&exec),
                                        };
                                        if let Err(err) = outcome {
                                            exec.machine.set_error(err.clone());
                                            eprintln!("[navi] raccourci pause/reprise échoué : {err}");
                                        }
                                    }
                                    crate::hotkey::HotkeyAction::Stop => {
                                        let _ = stop_reading_impl(&exec);
                                    }
                                    crate::hotkey::HotkeyAction::Dictation => {
                                        // La répétition de WM_HOTKEY pendant le maintien est ignorée.
                                        if !dictation_hold.try_begin() {
                                            continue;
                                        }
                                        let combo = crate::config::AppConfig::load().unwrap_or_default().hotkey_dictation;
                                        let Some(vk) = crate::hotkey::hold::main_key_vk(&combo) else {
                                            dictation_hold.end();
                                            eprintln!("[navi] dictée : touche de « {combo} » non gérée pour le maintien");
                                            continue;
                                        };
                                        crate::dictation::start(&exec, vk, dictation_hold.clone());
                                    }
                                }
                            }
                        });
                    }
                    Err(err) => {
                        eprintln!("[navi] raccourcis indisponibles — pas disponibles sur cet OS : {err}");
                    }
                }
            }

            // Surveillance de fin de lecture : le sink audio retombe vide → machine → Idle
            // (sans ça, le bouton « Lire » reste bloqué en Playing après une lecture complète).
            {
                let view = state.view();
                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(400));
                        let phase = view.machine.state();
                        if !matches!(phase, crate::state::PlaybackState::Playing) {
                            continue;
                        }
                        // N'enclenche le retour à Inactif qu'une fois que tous les
                        // chunks ont été écrits dans la file (sinon le sink est toujours
                        // « vide » quelques secondes, et on couperait la lecture trop tôt).
                        let (current_chunk, total_chunks) = view.machine.progress();
                        if total_chunks == 0 || current_chunk < total_chunks {
                            continue;
                        }
                        let playing = view
                            .audio
                            .lock()
                            .ok()
                            .and_then(|h| h.as_ref().map(|h| h.is_playing()))
                            .unwrap_or(false);
                        if !playing {
                            eprintln!("[navi] lecture terminée — retour à l'état Inactif");
                            view.machine.set_idle();
                        }
                    }
                });
            }

            // Déchargement du modèle après inactivité (carte N-075) — libère la RAM de la
            // session ONNX après N secondes sans utilisation (défaut 300, `NAVI_MODEL_IDLE_UNLOAD_SECS`).
            // L'environnement ONNX (global au processus) est conservé : le modèle se recharge
            // à la prochaine lecture (chargement à la demande, en parallèle de la capture).
            {
                let view = state.view();
                std::thread::spawn(move || {
                    let idle_limit = std::env::var("NAVI_MODEL_IDLE_UNLOAD_SECS")
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(300);
                    loop {
                        std::thread::sleep(std::time::Duration::from_secs(30));
                        let Some(idle) = crate::idle_secs() else { continue };
                        if idle < idle_limit {
                            continue;
                        }
                        let phase = view.machine.state();
                        if matches!(
                            phase,
                            crate::state::PlaybackState::Capturing
                                | crate::state::PlaybackState::Generating
                                | crate::state::PlaybackState::Playing
                                | crate::state::PlaybackState::Paused
                        ) {
                            continue;
                        }
                        let loaded = view
                            .engine
                            .lock()
                            .ok()
                            .and_then(|e| e.as_ref().map(|x| x.session_loaded()))
                            .unwrap_or(false);
                        if !loaded {
                            continue;
                        }
                        if let Ok(mut guard) = view.engine.lock()
                            && let Some(engine) = guard.as_mut()
                                && engine.unload_session() {
                                    eprintln!("[navi] modèle déchargé après {idle_limit} s sans utilisation (carte N-075)");
                                }
                    }
                });
            }

            // Migration N-095 : l'ancienne valeur Run (N-054) devient une tâche planifiée élevée.
            std::thread::spawn(|| {
                if let Err(e) = crate::autostart::migrate_legacy() {
                    eprintln!("[navi] migration du démarrage automatique : {e}");
                }
            });

            // Contrôle hebdomadaire de disponibilité de mise à jour des modèles (N-066, opt-in).
            std::thread::spawn(|| {
                let config = crate::config::AppConfig::load().unwrap_or_default();
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                if crate::models_update::is_due(config.update_check_weekly, config.last_update_check, now)
                    && let Err(e) = crate::models_update::run_check()
                {
                    eprintln!("[navi] contrôle de mise à jour des modèles : {e}");
                }
            });

            // Signature sonore de démarrage : joue « Hey listen » (original, voix aiguë)
            // une seule fois, une fois raccourcis et capture prêts.
            let config = crate::config::AppConfig::load().unwrap_or_else(|e| {
                eprintln!("[navi] config illisible, valeurs par défaut : {e}");
                Default::default()
            });
            if config.startup_sound {
                crate::audio::play_signature_once(config.startup_sound_gain());
            }

            app.manage(state);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
