<p align="center">
  <img src="veylo logo.png" width="160" alt="Veylo Logo" />
</p>

<h1 align="center">Veylo (Native Rust Voice AI)</h1>

<p align="center">
  <b>Ultra-accurate, lightweight, 100% free, offline intelligent voice dictation.</b><br />
  <i>Built with native Rust, Liquid Glass design, local Whisper transcription, and custom fine-tuned SLM speech cleanup.</i>
</p>

---

## 💎 Features

- **100% Free & Air-Gapped Offline:** Zero API keys, zero subscription fees, and complete data privacy.
- **Ultra-Lightweight (~1 MB Native Binary):** Runs on only ~12 MB RAM—eliminating Wispr Flow's 400 MB+ heavy Electron overhead.
- **Authentic Liquid Glass Floating Pill:** Custom per-pixel alpha blended capsule UI with dynamic spring physics ($k=300, c=22$) and 10 cream animated waveform bars (`#F6F1DC`).
- **Never Steals Focus (`WS_EX_NOACTIVATE`):** Dictate directly into VS Code, Slack, Word, Discord, or any browser seamlessly.
- **Tactile Sound Cues:** Built-in low-latency audio feedback for start, stop, and paste events.
- **Global Low-Level Hotkeys:** 
  - `F8`: One-key toggle on/off
  - `Ctrl + Win` / `Ctrl + Space` / `Right Ctrl`: Instant Push-to-Talk (Hold-to-Talk)
- **Context-Aware Speech Intelligence:** UI Automation context reader paired with local SLM for filler removal, Hinglish speech cleanup, and code formatting.

## 🚀 Quick Start

1. Double-click **`Start-Veylo.bat`** on your Desktop.
2. Press **`F8`** or hold **`Ctrl + Win`** to speak.
3. Release or press `F8` again to finish and transcribe.

## 📐 Architecture & Reverse Engineering
- Detailed system specifications: [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)
- Reverse-engineered findings: [`docs/REA_FINDINGS.md`](docs/REA_FINDINGS.md)

## 📄 License
MIT License - Created by Abneesh Singh.
