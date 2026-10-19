use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    VK_CONTROL, VK_F8, VK_LCONTROL, VK_LMENU, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RWIN,
    VK_SPACE,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, MSG, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotkeyAction {
    Start,
    Stop,
    Toggle,
}

static mut HOOK_HANDLE: Option<HHOOK> = None;
static mut EVENT_SENDER: Option<Sender<HotkeyAction>> = None;
static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

// Atomic key state trackers — eliminates race conditions with GetAsyncKeyState
static CTRL_HELD: AtomicBool = AtomicBool::new(false);
static WIN_HELD: AtomicBool = AtomicBool::new(false);
static ALT_HELD: AtomicBool = AtomicBool::new(false);
static SPACE_HELD: AtomicBool = AtomicBool::new(false);
static RCTRL_HELD: AtomicBool = AtomicBool::new(false);

pub struct HotkeyListener {
    _thread: thread::JoinHandle<()>,
    pub receiver: Receiver<HotkeyAction>,
    pub sender: Sender<HotkeyAction>,
}

impl HotkeyListener {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        let s_clone = sender.clone();

        let thread_handle = thread::spawn(move || unsafe {
            EVENT_SENDER = Some(s_clone);

            // In Windows Win32, low-level hooks in an executable must pass None for hMod
            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(Self::low_level_keyboard_proc),
                None,
                0,
            );

            match hook {
                Ok(h) => {
                    HOOK_HANDLE = Some(h);
                    println!("[HotkeyListener] Global keyboard hook successfully active (handle: {:?})!", h);
                    println!("[HotkeyListener] Supported triggers:");
                    println!("   -> HOLD or TAP 'Ctrl + Space' or 'Alt + Space'");
                    println!("   -> HOLD 'Ctrl + Win'");
                    println!("   -> HOLD 'Right Ctrl' alone (laptop friendly)");
                    println!("   -> PRESS 'F8' (toggle dictation on/off)");

                    let mut msg = MSG::default();
                    while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                        let _ = TranslateMessage(&msg);
                        DispatchMessageW(&msg);
                    }

                    if let Some(h) = HOOK_HANDLE {
                        let _ = UnhookWindowsHookEx(h);
                    }
                }
                Err(e) => {
                    eprintln!("[HotkeyListener] FAILED to set low-level keyboard hook: {}", e);
                }
            }
        });

        Self {
            _thread: thread_handle,
            receiver,
            sender,
        }
    }

    unsafe extern "system" fn low_level_keyboard_proc(
        n_code: i32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if n_code >= 0 {
            let msg = w_param.0 as u32;
            let is_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
            let is_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

            let kbd = *(l_param.0 as *const KBDLLHOOKSTRUCT);
            let vk = kbd.vkCode as i32;
            let is_extended = (kbd.flags.0 & 1) != 0;

            // Classify key
            let is_ctrl = vk == VK_CONTROL.0 as i32 || vk == VK_LCONTROL.0 as i32 || vk == VK_RCONTROL.0 as i32;
            let is_rctrl = vk == VK_RCONTROL.0 as i32 || (is_ctrl && is_extended);
            let is_win = vk == VK_LWIN.0 as i32 || vk == VK_RWIN.0 as i32;
            let is_alt = vk == VK_MENU.0 as i32 || vk == VK_LMENU.0 as i32 || vk == VK_RMENU.0 as i32;
            let is_space = vk == VK_SPACE.0 as i32;
            let is_f8 = vk == VK_F8.0 as i32;

            if is_down {
                // Update tracker
                if is_ctrl {
                    CTRL_HELD.store(true, Ordering::SeqCst);
                    if is_rctrl {
                        RCTRL_HELD.store(true, Ordering::SeqCst);
                    }
                }
                if is_win {
                    WIN_HELD.store(true, Ordering::SeqCst);
                }
                if is_alt {
                    ALT_HELD.store(true, Ordering::SeqCst);
                }
                if is_space {
                    SPACE_HELD.store(true, Ordering::SeqCst);
                }

                // 1. Single Key Toggle: F8
                if is_f8 {
                    let currently_active = IS_ACTIVE.load(Ordering::SeqCst);
                    if currently_active {
                        IS_ACTIVE.store(false, Ordering::SeqCst);
                        if let Some(ref sender) = EVENT_SENDER {
                            let _ = sender.send(HotkeyAction::Stop);
                        }
                    } else {
                        IS_ACTIVE.store(true, Ordering::SeqCst);
                        if let Some(ref sender) = EVENT_SENDER {
                            let _ = sender.send(HotkeyAction::Start);
                        }
                    }
                    return CallNextHookEx(None, n_code, w_param, l_param);
                }

                // 2. Combination & Push-to-Talk Triggers:
                let ctrl = CTRL_HELD.load(Ordering::SeqCst);
                let win = WIN_HELD.load(Ordering::SeqCst);
                let alt = ALT_HELD.load(Ordering::SeqCst);
                let rctrl = RCTRL_HELD.load(Ordering::SeqCst);

                let trigger_fired = (is_space && (ctrl || alt))
                    || (is_win && ctrl)
                    || (is_ctrl && win)
                    || is_rctrl
                    || rctrl;

                let currently_active = IS_ACTIVE.load(Ordering::SeqCst);
                if trigger_fired && !currently_active {
                    println!(
                        "[Hotkey] Trigger ACTIVATED -> Start dictation (space={}, ctrl={}, win={}, alt={}, rctrl={})",
                        is_space, ctrl, win, alt, is_rctrl
                    );
                    IS_ACTIVE.store(true, Ordering::SeqCst);
                    if let Some(ref sender) = EVENT_SENDER {
                        let _ = sender.send(HotkeyAction::Start);
                    }
                }
            } else if is_up {
                // Update tracker
                if is_ctrl {
                    CTRL_HELD.store(false, Ordering::SeqCst);
                    RCTRL_HELD.store(false, Ordering::SeqCst);
                }
                if is_win {
                    WIN_HELD.store(false, Ordering::SeqCst);
                }
                if is_alt {
                    ALT_HELD.store(false, Ordering::SeqCst);
                }
                if is_space {
                    SPACE_HELD.store(false, Ordering::SeqCst);
                }

                let currently_active = IS_ACTIVE.load(Ordering::SeqCst);
                if currently_active {
                    let released_relevant = is_ctrl || is_win || is_alt || is_space || is_rctrl;
                    if released_relevant {
                        println!("[Hotkey] Trigger RELEASED -> Stop dictation and transcribe");
                        IS_ACTIVE.store(false, Ordering::SeqCst);
                        if let Some(ref sender) = EVENT_SENDER {
                            let _ = sender.send(HotkeyAction::Stop);
                        }
                    }
                }
            }
        }

        CallNextHookEx(None, n_code, w_param, l_param)
    }
}
