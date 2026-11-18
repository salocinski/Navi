//! Lecture audio (rodio)

use rodio::{buffer::SamplesBuffer, OutputStream, OutputStreamHandle, Sink};
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

/// Gestionnaire de lecture audio avec file (queue).
///
/// Cycle de vie : `new` → `start_consumer` → `enqueue…` → `pause`/`resume`/`stop`.
/// Après `stop`, la file est vidée et le sink est consommé ; appeler `reset`
/// avant une nouvelle session de lecture.
pub struct AudioPlayer {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
    pending: Arc<Mutex<SegmentQueue>>,
    condvar: Arc<Condvar>,
    stopped: Arc<Mutex<bool>>,
    paused: Arc<Mutex<bool>>,
}

impl AudioPlayer {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let (stream, handle) = OutputStream::try_default()?;
        let sink = Sink::try_new(&handle)?;

        Ok(Self {
            _stream: stream,
            handle,
            sink: Arc::new(Mutex::new(Some(sink))),
            pending: Arc::new(Mutex::new(SegmentQueue::new())),
            condvar: Arc::new(Condvar::new()),
            stopped: Arc::new(Mutex::new(false)),
            paused: Arc::new(Mutex::new(false)),
        })
    }

    /// Enfile un chunk audio (f32, 24kHz mono).
    pub fn enqueue(&self, samples: Vec<f32>) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(samples);
        self.condvar.notify_one();
    }

    /// Vide la file sans toucher au sink (arrêt de la génération).
    pub fn clear_queue(&self) {
        self.pending.lock().unwrap().clear();
    }

    /// Nombre de segments en attente (non encore joués).
    pub fn queue_len(&self) -> usize {
        self.pending.lock().unwrap().len()
    }

    /// Démarre le consommateur audio (thread dédié).
    pub fn start_consumer(&self) {
        let sink = self.sink.clone();
        let pending = self.pending.clone();
        let condvar = self.condvar.clone();
        let stopped = self.stopped.clone();
        let paused = self.paused.clone();

        thread::spawn(move || loop {
            let samples = {
                let mut pending = pending.lock().unwrap();
                loop {
                    if let Some(samples) = pending.pop() {
                        break Some(samples);
                    }
                    if *stopped.lock().unwrap() {
                        break None;
                    }
                    pending = condvar.wait(pending).unwrap();
                }
            };

            let Some(samples) = samples else { return };

            // Ne pas jouer si l'arrêt est survenu entre l'attente et ici.
            if *stopped.lock().unwrap() {
                return;
            }

            // Attendre la reprise si en pause.
            while *paused.lock().unwrap() {
                if *stopped.lock().unwrap() {
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }

            let sink_lock = sink.lock().unwrap();
            if let Some(sink) = sink_lock.as_ref() {
                let source = SamplesBuffer::new(1, SAMPLE_RATE, samples);
                sink.append(source);
            }
        });
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

    /// Arrête la lecture : vide la file, signale l'arrêt (le consommateur
    /// sort de sa boucle) et coupe le sink courant.
    pub fn stop(&self) {
        *self.stopped.lock().unwrap() = true;
        *self.paused.lock().unwrap() = false;
        self.clear_queue();
        self.condvar.notify_all();
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
    }

    /// Prépare une nouvelle session de lecture après un `stop`/`clear` :
    /// nouveau sink, file vidée, drapeaux réinitialisés.
    pub fn reset(&self) -> Result<(), rodio::PlayError> {
        *self.stopped.lock().unwrap() = false;
        *self.paused.lock().unwrap() = false;
        self.clear_queue();
        *self.sink.lock().unwrap() = Some(Sink::try_new(&self.handle)?);
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
}
