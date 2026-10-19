mod audio;
mod client;
mod context;
mod hotkey;
mod hub;
mod injector;
mod sounds;
mod tray;
mod ui;

use audio::AudioRecorder;
use context::ActiveContext;
use hotkey::{HotkeyAction, HotkeyListener};
use sounds::SoundEngine;
use tray::TrayIcon;
use ui::{FloatingPill, PillMode};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use windows::core::w;
use windows::Win32::Foundation::GetLastError;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, MOD_ALT, MOD_CONTROL, MOD_NOREPEAT, MOD_SHIFT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, TranslateMessage, MSG, WM_HOTKEY,
};

pub fn attach_to_default_desktop() {
    unsafe {
        use windows::Win32::System::StationsAndDesktops::{
            OpenDesktopW, SetThreadDesktop, DESKTOP_CONTROL_FLAGS,
        };
        // 0x10000000 is GENERIC_ALL
        if let Ok(hdesk) = OpenDesktopW(w!("Default"), DESKTOP_CONTROL_FLAGS(0), false, 0x10000000) {
            if !hdesk.0.is_null() {
                let _ = SetThreadDesktop(hdesk);
            }
        }
    }
}

fn main() {
    // 0. Ensure Single Instance
    let _app_mutex = unsafe {
        let handle = CreateMutexW(None, true, w!("Global\\VeyloVoiceAIInstanceMutex"));
        if GetLastError().0 == 183 {
            // ERROR_ALREADY_EXISTS: Notify existing instance to open Hub
            println!("[Veylo] Existing Veylo instance detected! Bringing Hub to front...");
            if let Ok(sock) = std::net::UdpSocket::bind("127.0.0.1:0") {
                let _ = sock.send_to(b"hub", "127.0.0.1:48124");
            }
            return;
        }
        handle
    };

    // Ensure thread is attached to physical interactive desktop station
    attach_to_default_desktop();

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

    // 7. Initialize Windows System Tray Icon
    let _tray = TrayIcon::new(hotkey_listener.sender.clone());

    // 8. Start Local UDP Control Channel Listener on 127.0.0.1:48124
    let sender_for_udp = hotkey_listener.sender.clone();
    thread::spawn(move || {
        if let Ok(socket) = std::net::UdpSocket::bind("127.0.0.1:48124") {
            let mut buf = [0u8; 64];
            while let Ok((n, _)) = socket.recv_from(&mut buf) {
                let cmd = String::from_utf8_lossy(&buf[..n]);
                if cmd.contains("toggle") {
                    println!("[Control Channel] Dictation toggle requested via API/Hub!");
                    let _ = sender_for_udp.send(HotkeyAction::Toggle);
                } else if cmd.contains("hub") {
                    println!("[Control Channel] Open Hub requested via API/CLI!");
                    hub::open_hub();
                }
            }
        }
    });

    // 9. Register Kernel-Level Global Hotkeys with OS
    unsafe {
        // Hotkey 101: F8 (VK_F8 = 0x77) -> Toggle Dictation
        let _ = RegisterHotKey(None, 101, MOD_NOREPEAT, 0x77);
        // Hotkey 102: F9 (VK_F9 = 0x78) -> Open Veylo Hub
        let _ = RegisterHotKey(None, 102, MOD_NOREPEAT, 0x78);
        // Hotkey 103: Ctrl + Alt + V (VK_V = 0x56) -> Toggle Dictation (Universal, zero conflicts)
        let _ = RegisterHotKey(None, 103, MOD_CONTROL | MOD_ALT | MOD_NOREPEAT, 0x56);
        // Hotkey 104: Ctrl + Shift + V -> Toggle Dictation
        let _ = RegisterHotKey(None, 104, MOD_CONTROL | MOD_SHIFT | MOD_NOREPEAT, 0x56);
        // Hotkey 105: Ctrl + Space (VK_SPACE = 0x20) -> Toggle Dictation
        let _ = RegisterHotKey(None, 105, MOD_CONTROL | MOD_NOREPEAT, 0x20);
        // Hotkey 106: Alt + Space (VK_SPACE = 0x20) -> Toggle Dictation
        let _ = RegisterHotKey(None, 106, MOD_ALT | MOD_NOREPEAT, 0x20);
        println!("[KernelHotkeys] System hotkeys active: F8, F9, Ctrl+Alt+V, Ctrl+Shift+V, Ctrl+Space, Alt+Space");
    }

    // 10. Spawn Main Event Processor with Debounce Guard
    let hotkey_rx = hotkey_listener.receiver.clone();
    let audio_for_hotkey = audio_recorder.clone();
    let sounds_for_hotkey = sound_engine.clone();
    let pill_for_hotkey = pill.clone();

    thread::spawn(move || {
        let mut last_action_time = std::time::Instant::now() - Duration::from_secs(5);

        while let Ok(action) = hotkey_rx.recv() {
            let is_recording = audio_for_hotkey.is_recording();
            let should_start = match action {
                HotkeyAction::Start => true,
                HotkeyAction::Stop => false,
                HotkeyAction::Toggle => !is_recording,
            };

            let elapsed = last_action_time.elapsed();

            if should_start {
                if is_recording {
                    continue;
                }
                // Debounce Guard: ignore duplicate rapid triggers (< 400ms)
                if elapsed < Duration::from_millis(400) {
                    println!("[EventProcessor] Debounce: Start ignored (elapsed: {:?})", elapsed);
                    continue;
                }

                last_action_time = std::time::Instant::now();
                println!("\n>>> [VOICE RECORDING STARTED]");
                sounds_for_hotkey.play_start();
                pill_for_hotkey.show();
                audio_for_hotkey.start_recording();
            } else {
                if !is_recording {
                    continue;
                }
                // Debounce Guard: prevent immediately stopping if start just fired (< 400ms)
                if elapsed < Duration::from_millis(400) {
                    println!("[EventProcessor] Debounce: Stop ignored (elapsed: {:?})", elapsed);
                    continue;
                }

                last_action_time = std::time::Instant::now();
                println!("<<< [VOICE RECORDING STOPPED]");
                sounds_for_hotkey.play_stop();

                let active_ctx = ActiveContext::capture();
                let (wav_bytes, duration) = audio_for_hotkey.stop_recording_as_wav();
                println!(
                    "[Pipeline] Captured {:.2}s of audio ({} bytes WAV). Active: \"{}\"",
                    duration,
                    wav_bytes.len(),
                    active_ctx.window_title
                );

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
                                sounds_worker.play_paste();
                                injector::inject_text(&resp.text);
                            }

                            pill_worker.set_mode(PillMode::Done);
                            thread::sleep(Duration::from_millis(150));
                            pill_worker.hide();
                        }
                        Err(err) => {
                            eprintln!("[Veylo AI Error] {}", err);
                            pill_worker.set_mode(PillMode::Error);
                            thread::sleep(Duration::from_millis(1200));
                            pill_worker.hide();
                        }
                    }
                });
            }
        }
    });

    println!("\n[Ready] Veylo Real App is Running and Ready!");
    println!("------------------------------------------------------");
    println!(" Dictation Triggers:");
    println!("   -> PRESS or HOLD: 'F8' (Single Key Toggle)");
    println!("   -> PRESS or HOLD: 'Ctrl + Space' or 'Alt + Space'");
    println!("   -> PRESS: 'Ctrl + Alt + V' (Universal Shortcut)");
    println!("   -> HOLD: 'Right Ctrl' alone (Push-to-Talk)");
    println!("   -> CLICK: Veylo System Tray Icon or 'Dictate' in Hub");
    println!(" Hub Window:");
    println!("   -> PRESS: 'F9'");
    println!("   -> OR LEFT-CLICK the Veylo taskbar tray icon");
    println!("------------------------------------------------------\n");

    // Welcome flash: show the Liquid Glass Pill on screen for 1.2 seconds
    sound_engine.play_start();
    let pill_welcome = pill.clone();
    thread::spawn(move || {
        pill_welcome.show();
        thread::sleep(Duration::from_millis(1200));
        pill_welcome.hide();
    });

    // Main Win32 event pump
    let main_tx = hotkey_listener.sender.clone();
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                let id = msg.wParam.0;
                println!("[Kernel Event] WM_HOTKEY received! (ID: {})", id);
                if id == 102 {
                    // F9: Open Veylo Hub
                    hub::open_hub();
                } else {
                    // Dictation Toggle
                    let _ = main_tx.send(HotkeyAction::Toggle);
                }
            }
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    is_running.store(false, Ordering::Relaxed);
}
