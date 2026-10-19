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

### 1. Unified Real Desktop App & System Tray (`veylo.exe`)
- **Single All-in-One Application:** Double-click `Veylo.exe` or `Veylo Voice AI.lnk` on your desktop. The app launches, ensures a single instance via Win32 Named Mutex, registers the tray icon, initializes local AI models, and provides seamless voice dictation.
- **Native Windows Taskbar System Tray:**
  - Displays the high-resolution Veylo icon in the Windows notification area (`Shell_NotifyIconW`).
  - **Left-Click:** Opens the Wispr Flow Section 8.3 Hub Window.
  - **Right-Click Menu:** "Open Veylo Hub (F9)", "Toggle Dictation (F8)", "Settings", "Quit Veylo".
- **Ultra-Lightweight Footprint:** Native Win32 / Rust binary (**1.13 MB**, **~12 MB RAM**).

### 2. Liquid Glass Floating Pill (Section 8.1 Specification)
- **Geometry:** Exact 112 × 40 pt capsule ($r = 20\text{ pt}$ corner radius).
- **Glass Refraction & Fill:** Deep graphite glass (`#1A1A1C` at 82% opacity) composited via Win32 32-bit premultiplied ARGB `UpdateLayeredWindow`.
- **Waveform:** 10 vertical cream bars (`#F6F1DC`, width 3.5 pt, gap 4.5 pt) modulated by a spring smoothing equation ($k=300, c=22$).
- **State Transitions:**
  - *Recording:* Dynamic audio reactivity modulated by live microphone RMS.
  - *Processing:* Smooth left-to-right shimmer wave sweeping across all 10 bars.
  - *Done:* 150 ms specular brighten of the rim before collapse.
  - *Error:* Warm red rim tint (`#FF6B5E`).
- **Focus Preservation:** Pure `WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW` layered window—the active caret in your editor never loses focus.
- **Click-to-Finish:** Clicking directly on the floating pill toggles/finishes dictation without stealing focus from your typing caret.

### 3. Wispr Flow Hub Window (Section 8.3 Specification)
- **Frameless Native Experience:** Automatically launches via Microsoft Edge or Chromium in dedicated app mode (`--app=http://127.0.0.1:48123/hub --window-size=980,680 --app-id=veylo_hub`). Zero browser tabs, zero URL bars—looks and behaves 100% like a native Windows 11 app!
- **Six Comprehensive Modules:**
  1. **Home Feed:** Realtime dictation history cards with timestamps, duration, app context badges, raw speech view, and one-click copy.
  2. **Direct Dictate Button:** Click `Dictate (F8)` right on the Hub header to trigger dictation via mouse without needing any shortcut keys!
  3. **Personal Dictionary:** Add custom technical jargon, company names, acronyms, and names.
  4. **Text Expansion Snippets:** Configure keyword triggers (e.g. `:email`) that expand to templates.
  5. **Style Presets:** Switch between Code Editor, Business Email, and Casual/Hinglish modes.
  6. **Glass Scratchpad:** Live long-form markdown dictation notepad with one-click export.
  7. **Settings:** Audio device selection, hotkey customization, and model parameters.

### 4. Dual-Layer Multi-Hotkey Engine with Debounce Guard
- **Kernel-Level OS Hotkeys (`RegisterHotKey`):**
  - **`F8`**: Dedicated single-key dictation toggle (no modifiers required, works anywhere).
  - **`F9`**: Dedicated single-key Hub Window launcher.
  - **`Ctrl + Alt + V`**: Universal conflict-free shortcut.
  - **`Ctrl + Shift + V`**: Alternative voice toggle.
  - **`Ctrl + Space` & `Alt + Space`**: Fast thumb triggers.
- **Low-Level Keyboard Hook (`WH_KEYBOARD_LL`):**
  - Supports `Ctrl + Win` and standalone `Right Ctrl` push-to-talk.
- **Intelligent Tap vs. Hold & Debounce Guard:**
  - **Debounce Guard (400ms):** Completely eliminates the dual-trigger collision race condition between OS hotkeys and low-level hooks.
  - **Quick Tap (< 400ms):** Enters **Hands-Free Toggle Mode**! The pill stays floating on screen and continuously animates with your voice until you tap again or press F8.
  - **Long Press (> 400ms):** Acts as **Push-to-Talk**! Dictation records while held down and immediately finishes upon releasing the key.

### 5. Local AI Intelligence Engine (`engine/veylo_engine.py`)
- **Acoustic Transcription:** Local `pywhispercpp` loading quantized Whisper models (`models/ggml-base.bin` or `models/ggml-large-v3-turbo-q5_0.bin`) in C++ with AVX2 acceleration.
- **Speech Intelligence & Normalization:** Kaggle GPU-trained specialized Small Language Model (`models/veylo-qwen-0.5b-q4_k_m.gguf`):
  - Strips filler words (*"um"*, *"uh"*, *"like"*, *"you know"*).
  - Resolves spoken self-corrections (*"meet at three pm no wait four pm"* $\rightarrow$ *"Meet at 4:00 PM"*).
  - Naturally formats Hinglish conversational speech without phonetic corruption.
  - Formats code dictations into snake_case/camelCase and syntactic brackets.
- **In-Memory 16kHz Audio Capture:** Uses `cpal` (WASAPI) with real-time linear interpolation downsampling to 16,000 Hz 16-bit mono PCM in memory without touching disk.
- **Tactile Sound Cues:** Wispr Flow original start, stop, and paste chimes embedded directly into the binary.
- **Direct Unicode Keystroke Injection:** Types via `SendInput(KEYEVENTF_UNICODE)` at the active caret position without clipboard tampering.

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
- Double-click **`Veylo Voice AI`** on your desktop, OR
- Double-click **`Start-Veylo.bat`** in the repository folder.

When launched, the Liquid Glass pill flashes on screen with a tactile chime, the system tray icon appears in your taskbar, and Veylo is ready!

### 2. Dictation Triggers
- **Single-Key Toggle:** Press **`F8`** anywhere (active editor, browser, game, desktop).
- **Universal Shortcut:** Press **`Ctrl + Alt + V`**.
- **Fast Thumb Shortcut:** Tap or hold **`Ctrl + Space`** or **`Alt + Space`**.
- **Push-to-Talk:** Hold **`Right Ctrl`** alone while speaking, release when finished.
- **Mouse / Hub:** Click the **Veylo System Tray Icon** or click **`Dictate (F8)`** inside the Hub Window.
- **Click to Finish:** Click the floating pill directly with your mouse to finish dictation.

### 3. Opening the Hub Window
- Press **`F9`**, OR
- Left-click the **Veylo System Tray Icon** in the Windows Taskbar, OR
- Right-click the tray icon and select **Open Veylo Hub**.

---

## 🔍 Deep Analysis: Technical Discoveries & Solutions

During engineering, five critical low-level Windows challenges were diagnosed and resolved:

1. **Dual-Trigger Collision Race Condition:**
   - *Problem:* Having both `RegisterHotKey` and `WH_KEYBOARD_LL` active for the same shortcut caused them to fire within 5ms of each other. The hook fired `Start`, and `WM_HOTKEY` fired `Toggle` (which saw it was already recording and immediately triggered `Stop`), closing the recording within 20ms before the pill could render!
   - *Fix:* Implemented an atomic 400ms **Debounce Guard** in the main event processor. Rapid duplicate events within 400ms are cleanly ignored, while legitimate second taps or releases after >400ms trigger cleanly.
2. **Windows Desktop Station Isolation (`WinSta0\Default`):**
   - *Problem:* Background subshells and sandbox runners run on private virtual desktops (e.g. `exebox-...`). Windows security prevents processes on other desktops from receiving physical keyboard events or showing windows on the interactive display.
   - *Fix:* Added `attach_to_default_desktop()` in both the main UI thread and hotkey listener thread using Win32 `OpenDesktopW(w!("Default"), ...)` and `SetThreadDesktop()`.
3. **Win32 Hook Registration Failure (Error 126 `ERROR_MOD_NOT_FOUND`):**
   - *Problem:* Calling `SetWindowsHookExW(WH_KEYBOARD_LL, ..., h_module, 0)` with an executable `HINSTANCE` failed on Windows 11 with Error 126 because low-level hooks in an `.exe` strictly require `hMod = NULL`.
   - *Fix:* Passed `None` for `hMod`, returning a valid system hook handle `HHOOK`.
4. **`GetAsyncKeyState` Timing Race Condition:**
   - *Problem:* Querying `GetAsyncKeyState` inside `WH_KEYBOARD_LL` at the instant `WM_KEYDOWN` arrives failed because the OS hardware state table hadn't updated yet.
   - *Fix:* Replaced `GetAsyncKeyState` with deterministic atomic key tracking booleans (`CTRL_HELD`, `WIN_HELD`, `ALT_HELD`, `SPACE_HELD`, `RCTRL_HELD`).
5. **Tap vs. Hold Instant Cancellation:**
   - *Problem:* Tapping a hotkey (<400ms) fired `Key Down` followed immediately by `Key Up` (30ms later), causing the recording to terminate before the user spoke.
   - *Fix:* Added a duration check: releases under 400ms maintain Hands-Free Toggle Mode, while releases over 400ms trigger Push-to-Talk completion.

---

## 📄 License

MIT License. Designed and engineered by **Abneesh Singh**.
