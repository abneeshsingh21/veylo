mod audio;
mod client;
mod context;
mod hotkey;
mod injector;
mod sounds;
mod ui;

use audio::AudioRecorder;
use context::ActiveContext;
use hotkey::{HotkeyAction, HotkeyListener};
use sounds::SoundEngine;
use ui::{FloatingPill, PillMode};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG,
};

fn main() {
    // Ignore stdout broken pipe errors if running detached from console
    std::panic::set_hook(Box::new(|info| {
        let msg = info.to_string();
        if !msg.contains("failed printing to stdout") && !msg.contains("os error 6") {
            eprintln!("[Veylo Panic] {}", info);
        }
    }));

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
            eprintln!("[Main] Exiting: microphone is required for voice intelligence.");
            return;
        }
    };

    // 3. Initialize Windows 11 Liquid Glass Floating Pill
    let pill = match FloatingPill::new() {
        Ok(p) => Arc::new(p),
        Err(e) => {
            eprintln!("[Main] Liquid Glass UI initialization failed: {}", e);
            return;
        }
    };
    println!("[Main] Liquid Glass Floating Pill window initialized.");

    // 4. Asynchronously Ensure Local Inference Engine (Whisper + Qwen SLM) is Running
    thread::spawn(|| {
        client::ensure_engine_running();
    });

    // 5. Start 60 FPS Waveform Animation Pump (runs whenever pill is visible)
    let audio_for_anim = audio_recorder.clone();
    let pill_for_anim = pill.clone();
    let is_running = Arc::new(AtomicBool::new(true));
    let running_clone = is_running.clone();

    thread::spawn(move || {
        while running_clone.load(Ordering::Relaxed) {
            if pill_for_anim.is_visible() {
                let rms = audio_for_anim.get_rms();
                pill_for_anim.update_rms(rms);
            }
            thread::sleep(Duration::from_millis(16)); // ~60 FPS
        }
    });

    // 6. Initialize Global Low-Level Keyboard Hook
    let hotkey_listener = HotkeyListener::new();

    // 7. Spawn Event Processor Thread
    let hotkey_rx = hotkey_listener.receiver.clone();
    let audio_for_hotkey = audio_recorder.clone();
    let sounds_for_hotkey = sound_engine.clone();
    let pill_for_hotkey = pill.clone();

    thread::spawn(move || {
        while let Ok(action) = hotkey_rx.recv() {
            match action {
                HotkeyAction::Start => {
                    println!("\n>>> [HOTKEY: START RECORDING]");
                    sounds_for_hotkey.play_start();
                    pill_for_hotkey.show();
                    audio_for_hotkey.start_recording();
                }
                HotkeyAction::Stop => {
                    println!("<<< [HOTKEY: STOP RECORDING]");
                    sounds_for_hotkey.play_stop();

                    // Snapshot the active application context before any window changes
                    let active_ctx = ActiveContext::capture();
                    let (wav_bytes, duration) = audio_for_hotkey.stop_recording_as_wav();
                    println!(
                        "[Pipeline] Captured {:.2}s of audio ({} bytes WAV). Context: \"{}\" [Code: {}, Email: {}, Chat: {}]",
                        duration,
                        wav_bytes.len(),
                        active_ctx.window_title,
                        active_ctx.is_code_editor,
                        active_ctx.is_email_app,
                        active_ctx.is_chat_app
                    );

                    // Ignore accidental micro-clicks (< 350ms)
                    if duration < 0.35 || wav_bytes.is_empty() {
                        println!("[Pipeline] Audio clip too short (< 350ms), ignoring.");
                        pill_for_hotkey.hide();
                        continue;
                    }

                    // Transition pill to Processing state (Section 8.1 shimmer wave)
                    pill_for_hotkey.set_mode(PillMode::Processing);

                    let pill_worker = pill_for_hotkey.clone();
                    let sounds_worker = sounds_for_hotkey.clone();

                    thread::spawn(move || {
                        let t_start = std::time::Instant::now();
                        match client::transcribe(&wav_bytes, &active_ctx) {
                            Ok(resp) => {
                                let total_ms = t_start.elapsed().as_millis();
                                println!(
                                    "[Veylo AI] Raw: \"{}\" -> Polished: \"{}\" (Engine: {:?}ms, Total: {}ms)",
                                    resp.raw.as_deref().unwrap_or(""),
                                    resp.text,
                                    resp.duration_ms.unwrap_or(0.0),
                                    total_ms
                                );

                                if !resp.text.is_empty() {
                                    // Play tactile paste chime & inject into caret
                                    sounds_worker.play_paste();
                                    injector::inject_text(&resp.text);
                                }

                                // Section 8.1: brief 150 ms brighten of the rim, then collapse to idle
                                pill_worker.set_mode(PillMode::Done);
                                thread::sleep(Duration::from_millis(150));
                                pill_worker.hide();
                            }
                            Err(err) => {
                                eprintln!("[Veylo AI Error] {}", err);
                                // Section 8.1: rim tints warm red #FF6B5E on error
                                pill_worker.set_mode(PillMode::Error);
                                thread::sleep(Duration::from_millis(1200));
                                pill_worker.hide();
                            }
                        }
                    });
                }
            }
        }
    });

    println!("\n[Ready] Veylo running and active!");
    println!("[Shortcut] HOLD: 'Ctrl + Win' OR 'Ctrl + Space' OR 'Right Ctrl'");
    println!("[Shortcut] OR PRESS 'F8' to toggle recording on/off.\n");

    // Welcome flash: show the Liquid Glass Pill on screen for 1.2 seconds
    sound_engine.play_start();
    let pill_welcome = pill.clone();
    thread::spawn(move || {
        pill_welcome.show();
        thread::sleep(Duration::from_millis(1200));
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
