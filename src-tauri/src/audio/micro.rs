//! Capture du micro pour la dictée (carte N-088) : f32 mono 16 kHz, format attendu par Whisper.
//!
//! `cpal::Stream` n'est pas `Send` : le flux vit dans un thread dédié, piloté par un canal d'arrêt.
//! Rien n'est écrit sur disque : les échantillons restent en mémoire jusqu'à `stop()`.

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

/// Fréquence d'échantillonnage attendue par Whisper.
pub const TARGET_RATE: u32 = 16_000;
/// Durée maximale d'un enregistrement (appui maintenu plafonné à 60 s, spike N-087).
pub const MAX_SECONDS: u32 = 60;

/// Noms des périphériques d'entrée disponibles (pour le choix dans les réglages).
pub fn list_input_devices() -> Vec<String> {
    cpal::default_host()
        .input_devices()
        .map(|it| it.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}

fn find_device(name: Option<&str>) -> Result<cpal::Device, String> {
    let host = cpal::default_host();
    match name {
        Some(n) => host
            .input_devices()
            .map_err(|e| format!("Périphériques d'entrée illisibles : {e}"))?
            .find(|d| d.name().is_ok_and(|dn| dn == n))
            .ok_or_else(|| format!("Micro introuvable : « {n} »")),
        None => host
            .default_input_device()
            .ok_or_else(|| "Aucun micro par défaut".to_string()),
    }
}

/// Enregistrement en cours ; `stop()` rend les échantillons f32 mono 16 kHz.
pub struct MicCapture {
    stop_tx: Sender<()>,
    handle: JoinHandle<()>,
    samples: Arc<Mutex<Vec<f32>>>,
    rate: u32,
}

impl MicCapture {
    /// Démarre la capture sur le micro nommé (`None` = défaut système).
    /// Échoue si le périphérique est absent ou l'accès au micro refusé.
    pub fn start(device: Option<&str>) -> Result<Self, String> {
        let device = find_device(device)?;
        let config = device
            .default_input_config()
            .map_err(|e| format!("Configuration micro indisponible : {e}"))?;
        let (rate, channels, format) = (
            config.sample_rate().0,
            config.channels() as usize,
            config.sample_format(),
        );
        let samples = Arc::new(Mutex::new(Vec::new()));
        // Plafond en échantillons mono natifs : évite une croissance mémoire sans borne.
        let cap = rate as usize * MAX_SECONDS as usize;
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let (ready_tx, ready_rx) = mpsc::channel::<Result<(), String>>();
        let buf = samples.clone();
        let handle = std::thread::spawn(move || {
            let stream = match format {
                SampleFormat::F32 => build::<f32>(&device, &config.into(), channels, cap, buf),
                SampleFormat::I16 => build::<i16>(&device, &config.into(), channels, cap, buf),
                SampleFormat::U16 => build::<u16>(&device, &config.into(), channels, cap, buf),
                f => Err(format!("Format d'échantillon non géré : {f:?}")),
            }
            .and_then(|s| {
                s.play()
                    .map(|_| s)
                    .map_err(|e| format!("Démarrage du micro : {e}"))
            });
            match stream {
                Ok(s) => {
                    let _ = ready_tx.send(Ok(()));
                    let _ = stop_rx.recv(); // bloque jusqu'à stop() ou abandon de l'émetteur
                    drop(s);
                }
                Err(e) => {
                    let _ = ready_tx.send(Err(e));
                }
            }
        });
        ready_rx
            .recv()
            .map_err(|_| "Thread micro interrompu".to_string())??;
        Ok(Self {
            stop_tx,
            handle,
            samples,
            rate,
        })
    }

    /// N-107 : copie de l'audio déjà capturé (mono 16 kHz), utilisable depuis un autre thread
    /// pendant l'enregistrement (transcription en temps réel).
    pub fn snapshotter(&self) -> impl Fn() -> Vec<f32> + Send + 'static {
        let (samples, rate) = (self.samples.clone(), self.rate);
        move || {
            let native = samples.lock().unwrap_or_else(|e| e.into_inner()).clone();
            resample_mono(&native, rate, TARGET_RATE)
        }
    }

    /// Arrête la capture et rend l'audio converti en mono 16 kHz.
    pub fn stop(self) -> Vec<f32> {
        let _ = self.stop_tx.send(());
        let _ = self.handle.join();
        let native = std::mem::take(&mut *self.samples.lock().unwrap_or_else(|e| e.into_inner()));
        resample_mono(&native, self.rate, TARGET_RATE)
    }
}

fn build<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    channels: usize,
    cap: usize,
    buf: Arc<Mutex<Vec<f32>>>,
) -> Result<cpal::Stream, String>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mut b = buf.lock().unwrap_or_else(|e| e.into_inner());
                downmix_into(data, channels, cap, &mut b);
            },
            |e| eprintln!("[micro] erreur de flux : {e}"),
            None,
        )
        .map_err(|e| format!("Ouverture du micro refusée ou impossible : {e}"))
}

/// Moyenne les canaux de chaque trame et ajoute le mono à `out`, sans dépasser `cap`.
fn downmix_into<T: SizedSample>(data: &[T], channels: usize, cap: usize, out: &mut Vec<f32>)
where
    f32: FromSample<T>,
{
    for frame in data.chunks_exact(channels.max(1)) {
        if out.len() >= cap {
            return;
        }
        let sum: f32 = frame.iter().map(|&s| f32::from_sample_(s)).sum();
        out.push(sum / frame.len() as f32);
    }
}

/// Rééchantillonne du mono `from` Hz vers `to` Hz par moyenne des échantillons couverts
/// (filtre passe-bas rudimentaire).
/// shortcut: moyenne par fenêtre au lieu d'un filtre polyphasé, suffisante pour la parole ;
/// passer à `rubato` si la qualité de transcription l'exige.
pub fn resample_mono(input: &[f32], from: u32, to: u32) -> Vec<f32> {
    if from == to || input.is_empty() {
        return input.to_vec();
    }
    let ratio = from as f64 / to as f64;
    let n_out = (input.len() as f64 / ratio).floor() as usize;
    (0..n_out)
        .map(|i| {
            let start = (i as f64 * ratio) as usize;
            let end = (((i + 1) as f64 * ratio) as usize).clamp(start + 1, input.len());
            input[start..end].iter().sum::<f32>() / (end - start) as f32
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resample_identique_ne_change_rien() {
        let v = vec![0.1, -0.2, 0.3];
        assert_eq!(resample_mono(&v, 16_000, 16_000), v);
    }

    #[test]
    fn resample_48k_vers_16k_divise_par_trois() {
        let v: Vec<f32> = (0..4800).map(|i| (i % 3) as f32).collect();
        let out = resample_mono(&v, 48_000, 16_000);
        assert_eq!(out.len(), 1600);
        assert!(out.iter().all(|&s| (s - 1.0).abs() < 1e-6)); // moyenne de 0,1,2
    }

    #[test]
    fn resample_44k1_longueur_proportionnelle() {
        let out = resample_mono(&vec![0.5; 44_100], 44_100, 16_000);
        assert_eq!(out.len(), 16_000);
    }

    /// Matériel requis : `cargo test micro_reel -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn micro_reel_capture_2_secondes() {
        let cap = MicCapture::start(None).expect("micro par défaut");
        std::thread::sleep(std::time::Duration::from_secs(2));
        let pcm = cap.stop();
        let rms = (pcm.iter().map(|s| s * s).sum::<f32>() / pcm.len() as f32).sqrt();
        println!(
            "{} échantillons (~{:.1} s), RMS {rms:.4}",
            pcm.len(),
            pcm.len() as f32 / 16_000.0
        );
        assert!((28_000..36_000).contains(&pcm.len()));
    }

    #[test]
    fn resample_vide() {
        assert!(resample_mono(&[], 48_000, 16_000).is_empty());
    }

    #[test]
    fn downmix_stereo_moyenne_les_canaux() {
        let mut out = Vec::new();
        downmix_into::<f32>(&[1.0, 0.0, 0.5, 0.5], 2, 100, &mut out);
        assert_eq!(out, vec![0.5, 0.5]);
    }

    #[test]
    fn downmix_i16_normalise() {
        let mut out = Vec::new();
        downmix_into::<i16>(&[i16::MAX, i16::MIN], 1, 100, &mut out);
        assert!((out[0] - 1.0).abs() < 1e-3 && (out[1] + 1.0).abs() < 1e-3);
    }

    #[test]
    fn downmix_respecte_le_plafond() {
        let mut out = Vec::new();
        downmix_into::<f32>(&[0.1; 20], 1, 5, &mut out);
        assert_eq!(out.len(), 5);
    }
}
