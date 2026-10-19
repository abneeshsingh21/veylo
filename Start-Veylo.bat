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
echo   - HOLD 'Ctrl + Win' or 'Ctrl + Space' or 'Right Ctrl'
echo   - PRESS 'F8' to toggle recording on/off
echo.
start "" "target\release\veylo.exe"
