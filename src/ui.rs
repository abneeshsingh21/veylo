use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;
use windows::core::w;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, POINT, SIZE, WPARAM};
use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, GetMonitorInfoW,
    MonitorFromPoint, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    BLENDFUNCTION, DIB_RGB_COLORS, MONITORINFO, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetCursorPos, RegisterClassW, SetWindowPos, ShowWindow,
    UpdateLayeredWindow, HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE,
    ULW_ALPHA, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_POPUP, WM_ERASEBKGND,
};

// Exact specification dimensions
// Pill recording size: 112 x 40 pt. With shadow padding: 136 x 64 px.
pub const PILL_W: f32 = 112.0;
pub const PILL_H: f32 = 40.0;
pub const SHADOW_PAD: i32 = 12;
pub const CANVAS_W: i32 = (PILL_W as i32) + SHADOW_PAD * 2; // 136
pub const CANVAS_H: i32 = (PILL_H as i32) + SHADOW_PAD * 2; // 64

pub const NUM_BARS: usize = 10;
pub const BAR_WIDTH: f32 = 3.5;
pub const BAR_GAP: f32 = 4.5;
pub const MIN_BAR_H: f32 = 8.0;
pub const MAX_BAR_H: f32 = 22.0;

// Colors matching exact specification:
// Fill: Deep graphite glass #1A1A1C at ~82% opacity
// Outline: #F2F2F0 at 90% opacity, 1.5 pt
// Waveform bars: Cream #F6F1DC
const COLOR_GRAPHITE_R: f32 = 26.0;
const COLOR_GRAPHITE_G: f32 = 26.0;
const COLOR_GRAPHITE_B: f32 = 28.0;
const COLOR_GRAPHITE_A: f32 = 0.82;

const COLOR_CREAM_R: u8 = 246;
const COLOR_CREAM_G: u8 = 241;
const COLOR_CREAM_B: u8 = 220;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PillMode {
    Idle,
    Recording,
    Processing,
    Done,
    Error,
}

pub struct FloatingPill {
    hwnd: HWND,
    current_rms: Arc<AtomicU32>,
    is_visible: Arc<AtomicBool>,
    mode: Arc<parking_lot::Mutex<PillMode>>,
    bars: Arc<parking_lot::Mutex<[SpringBar; NUM_BARS]>>,
    mode_start: Arc<parking_lot::Mutex<Instant>>,
}

unsafe impl Send for FloatingPill {}
unsafe impl Sync for FloatingPill {}

static mut GLOBAL_PILL: Option<Arc<FloatingPillState>> = None;

#[allow(dead_code)]
struct FloatingPillState {
    hwnd: HWND,
}

#[derive(Clone, Copy)]
pub struct SpringBar {
    pub current_h: f32,
    pub velocity: f32,
    pub target_h: f32,
}

impl SpringBar {
    fn new() -> Self {
        Self {
            current_h: MIN_BAR_H,
            velocity: 0.0,
            target_h: MIN_BAR_H,
        }
    }

    fn update(&mut self, dt: f32) {
        // Spring smoothing: stiffness ~300, damping ~22
        let stiffness = 300.0f32;
        let damping = 22.0f32;
        let force = -stiffness * (self.current_h - self.target_h) - damping * self.velocity;
        self.velocity += force * dt;
        self.current_h += self.velocity * dt;
        self.current_h = self.current_h.clamp(MIN_BAR_H, MAX_BAR_H);
    }
}

impl FloatingPill {
    pub fn new() -> Result<Self, String> {
        let is_visible = Arc::new(AtomicBool::new(false));
        let current_rms = Arc::new(AtomicU32::new(0));
        let mode = Arc::new(parking_lot::Mutex::new(PillMode::Idle));
        let bars = Arc::new(parking_lot::Mutex::new([SpringBar::new(); NUM_BARS]));
        let mode_start = Arc::new(parking_lot::Mutex::new(Instant::now()));

        unsafe {
            let class_name = w!("WhisperFlowLiquidPill");
            let wc = WNDCLASSW {
                lpfnWndProc: Some(Self::wnd_proc),
                lpszClassName: class_name,
                ..Default::default()
            };
            let _ = RegisterClassW(&wc);

            // Position at bottom-center of active monitor, 24 pt above taskbar
            let (pos_x, pos_y) = Self::calculate_screen_position();

            // Layered + Topmost + ToolWindow + NoActivate
            let ex_style = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED;

            let hwnd = CreateWindowExW(
                ex_style,
                class_name,
                w!("WhisperFlowPill"),
                WS_POPUP,
                pos_x,
                pos_y,
                CANVAS_W,
                CANVAS_H,
                None,
                None,
                None,
                None,
            )
            .map_err(|e| format!("CreateWindowExW failed: {}", e))?;

            GLOBAL_PILL = Some(Arc::new(FloatingPillState { hwnd }));

            let pill = Self {
                hwnd,
                current_rms,
                is_visible,
                mode,
                bars,
                mode_start,
            };

            pill.render();

            Ok(pill)
        }
    }

    /// Calculates position on the active screen, 24 pt above the taskbar
    pub fn calculate_screen_position() -> (i32, i32) {
        unsafe {
            let mut cursor_pos = POINT::default();
            let _ = GetCursorPos(&mut cursor_pos);

            let hmonitor = MonitorFromPoint(cursor_pos, MONITOR_DEFAULTTONEAREST);
            let mut mi = MONITORINFO {
                cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                ..Default::default()
            };

            if GetMonitorInfoW(hmonitor, &mut mi).as_bool() {
                let work_rect = mi.rcWork;
                let screen_w = work_rect.right - work_rect.left;
                let pos_x = work_rect.left + (screen_w - CANVAS_W) / 2;
                // 24 pt above taskbar (taking work_rect.bottom into account)
                let pos_y = work_rect.bottom - CANVAS_H - 24;
                (pos_x, pos_y)
            } else {
                (200, 200)
            }
        }
    }

    pub fn set_mode(&self, new_mode: PillMode) {
        *self.mode.lock() = new_mode;
        *self.mode_start.lock() = Instant::now();
        self.render();
    }

    pub fn show(&self) {
        self.is_visible.store(true, Ordering::Release);
        self.set_mode(PillMode::Recording);

        // Reposition dynamically to active monitor before showing
        let (pos_x, pos_y) = Self::calculate_screen_position();
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                HWND_TOPMOST,
                pos_x,
                pos_y,
                CANVAS_W,
                CANVAS_H,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
        }
    }

    pub fn hide(&self) {
        self.is_visible.store(false, Ordering::Release);
        self.set_mode(PillMode::Idle);
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    pub fn update_rms(&self, rms: f32) {
        self.current_rms.store(rms.to_bits(), Ordering::Relaxed);
        if !self.is_visible.load(Ordering::Relaxed) {
            return;
        }

        let mode = *self.mode.lock();
        let dt = 0.016f32; // ~60 FPS

        // Center bias for the 10 bars (indices 0..9)
        let center_weights = [0.42, 0.60, 0.80, 0.95, 1.0, 1.0, 0.95, 0.80, 0.60, 0.42];

        // Idle breathing sine wave
        let elapsed = self.mode_start.lock().elapsed().as_secs_f32();
        let breathe = (elapsed * 3.0).sin() * 0.5 + 0.5;

        let mut bars = self.bars.lock();
        for i in 0..NUM_BARS {
            match mode {
                PillMode::Recording => {
                    let weight = center_weights[i];
                    // Map RMS audio level to height with idle breathing floor
                    let audio_target = MIN_BAR_H + (MAX_BAR_H - MIN_BAR_H) * (rms * 1.8 * weight).min(1.0);
                    let idle_target = MIN_BAR_H + breathe * 2.0 * weight;
                    bars[i].target_h = audio_target.max(idle_target);
                }
                PillMode::Processing => {
                    // Shimmer wave sweeping left to right across the 10 bars
                    let wave_phase = (elapsed * 6.0 - (i as f32) * 0.6).sin() * 0.5 + 0.5;
                    bars[i].target_h = MIN_BAR_H + wave_phase * (MAX_BAR_H - MIN_BAR_H) * 0.6;
                }
                _ => {
                    bars[i].target_h = MIN_BAR_H;
                }
            }
            bars[i].update(dt);
        }

        self.render();
    }

    /// Renders the exact specification:
    /// - 112 x 40 pt capsule
    /// - Deep graphite glass #1A1A1C at ~82% opacity
    /// - 1.5 pt inner stroke #F2F2F0 at 90% opacity
    /// - Soft outer shadow (0 6 20 rgba(0,0,0,0.35))
    /// - Top rim specular highlight + bottom inner shadow + chromatic fringe
    /// - 10 vertical cream bars (#F6F1DC), rounded caps, width 3.5, gap 4.5
    fn render(&self) {
        unsafe {
            let screen_dc = GetDC(None);
            let mem_dc = CreateCompatibleDC(screen_dc);

            let bmi = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: CANVAS_W,
                    biHeight: -CANVAS_H, // Top-down
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

            let pixel_slice = std::slice::from_raw_parts_mut(
                bits as *mut u32,
                (CANVAS_W * CANVAS_H) as usize,
            );
            pixel_slice.fill(0); // Fully transparent background

            let current_mode = *self.mode.lock();
            let bars_copy = *self.bars.lock();

            // Geometry coordinates inside the padded canvas
            let pad_x = SHADOW_PAD as f32;
            let pad_y = SHADOW_PAD as f32;
            let pill_w = PILL_W;
            let pill_h = PILL_H;
            let pill_r = pill_h / 2.0; // 20.0 pt perfect capsule
            let pill_half_len = (pill_w / 2.0) - pill_r; // 56 - 20 = 36 pt
            let pill_cx = pad_x + pill_w / 2.0;
            let pill_cy = pad_y + pill_h / 2.0;

            // 1. Draw Capsule Base with Shadow, Glass Refraction, and Specular Rim
            for y_i in 0..CANVAS_H {
                let y = y_i as f32 + 0.5;
                for x_i in 0..CANVAS_W {
                    let x = x_i as f32 + 0.5;

                    let dx = (x - pill_cx).abs() - pill_half_len;
                    let clamped_dx = dx.max(0.0);
                    let dy = (y - pill_cy).abs();
                    let dist = (clamped_dx * clamped_dx + dy * dy).sqrt();
                    let d = dist - pill_r;

                    // --- Soft Outer Shadow (0 6 20 rgba(0,0,0,0.35)) ---
                    // Shift shadow center down by 4px
                    let shadow_dy = (y - (pill_cy + 4.0)).abs();
                    let shadow_dist = (clamped_dx * clamped_dx + shadow_dy * shadow_dy).sqrt() - pill_r;

                    if d > 0.5 {
                        if shadow_dist < 10.0 {
                            let s_factor = (1.0 - (shadow_dist / 10.0).clamp(0.0, 1.0)).powi(2);
                            let s_alpha = s_factor * 0.35;
                            let a_b = (s_alpha * 255.0) as u8;
                            pixel_slice[(y_i * CANVAS_W + x_i) as usize] = (a_b as u32) << 24;
                        }
                        continue;
                    }

                    // --- Pill Interior & Border ---
                    let edge_aa = (1.0 - (d / 1.0).max(0.0)).min(1.0);

                    // Deep graphite glass #1A1A1C at ~82% opacity
                    let mut alpha = COLOR_GRAPHITE_A * edge_aa;
                    let mut r = COLOR_GRAPHITE_R;
                    let mut g = COLOR_GRAPHITE_G;
                    let mut b = COLOR_GRAPHITE_B;

                    // Subtle bottom inner shadow
                    if y > pill_cy && d < -2.0 {
                        let bottom_factor = ((y - pill_cy) / pill_r).clamp(0.0, 1.0) * 0.25;
                        r *= 1.0 - bottom_factor;
                        g *= 1.0 - bottom_factor;
                        b *= 1.0 - bottom_factor;
                    }

                    // 1.5 pt Inner Stroke: #F2F2F0 at 90% opacity
                    // With subtle chromatic fringe at the rim (blue on left, warm on right)
                    if d >= -2.0 && d <= 0.2 {
                        let stroke_t = 1.0 - ((d + 0.9).abs() / 1.1).clamp(0.0, 1.0);
                        let stroke_alpha = stroke_t * 0.90;

                        // Base stroke color #F2F2F0
                        let mut sr = 242.0f32;
                        let mut sg = 242.0f32;
                        let mut sb = 240.0f32;

                        // Error mode: warm red rim tint #FF6B5E
                        if current_mode == PillMode::Error {
                            sr = 255.0;
                            sg = 107.0;
                            sb = 94.0;
                        } else {
                            // Light chromatic fringe at the rim only
                            let norm_x = (x - pad_x) / pill_w;
                            if norm_x < 0.3 {
                                sb += 15.0; // Faint blue fringe on left
                            } else if norm_x > 0.7 {
                                sr += 15.0; // Faint amber/rose fringe on right
                            }
                        }

                        r = r * (1.0 - stroke_alpha) + sr * stroke_alpha;
                        g = g * (1.0 - stroke_alpha) + sg * stroke_alpha;
                        b = b * (1.0 - stroke_alpha) + sb * stroke_alpha;
                        alpha = (alpha + stroke_alpha * 0.25).min(1.0);
                    }

                    // Specular Highlight along the top rim (thin gradient arc)
                    if d >= -2.5 && d <= -0.2 && y < pill_cy - 4.0 {
                        let top_proximity = 1.0 - ((y - (pill_cy - pill_r)) / 6.0).clamp(0.0, 1.0);
                        let spec_strength = top_proximity * 0.45;
                        r += 255.0 * spec_strength;
                        g += 255.0 * spec_strength;
                        b += 255.0 * spec_strength;
                        alpha = (alpha + spec_strength * 0.2).min(1.0);
                    }

                    // Premultiply alpha for UpdateLayeredWindow
                    let a_byte = (alpha * 255.0).clamp(0.0, 255.0) as u8;
                    let r_byte = (r * alpha).clamp(0.0, 255.0) as u8;
                    let g_byte = (g * alpha).clamp(0.0, 255.0) as u8;
                    let b_byte = (b * alpha).clamp(0.0, 255.0) as u8;

                    pixel_slice[(y_i * CANVAS_W + x_i) as usize] =
                        ((a_byte as u32) << 24) | ((r_byte as u32) << 16) | ((g_byte as u32) << 8) | (b_byte as u32);
                }
            }

            // 2. Draw 10 Vertical Waveform Bars (Cream #F6F1DC with Rounded Caps)
            // Total width for 10 bars: 10 * 3.5 + 9 * 4.5 = 75.5 pt
            let total_bars_w = (NUM_BARS as f32) * BAR_WIDTH + ((NUM_BARS - 1) as f32) * BAR_GAP;
            let bars_start_x = pill_cx - (total_bars_w / 2.0);

            for i in 0..NUM_BARS {
                let bx = bars_start_x + (i as f32) * (BAR_WIDTH + BAR_GAP);
                let bar_h = bars_copy[i].current_h;
                let half_h = bar_h / 2.0;
                let cap_r = BAR_WIDTH / 2.0; // 1.75 pt radius for rounded caps
                let by_top = pill_cy - half_h;
                let by_bot = pill_cy + half_h;

                // Render each bar as a rounded capsule
                let min_x = (bx - 0.5) as i32;
                let max_x = (bx + BAR_WIDTH + 0.5) as i32;
                let min_y = (by_top - 1.0) as i32;
                let max_y = (by_bot + 1.0) as i32;

                for py in min_y..=max_y {
                    if py < 0 || py >= CANVAS_H {
                        continue;
                    }
                    let cy_pos = (py as f32 + 0.5).clamp(by_top + cap_r, by_bot - cap_r);
                    for px in min_x..=max_x {
                        if px < 0 || px >= CANVAS_W {
                            continue;
                        }
                        let cx_pos = bx + cap_r;
                        let d_bar = (((px as f32 + 0.5 - cx_pos).powi(2) + (py as f32 + 0.5 - cy_pos).powi(2)).sqrt()) - cap_r;

                        if d_bar <= 0.5 {
                            let bar_aa = (1.0 - (d_bar / 0.5).max(0.0)).min(1.0);
                            let a_val = (bar_aa * 255.0) as u8;
                            let r_val = ((COLOR_CREAM_R as f32) * bar_aa) as u8;
                            let g_val = ((COLOR_CREAM_G as f32) * bar_aa) as u8;
                            let b_val = ((COLOR_CREAM_B as f32) * bar_aa) as u8;

                            let idx = (py * CANVAS_W + px) as usize;
                            pixel_slice[idx] =
                                ((a_val as u32) << 24) | ((r_val as u32) << 16) | ((g_val as u32) << 8) | (b_val as u32);
                        }
                    }
                }
            }

            // 3. Blit to Screen via UpdateLayeredWindow
            let (pos_x, pos_y) = Self::calculate_screen_position();
            let pt_dst = POINT { x: pos_x, y: pos_y };
            let size = SIZE {
                cx: CANVAS_W,
                cy: CANVAS_H,
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
        match msg {
            WM_ERASEBKGND => LRESULT(1),
            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}
