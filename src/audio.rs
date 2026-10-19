use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Stream, StreamConfig};
use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;

pub struct AudioRecorder {
    _stream: Option<Stream>,
    is_recording: Arc<AtomicBool>,
    buffer: Arc<Mutex<Vec<f32>>>,
    current_rms: Arc<AtomicU32>, // Store float as bits for lock-free atomic reads
    sample_rate: u32,
}

unsafe impl Send for AudioRecorder {}
unsafe impl Sync for AudioRecorder {}


impl AudioRecorder {
    pub fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .ok_or_else(|| "No default microphone input device found".to_string())?;

        let device_name = device.name().unwrap_or_else(|_| "Unknown Device".to_string());
        println!("[AudioRecorder] Using microphone: {}", device_name);

        let default_config = device
            .default_input_config()
            .map_err(|e| format!("Failed to get default input config: {}", e))?;

        let sample_rate = default_config.sample_rate().0;
        let channels = default_config.channels();
        println!(
            "[AudioRecorder] Default format: {} Hz, {} channels, {:?}",
            sample_rate,
            channels,
            default_config.sample_format()
        );

        let config: StreamConfig = default_config.into();
        let is_recording = Arc::new(AtomicBool::new(false));
        let buffer = Arc::new(Mutex::new(Vec::with_capacity(sample_rate as usize * 10)));
        let current_rms = Arc::new(AtomicU32::new(0));

        let is_rec_clone = is_recording.clone();
        let buffer_clone = buffer.clone();
        let rms_clone = current_rms.clone();

        let err_fn = |err| eprintln!("[AudioRecorder] Audio stream error: {}", err);

        let stream = device
            .build_input_stream(
                &config,
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if data.is_empty() {
                        return;
                    }

                    // Compute RMS for real-time waveform visualization
                    let mut sum = 0.0f32;
                    let num_samples = data.len() as f32;
                    for &sample in data {
                        sum += sample * sample;
                    }
                    let rms = (sum / num_samples).sqrt();
                    // Smooth amplify for visual aesthetic (scale 0.0 to 1.0)
                    let visual_rms = (rms * 8.0).min(1.0);
                    rms_clone.store(visual_rms.to_bits(), Ordering::Relaxed);

                    // If recording is active, append mono samples
                    if is_rec_clone.load(Ordering::Acquire) {
                        let mut buf = buffer_clone.lock();
                        if channels == 1 {
                            buf.extend_from_slice(data);
                        } else {
                            // Downmix to mono
                            for chunk in data.chunks(channels as usize) {
                                let mono: f32 = chunk.iter().sum::<f32>() / (channels as f32);
                                buf.push(mono);
                            }
                        }
                    }
                },
                err_fn,
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        stream
            .play()
            .map_err(|e| format!("Failed to start input stream: {}", e))?;

        Ok(Self {
            _stream: Some(stream),
            is_recording,
            buffer,
            current_rms,
            sample_rate,
        })
    }

    pub fn start_recording(&self) {
        let mut buf = self.buffer.lock();
        buf.clear();
        self.is_recording.store(true, Ordering::Release);
        println!("[AudioRecorder] Recording started...");
    }

    pub fn stop_recording(&self) -> (Vec<f32>, u32) {
        self.is_recording.store(false, Ordering::Release);
        self.current_rms.store(0, Ordering::Relaxed);
        let mut buf = self.buffer.lock();
        let samples = std::mem::take(&mut *buf);
        println!(
            "[AudioRecorder] Recording stopped. Captured {} samples ({:.2}s)",
            samples.len(),
            samples.len() as f32 / self.sample_rate as f32
        );
        (samples, self.sample_rate)
    }

    #[allow(dead_code)]
    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_rms(&self) -> f32 {
        f32::from_bits(self.current_rms.load(Ordering::Relaxed))
    }

    #[allow(dead_code)]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Resamples recorded float samples to 16,000 Hz 16-bit mono PCM and wraps them in a 44-byte RIFF/WAV container.
    pub fn samples_to_16k_wav(samples: &[f32], source_sample_rate: u32) -> Vec<u8> {
        if samples.is_empty() || source_sample_rate == 0 {
            return Vec::new();
        }

        const TARGET_SAMPLE_RATE: u32 = 16000;
        let num_output_samples = ((samples.len() as u64 * TARGET_SAMPLE_RATE as u64)
            / (source_sample_rate as u64)) as usize;

        let mut pcm_bytes = Vec::with_capacity(num_output_samples * 2);
        let ratio = source_sample_rate as f64 / TARGET_SAMPLE_RATE as f64;

        for i in 0..num_output_samples {
            let src_idx = i as f64 * ratio;
            let idx0 = src_idx.floor() as usize;
            let idx1 = (idx0 + 1).min(samples.len() - 1);
            let frac = (src_idx - idx0 as f64) as f32;

            let sample = samples[idx0] * (1.0 - frac) + samples[idx1] * frac;
            let clamped = sample.clamp(-1.0, 1.0);
            let val = (clamped * 32767.0) as i16;
            pcm_bytes.extend_from_slice(&val.to_le_bytes());
        }

        let data_len = pcm_bytes.len() as u32;
        let mut wav = Vec::with_capacity(44 + pcm_bytes.len());

        // RIFF chunk descriptor
        wav.extend_from_slice(b"RIFF");
        let file_size = 36 + data_len;
        wav.extend_from_slice(&file_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");

        // "fmt " subchunk
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes()); // Subchunk1Size (16 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes());  // AudioFormat (1 for PCM)
        wav.extend_from_slice(&1u16.to_le_bytes());  // NumChannels (1 mono)
        wav.extend_from_slice(&TARGET_SAMPLE_RATE.to_le_bytes()); // SampleRate (16000)
        let byte_rate = TARGET_SAMPLE_RATE * 1 * 2; // SampleRate * NumChannels * BitsPerSample/8
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        let block_align = 1u16 * 2; // NumChannels * BitsPerSample/8
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&16u16.to_le_bytes()); // BitsPerSample (16)

        // "data" subchunk
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&data_len.to_le_bytes());
        wav.extend_from_slice(&pcm_bytes);

        wav
    }

    /// Stops recording and directly produces a 16kHz WAV byte vector and duration in seconds.
    pub fn stop_recording_as_wav(&self) -> (Vec<u8>, f32) {
        let (samples, sr) = self.stop_recording();
        let duration = samples.len() as f32 / sr as f32;
        let wav = Self::samples_to_16k_wav(&samples, sr);
        (wav, duration)
    }
}
