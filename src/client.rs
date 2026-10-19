use crate::context::ActiveContext;
use serde::Deserialize;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::process::Command;
use std::thread;
use std::time::Duration;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const ENGINE_ADDR: &str = "127.0.0.1:48123";
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

#[allow(dead_code)]
#[derive(Deserialize, Debug, Clone)]
pub struct TranscribeResponse {
    pub status: String,
    pub raw: Option<String>,
    pub text: String,
    pub duration_ms: Option<f64>,
}

/// Checks if the local Veylo AI engine is running and responding to health requests.
pub fn is_engine_ready() -> bool {
    let addr: SocketAddr = match ENGINE_ADDR.parse() {
        Ok(a) => a,
        Err(_) => return false,
    };

    if let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(400)) {
        let _ = stream.set_read_timeout(Some(Duration::from_millis(600)));
        let _ = stream.set_write_timeout(Some(Duration::from_millis(400)));
        let req = "GET /health HTTP/1.1\r\nHost: 127.0.0.1:48123\r\nConnection: close\r\n\r\n";
        if stream.write_all(req.as_bytes()).is_ok() {
            let mut buf = [0u8; 256];
            if let Ok(n) = stream.read(&mut buf) {
                let resp = String::from_utf8_lossy(&buf[..n]);
                return resp.contains("200 OK") || resp.contains("ready");
            }
        }
    }
    false
}

/// Spawns the local Veylo Python inference engine in the background without creating a console window.
pub fn ensure_engine_running() {
    if is_engine_ready() {
        println!("[Veylo Engine] Background inference engine is active and ready.");
        return;
    }

    println!("[Veylo Engine] Starting local AI inference engine (Whisper + Qwen SLM)...");
    let mut cmd = Command::new("python");
    cmd.arg("engine/veylo_engine.py");

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

    match cmd.spawn() {
        Ok(_) => {
            println!("[Veylo Engine] Engine process spawned. Waiting for models to initialize...");
            for _ in 0..30 {
                thread::sleep(Duration::from_millis(500));
                if is_engine_ready() {
                    println!("[Veylo Engine] AI models fully loaded and verified online!");
                    return;
                }
            }
            eprintln!("[Veylo Engine] Warning: Engine did not respond within 15 seconds.");
        }
        Err(e) => {
            eprintln!("[Veylo Engine] Failed to spawn python engine: {}. Please ensure python is installed.", e);
        }
    }
}

/// Sends captured 16kHz WAV audio bytes and active window context to the local engine for transcription.
pub fn transcribe(wav_bytes: &[u8], ctx: &ActiveContext) -> Result<TranscribeResponse, String> {
    if wav_bytes.is_empty() {
        return Ok(TranscribeResponse {
            status: "empty".to_string(),
            raw: None,
            text: String::new(),
            duration_ms: Some(0.0),
        });
    }

    let addr: SocketAddr = ENGINE_ADDR
        .parse()
        .map_err(|e| format!("Invalid socket address: {}", e))?;

    let mut stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3))
        .map_err(|e| format!("Could not connect to Veylo engine on {}: {}", ENGINE_ADDR, e))?;

    stream
        .set_read_timeout(Some(Duration::from_secs(45)))
        .map_err(|e| e.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;

    let is_code = if ctx.is_code_editor { "1" } else { "0" };
    let is_email = if ctx.is_email_app { "1" } else { "0" };
    let is_chat = if ctx.is_chat_app { "1" } else { "0" };

    let header = format!(
        "POST /transcribe HTTP/1.1\r\n\
         Host: 127.0.0.1:48123\r\n\
         Content-Type: audio/wav\r\n\
         Content-Length: {}\r\n\
         X-Context-Code: {}\r\n\
         X-Context-Email: {}\r\n\
         X-Context-Chat: {}\r\n\
         Connection: close\r\n\r\n",
        wav_bytes.len(),
        is_code,
        is_email,
        is_chat
    );

    stream
        .write_all(header.as_bytes())
        .map_err(|e| format!("Failed to send HTTP headers: {}", e))?;
    stream
        .write_all(wav_bytes)
        .map_err(|e| format!("Failed to send audio bytes: {}", e))?;
    stream
        .flush()
        .map_err(|e| format!("Flush failed: {}", e))?;

    // Read full HTTP response
    let mut response_bytes = Vec::new();
    stream
        .read_to_end(&mut response_bytes)
        .map_err(|e| format!("Failed to read engine response: {}", e))?;

    let response_str = String::from_utf8_lossy(&response_bytes);
    let body = if let Some(idx) = response_str.find("\r\n\r\n") {
        &response_str[idx + 4..]
    } else {
        &response_str
    };

    let parsed: TranscribeResponse = serde_json::from_str(body.trim())
        .map_err(|e| format!("Failed to parse engine JSON: {} (Body: {})", e, body))?;

    Ok(parsed)
}
