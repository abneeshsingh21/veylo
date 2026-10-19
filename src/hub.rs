use std::path::Path;
use std::process::Command;

pub const HUB_URL: &str = "http://127.0.0.1:48123/hub";

/// Launches the Wispr Flow Section 8.3 Hub Window in a native frameless app container.
pub fn open_hub() {
    println!("[Veylo Hub] Opening Hub window at {}...", HUB_URL);

    // 1. Prefer Microsoft Edge in standalone App Mode (--app=URL)
    let edge_paths = [
        r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
        r"C:\Program Files\Microsoft\Edge\Application\msedge.exe",
    ];

    for path in &edge_paths {
        if Path::new(path).exists() {
            let res = Command::new(path)
                .args([
                    &format!("--app={}", HUB_URL),
                    "--window-size=980,680",
                    "--app-id=veylo_hub",
                ])
                .spawn();
            if res.is_ok() {
                println!("[Veylo Hub] Launched in native Edge App Mode.");
                return;
            }
        }
    }

    // 2. Check for Google Chrome App Mode
    let chrome_paths = [
        r"C:\Program Files\Google\Chrome\Application\chrome.exe",
        r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe",
    ];
    for path in &chrome_paths {
        if Path::new(path).exists() {
            let res = Command::new(path)
                .args([
                    &format!("--app={}", HUB_URL),
                    "--window-size=980,680",
                    "--app-id=veylo_hub",
                ])
                .spawn();
            if res.is_ok() {
                println!("[Veylo Hub] Launched in Chrome App Mode.");
                return;
            }
        }
    }

    // 3. System default fallback
    let _ = Command::new("cmd")
        .args(["/c", "start", HUB_URL])
        .spawn();
    println!("[Veylo Hub] Launched via default browser.");
}
