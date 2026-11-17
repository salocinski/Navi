//! Lecture audio (rodio)

use rodio::{OutputStream, Sink, buffer::SamplesBuffer};
use std::sync::{Arc, Mutex, Condvar};
use std::thread;
use std::time::Duration;

/// Gestionnaire de lecture audio avec file (queue)
pub struct AudioPlayer {
    _stream: OutputStream,
    handle: rodio::OutputStreamHandle,
    sink: Arc<Mutex<Option<Sink>>>,
    pending: Arc<Mutex<Vec<Vec<f32>>>>,
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
            pending: Arc::new(Mutex::new(Vec::new())),
            condvar: Arc::new(Condvar::new()),
            stopped: Arc::new(Mutex::new(false)),
            paused: Arc::new(Mutex::new(false)),
        })
    }
    
    /// Enfile un chunk audio (f32, 24kHz mono)
    pub fn enqueue(&self, samples: Vec<f32>) {
        let mut pending = self.pending.lock().unwrap();
        pending.push(samples);
        self.condvar.notify_one();
    }
    
    /// Démarre le consommateur audio (thread dédié)
    pub fn start_consumer(&self) {
        let sink = self.sink.clone();
        let pending = self.pending.clone();
        let condvar = self.condvar.clone();
        let stopped = self.stopped.clone();
        let paused = self.paused.clone();
        
        thread::spawn(move || {
            loop {
                let samples = {
                    let mut pending = pending.lock().unwrap();
                    while pending.is_empty() {
                        let stopped = *stopped.lock().unwrap();
                        if stopped { return; }
                        pending = condvar.wait(pending).unwrap();
                    }
                    pending.pop()
                };
                
                if let Some(samples) = samples {
                    // Attendre si en pause
                    while *paused.lock().unwrap() {
                        thread::sleep(Duration::from_millis(50));
                    }
                    
                    let sink_lock = sink.lock().unwrap();
                    if let Some(sink) = sink_lock.as_ref() {
                        let source = SamplesBuffer::new(1, 24000, samples);
                        sink.append(source);
                    }
                }
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
    
    pub fn stop(&self) {
        *self.stopped.lock().unwrap() = true;
        *self.paused.lock().unwrap() = false;
        self.condvar.notify_all();
        if let Some(sink) = self.sink.lock().unwrap().take() {
            sink.stop();
        }
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

impl Default for AudioPlayer {
    fn default() -> Self {
        Self::new().expect("Failed to create AudioPlayer")
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop();
    }
}