use std::f32::consts::PI;
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

const PILL_WIDTH: i32 = 320;
const PILL_HEIGHT: i32 = 64;

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

            // Layered, Topmost, Non-activating (never steals keyboard focus!)
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

            // DWM Acrylic blur backdrop
            let margins = MARGINS {
                cxLeftWidth: -1,
                cxRightWidth: -1,
                cyTopHeight: -1,
                cyBottomHeight: -1,
            };
            let _ = DwmExtendFrameIntoClientArea(hwnd, &margins);

            let corner_pref = DWMWCP_ROUND;
            let _ = DwmSetWindowAttribute(
                hwnd,
                DWMWA_WINDOW_CORNER_PREFERENCE,
                &corner_pref as *const _ as *const _,
                std::mem::size_of_val(&corner_pref) as u32,
            );

            let backdrop_type = 3u32; // Acrylic blur
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

            pill.render_liquid_glass();

            Ok(pill)
        }
    }

    pub fn show(&self) {
        self.is_visible.store(true, Ordering::Release);
        *self.start_time.lock() = Some(Instant::now());
        self.render_liquid_glass();
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
            self.render_liquid_glass();
        }
    }

    /// Renders true Liquid Glass with iridescent cyan-to-amber rim reflections,
    /// smoky glass refraction, glossy specular highlights, and an organic flowing waveform ribbon.
    fn render_liquid_glass(&self) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: PILL_WIDTH,
                    biHeight: -PILL_HEIGHT, // Top-down
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

            let pixel_slice = std::slice::from_raw_parts_mut(
                bits as *mut u32,
                (PILL_WIDTH * PILL_HEIGHT) as usize,
            );

            // Clear to transparent
            pixel_slice.fill(0);

            let w = PILL_WIDTH as f32;
            let h = PILL_HEIGHT as f32;
            let radius = (h / 2.0) - 2.0; // ~30px radius
            let pill_half_len = (w / 2.0) - radius - 3.0;
            let center_x = w / 2.0;
            let center_y = h / 2.0;

            // 1. Draw Liquid Glass Pill (Smoky translucent base + chromatic rim lighting)
            for y_i in 0..PILL_HEIGHT {
                let y = y_i as f32 + 0.5;
                for x_i in 0..PILL_WIDTH {
                    let x = x_i as f32 + 0.5;

                    let dx = (x - center_x).abs() - pill_half_len;
                    let clamped_dx = dx.max(0.0);
                    let dy = (y - center_y).abs();
                    let dist = (clamped_dx * clamped_dx + dy * dy).sqrt();
                    let d = dist - radius;

                    // Outside boundary with smooth subpixel anti-aliasing
                    if d > 1.2 {
                        continue;
                    }
                    let edge_alpha = (1.0 - (d / 1.2).max(0.0)).min(1.0);

                    // Deep Obsidian Glass Base (Dark translucent with blue/violet depth)
                    let mut alpha = 0.76 * edge_alpha;
                    let mut r = 12.0f32;
                    let mut g = 15.0f32;
                    let mut b = 25.0f32;

                    // Chromatic Iridescent Rim Lighting (matches the Wispr Liquid Glass Logo!):
                    // Left edge = Cool Cyan / Electric Blue (x < center_x)
                    // Right edge = Warm Amber / Peach Glow (x > center_x)
                    let normalized_x = (x / w).clamp(0.0, 1.0);
                    let rim_proximity = 1.0 - ((-d).max(0.0) / 3.0).min(1.0);

                    if d >= -3.0 && d <= 0.8 {
                        // Blend from Cyan (#5AB0FF) on the left to Amber (#FFAE75) on the right
                        let left_weight = (1.0 - normalized_x).powf(1.4);
                        let right_weight = normalized_x.powf(1.4);

                        // Cyan component
                        r += 80.0 * left_weight * rim_proximity;
                        g += 170.0 * left_weight * rim_proximity;
                        b += 255.0 * left_weight * rim_proximity;

                        // Amber/Peach component
                        r += 255.0 * right_weight * rim_proximity;
                        g += 175.0 * right_weight * rim_proximity;
                        b += 115.0 * right_weight * rim_proximity;

                        alpha = (alpha + 0.22 * rim_proximity).min(1.0);
                    }

                    // Upper Specular Glass Highlight (Glossy top reflection)
                    if d >= -2.0 && d <= 0.0 && y < center_y - 2.0 {
                        let spec_factor = (1.0 - (y / (center_y - 2.0)).min(1.0)) * 0.65;
                        r += 220.0 * spec_factor;
                        g += 235.0 * spec_factor;
                        b += 255.0 * spec_factor;
                        alpha = (alpha + spec_factor * 0.25).min(1.0);
                    }

                    // Write 32-bit premultiplied pixel
                    let a_byte = (alpha * 255.0).clamp(0.0, 255.0) as u8;
                    let r_byte = (r * alpha).clamp(0.0, 255.0) as u8;
                    let g_byte = (g * alpha).clamp(0.0, 255.0) as u8;
                    let b_byte = (b * alpha).clamp(0.0, 255.0) as u8;

                    pixel_slice[(y_i * PILL_WIDTH + x_i) as usize] =
                        ((a_byte as u32) << 24) | ((r_byte as u32) << 16) | ((g_byte as u32) << 8) | (b_byte as u32);
                }
            }

            // 2. Draw Left Glowing Status Gem (Iridescent Orb)
            let orb_x = 26.0f32;
            let orb_y = center_y;
            let orb_r = 5.0f32;
            let is_speaking = rms > 0.03;

            for yi in (orb_y as i32 - 8)..=(orb_y as i32 + 8) {
                if yi < 0 || yi >= PILL_HEIGHT {
                    continue;
                }
                for xi in (orb_x as i32 - 8)..=(orb_x as i32 + 8) {
                    if xi < 0 || xi >= PILL_WIDTH {
                        continue;
                    }
                    let dist = (((xi as f32 - orb_x).powi(2) + (yi as f32 - orb_y).powi(2)) as f32).sqrt();
                    if dist <= orb_r + 3.0 {
                        let intensity = if dist <= orb_r {
                            1.0
                        } else {
                            (1.0 - (dist - orb_r) / 3.0).max(0.0)
                        };
                        let idx = (yi * PILL_WIDTH + xi) as usize;
                        let (cr, cg, cb) = if is_speaking {
                            (255.0, 110.0, 150.0) // Neon Rose / Coral
                        } else {
                            (110.0, 180.0, 255.0) // Cyan Blue
                        };
                        let a = (intensity * 250.0) as u8;
                        let r = (cr * intensity) as u8;
                        let g = (cg * intensity) as u8;
                        let b = (cb * intensity) as u8;
                        pixel_slice[idx] = ((a as u32) << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                    }
                }
            }

            // 3. Draw Liquid Waveform in Center (Continuous Liquid Harmonic Ribbon)
            // Just like the wave in the logo: a glossy, luminous 3D wave ribbon
            let wave_start_x = 55.0f32;
            let wave_end_x = w - 85.0f32;
            let wave_w = wave_end_x - wave_start_x;
            let amp = 5.0 + (rms * 18.0).min(18.0); // Dynamic height reacting to mic

            for xi in (wave_start_x as i32)..=(wave_end_x as i32) {
                let x_rel = (xi as f32 - wave_start_x) / wave_w;
                // Dual harmonic wave with smooth window envelope
                let window = (x_rel * PI).sin();
                let wave_y = center_y + (x_rel * 4.0 * PI).sin() * amp * window;

                // Render ribbon thickness of 4px with iridescent gradient
                let ribbon_thick = 3.5f32;
                for yi in ((wave_y - ribbon_thick - 1.0) as i32)..=((wave_y + ribbon_thick + 1.0) as i32) {
                    if yi < 0 || yi >= PILL_HEIGHT {
                        continue;
                    }
                    let dy = (yi as f32 + 0.5 - wave_y).abs();
                    if dy <= ribbon_thick {
                        let ribbon_alpha = (1.0 - (dy / ribbon_thick).powi(2)).max(0.0);
                        let idx = (yi * PILL_WIDTH + xi) as usize;

                        // Color gradient across the wave: Cyan (#60A5FA) -> White Shine -> Amber (#FBBF24)
                        let t = x_rel;
                        let r_wave = 100.0 * (1.0 - t) + 250.0 * t;
                        let g_wave = 180.0 * (1.0 - t) + 200.0 * t;
                        let b_wave = 255.0 * (1.0 - t) + 120.0 * t;

                        // Center of ribbon has a bright white glossy reflection
                        let core_shine = (1.0 - (dy / 1.5).min(1.0)) * 0.45;
                        let r_fin = ((r_wave + core_shine * 255.0) * ribbon_alpha).clamp(0.0, 255.0) as u8;
                        let g_fin = ((g_wave + core_shine * 255.0) * ribbon_alpha).clamp(0.0, 255.0) as u8;
                        let b_fin = ((b_wave + core_shine * 255.0) * ribbon_alpha).clamp(0.0, 255.0) as u8;
                        let a_fin = (ribbon_alpha * 245.0) as u8;

                        // Blend over existing glass pixel
                        pixel_slice[idx] = ((a_fin as u32) << 24) | ((r_fin as u32) << 16) | ((g_fin as u32) << 8) | (b_fin as u32);
                    }
                }
            }

            // 4. Draw Digital Elapsed Timer on Right (Crisp typography)
            render_timer_digits(pixel_slice, PILL_WIDTH - 66, (center_y - 6.0) as i32, &elapsed_str);

            // 5. Transfer to Layered Window via UpdateLayeredWindow
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
                BlendOp: 0,
                BlendFlags: 0,
                SourceConstantAlpha: 255,
                AlphaFormat: 1, // AC_SRC_ALPHA
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

/// Ultra-crisp, high-definition digital timer digits
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
                    let py = start_y + (row as i32) * 2;
                    if px >= 0 && px < PILL_WIDTH && py >= 0 && py + 1 < PILL_HEIGHT {
                        let color = 0xF5F1F5F9; // Soft frosty silver
                        buffer[(py * PILL_WIDTH + px) as usize] = color;
                        buffer[((py + 1) * PILL_WIDTH + px) as usize] = color;
                    }
                }
            }
        }
        cur_x += width + 3;
    }
}
