<!--
PURPOSE: Debug and internal commands reference
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# Debug Commands

## Hidden Commands

These commands are not shown in help.

### `auto-change`

Called by Task Scheduler.

```powershell
visuals auto-change
```

| Behavior | |
|----------|---|
| Output | Silent |
| Logging | `%APPDATA%\Prism Visuals\auto_change.log` |
| Called by | VBS wrapper |

---

### `silent-uninstall`

Called by MSI uninstaller.

```powershell
visuals silent-uninstall
```

| Behavior | |
|----------|---|
| Action | Delete task, update config |
| Output | None |
| Returns | Immediately |

---

### `test-flicker` / `tf`

Test window flicker fix.

```powershell
visuals tf
```

| Behavior | |
|----------|---|
| Creates | 1-minute schedule |
| Test | Watch for window flash |
| Stop | `visuals unset` |

---

### `setup`

Trigger Defender exclusion setup.

```powershell
visuals setup
```

| Behavior | |
|----------|---|
| Action | UAC elevation |
| Adds | Defender exclusions |

---

## Debug Utilities

### View Logs

```powershell
# Last 20 entries
Get-Content "$env:APPDATA\Prism Visuals\auto_change.log" -Tail 20

# Real-time
Get-Content "$env:APPDATA\Prism Visuals\auto_change.log" -Tail 10 -Wait
```

### Check Task

```powershell
schtasks /Query /TN "PrismVisuals-AutoChange" /V
```

### View Config

```powershell
Get-Content "$env:APPDATA\Prism Visuals\config.json" | ConvertFrom-Json
```

### List Wallpapers

```powershell
Get-ChildItem "$env:USERPROFILE\Pictures\Prism Visuals" | Sort-Object Name
```

### Reset Index

```powershell
$config = Get-Content "$env:APPDATA\Prism Visuals\config.json" | ConvertFrom-Json
$config.auto_change_index = 0
$config | ConvertTo-Json -Depth 10 | Set-Content "$env:APPDATA\Prism Visuals\config.json"
```

### Force Auto-Change

```powershell
visuals auto-change
Get-Content "$env:APPDATA\Prism Visuals\auto_change.log" -Tail 10
```

---

## File Locations

| File | Path |
|------|------|
| Config | `%APPDATA%\Prism Visuals\config.json` |
| Log | `%APPDATA%\Prism Visuals\auto_change.log` |
| Wallpapers | `%USERPROFILE%\Pictures\Prism Visuals\` |
| Executable | `C:\Program Files\Prism Visuals\visuals.exe` |
| VBS | `C:\Program Files\Prism Visuals\prism_auto_change.vbs` |

---

## Command Reference

### Public

| Command | Shortcut |
|---------|----------|
| `fetch` | `f` |
| `change` | `c` |
| `source` | `src` |
| `open` | `o` |
| `set` | `s` |
| `unset` | `un` |
| `status` | `st`, `ss` |
| `reset` | `r` |
| `rm` | - |
| `update` | - |
| `help` | `h`, `?` |
| `menu` | `m`, `v` |
| `exit` | `quit` |

### Internal

| Command | Purpose |
|---------|---------|
| `auto-change` | Task Scheduler |
| `silent-uninstall` | MSI uninstaller |
| `test-flicker` | Test flicker fix |
| `setup` | Defender exclusions |
