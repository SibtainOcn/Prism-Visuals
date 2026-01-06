<!--
PURPOSE: Testing commands reference for Prism Visuals
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# Testing Guide

## Quick Reference

| Test | Command |
|------|---------|
| View config | `Get-Content "$env:APPDATA\Prism Visuals\config.json" \| ConvertFrom-Json` |
| List wallpapers | `Get-ChildItem "$env:USERPROFILE\Pictures\Prism Visuals" -Filter "*.jpg"` |
| Run auto-change | `.\target\release\visuals.exe auto-change` |
| Build release | `cargo build --release` |
| Kill process | `taskkill /F /IM visuals.exe 2>&1` |

---

## Test Window Flicker Fix

```powershell
# Create 1-minute test schedule
.\visuals.exe tf

# Close console, wait 1 minute
# Wallpaper should change with NO window flash

# Stop test
.\visuals.exe unset
```

### Verify VBS File

```powershell
Get-ChildItem "C:\Program Files\Prism Visuals\prism_auto_change.vbs"
```

---

## View Config

```powershell
# Full config
$configPath = "$env:APPDATA\Prism Visuals\config.json"
Get-Content $configPath | ConvertFrom-Json | ConvertTo-Json -Depth 3

# Auto-change fields only
$config = Get-Content "$env:APPDATA\Prism Visuals\config.json" | ConvertFrom-Json
$config | Select-Object auto_change_index, auto_change_enabled, last_auto_change
```

---

## List Wallpapers

```powershell
# All wallpapers with details
$wallpaperDir = "$env:USERPROFILE\Pictures\Prism Visuals"
Get-ChildItem $wallpaperDir -Filter "*.jpg" | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize

# By source
Get-ChildItem "$env:USERPROFILE\Pictures\Prism Visuals" -Filter "bing_*.jpg" | Select-Object Name
Get-ChildItem "$env:USERPROFILE\Pictures\Prism Visuals" -Filter "unsplash_*.jpg" | Select-Object Name

# Count
(Get-ChildItem "$env:USERPROFILE\Pictures\Prism Visuals" -Filter "*.jpg").Count
```

---

## Run Commands

```powershell
cd 'c:\path\to\Prism Visuals'

# Auto-change (silent)
.\target\release\visuals.exe auto-change

# With input
echo "0" | .\target\release\visuals.exe fetch
echo "1" | .\target\release\visuals.exe source
```

---

## Build

```powershell
cargo build --release

# If exe locked
taskkill /F /IM visuals.exe 2>$null
cargo build --release
```

---

## Verification Scripts

### Test Index Increment

```powershell
Write-Host "=== TEST: Index Increment ===" -ForegroundColor Cyan

$config = Get-Content "$env:APPDATA\Prism Visuals\config.json" | ConvertFrom-Json
Write-Host "Index BEFORE:" $config.auto_change_index -ForegroundColor Yellow

.\target\release\visuals.exe auto-change

$config = Get-Content "$env:APPDATA\Prism Visuals\config.json" | ConvertFrom-Json
Write-Host "Index AFTER:" $config.auto_change_index -ForegroundColor Green
```

### Full System Check

```powershell
Write-Host "=== FULL SYSTEM CHECK ===" -ForegroundColor Cyan

# Config
Write-Host "`n1. Config State:" -ForegroundColor Yellow
$configPath = "$env:APPDATA\Prism Visuals\config.json"
if (Test-Path $configPath) {
    Get-Content $configPath | ConvertFrom-Json | ConvertTo-Json -Depth 3
} else {
    Write-Host "Config not found" -ForegroundColor Red
}

# Wallpapers
Write-Host "`n2. Wallpapers:" -ForegroundColor Yellow
$wallpaperDir = "$env:USERPROFILE\Pictures\Prism Visuals"
if (Test-Path $wallpaperDir) {
    Get-ChildItem $wallpaperDir -Filter "*.jpg" | Select-Object Name, Length, LastWriteTime | Format-Table -AutoSize
} else {
    Write-Host "Folder not found" -ForegroundColor Red
}
```

---

## File Paths

| Item | Path |
|------|------|
| Config | `%APPDATA%\Prism Visuals\config.json` |
| Wallpapers | `%USERPROFILE%\Pictures\Prism Visuals` |
| Binary | `.\target\release\visuals.exe` |

**PowerShell equivalents:**
+ `$env:APPDATA` = `C:\Users\<user>\AppData\Roaming`
+ `$env:USERPROFILE` = `C:\Users\<user>`
