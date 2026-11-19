//! Lecture audio (rodio)

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, buffer::SamplesBuffer};
use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
use std::time::Duration;

/// Fréquence d'échantillonnage Kokoro (mono f32).
pub const SAMPLE_RATE: u32 = 24_000;

/// File ordonnée de segments (logique pure, testable sans périphérique audio).
///
/// FIFO stricte : le premier segment enfilé est le premier joué.
/// `stop` / `clear` vident la file : tout segment non encore joué est abandonné.
#[derive(Default)]
pub struct SegmentQueue {
    inner: VecDeque<Vec<f32>>,
}

impl SegmentQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, samples: Vec<f32>) {
        self.inner.push_back(samples);
    }

    pub fn pop(&mut self) -> Option<Vec<f32>> {
        self.inner.pop_front()
    }

    pub fn clear(&mut self) {
        self.inner.clear();
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }
}

/// Messages reçus par le thread audio (propriétaire du périphérique).
enum AudioMessage {
    /// Enfile des échantillons (file FIFO).
    Enqueue(Vec<f32>),
    /// Pause la lecture (position conservée).
    Pause,
    /// Reprise.
    Resume,
    /// Arrêt : invalide le consommateur, vide la file, ferme le sink.
    Stop,
    /// Nouvelle session : vide la file, nouveau sink + consommateur.
    Reset,
    /// Volume (0.0–1.0).
    SetVolume(f32),
    /// Demande d'état lecture (réponse oneshot depuis le thread audio).
    IsPlaying(std::sync::mpsc::Sender<bool>),
}

/// Poignée Send + Sync vers le périphérique audio détenu par un thread dédié.
///
/// Le `OutputStream` rodio/cpal contient des pointeurs bruts → il ne **peut pas**
/// vivre dans le state Tauri. Le thread audio le possède ; les commandes arrivent
/// par un canal `mpsc` (ordre garanti) — aucun périphérique ne traverse les threads.
#[derive(Clone)]
pub struct AudioHandle {
    tx: std::sync::mpsc::Sender<AudioMessage>,
}

impl AudioHandle {
    /// Crée la poignée : spawn le thread audio et attend la confirmation d'ouverture
    /// du périphérique (ou l'erreur).
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (init_tx, init_rx) = std::sync::mpsc::channel();
        let (tx, rx) = std::sync::mpsc::channel();

        std::thread::spawn(move || {
            let player = AudioPlayer::new();
            match player {
                Ok(player) => {
                    let _ = init_tx.send(Ok(()));
                    for message in rx {
                        match message {
                            AudioMessage::Enqueue(samples) => player.enqueue(samples),
                            AudioMessage::Pause => player.pause(),
                            AudioMessage::Resume => player.resume(),
                            AudioMessage::Stop => player.stop(),
                            AudioMessage::Reset => {
                                let _ = player.reset();
                            }
                            AudioMessage::SetVolume(volume) => player.set_volume(volume),
                            AudioMessage::IsPlaying(reply) => {
                                let _ = reply.send(player.is_playing());
                            }
                        }
                    }
                }
                Err(err) => {
                    let _ = init_tx.send(Err(err.to_string()));
                }
            }
        });

        match init_rx.recv_timeout(std::time::Duration::from_secs(5)) {
            Ok(Ok(())) => Ok(Self { tx }),
            Ok(Err(message)) => Err(message.into()),
            Err(_) => Err("Périphérique audio : timeout à l'ouverture".into()),
        }
    }

    fn send(&self, message: AudioMessage) -> Result<(), String> {
        self.tx
            .send(message)
            .map_err(|_| "Périphérique audio : thread terminé".to_string())
    }

    /// Enfile des échantillons — non bloquant, ordre garanti par le canal.
    pub fn enqueue(&self, samples: Vec<f32>) -> Result<(), String> {
        self.send(AudioMessage::Enqueue(samples))
    }

    pub fn pause(&self) -> Result<(), String> {
        self.send(AudioMessage::Pause)
    }

    pub fn resume(&self) -> Result<(), String> {
        self.send(AudioMessage::Resume)
    }

    pub fn stop(&self) -> Result<(), String> {
        self.send(AudioMessage::Stop)
    }

    /// Nouvelle session : file vidée, nouveau sink, consommateur réinitialisé.
    pub fn reset(&self) -> Result<(), String> {
        self.send(AudioMessage::Reset)
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.send(AudioMessage::SetVolume(volume))
    }

    /// Demande (oneshot) si un sink joue réellement un flux. Ne bloque jamais (timeout 200 ms).
    pub fn is_playing(&self) -> bool {
        let (tx, rx) = std::sync::mpsc::channel();
        if self.tx.send(AudioMessage::IsPlaying(tx)).is_err() {
            return false;
        }
        rx.recv_timeout(std::time::Duration::from_millis(200))
            .unwrap_or(false)
    }
}

impl crate::pipeline::AudioSink for AudioHandle {
    fn write(&self, samples: &[f32]) -> Result<(), String> {
        self.enqueue(samples.to_vec())
    }
}

/// Gestionnaire de lecture audio avec file (queue).
///
/// Cycle de vie : `new` → `reset()` → `enqueue…` → `pause`/`resume`/`stop`.
/// `reset()` coupe tout consommateur précédent (via génération) et repart
/// proprement — c'est l'entrée de chaque nouvelle session de lecture.
pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
    pending: Arc<Mutex<SegmentQueue>>,
    condvar: Arc<Condvar>,
    paused: Arc<Mutex<bool>>,
    consumer_generation: Arc<Mutex<u64>>,
    consumer_alive: Arc<std::sync::atomic::AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&handle)?;
        let player = Self {
            _stream: stream,
            handle,
            sink: Arc::new(Mutex::new(Some(sink))),
            pending: Arc::new(Mutex::new(SegmentQueue::new())),
            condvar: Arc::new(Condvar::new()),
            paused: Arc::new(Mutex::new(false)),
            consumer_generation: Arc::new(Mutex::new(0)),
            consumer_alive: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        player.spawn_consumer();
        Ok(player)
    }

    /// Spawn (ou remplace) le consommateur audio.
    /// Chaque spawn incrémente `consumer_generation` ; les anciens consommateurs sortent d'eux-mêmes.
    ///
    /// Ne fait rien si un consommateur est déjà en vie (évite la multiplication des threads).
    fn spawn_consumer(&self) {
        if self
            .consumer_alive
            .load(std::sync::atomic::Ordering::SeqCst)
        {
            return;
        }
        self.consumer_alive
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let sink = self.sink.clone();
        let pending = self.pending.clone();
        let condvar = self.condvar.clone();
        let paused = self.paused.clone();
        let generation = self.consumer_generation.clone();
        let alive = self.consumer_alive.clone();

        let my_gen = {
            let mut counter = generation.lock().unwrap();
            *counter += 1;
            *counter
        };

        thread::spawn(move || {
            loop {
                let is_current = { *generation.lock().unwrap() == my_gen };
                if !is_current {
                    break;
                }

                let samples = {
                    let mut pending = pending.lock().unwrap();
                    loop {
                        if let Some(samples) = pending.pop() {
                            break Some(samples);
                        }
                        // Sortir proprement si ce consommateur n'est plus le courant
                        if *generation.lock().unwrap() != my_gen {
                            break None;
                        }
                        pending = condvar.wait(pending).unwrap();
                    }
                };

                let Some(samples) = samples else {
                    eprintln!("[audio] consommateur gen {my_gen} sorti");
                    break;
                };
                eprintln!(
                    "[audio] consommateur gen {my_gen} : pop {} échantillons, append au sink",
                    samples.len()
                );

                // Ne pas jouer si ce consommateur n'est plus le courant
                if *generation.lock().unwrap() != my_gen {
                    break;
                }

                // Attendre la reprise si en pause
                while *paused.lock().unwrap() {
                    if *generation.lock().unwrap() != my_gen {
                        break;
                    }
                    thread::sleep(Duration::from_millis(50));
                }
                if *generation.lock().unwrap() != my_gen {
                    break;
                }

                let sink_lock = sink.lock().unwrap();
                if let Some(sink) = sink_lock.as_ref() {
                    let n = samples.len();
                    let source = SamplesBuffer::new(1, SAMPLE_RATE, samples);
                    sink.append(source);
                    eprintln!("[audio] sink appendé — {} échantillons en vol", n);
                } else {
                    eprintln!(
                        "[audio] SINK ABSENT — échantillons perdus (sink consommé ou absent)"
                    );
                }
            }
            alive.store(false, std::sync::atomic::Ordering::SeqCst);
        });
    }

    /// Enfile un chunk audio (f32, 24kHz mono).
    pub fn enqueue(&self, samples: Vec<f32>) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(samples);
        // Tous les consommateurs se réveillent ; seuls ceux dont la génération
        // est à jour puisent effectivement (les autres sortent immédiatement).
        self.condvar.notify_all();
    }

    /// Vide la file sans toucher au sink (arrêt de la génération).
    pub fn clear_queue(&self) {
        self.pending.lock().unwrap().clear();
    }

    /// Nombre de segments en attente (non encore joués).
    pub fn queue_len(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// File partagée (Send + Sync) — le pipeline y pousse les échantillons ; zéro périphérique requis côté écrivain.
    pub fn shared_queue(&self) -> Arc<Mutex<SegmentQueue>> {
        self.pending.clone()
    }

    /// Condvar de la file partagée — à `notify_one()` après chaque `push`.
    pub fn shared_condvar(&self) -> Arc<Condvar> {
        self.condvar.clone()
    }

    pub fn pause(&self) {
        *self.paused.lock().unwrap() = true;
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.pause();
        }
    }

    pub fn resume(&self) {
        *self.paused.lock().unwrap() = false;
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.play();
        }
    }

    /// Arrête la lecture : invalide le consommateur courant (génération), vide la file,
    /// coupe le sink — la position est abandonnée volontairement (arrêt ≠ pause).
    pub fn stop(&self) {
        *self.consumer_generation.lock().unwrap() += 1;
        *self.paused.lock().unwrap() = false;
        self.clear_queue();
        self.condvar.notify_all();
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
    }

    /// Prépare une nouvelle session de lecture : invalide le consommateur courant,
    /// nouveau sink + consommateur spawné, file vidée.
    pub fn reset(&self) -> Result<(), rodio::PlayError> {
        *self.paused.lock().unwrap() = false;
        self.clear_queue();
        *self.sink.lock().unwrap() = Some(Sink::try_new(&self.handle)?);
        self.spawn_consumer();
        Ok(())
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            sink.set_volume(volume.clamp(0.0, 1.0));
        }
    }

    pub fn is_playing(&self) -> bool {
        if let Some(sink) = self.sink.lock().unwrap().as_ref() {
            !sink.is_paused() && !sink.empty()
        } else {
            false
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Joue la signature sonore de démarrage (« Hey listen » — voix féminine aiguë, original).
///
/// Non-bloquant : un thread dédié joue la WAV intégralement.
/// Ne panique jamais : toute erreur de périphérique/fichier/format est journalisée et tolérée.
pub fn play_signature_once() {
    let path = crate::utils::resolve_signature_wav();
    std::thread::spawn(move || {
        let file = match std::fs::File::open(&path) {
            Ok(f) => f,
            Err(err) => {
                eprintln!(
                    "[navi] signature introuvable ({}) : {}",
                    path.display(),
                    err
                );
                return;
            }
        };
        let stream = match OutputStream::try_default() {
            Ok(s) => s,
            Err(err) => {
                eprintln!(
                    "[navi] périphérique audio indisponible pour la signature : {}",
                    err
                );
                return;
            }
        };
        let sink = match Sink::try_new(&stream.1) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("[navi] initialisation sink signature impossible : {}", err);
                return;
            }
        };
        let source = match Decoder::new(std::io::BufReader::new(file)) {
            Ok(s) => s,
            Err(err) => {
                eprintln!("[navi] décodage signature impossible : {}", err);
                return;
            }
        };
        sink.append(source);
        sink.sleep_until_end();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_fifo_ordre_respecte() {
        let mut q = SegmentQueue::new();
        q.push(vec![1.0]);
        q.push(vec![2.0]);
        q.push(vec![3.0]);
        assert_eq!(q.pop(), Some(vec![1.0]));
        assert_eq!(q.pop(), Some(vec![2.0]));
        assert_eq!(q.pop(), Some(vec![3.0]));
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn file_clear_vide_la_file() {
        let mut q = SegmentQueue::new();
        q.push(vec![1.0]);
        q.push(vec![2.0]);
        assert_eq!(q.len(), 2);
        q.clear();
        assert!(q.is_empty());
        assert_eq!(q.pop(), None);
    }

    #[test]
    fn file_stressee_1000_segments_fifo() {
        let mut q = SegmentQueue::new();
        for i in 0..1000u32 {
            q.push(vec![i as f32]);
        }
        assert_eq!(q.len(), 1000);
        for i in 0..1000u32 {
            assert_eq!(q.pop(), Some(vec![i as f32]));
        }
        assert!(q.is_empty());
    }

    #[test]
    fn file_arret_pendant_debit_en_cours() {
        // Stop en plein milieu d'un débit de génération : les segments
        // déjà consommés son partis, les restants sont vidés.
        let mut q = SegmentQueue::new();
        for i in 0..10u32 {
            q.push(vec![i as f32]);
        }
        assert_eq!(q.pop(), Some(vec![0.0]));
        assert_eq!(q.pop(), Some(vec![1.0]));
        q.clear(); // arrêt
        assert_eq!(q.len(), 0);
        assert_eq!(q.pop(), None);
    }
}
