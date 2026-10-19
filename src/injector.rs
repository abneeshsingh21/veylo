use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VIRTUAL_KEY,
};

/// Sends Unicode text directly into the focused window at the caret position.
/// This preserves the user's clipboard and works across all Windows applications
/// (VS Code, Word, Slack, Chrome, Discord, Terminal, etc.).
#[allow(dead_code)]
pub fn inject_text(text: &str) {
    if text.is_empty() {
        return;
    }

    unsafe {
        // Collect UTF-16 code units
        let utf16_chars: Vec<u16> = text.encode_utf16().collect();
        let mut inputs: Vec<INPUT> = Vec::with_capacity(utf16_chars.len() * 2);

        for ch in utf16_chars {
            // Key Down (Unicode character code in wScan)
            let down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: ch,
                        dwFlags: KEYEVENTF_UNICODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            // Key Up
            let up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: VIRTUAL_KEY(0),
                        wScan: ch,
                        dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };

            inputs.push(down);
            inputs.push(up);
        }

        // Send all input events in an atomic batch
        let sent = SendInput(&inputs, std::mem::size_of::<INPUT>() as i32);
        if sent != inputs.len() as u32 {
            eprintln!(
                "[Injector] Warning: Sent {} of {} keystrokes.",
                sent,
                inputs.len()
            );
        } else {
            println!("[Injector] Successfully injected {} characters.", text.len());
        }
    }
}
