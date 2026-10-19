mod audio;
mod context;
mod hotkey;
mod injector;
mod sounds;
mod ui;

use audio::AudioRecorder;
use hotkey::{HotkeyAction, HotkeyListener};
use sounds::SoundEngine;
use ui::FloatingPill;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

fn main() {
    println!("======================================================");
    println!("             VEYLO (Native Rust Voice AI)             ");
    println!("   100% Free - Offline - Ultra-Lightweight - Private  ");
    println!("======================================================");

    // 1. Initialize Tactile Sound Cue Engine (extracted Wispr Flow chimes)
    let (_output_stream, stream_handle) = match rodio::OutputStream::try_default() {
        Ok((s, h)) => (Some(s), Some(h)),
        Err(e) => {
            eprintln!("[SoundEngine] Warning: Audio output device not found: {}", e);
            (None, None)
        }
    };
    let sound_engine = SoundEngine::new(stream_handle);
    println!("[Main] Tactile sound engine loaded.");


    // 2. Initialize Low-Latency Microphone Audio Recorder
    let audio_recorder = match AudioRecorder::new() {
        Ok(rec) => Arc::new(rec),
        Err(e) => {
            eprintln!("[Main] Audio initialisation failed: {}", e);
            eprintln!("[Main] Continuing without microphone for UI & hotkey testing...");
            // Create dummy or exit
            return;
        }
    };

    // 3. Initialize Windows 11 Liquid Glass (Acrylic/Mica) Floating Pill
    let pill = match FloatingPill::new() {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("[Main] Liquid Glass UI initialization failed: {}", e);
            return;
        }
    };
    println!("[Main] Liquid Glass Floating Pill window initialized.");

    // 4. Start 60 FPS Waveform Animation Pump (reads real-time audio RMS)
    let audio_for_anim = audio_recorder.clone();
    let pill_for_anim = pill.clone();
    let is_running = Arc::new(AtomicBool::new(true));
    let running_clone = is_running.clone();

    thread::spawn(move || {
        while running_clone.load(Ordering::Relaxed) {
            if audio_for_anim.is_recording() {
                let rms = audio_for_anim.get_rms();
                pill_for_anim.update_rms(rms);
            }
            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }
    });

    // 5. Initialize Global Low-Level Keyboard Hook
    let hotkey_listener = HotkeyListener::new();

    // 6. Spawn Event Processor Thread
    let hotkey_rx = hotkey_listener.receiver.clone();
    let audio_for_hotkey = audio_recorder.clone();
    let sounds_for_hotkey = sound_engine.clone();
    let pill_for_hotkey = pill.clone();

    thread::spawn(move || {
        while let Ok(action) = hotkey_rx.recv() {
            match action {
                HotkeyAction::Start => {
                    println!("\n>>> [HOTKEY DETECTED: START]");
                    // Immediate audio tactile feedback (Wispr Flow start chime)
                    sounds_for_hotkey.play_start();
                    // Show Liquid Glass floating pill without stealing focus
                    pill_for_hotkey.show();
                    // Start microphone stream buffer
                    audio_for_hotkey.start_recording();
                }
                HotkeyAction::Stop => {
                    println!("<<< [HOTKEY DETECTED: STOP]");
                    // Hide Floating Pill
                    pill_for_hotkey.hide();
                    // Play stop sound
                    sounds_for_hotkey.play_stop();
                    // Capture recorded audio samples
                    let (samples, sr) = audio_for_hotkey.stop_recording();
                    let duration = samples.len() as f32 / sr as f32;
                    println!(
                        "[Pipeline] Captured {:.2} seconds of voice data ({} samples).",
                        duration,
                        samples.len()
                    );

                    if duration >= 0.3 {
                        // Simulate post-transcription paste feedback
                        thread::sleep(Duration::from_millis(120));
                        sounds_for_hotkey.play_paste();
                        println!("[Pipeline] Ready for Local Whisper + Qwen SLM transcription pass!");
                    } else {
                        println!("[Pipeline] Audio clip too short, ignoring.");
                    }
                }
            }
        }
    });

    println!("\n[Ready] Application running in background!");
    println!("[Shortcut] HOLD: 'Ctrl + Win' OR 'Ctrl + Space' OR 'Right Ctrl'");
    println!("[Shortcut] OR PRESS 'F8' to toggle recording on/off.\n");

    // Play startup chime and flash the Liquid Glass Pill on screen for 1.5 seconds
    sound_engine.play_start();
    let pill_welcome = pill.clone();
    thread::spawn(move || {
        pill_welcome.show();
        thread::sleep(Duration::from_millis(1500));
        pill_welcome.hide();
    });

    // Main Win32 event pump
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }



    is_running.store(false, Ordering::Relaxed);
}
