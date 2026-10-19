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

    pub fn is_recording(&self) -> bool {
        self.is_recording.load(Ordering::Relaxed)
    }

    pub fn get_rms(&self) -> f32 {
        f32::from_bits(self.current_rms.load(Ordering::Relaxed))
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }
}
