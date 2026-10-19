use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, VK_CONTROL, VK_LCONTROL, VK_LWIN, VK_RCONTROL, VK_RWIN, VK_SPACE,
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
}

static mut HOOK_HANDLE: Option<HHOOK> = None;
static mut EVENT_SENDER: Option<Sender<HotkeyAction>> = None;
static IS_ACTIVE: AtomicBool = AtomicBool::new(false);

pub struct HotkeyListener {
    _thread: thread::JoinHandle<()>,
    pub receiver: Receiver<HotkeyAction>,
}

impl HotkeyListener {
    pub fn new() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let thread_handle = thread::spawn(move || unsafe {
            EVENT_SENDER = Some(sender);

            let hook = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(Self::low_level_keyboard_proc),
                HINSTANCE(std::ptr::null_mut()),
                0,
            );

            match hook {
                Ok(h) => {
                    HOOK_HANDLE = Some(h);
                    println!("[HotkeyListener] Global keyboard hook active (Hold Ctrl + Win or Ctrl + Space to dictate)...");

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
                    eprintln!("[HotkeyListener] Failed to set low-level keyboard hook: {}", e);
                }
            }
        });

        Self {
            _thread: thread_handle,
            receiver,
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

            let ctrl_down = (GetAsyncKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(VK_LCONTROL.0 as i32) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(VK_RCONTROL.0 as i32) as u16 & 0x8000) != 0;

            let win_down = (GetAsyncKeyState(VK_LWIN.0 as i32) as u16 & 0x8000) != 0
                || (GetAsyncKeyState(VK_RWIN.0 as i32) as u16 & 0x8000) != 0;

            let space_down = (GetAsyncKeyState(VK_SPACE.0 as i32) as u16 & 0x8000) != 0;

            // Trigger when (Ctrl + Win) or (Ctrl + Space)
            let combo_pressed = (ctrl_down && win_down) || (ctrl_down && space_down);

            let currently_active = IS_ACTIVE.load(Ordering::Relaxed);

            if is_down && combo_pressed && !currently_active {
                IS_ACTIVE.store(true, Ordering::SeqCst);
                if let Some(ref sender) = EVENT_SENDER {
                    let _ = sender.send(HotkeyAction::Start);
                }
            } else if is_up && currently_active {
                // If either trigger key was released while active
                let released_relevant_key = vk == VK_CONTROL.0 as i32
                    || vk == VK_LCONTROL.0 as i32
                    || vk == VK_RCONTROL.0 as i32
                    || vk == VK_LWIN.0 as i32
                    || vk == VK_RWIN.0 as i32
                    || vk == VK_SPACE.0 as i32;

                if released_relevant_key {
                    IS_ACTIVE.store(false, Ordering::SeqCst);
                    if let Some(ref sender) = EVENT_SENDER {
                        let _ = sender.send(HotkeyAction::Stop);
                    }
                }
            }
        }

        CallNextHookEx(None, n_code, w_param, l_param)
    }
}
