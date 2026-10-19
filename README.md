# WhisperFlow-RS

An ultra-accurate, lightweight, 100% free, offline clone of Wispr Flow built in **Rust**.

Designed with reverse-engineering insights from Wispr Flow binaries, featuring a native Windows **Liquid Glass (Acrylic/Mica)** floating pill UI, low-latency audio capture, and local AI intelligence.

## Features
- **100% Free & Offline:** Zero API keys, zero cloud costs, air-gapped privacy.
- **Ultra-Lightweight:** ~25MB native Rust binary (replaces Wispr Flow's 350MB+ Electron footprint).
- **Liquid Glass UI:** Windows 11 DWM Acrylic/Mica blurred floating pill that never steals focus (`WS_EX_NOACTIVATE`).
- **Real-Time Sound Waveform:** Dynamic audio visualizer reacting to spoken speech.
- **Push-to-Talk & Hands-Free:** Global hotkey (`Ctrl + Win` / `Fn`) via low-level keyboard hooks.
- **Context-Aware:** Reads surrounding text using `IUIAutomationTextPattern` before dictation.
- **Hinglish & Accent Mastered:** Fine-tuned small language model (SLM) for filler stripping, self-correction, and natural Indian code-switching.

## Architecture & Reverse Engineering
See [Architecture Document](docs/ARCHITECTURE.md) and [REA Reverse Engineering Findings](docs/REA_FINDINGS.md) for full technical documentation.

## License
MIT
