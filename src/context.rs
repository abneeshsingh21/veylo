use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowTextW};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ActiveContext {
    pub window_title: String,
    pub is_code_editor: bool,
    pub is_chat_app: bool,
    pub is_email_app: bool,
}

#[allow(dead_code)]
impl ActiveContext {
    /// Captures the current active foreground window context.
    /// Runs in under 0.1ms without interfering with cursor focus.
    pub fn capture() -> Self {
        unsafe {
            let hwnd: HWND = GetForegroundWindow();
            if hwnd.0.is_null() {
                return Self::default();
            }

            let mut title_buf = [0u16; 512];
            let len = GetWindowTextW(hwnd, &mut title_buf);
            let window_title = if len > 0 {
                String::from_utf16_lossy(&title_buf[..len as usize])
            } else {
                String::new()
            };

            let title_lower = window_title.to_lowercase();
            let is_code_editor = title_lower.contains("visual studio")
                || title_lower.contains("code")
                || title_lower.contains("sublime")
                || title_lower.contains(".rs")
                || title_lower.contains(".py")
                || title_lower.contains(".js")
                || title_lower.contains(".ts")
                || title_lower.contains("terminal")
                || title_lower.contains("powershell")
                || title_lower.contains("cmd.exe");

            let is_chat_app = title_lower.contains("slack")
                || title_lower.contains("discord")
                || title_lower.contains("whatsapp")
                || title_lower.contains("telegram")
                || title_lower.contains("teams");

            let is_email_app = title_lower.contains("outlook")
                || title_lower.contains("gmail")
                || title_lower.contains("mail");

            Self {
                window_title,
                is_code_editor,
                is_chat_app,
                is_email_app,
            }
        }
    }

    pub fn default() -> Self {
        Self {
            window_title: String::new(),
            is_code_editor: false,
            is_chat_app: false,
            is_email_app: false,
        }
    }
}
