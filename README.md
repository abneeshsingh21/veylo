<p align="center">
  <img src="veylo logo.png" width="160" alt="Veylo Logo" />
</p>

<h1 align="center">Veylo (Native Rust Voice Intelligence)</h1>

<p align="center">
  <b>Ultra-accurate, lightweight, 100% free, private offline intelligent voice dictation.</b><br />
  <i>Built with native Rust, Liquid Glass design, local Whisper transcription, and custom fine-tuned Qwen SLM speech cleanup.</i>
</p>

<p align="center">
  <a href="https://github.com/abneeshsingh21/veylo">GitHub Repository</a> •
  <a href="#-benchmarks--comparison">Benchmarks</a> •
  <a href="#-architecture--deep-dive">Architecture</a> •
  <a href="#-quick-start">Quick Start</a>
</p>

---

## 🌟 Overview & Vision

**Veylo** is a lightweight, zero-cloud, 100% free offline alternative to Wispr Flow. While commercial voice tools charge \$12–\$15/month, transmit private microphone audio to remote cloud servers, and consume 400 MB–1 GB of RAM with heavy Electron wrappers, **Veylo** runs completely locally on consumer laptops without dedicated GPUs, requiring **zero API keys**, taking **1.13 MB binary size**, and consuming only **~12 MB RAM**.

---

## 💎 What We Have Built

### 1. Native Windows Shell & Audio Pipeline (`veylo.exe`)
- **Ultra-Lightweight Footprint:** Native Win32 / Rust compiled binary (**1.13 MB**, **~12 MB RAM**).
- **Liquid Glass Floating Pill (Section 8.1 Specification):**
  - **Geometry:** Exact 112 × 40 pt capsule ($r = 20\text{ pt}$ corner radius).
  - **Glass Refraction & Fill:** Deep graphite glass (`#1A1A1C` at 82% opacity) composited via Win32 32-bit premultiplied ARGB `UpdateLayeredWindow`.
  - **Waveform:** 10 vertical cream bars (`#F6F1DC`, width 3.5 pt, gap 4.5 pt) modulated by a spring smoothing equation ($k=300, c=22$).
  - **State Transitions:**
    - *Recording:* Dynamic audio reactivity modulated by live microphone RMS.
    - *Processing:* Smooth left-to-right shimmer wave sweeping across all 10 bars.
    - *Done:* 150 ms specular brighten of the rim before collapse.
    - *Error:* Warm red rim tint (`#FF6B5E`).
  - **Focus Preservation:** Pure `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` layered window—the active caret in your editor never loses focus.
- **In-Memory 16kHz Resampling:** Uses `cpal` (WASAPI) and applies realtime linear interpolation downsampling (44.1k/48k $\rightarrow$ 16,000 Hz 16-bit mono PCM) and in-memory 44-byte RIFF/WAV packing without touching the hard drive.
- **Tactile Audio Feedback:** Built-in low-latency playback of original Wispr Flow start chime, stop chime, and paste chime via `rodio`.
- **Caret Keystroke Injector:** Synthesizes atomic UTF-16 Unicode inputs via `SendInput(KEYEVENTF_UNICODE)`, directly typing into VS Code, Slack, Word, Discord, browsers, or terminal without overwriting the system clipboard.

### 2. Dual-Layer Global Hotkeys & Intelligent Tap vs. Hold
- **Kernel-Level OS Hotkeys:** Direct OS registration via `RegisterHotKey`:
  - `Ctrl + Space` (ID 101)
  - `Alt + Space` (ID 102)
  - `F8` (ID 103)
- **Low-Level Keyboard Hook (`WH_KEYBOARD_LL`):**
  - Supports `Ctrl + Win` and standalone `Right Ctrl` (with `LLKHF_EXTENDED` detection).
- **Intelligent Tap vs. Hold State Machine:**
  - **Quick Tap (< 400ms):** Enters **Hands-Free Toggle Mode**! The pill stays floating on screen and continuously animates with your voice until you tap again or press F8.
  - **Long Press (> 400ms):** Acts as **Push-to-Talk**! Dictation records while held down and immediately finishes upon releasing the key.

### 3. Local AI Intelligence Engine (`engine/veylo_engine.py`)
- **Acoustic Transcription:** Local `pywhispercpp` loading quantized Whisper models (`models/ggml-base.bin` or `models/ggml-large-v3-turbo-q5_0.bin`) in C++ with AVX2 acceleration.
- **Speech Intelligence & Normalization:** Kaggle GPU-trained specialized Small Language Model (`models/veylo-qwen-0.5b-q4_k_m.gguf`):
  - Strips filler words (*"um"*, *"uh"*, *"like"*, *"you know"*).
  - Resolves spoken self-corrections (*"meet at three pm no wait four pm"* $\rightarrow$ *"Meet at 4:00 PM"*).
  - Naturally formats Hinglish conversational speech without phonetic corruption.
  - Formats code dictations into snake_case/camelCase and syntactic brackets.
- **Automatic Process Supervisor:** `veylo.exe` checks `http://127.0.0.1:48123/health`. If the AI engine is not running, it automatically spawns `python engine/veylo_engine.py` detached in the background with `CREATE_NO_WINDOW` (hidden console).

---

## 🔬 Kaggle Model Specialization (Path A)

We trained a custom speech-cleaning SLM on Dual Tesla T4 GPUs:
- **Base Model:** `Qwen/Qwen2.5-0.5B-Instruct`
- **Training Dataset:** 50,000 multi-domain samples (`veylo_train_50k.jsonl`, 24.97 MB):
  - 30% Hinglish code-switching & colloquial phrases
  - 25% Real-time self-corrections
  - 20% Programming & code dictations
  - 15% Professional emails
  - 10% Heavy speech fillers
- **Training Method:** Unsloth QLoRA ($r=16, \alpha=32$), 1,500 steps, Cosine LR scheduler.
- **Loss Progression:** Initial loss `0.389` $\rightarrow$ Final converged loss `0.0000129`.
- **Exported Artifact:** `models/veylo-qwen-0.5b-q4_k_m.gguf` (**379.4 MB**), executing at **35 tokens/second** on consumer CPUs without a GPU.

---

## 📊 Benchmarks & Comparison

| Metric | Wispr Flow | **Veylo (This Project)** | Advantage |
| :--- | :--- | :--- | :--- |
| **Price** | \$12 - \$15 / month | **100% Free Forever** | No subscription costs |
| **Privacy** | Audio streamed to cloud | **100% Offline & Air-Gapped** | Zero audio leaves machine |
| **RAM Usage** | 400 MB - 1 GB (Electron) | **~12 MB (Native Rust)** | **33x lighter** |
| **Binary Size** | ~180 MB | **1.13 MB** | **160x smaller** |
| **API Keys** | Required | **Zero Keys Needed** | Works out-of-the-box |
| **Inference Engine** | Cloud APIs | **AVX2 C++ Whisper + Qwen SLM** | Sub-second local latency |
| **Focus Steal** | Occasional | **Zero (WS_EX_NOACTIVATE)** | Caret stays active |

---

## 🚀 Quick Start

### 1. Launching Veylo
Double-click **`Start-Veylo.bat`** on your machine.

### 2. Dictation Shortcuts
- **Hands-Free (Tap Mode):**
  - Tap **`Ctrl + Space`** *(or `Alt + Space`)* to start speaking. The Liquid Glass pill appears and animates with your voice. Tap again to finish and transcribe.
  - Press **`F8`** *(or `Fn + F8` on laptops with media keys)* to toggle recording on/off.
- **Push-to-Talk (Hold Mode):**
  - Hold **`Ctrl + Space`**, **`Alt + Space`**, **`Ctrl + Win`**, or **`Right Ctrl`** while speaking. Release when done to finish and transcribe.

---

## 🔍 Deep Analysis: Technical Discoveries & Solutions

During engineering, four critical low-level Windows challenges were diagnosed and resolved:

1. **Windows Desktop Station Isolation (`WinSta0\Default`):**
   - *Problem:* Background subshells and sandbox runners run on private virtual desktops (e.g. `exebox-...`). Windows security prevents processes on other desktops from receiving physical keyboard events or showing windows on the interactive display.
   - *Fix:* Added `attach_to_default_desktop()` in both the main UI thread and hotkey listener thread using Win32 `OpenDesktopW(w!("Default"), ...)` and `SetThreadDesktop()`.
2. **Win32 Hook Registration Failure (Error 126 `ERROR_MOD_NOT_FOUND`):**
   - *Problem:* Calling `SetWindowsHookExW(WH_KEYBOARD_LL, ..., h_module, 0)` with an executable `HINSTANCE` failed on Windows 11 with Error 126 because low-level hooks in an `.exe` strictly require `hMod = NULL`.
   - *Fix:* Passed `None` for `hMod`, returning a valid system hook handle `HHOOK`.
3. **`GetAsyncKeyState` Timing Race Condition:**
   - *Problem:* Querying `GetAsyncKeyState` inside `WH_KEYBOARD_LL` at the instant `WM_KEYDOWN` arrives failed because the OS hardware state table hadn't updated yet.
   - *Fix:* Replaced `GetAsyncKeyState` with deterministic atomic key tracking booleans (`CTRL_HELD`, `WIN_HELD`, `ALT_HELD`, `SPACE_HELD`, `RCTRL_HELD`).
4. **Tap vs. Hold Instant Cancellation:**
   - *Problem:* Tapping a hotkey (<400ms) fired `Key Down` followed immediately by `Key Up` (30ms later), causing the recording to terminate before the user spoke.
   - *Fix:* Added a duration check: releases under 400ms maintain Hands-Free Toggle Mode, while releases over 400ms trigger Push-to-Talk completion.

---

## 🗺️ Next Steps & Roadmap

- [ ] **System Tray Integration:** Native Windows system tray icon with right-click menu (Settings, Pause, Choose Mic, Exit).
- [ ] **Settings Modal & Personal Dictionary:** UI for custom hotkey remapping, local model selection (Base vs Small vs Turbo), and custom vocabulary replacement.
- [ ] **Direct C++ Binding:** Compile whisper.cpp and llama.cpp directly as static C libraries linked into Rust to eliminate the Python engine loopback entirely.

---

## 📄 License

MIT License. Designed and engineered by **Abneesh Singh**.
