use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE, DWMWA_WINDOW_CORNER_PREFERENCE,
    DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreatePen, CreateSolidBrush,
    DeleteDC, DeleteObject, EndPaint, GetStockObject, InvalidateRect, LineTo, MoveToEx,
    RoundRect, SelectObject, SetBkMode, SetTextColor, TextOutW, HBRUSH, HDC,
    PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT, WHITE_BRUSH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, RegisterClassW, SetWindowPos, ShowWindow,
    HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_HIDE,
    SW_SHOWNOACTIVATE, WNDCLASSW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};


const PILL_WIDTH: i32 = 280;
const PILL_HEIGHT: i32 = 56;
const NUM_BARS: usize = 14;

pub struct FloatingPill {
    hwnd: HWND,
    current_rms: Arc<AtomicU32>,
    is_visible: Arc<AtomicBool>,
    start_time: Arc<parking_lot::Mutex<Option<Instant>>>,
}

unsafe impl Send for FloatingPill {}
unsafe impl Sync for FloatingPill {}


// Global state accessed by the window procedure
static mut GLOBAL_PILL: Option<Arc<PillState>> = None;

#[allow(dead_code)]
struct PillState {
    hwnd: HWND,
    current_rms: Arc<AtomicU32>,
    is_visible: Arc<AtomicBool>,
    start_time: Arc<parking_lot::Mutex<Option<Instant>>>,
}


impl FloatingPill {
    pub fn new() -> Result<Self, String> {
        let is_visible = Arc::new(AtomicBool::new(false));
        let current_rms = Arc::new(AtomicU32::new(0));
        let start_time = Arc::new(parking_lot::Mutex::new(None));

        unsafe {
            let class_name = w!("WhisperFlowLiquidPill");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                lpszClassName: class_name,
                hbrBackground: HBRUSH(GetStockObject(WHITE_BRUSH).0),
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            // Calculate screen position: center bottom, ~80px above taskbar
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let pos_x = (screen_w - PILL_WIDTH) / 2;
            let pos_y = screen_h - PILL_HEIGHT - 90;

            // WS_EX_NOACTIVATE: will NEVER steal focus from user's active window!
            let ex_style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                w!("WhisperFlowPill"),
                WS_POPUP,
                pos_x,
                pos_y,
                PILL_WIDTH,
                PILL_HEIGHT,
                None,
                None,
                None,
                None,
            )
            .map_err(|e| format!("CreateWindowExW failed: {}", e))?;

            // Apply Windows 11 Liquid Glass / Acrylic & Rounded Corner DWM attributes
            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            // DWMSBT_TRANSIENTWINDOW (3) = Modern Windows 11 Acrylic blur
            let backdrop_type = 3u32; // Acrylic
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_SYSTEMBACKDROP_TYPE,
                &backdrop_type as *const _ as *const _,
                std::mem::size_of_val(&backdrop_type) as u32,
            );

            let state = Arc::new(PillState {
                hwnd,
                current_rms: current_rms.clone(),
                is_visible: is_visible.clone(),
                start_time: start_time.clone(),
            });

            GLOBAL_PILL = Some(state);

            Ok(Self {
                hwnd,
                current_rms,
                is_visible,
                start_time,
            })
        }
    }

    pub fn show(&self) {
        self.is_visible.store(true, Ordering::Release);
        *self.start_time.lock() = Some(Instant::now());
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
            let _ = InvalidateRect(self.hwnd, None, false);
        }
    }

    pub fn hide(&self) {
        self.is_visible.store(false, Ordering::Release);
        *self.start_time.lock() = None;
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn update_rms(&self, rms: f32) {
        self.current_rms.store(rms.to_bits(), Ordering::Relaxed);
        if self.is_visible.load(Ordering::Relaxed) {
            unsafe {
                let _ = InvalidateRect(self.hwnd, None, false);
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        const WM_PAINT: u32 = 0x000F;
        const WM_ERASEBKGND: u32 = 0x0014;

        match msg {
            WM_ERASEBKGND => LRESULT(1), // Suppress background erasing to avoid flicker
            WM_PAINT => {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.is_invalid() {
                    Self::draw_pill(hwnd, hdc);
                    let _ = EndPaint(hwnd, &ps);
                }
                LRESULT(0)
            }
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }

    unsafe fn draw_pill(_hwnd: HWND, hdc: HDC) {
        let (rms, elapsed_str) = if let Some(ref state) = GLOBAL_PILL {
            let r = f32::from_bits(state.current_rms.load(Ordering::Relaxed));
            let str_time = if let Some(st) = *state.start_time.lock() {
                let elapsed = st.elapsed().as_secs();
                format!("{:02}:{:02}", elapsed / 60, elapsed % 60)
            } else {
                "00:00".to_string()
            };
            (r, str_time)
        } else {
            (0.0, "00:00".to_string())
        };

        // Create memory DC for 100% flicker-free double buffering
        let mem_dc = CreateCompatibleDC(hdc);
        let mem_bmp = CreateCompatibleBitmap(hdc, PILL_WIDTH, PILL_HEIGHT);
        let old_bmp = SelectObject(mem_dc, mem_bmp);

        // 1. Draw Liquid Glass Capsule Background (Deep translucent dark slate with purple tint)
        let bg_color = COLORREF(0x001E1814); // BGR for dark slate #14181E
        let bg_brush = CreateSolidBrush(bg_color);
        let border_pen = CreatePen(PS_SOLID, 1, COLORREF(0x00554433)); // Subtle luminous border
        let old_brush = SelectObject(mem_dc, bg_brush);
        let old_pen = SelectObject(mem_dc, border_pen);

        let _ = RoundRect(mem_dc, 2, 2, PILL_WIDTH - 2, PILL_HEIGHT - 2, 48, 48);

        // 2. Draw Pulsing Glowing Mic Status Dot on the Left
        let dot_color = if rms > 0.05 {
            COLORREF(0x005522E8) // Bright Violet / Purple when speaking
        } else {
            COLORREF(0x00AA5522) // Cyan / Blue idle
        };
        let dot_brush = CreateSolidBrush(dot_color);
        let old_dot_brush = SelectObject(mem_dc, dot_brush);
        let _ = RoundRect(mem_dc, 18, (PILL_HEIGHT - 12) / 2, 30, (PILL_HEIGHT + 12) / 2, 12, 12);
        SelectObject(mem_dc, old_dot_brush);
        let _ = DeleteObject(dot_brush);

        // 3. Draw Audio-Reactive Harmonic Waveform in Center
        let wave_center_x = PILL_WIDTH / 2;
        let wave_center_y = PILL_HEIGHT / 2;
        let bar_width = 4;
        let bar_gap = 4;
        let total_wave_w = (NUM_BARS as i32) * (bar_width + bar_gap);
        let start_x = wave_center_x - (total_wave_w / 2);

        let wave_pen = CreatePen(PS_SOLID, bar_width, COLORREF(0x00E8669B)); // Wispr Purple #9B66E8 (BGR: 0x00E8669B)
        let old_wave_pen = SelectObject(mem_dc, wave_pen);

        for i in 0..NUM_BARS {
            let offset_factor = (i as f32 - (NUM_BARS as f32 / 2.0)).abs() / (NUM_BARS as f32 / 2.0);
            // Bell curve dampening towards outer bars
            let bell = (1.0 - offset_factor * 0.6).max(0.2);
            // Minimum resting bar height is 6px, maximum reacting height is 34px
            let bar_height = (6.0 + rms * 28.0 * bell).min(34.0) as i32;
            let x = start_x + (i as i32) * (bar_width + bar_gap);
            let y1 = wave_center_y - (bar_height / 2);
            let y2 = wave_center_y + (bar_height / 2);

            let _ = MoveToEx(mem_dc, x, y1, None);
            let _ = LineTo(mem_dc, x, y2);
        }
        SelectObject(mem_dc, old_wave_pen);
        let _ = DeleteObject(wave_pen);

        // 4. Draw Timer on the Right
        SetBkMode(mem_dc, TRANSPARENT);
        SetTextColor(mem_dc, COLORREF(0x00CCCCCC)); // Soft silver text
        let wide_str: Vec<u16> = elapsed_str.encode_utf16().collect();
        let _ = TextOutW(mem_dc, PILL_WIDTH - 65, (PILL_HEIGHT - 16) / 2, &wide_str);

        // 5. Blit buffer to screen HDC
        let _ = BitBlt(hdc, 0, 0, PILL_WIDTH, PILL_HEIGHT, mem_dc, 0, 0, SRCCOPY);

        // Cleanup GDI objects
        SelectObject(mem_dc, old_brush);
        SelectObject(mem_dc, old_pen);
        SelectObject(mem_dc, old_bmp);
        let _ = DeleteObject(bg_brush);
        let _ = DeleteObject(border_pen);
        let _ = DeleteObject(mem_bmp);
        let _ = DeleteDC(mem_dc);

    }
}
