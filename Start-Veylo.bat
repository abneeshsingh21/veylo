@echo off
title Veylo - Offline Voice Intelligence
cd /d "%~dp0"

echo ======================================================
echo              VEYLO - Voice Intelligence
echo    100% Free - Offline - Ultra-Lightweight - Private
echo ======================================================
echo.

if not exist "target\release\veylo.exe" (
    echo [Setup] Building Veylo release binary...
    cargo build --release
)

echo [Veylo] Launching Veylo Voice AI...
echo [Shortcuts]
echo   - PRESS 'F8' to toggle recording on/off (Single Key!)
echo   - PRESS 'F9' to open Veylo Hub window
echo   - PRESS 'Ctrl + Alt + V' (Universal voice shortcut)
echo   - TAP or HOLD 'Ctrl + Space' or 'Alt + Space'
echo   - HOLD 'Right Ctrl' alone (Push-to-Talk)
echo   - CLICK the Veylo taskbar tray icon anytime
echo.
"target\release\veylo.exe"
