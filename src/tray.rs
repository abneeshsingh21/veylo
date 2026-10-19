use crossbeam_channel::Sender;
use std::thread;
use windows::core::w;
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
    DispatchMessageW, GetCursorPos, GetMessageW, LoadImageW, RegisterClassW,
    SetForegroundWindow, TrackPopupMenu, TranslateMessage, HICON, IMAGE_ICON, LR_LOADFROMFILE,
    MF_SEPARATOR, MF_STRING, MSG, TPM_NONOTIFY, TPM_RETURNCMD, TPM_RIGHTBUTTON,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WM_USER, WNDCLASSW,
};

use crate::hotkey::HotkeyAction;

const WM_TRAY_CALLBACK: u32 = WM_USER + 100;
const IDM_OPEN_HUB: usize = 2001;
const IDM_TOGGLE_DICTATION: usize = 2002;
const IDM_SETTINGS: usize = 2003;
const IDM_QUIT: usize = 2004;

pub struct TrayIcon {
    _thread: thread::JoinHandle<()>,
}

static mut TRAY_HWND: Option<HWND> = None;
static mut EVENT_SENDER: Option<Sender<HotkeyAction>> = None;

impl TrayIcon {
    pub fn new(sender: Sender<HotkeyAction>) -> Self {
        let s_clone = sender.clone();
        let handle = thread::spawn(move || unsafe {
            crate::attach_to_default_desktop();
            EVENT_SENDER = Some(s_clone);

            let class_name = w!("VeyloTrayClass");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                lpszClassName: class_name,
                hInstance: GetModuleHandleW(None).unwrap_or_default().into(),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let hwnd = match CreateWindowExW(
                Default::default(),
                class_name,
                w!("VeyloTrayWindow"),
                Default::default(),
                0,
                0,
                0,
                0,
                None,
                None,
                GetModuleHandleW(None).unwrap_or_default(),
                None,
            ) {
                Ok(h) => h,
                Err(e) => {
                    eprintln!("[Tray] CreateWindowExW failed: {}", e);
                    return;
                }
            };

            TRAY_HWND = Some(hwnd);

            // Load Veylo Icon
            let hicon: HICON = if let Ok(h) = LoadImageW(
                None,
                w!("assets/icons/veylo.ico"),
                IMAGE_ICON,
                32,
                32,
                LR_LOADFROMFILE,
            ) {
                HICON(h.0)
            } else {
                HICON::default()
            };

            let mut nid = NOTIFYICONDATAW::default();
            nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            nid.hWnd = hwnd;
            nid.uID = 1001;
            nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            nid.uCallbackMessage = WM_TRAY_CALLBACK;
            nid.hIcon = hicon;

            // Set tooltip
            let tip = "Veylo - Intelligent Voice AI (Active)\0";
            let tip_utf16: Vec<u16> = tip.encode_utf16().collect();
            let copy_len = tip_utf16.len().min(nid.szTip.len());
            nid.szTip[..copy_len].copy_from_slice(&tip_utf16[..copy_len]);

            let added = Shell_NotifyIconW(NIM_ADD, &nid);
            if added.as_bool() {
                println!("[Tray] System tray icon successfully registered in Windows Taskbar!");
            } else {
                eprintln!("[Tray] Warning: Could not register system tray icon.");
            }

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }

            // Cleanup
            let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
        });

        Self { _thread: handle }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        w_param: WPARAM,
        l_param: LPARAM,
    ) -> LRESULT {
        if msg == WM_TRAY_CALLBACK {
            let mouse_event = l_param.0 as u32;
            match mouse_event {
                WM_LBUTTONUP | WM_LBUTTONDBLCLK => {
                    // Left click: Open Veylo Hub Window
                    println!("[Tray] Left click -> Opening Veylo Hub");
                    crate::hub::open_hub();
                }
                WM_RBUTTONUP => {
                    // Right click: Display Context Menu
                    let mut cursor = POINT::default();
                    let _ = GetCursorPos(&mut cursor);

                    if let Ok(hmenu) = CreatePopupMenu() {
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_OPEN_HUB, w!("Open Veylo Hub (F9)"));
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE_DICTATION, w!("Toggle Dictation (F8)"));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_SETTINGS, w!("Settings"));
                        let _ = AppendMenuW(hmenu, MF_SEPARATOR, 0, None);
                        let _ = AppendMenuW(hmenu, MF_STRING, IDM_QUIT, w!("Quit Veylo"));

                        let _ = SetForegroundWindow(hwnd);
                        let cmd = TrackPopupMenu(
                            hmenu,
                            TPM_RETURNCMD | TPM_NONOTIFY | TPM_RIGHTBUTTON,
                            cursor.x,
                            cursor.y,
                            0,
                            hwnd,
                            None,
                        );
                        let _ = DestroyMenu(hmenu);

                        match cmd.0 as usize {
                            IDM_OPEN_HUB => {
                                crate::hub::open_hub();
                            }
                            IDM_TOGGLE_DICTATION => {
                                println!("[Tray Menu] Dictation toggled via Tray Menu");
                                if let Some(ref sender) = EVENT_SENDER {
                                    let _ = sender.send(HotkeyAction::Toggle);
                                }
                            }
                            IDM_SETTINGS => {
                                crate::hub::open_hub();
                            }
                            IDM_QUIT => {
                                println!("[Tray Menu] User selected Quit Veylo.");
                                std::process::exit(0);
                            }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
            return LRESULT(0);
        }

        DefWindowProcW(hwnd, msg, w_param, l_param)
    }
}
