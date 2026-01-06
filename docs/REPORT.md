<!--
PURPOSE: Project overview and technical summary
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-05
-->

# Project Report

## Summary

| Field | Value |
|-------|-------|
| Name | Prism Visuals |
| Type | CLI Tool |
| Version | 1.2.5 |
| Author | SibtainOcn |
| Language | Rust |
| Platform | Windows |
| License | Proprietary |

---

## Status

+ Version: 1.2.5 (Stable)
+ Release Date: January 2026
+ Known Issues: None critical

---

## Features

1. Multi-source wallpaper downloads (Spotlight, Wallhaven, Unsplash, Pexels)
2. Interactive CLI with shortcuts
3. **Universal Pick Mode** - browse & download from 4 sources with original quality
4. Theme-based search
5. Hash-based duplicate detection
6. Windows Task Scheduler integration
7. Silent execution (VBScript wrapper)
8. Windows installer (MSI)
9. Self-update from GitHub Releases
10. Automatic cleanup service

---

## Technical Stack

### Dependencies

| Crate | Purpose |
|-------|---------|
| reqwest | HTTP client |
| serde | Config management |
| chrono | Date/time |
| colored | Terminal colors |
| windows | Windows API |
| base64 | Script encoding |

### APIs

| API | Rate Limit |
|-----|------------|
| Spotlight | Unlimited |
| Wallhaven | 45/min |
| Unsplash | 50/hr |
| Pexels | 200/hr |

---

## File Structure

| File | Purpose |
|------|---------|
| `src/main.rs` | Main application |
| `src/scheduler.rs` | Task Scheduler integration |
| `src/wallhaven.rs` | Wallhaven API |
| `src/pexels.rs` | Pexels API |
| `src/spotlight_archive.rs` | Archive URL parsing & fetching |
| `main.wxs` | Installer definition |
| `prism_auto_change.vbs` | Silent execution wrapper |

---

## Configuration

Location: `%APPDATA%\Prism Visuals\config.json`

```json
{
  "source": "spotlight",
  "auto_change_enabled": true,
  "auto_change_frequency": "auto_daily"
}
```

---

## Limitations

1. Windows only
2. Unsplash/Pexels require API key
3. Rate limits apply for premium sources
4. Task Scheduler minimum interval: 1 minute

---

## Technical Notes

| Issue | Solution |
|-------|----------|
| PowerShell Hidden still flashes | VBScript with `Run(..., 0)` |
| Task Scheduler minimum interval | 1 minute (PT1M) |
| PowerShell escaping issues | Base64-encoded scripts |
| UAC elevation arguments | Comma-separated list |

---

## Files

+ VBScript created next to exe when scheduling
+ VBScript cleaned up on `visuals unset`
+ Test flicker fix with `visuals tf`
