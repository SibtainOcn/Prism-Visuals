<!--
PURPOSE: System design and component overview
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# Architecture

## System Overview

```
User Input
    |
    v
+-------------------+
|   CLI Interface   |
+-------------------+
    |
    v
+-------------------+
|  Command Handler  |
+-------------------+
    |
    +--------+--------+--------+--------+--------+
    v        v        v        v        v        v
+------+ +-------+ +--------+ +------+ +--------+ +--------+
| Spot | |Wallha | |Unsplash| |Pexels| |Archive | | Config |
+------+ +-------+ +--------+ +------+ +--------+ +--------+
    |        |         |         |         |          |
    v        v         v         v         v          v
+------------------------------------------------------------+
|                   Local File System                        |
|    Pictures/Prism Visuals/    |    config.json             |
+------------------------------------------------------------+
```

---

## File Locations

| File | Location |
|------|----------|
| Wallpapers | `%USERPROFILE%\Pictures\Prism Visuals\` |
| Config | `%APPDATA%\Prism Visuals\config.json` |
| Log | `%APPDATA%\Prism Visuals\auto_change.log` |
| Executable | `C:\Program Files\Prism Visuals\` |
| VBS Wrapper | Next to executable |

---

## Auto-Change Service

### Execution Chain

```
Task Scheduler
      |
      v
wscript.exe (hidden)
      |
      v
prism_auto_change.vbs
      |
      v
visuals.exe auto-change
      |
      v
IDesktopWallpaper COM API
```

### Logic Flow

```
AUTO-CHANGE
    |
    v
Read wallpaper list
    |
    v
Check current Windows wallpaper
    |
    v
[ Same as next? ] --YES--> Set next wallpaper
    |
    NO
    |
    v
[ In our folder? ] --YES--> Sync index
    |
    NO
    |
    v
Ignore (external wallpaper)
```

---

## Scenarios

### Normal Cycle

| Run | Index | Action |
|-----|-------|--------|
| 1st | 0 | Set wallpaper[0] |
| 2nd | 1 | Set wallpaper[1] |
| 3rd | 2 | Set wallpaper[2] |
| 4th | 3 | Fetch new if index >= count |

### Out of Wallpapers

1. Detect: `index >= count`
2. Call `fetch_silent()`
3. Find newest by sequence prefix
4. Set wallpaper
5. Increment index

### User Manual Change

1. Detect: Windows wallpaper != expected next
2. Check if in our folder
3. If yes: sync index to user position + 1
4. If no: ignore, continue sequence

---

## Config Fields

| Field | Purpose |
|-------|---------|
| `auto_change_index` | Current position (never resets) |
| `next_seq_number` | Next file prefix (0001_, 0002_...) |
| `auto_change_frequency` | Schedule type |

---

## File Naming

Format: `{seq}_{source}_{theme}_{id}.{ext}`

Examples:
+ `0001_spotlight_MountainView_abc12345.jpg`
+ `0002_unsplash_NATURE_abc123.jpg`
+ `0003_wallhaven_MOUNTAINS_0jre3y.jpg`

---

## Logging

Log file: `%APPDATA%\Prism Visuals\auto_change.log`

```powershell
Get-Content "$env:APPDATA\Prism Visuals\auto_change.log" -Tail 20
```

---

## Cleanup Service

Runs on startup (except silent modes).

| Item | Retention |
|------|-----------|
| Wallpapers | 30 days |
| Log | Last 100 lines |
| Sequence numbers | Auto-recalculated |

Preserved:
+ Config file
+ API keys
+ Recent wallpapers

---

## Source Files

| File | Purpose |
|------|---------|
| `main.rs` → `auto_change()` | Wallpaper rotation |
| `main.rs` → `fetch_silent()` | Silent fetch dispatcher |
| `main.rs` → `picker_mode()` | Universal Image Picker (4 sources) |
| `main.rs` → `cleanup_old_data()` | Cleanup service |
| `picker_archive.rs` | Multi-source URL parsing |
| `scheduler.rs` → `TaskScheduler` | Task Scheduler integration |
| `scheduler.rs` → `ScheduleFrequency` | Frequency parsing |

---

## Universal Pick Mode

### Supported Sources

| Source | Resolution | API Key |
|--------|:----------:|:-------:|
| Spotlight Archive | 1920x1080 | Not needed |
| Unsplash | Original (5472x3648+) | Not needed |
| Pexels | Original (4K+) | Not needed |
| Wallhaven | Original (4K+) | Not needed |

### URL Parsing Flow

```
User pastes URL
    |
    v
validate_url(url, source) → Check domain
    |
    v
get_image_url(url, source) → Dispatch to parser
    |
    +→ spotlight: get_full_res_url()
    +→ unsplash: get_unsplash_url() → download endpoint
    +→ pexels: get_pexels_url() → no size params
    +→ wallhaven: get_wallhaven_url() → construct full URL
    |
    v
Download with progress → Save to folder
```

---

## Silent Commands

### `auto-change`

Called by Task Scheduler via VBS.
+ No console output
+ Logs to file
+ Returns immediately

### `silent-uninstall`

Called by MSI uninstaller.
+ Deletes scheduled task
+ Updates config
+ No user interaction
