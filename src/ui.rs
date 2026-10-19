use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmExtendFrameIntoClientArea, DwmSetWindowAttribute, DWMWA_SYSTEMBACKDROP_TYPE,
    DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC, SelectObject,
    BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS,
};
use windows::Win32::UI::Controls::MARGINS;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetSystemMetrics, RegisterClassW, SetWindowPos, ShowWindow,
    UpdateLayeredWindow, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SW_HIDE, SW_SHOWNOACTIVATE, ULW_ALPHA, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};


const PILL_WIDTH: i32 = 300;
const PILL_HEIGHT: i32 = 60;
const NUM_BARS: usize = 14;

pub struct FloatingPill {
    hwnd: HWND,
    current_rms: Arc<AtomicU32>,
    is_visible: Arc<AtomicBool>,
    start_time: Arc<parking_lot::Mutex<Option<Instant>>>,
}

unsafe impl Send for FloatingPill {}
unsafe impl Sync for FloatingPill {}

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
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let pos_x = (screen_w - PILL_WIDTH) / 2;
            let pos_y = screen_h - PILL_HEIGHT - 90;

            // WS_EX_LAYERED + WS_EX_TOPMOST + WS_EX_TOOLWINDOW + WS_EX_NOACTIVATE
            let ex_style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED;

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

            // Extend glass frame into client area (-1 margins = whole window is blurred acrylic)
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

            // Windows 11 rounded corners
            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            // DWMSBT_TRANSIENTWINDOW (3) = Acrylic blur backdrop
            let backdrop_type = 3u32;
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

            GLOBAL_PILL = Some(state.clone());

            let pill = Self {
                hwnd,
                current_rms,
                is_visible,
                start_time,
            };

            // Initial render
            pill.render_glass();

            Ok(pill)
        }
    }

    pub fn show(&self) {
        self.is_visible.store(true, Ordering::Release);
        *self.start_time.lock() = Some(Instant::now());
        self.render_glass();
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
            self.render_glass();
        }
    }

    /// Renders a true 32-bit premultiplied ARGB Liquid Glass pill with sub-pixel
    /// anti-aliased borders, specular glass refraction highlights, glowing waveform, and timer.
    fn render_glass(&self) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: PILL_WIDTH,
                    biHeight: -PILL_HEIGHT, // Top-down DIB
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };

            let mut bits: *mut u8 = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                mem_dc,
                &bmi,
                DIB_RGB_COLORS,
                &mut bits as *mut _ as *mut *mut _,
                None,
                0,
            );

            if bitmap.is_err() || bits.is_null() {
                ReleaseDC(None, screen_dc);
                let _ = DeleteDC(mem_dc);
                return;
            }

            let bitmap = bitmap.unwrap();
            let old_bmp = SelectObject(mem_dc, bitmap);

            let rms = f32::from_bits(self.current_rms.load(Ordering::Relaxed));
            let elapsed_str = if let Some(st) = *self.start_time.lock() {
                let elapsed = st.elapsed().as_secs();
                format!("{:02}:{:02}", elapsed / 60, elapsed % 60)
            } else {
                "00:00".to_string()
            };

            // Software Liquid Glass Shader
            let pixel_slice = std::slice::from_raw_parts_mut(
                bits as *mut u32,
                (PILL_WIDTH * PILL_HEIGHT) as usize,
            );

            // Clear buffer to fully transparent
            pixel_slice.fill(0);

            let w = PILL_WIDTH as f32;
            let h = PILL_HEIGHT as f32;
            let radius = (h / 2.0) - 2.0; // Capsule radius ~28px
            let pill_half_len = (w / 2.0) - radius - 4.0;
            let center_x = w / 2.0;
            let center_y = h / 2.0;

            // 1. Draw Liquid Glass Pill Body with Signed Distance Field (SDF)
            for y_i in 0..PILL_HEIGHT {
                let y = y_i as f32 + 0.5;
                for x_i in 0..PILL_WIDTH {
                    let x = x_i as f32 + 0.5;

                    // Distance to capsule axis
                    let dx = (x - center_x).abs() - pill_half_len;
                    let clamped_dx = dx.max(0.0);
                    let dy = (y - center_y).abs();
                    let dist = (clamped_dx * clamped_dx + dy * dy).sqrt();

                    let d = dist - radius;

                    if d > 1.5 {
                        // Completely outside
                        continue;
                    }

                    // Anti-aliased outer edge coverage (0.0 to 1.0)
                    let edge_alpha = (1.0 - (d / 1.5).max(0.0)).min(1.0);

                    // Liquid Glass Color Palette (Dark Obsidian Glass with Violet undertone)
                    // Glass body opacity: ~65% to let DWM Acrylic blur shine through
                    let mut alpha = 0.68 * edge_alpha;
                    let mut red = 20.0f32;
                    let mut green = 18.0f32;
                    let mut blue = 28.0f32;

                    // Specular Highlight on upper rim (light reflection on glass)
                    if d >= -2.0 && d <= 0.0 && y < center_y {
                        let spec_strength = (1.0 - (y / (center_y * 0.8)).min(1.0)) * 0.55;
                        red += 200.0 * spec_strength;
                        green += 200.0 * spec_strength;
                        blue += 255.0 * spec_strength;
                        alpha = (alpha + spec_strength * 0.3).min(1.0);
                    }

                    // Luminous 1px Glass Border
                    if d >= -1.2 && d <= 0.2 {
                        let border_factor = 1.0 - (d + 0.5).abs();
                        red = red * (1.0 - border_factor) + 180.0 * border_factor;
                        green = green * (1.0 - border_factor) + 160.0 * border_factor;
                        blue = blue * (1.0 - border_factor) + 255.0 * border_factor;
                        alpha = (alpha + 0.25 * border_factor).min(1.0);
                    }

                    // Premultiply alpha for Windows UpdateLayeredWindow
                    let a_byte = (alpha * 255.0).clamp(0.0, 255.0) as u8;
                    let r_byte = ((red * alpha).clamp(0.0, 255.0)) as u8;
                    let g_byte = ((green * alpha).clamp(0.0, 255.0)) as u8;
                    let b_byte = ((blue * alpha).clamp(0.0, 255.0)) as u8;

                    let pixel_val = ((a_byte as u32) << 24)
                        | ((r_byte as u32) << 16)
                        | ((g_byte as u32) << 8)
                        | (b_byte as u32);

                    pixel_slice[(y_i * PILL_WIDTH + x_i) as usize] = pixel_val;
                }
            }

            // 2. Draw Pulsing Neon Status Orb on Left (x ~ 26, y ~ center)
            let orb_cx = 28.0f32;
            let orb_cy = center_y;
            let orb_r = 5.0f32;
            let (orb_r_base, orb_g_base, orb_b_base) = if rms > 0.04 {
                (236.0, 72.0, 153.0) // Glowing Rose / Fuchsia
            } else {
                (168.0, 85.0, 247.0) // Violet Glow
            };

            for y_i in (orb_cy as i32 - 10)..=(orb_cy as i32 + 10) {
                if y_i < 0 || y_i >= PILL_HEIGHT {
                    continue;
                }
                for x_i in (orb_cx as i32 - 10)..=(orb_cx as i32 + 10) {
                    if x_i < 0 || x_i >= PILL_WIDTH {
                        continue;
                    }
                    let dist = (((x_i as f32 - orb_cx).powi(2) + (y_i as f32 - orb_cy).powi(2)) as f32).sqrt();
                    if dist <= orb_r + 4.0 {
                        let intensity = if dist <= orb_r {
                            1.0
                        } else {
                            (1.0 - (dist - orb_r) / 4.0).max(0.0)
                        };
                        let idx = (y_i * PILL_WIDTH + x_i) as usize;
                        let a = (intensity * 240.0) as u8;
                        let r = ((orb_r_base * intensity) as u32).min(255);
                        let g = ((orb_g_base * intensity) as u32).min(255);
                        let b = ((orb_b_base * intensity) as u32).min(255);
                        pixel_slice[idx] = ((a as u32) << 24) | (r << 16) | (g << 8) | b;
                    }
                }
            }

            // 3. Draw 14 Vibrant Audio-Reactive Waveform Bars (Center)
            let wave_start_x = (center_x - 55.0) as i32;
            let bar_w = 4;
            let bar_gap = 4;

            for i in 0..NUM_BARS {
                let bx = wave_start_x + (i as i32) * (bar_w + bar_gap);
                let offset_factor = (i as f32 - (NUM_BARS as f32 / 2.0)).abs() / (NUM_BARS as f32 / 2.0);
                let bell = (1.0 - offset_factor * 0.55).max(0.3);
                let bar_h = (6.0 + rms * 34.0 * bell).min(38.0) as i32;
                let by1 = (center_y - (bar_h as f32 / 2.0)) as i32;
                let by2 = (center_y + (bar_h as f32 / 2.0)) as i32;

                for y in by1..=by2 {
                    if y < 0 || y >= PILL_HEIGHT {
                        continue;
                    }
                    // Vertical Gradient: Electric Fuchsia (top) to Wispr Purple (bottom)
                    let t = (y - by1) as f32 / (bar_h.max(1) as f32);
                    let r_bar = 240.0 * (1.0 - t) + 160.0 * t;
                    let g_bar = 70.0 * (1.0 - t) + 90.0 * t;
                    let b_bar = 180.0 * (1.0 - t) + 255.0 * t;

                    for x in bx..(bx + bar_w) {
                        if x >= 0 && x < PILL_WIDTH {
                            let idx = (y * PILL_WIDTH + x) as usize;
                            pixel_slice[idx] = (250 << 24)
                                | ((r_bar as u32) << 16)
                                | ((g_bar as u32) << 8)
                                | (b_bar as u32);
                        }
                    }
                }
            }

            // 4. Draw Crisp Digital Timer on Right
            // Simple 5x7 bitmap font rendering for 100% crisp typography without GDI font delays
            render_timer_digits(pixel_slice, PILL_WIDTH - 68, (center_y - 6.0) as i32, &elapsed_str);

            // 5. Transfer to Layered Window with UpdateLayeredWindow
            let screen_w = GetSystemMetrics(SM_CXSCREEN);
            let screen_h = GetSystemMetrics(SM_CYSCREEN);
            let pt_dst = POINT {
                x: (screen_w - PILL_WIDTH) / 2,
                y: screen_h - PILL_HEIGHT - 90,
            };
            let size = SIZE {
                cx: PILL_WIDTH,
                cy: PILL_HEIGHT,
            };
            let pt_src = POINT { x: 0, y: 0 };
            let blend = BLENDFUNCTION {
                BlendOp: 0, // AC_SRC_OVER
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA (per-pixel alpha)
            };

            let _ = UpdateLayeredWindow(
                self.hwnd,
                screen_dc,
                Some(&pt_dst),
                Some(&size),
                mem_dc,
                Some(&pt_src),
                COLORREF(0),
                Some(&blend),
                ULW_ALPHA,
            );

            // Cleanup
            SelectObject(mem_dc, old_bmp);
            let _ = DeleteObject(bitmap);
            let _ = DeleteDC(mem_dc);
            ReleaseDC(None, screen_dc);
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

/// Minimalist, ultra-crisp 5x7 font rendering for the timer digits (0-9 and ':')
fn render_timer_digits(buffer: &mut [u32], start_x: i32, start_y: i32, text: &str) {
    let font: [&[u8; 7]; 11] = [
        &[0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110], // 0
        &[0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110], // 1
        &[0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111], // 2
        &[0b11111, 0b00010, 0b00100, 0b00010, 0b00001, 0b10001, 0b01110], // 3
        &[0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010], // 4
        &[0b11111, 0b10000, 0b11110, 0b00001, 0b00001, 0b10001, 0b01110], // 5
        &[0b00110, 0b01000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110], // 6
        &[0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000], // 7
        &[0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110], // 8
        &[0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00010, 0b01100], // 9
        &[0b00000, 0b00100, 0b00000, 0b00000, 0b00100, 0b00000, 0b00000], // :
    ];

    let mut cur_x = start_x;
    for ch in text.chars() {
        let glyph_idx = match ch {
            '0'..='9' => (ch as usize) - ('0' as usize),
            ':' => 10,
            _ => continue,
        };
        let glyph = font[glyph_idx];
        let width = if ch == ':' { 3 } else { 5 };

        for row in 0..7 {
            let row_bits = glyph[row];
            for col in 0..width {
                if (row_bits & (1 << (4 - col))) != 0 {
                    let px = cur_x + col;
                    let py = start_y + (row as i32) * 2; // scale 2x for readability
                    if px >= 0 && px < PILL_WIDTH && py >= 0 && py + 1 < PILL_HEIGHT {
                        let color = 0xF0E2E8F0; // Crisp silver white with alpha
                        buffer[(py * PILL_WIDTH + px) as usize] = color;
                        buffer[((py + 1) * PILL_WIDTH + px) as usize] = color;
                    }
                }
            }
        }
        cur_x += width + 3;
    }
}
