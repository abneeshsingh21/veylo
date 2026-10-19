use rodio::{Decoder, OutputStreamHandle, Sink};
use std::io::Cursor;
use std::sync::Arc;

// Embedded audio cues extracted from Wispr Flow
const SOUND_START: &[u8] = include_bytes!("../assets/sounds/dictation-start.wav");
const SOUND_STOP: &[u8] = include_bytes!("../assets/sounds/dictation-stop.wav");
const SOUND_PASTE: &[u8] = include_bytes!("../assets/sounds/paste.wav");

pub struct SoundEngine {
    stream_handle: Option<OutputStreamHandle>,
}

impl SoundEngine {
    pub fn new(handle: Option<OutputStreamHandle>) -> Arc<Self> {
        Arc::new(Self {
            stream_handle: handle,
        })
    }

    pub fn play_start(&self) {
        self.play_wav(SOUND_START, 0.4);
    }

    pub fn play_stop(&self) {
        self.play_wav(SOUND_STOP, 0.35);
    }

    pub fn play_paste(&self) {
        self.play_wav(SOUND_PASTE, 0.4);
    }

    fn play_wav(&self, data: &'static [u8], volume: f32) {
        if let Some(ref handle) = self.stream_handle {
            if let Ok(sink) = Sink::try_new(handle) {
                sink.set_volume(volume);
                let cursor = Cursor::new(data);
                if let Ok(source) = Decoder::new(cursor) {
                    sink.append(source);
                    sink.detach();
                }
            }
        }
    }
}
