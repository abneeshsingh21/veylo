# REA Reverse Engineering Findings on Wispr Flow

## 1. Artifact Provenance & Inspection
- **Source Target:** `Wispr Flow Setup.exe` (`https://dl.wisprflow.ai/windows/latest`) -> redirects to `WisprFlowInstaller.exe`
- **Full Payload:** `WisprFlow-1.6.1174-full.nupkg` (Size: 331,745,546 bytes)
- **Extracted Binary:** `Wispr Flow Helper.exe` (SHA256: `4d802d1902e215bb2c2dacdf900253eed74b4cbc90e38f635bea306b206f2b8e`)

## 2. Key Architecture Findings
1. **Wispr Flow's Core is Already Rust:**
   The desktop client offloads all performance-critical and OS-level operations to `Wispr Flow Helper.exe`, which was confirmed to be compiled with `rustc`.
   - Electron (~235MB) acts only as the UI shell and audio streamer.
   - The native Rust helper communicates with the Electron app via Named Pipes / Windows IPC.
   - *Our Implementation Improvement:* Eliminate the 235MB Electron layer entirely. Replace it with a single, standalone 25MB Rust executable.

2. **Context Inspection (`IUIAutomationTextPattern`):**
   - Wispr Flow queries the Windows UI Automation tree (`IUIAutomationTextPattern`, `ResolveKeystrokeScopeWindow`) to extract surrounding text at the current cursor before transcribing.
   - This provides the LLM with surrounding context to distinguish homophones and apply proper formatting.

3. **Keystroke Simulation & Hook:**
   - Identified symbols: `SetWindowsHookExW` for low-level keyboard monitoring (`WH_KEYBOARD_LL`).
   - `SendInput` (`get_WindowsKeyUpSimulation`, `set_WindowsKeyUpSimulation`) for typing text directly into the focused window.

4. **Sound Assets:**
   - Extracted the exact sound cues from `resources/assets/sounds/v12/`:
     - `dictation-start.wav` (43,130 bytes): Soft rising audio cue.
     - `dictation-stop.wav` (80,342 bytes): Soft descending audio cue.
     - `paste.wav` (41,870 bytes): Subtle mechanical keystroke confirmation.

5. **Window Positioning & Non-Activating Behavior:**
   - Symbols: `PositionBrowserWindowMode`, `SystemSettingsWindowBounds`, `TryEnumerateVisibleWindows`.
   - The floating indicator positions itself dynamically near the user's active window and avoids taking window focus (`WS_EX_NOACTIVATE`).
