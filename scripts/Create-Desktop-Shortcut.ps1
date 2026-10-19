$WshShell = New-Object -ComObject WScript.Shell
$DesktopPath = [System.Environment]::GetFolderPath('Desktop')
$ShortcutPath = Join-Path $DesktopPath "Veylo Voice AI.lnk"

$TargetDir = Split-Path -Parent $PSScriptRoot
$ExePath = Join-Path $TargetDir "target\release\veylo.exe"
$IconPath = Join-Path $TargetDir "assets\icons\veylo.ico"

if (-not (Test-Path $ExePath)) {
    $ExePath = Join-Path $TargetDir "Start-Veylo.bat"
}

$Shortcut = $WshShell.CreateShortcut($ShortcutPath)
$Shortcut.TargetPath = $ExePath
$Shortcut.WorkingDirectory = $TargetDir
$Shortcut.Description = "Veylo - Intelligent Offline Voice Dictation"
if (Test-Path $IconPath) {
    $Shortcut.IconLocation = "$IconPath,0"
}
$Shortcut.Save()

Write-Host "Created Desktop Shortcut: $ShortcutPath" -ForegroundColor Green
