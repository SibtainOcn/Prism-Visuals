<!--
PURPOSE: Track all version changes and releases
MAINTAINER: SibtainOcn
LAST UPDATED: 2026-01-02
-->

# Changelog

All notable changes to Prism Visuals are documented here.

Format based on [Keep a Changelog](https://keepachangelog.com/).

---
## [1.2.6] - 2026-01-07

### Added
- **Coffee Command**: New `coffee` command that shows thank you message and opens Buy Me a Coffee page

### Changed
- **README.md**: Resized "Buy Me a Coffee" button from default large size to 50px height for better visual balance

---
## [1.2.5] - 2026-01-06

### Changed (Breaking)
- **Bing → Spotlight**: Replaced Bing image source with Windows Spotlight API v4
  - 4K resolution instead of 1080p
  - Unlimited fetches (no 8-day limit)  
  - Same Microsoft-curated quality
  - Legacy "bing" configs auto-migrate to "spotlight"

### Added
- **Universal Pick Mode**: Enhanced `pick`/`p` command now supports 4 sources
  - Source selection menu: Spotlight Archive, Unsplash, Pexels, Wallhaven
  - Opens browser in right-half of screen for side-by-side workflow
  - **ORIGINAL quality downloads** - no size limits!
  - Unsplash: Full original (5472x3648+) via download endpoint
  - Pexels: Original resolution (4K+)
  - Wallhaven: Full HD/4K originals
  - Spotlight Archive: 10,000+ archived images (1920x1080)
  - **No API key required** for any source in pick mode
  - Smart prompts and Runtime spinner during downloads

### Enhanced
- **PEXELS_TEMPLATES**: Expanded from 10 to 20 curated keywords for better wallpaper variety
- **Unsplash auto_fetch_themes**: Expanded from 10 to 20 enhanced keywords
- **Main Menu Status**: Now displays autochange status (Active/Not Active) alongside Source and Wallpapers count
  - "Active" shown in red color for visibility when auto-change is enabled


---
## [1.2.4] - 2026-01-06

### Fixed
- **License RTF Blank in Installer**: Fixed EULA screen showing blank during MSI installation
  - Root cause: `License.rtf` was plain text instead of proper RTF format
  - Solution: Converted to proper RTF with headers, fonts, and formatting
- **Schedule Command "Access Denied"**: Fixed `visuals s` failing with "Access is denied (os error 5)"
  - Root cause: VBS wrapper was written to Program Files (requires admin)
  - Solution: VBS now stored in user's AppData folder (no UAC needed)
  - Fallback: If AppData write fails, auto-elevates with UAC prompt

### Changed
- **Removed Interactive Input Panel**: CLI now uses simple `> ` prompt instead of boxed input
  - Removed cursor manipulation and box drawing
  - Cleaner, faster, and behaves like a normal CLI

---
## [1.2.3] - 2026-01-04

### Fixed
- **Auto-Change Wallpaper Selection**: Fixed bug where auto-change set the wrong wallpaper after fetching
  - Root cause: `wallpapers.last()` returned alphabetically-last file (old unprefixed `pexels_MOUNTAINS...`)
  - Solution: Now finds file with highest sequence prefix (e.g., `0037_...`) instead of alphabetically-last
  - Verified: Correctly selects newest downloaded file
- **Uninstall Console Stuck**: Fixed MSI uninstaller opening console that gets stuck waiting for input
  - Root cause: `unschedule` command called `pause_before_exit()` which waited for user interaction
  - Solution: New `silent-uninstall` internal command that runs without interaction
  - WiX installer now uses `silent-uninstall` instead of `unschedule`

### Changed
- **Exit Command Separation**: Separated `exit`/`quit` from `0` (go-back)
  - `exit` or `quit` = Exit the CLI completely, return to PowerShell
  - `0` = Cancel/Go-back (in sub-menus only, e.g., source selection, schedule)
  - Updated goodbye message to: "See you soon, gorgeous! Stay stunning! ✨"

### Added
- **Automatic Cleanup Service**: Runs on startup to clean old data and prevent disk space accumulation
  - Deletes wallpapers older than 30 days (by file modification time)
  - Truncates `auto_change.log` to last 100 lines
  - Recalculates sequence numbers after deletion to prevent gaps
  - Preserves API keys, config settings, and recent data
  - Skipped for silent commands (`auto-change`, `silent-uninstall`)

---
## [1.2.2] - 2026-01-04

### Fixed
- **Auto-Change Wallpaper Selection**: Fixed bug where auto-change set the wrong wallpaper after fetching
  - Root cause: `wallpapers.last()` returned alphabetically-last file (old unprefixed `pexels_MOUNTAINS...`)
  - Solution: Now finds file with highest sequence prefix (e.g., `0011_...`) instead of alphabetically-last
  - Affects: Auto-change when out of wallpapers and fetching new ones
- **First-Run Setup UAC Trigger**: Fixed Windows Defender exclusion setup not triggering UAC prompt
  - Root cause: PowerShell command nesting/escaping issues caused elevation to fail silently
  - Solution: Changed to Base64-encoded PowerShell scripts using `-EncodedCommand` parameter
  - PowerShell scripts now encoded as UTF-16LE Base64 to avoid all quoting issues
  - Added verification step to confirm exclusions were actually added
  - Better user feedback when setup succeeds or is skipped
- **Manual Setup Command**: Same fix applied to `visuals setup` command
  - Both first-run and manual setup now reliably trigger UAC
  - Shows configured paths in success message
- **Window Flicker During Auto-Change**: Fixed CMD/PowerShell window briefly flashing when scheduled task runs
  - Root cause: PowerShell `-WindowStyle Hidden` still shows brief window flash on process creation
  - Solution: Changed to VBScript wrapper using `WScript.Shell.Run` with mode 0 (completely hidden)
  - Task now runs `wscript.exe` → VBScript → `visuals.exe auto-change` (completely invisible)
  - VBScript file `prism_auto_change.vbs` created next to exe, cleaned up on unschedule

### Added
- **base64 Dependency**: Added `base64 = "0.21"` for PowerShell script encoding
- **Test Command**: New `test-flicker` (or `tf`) command to test the window flicker fix
  - Creates a 1-minute schedule for rapid testing (Windows Task Scheduler minimum interval)
  - User can close console and verify no window flash appears
  - Run `visuals unset` to stop the test
- **Universal Go-Back Option**: Added "0" cancel option to all interactive prompts
  - Works in: source selection, theme input, image count, sort preference
  - Consistent experience across fetch_unsplash, fetch_wallhaven, fetch_pexels, set_source, and schedule
- **Smart Wallpaper Index Sync**: Auto-change detects manual wallpaper changes
  - If user manually sets a wallpaper via Windows, auto-change continues from that position
  - Fixed: Now only syncs if current wallpaper differs from what would be set next
  - Prevents the repeated wallpaper bug from earlier implementation
- **Sequential File Naming**: All downloaded wallpapers now have sequence prefix
  - Format: `0001_source_theme_id.jpg`, `0002_...`, etc.
  - Ensures consistent chronological ordering regardless of source or filename
  - Works across all sources (Bing, Unsplash, Wallhaven, Pexels)
- **Wallhaven Safe Categories for Auto-Fetch**: Silent/auto fetch uses General category only
  - Prevents suggestive poses or anime content in automatically downloaded images
  - Manual fetch still allows General + Anime categories
- **Uninstall Console Stuck**: Fixed MSI uninstaller opening console that gets stuck
  - Root cause: `unschedule` command called `pause_before_exit()` which waited for user input
  - Solution: New `silent-uninstall` internal command that runs without interaction
  - WiX installer now uses `silent-uninstall` instead of `unschedule`
- **Automatic Cleanup Service**: Runs on startup to clean old data
  - Deletes wallpapers older than 30 days (by file modification time)
  - Truncates auto_change.log to last 100 lines
  - Recalculates sequence numbers after deletion to prevent gaps
  - Preserves API keys and config - only cleans images and logs

---
## [1.2.1] - 2026-01-04

### Changed
- **Source Selection Menu**: Updated source descriptions with more attractive, descriptive text

  - Wallhaven: "Where wallpaper enthusiasts unite, HD heaven"
  - Unsplash: "5M+ photos by world-class photographers" (with signup link)
  - Pexels: "Studio-grade photos for your desktop canvas" (with signup link)
  - Added API signup links inline for premium sources
- **Download Progress Color**: Changed percentage number color from yellow to green during downloads
  - Affected functions: `print_progress_bar()` and `RuntimeLoader::start_with_progress()`


### Added
- **Silent Fetch for Wallhaven**: New `fetch_wallhaven_silent()` function for auto-change service
  - Uses random template words (nature, mountains, space, etc.) for variety
  - Falls back to Bing if API fails
- **Silent Fetch for Pexels**: New `fetch_pexels_silent()` function for auto-change service
  - Uses random template words for variety
  - Falls back to Bing if no API key set or API fails
- **New Command: `rm`**: Reset current source API key only (keeps other settings intact)
  - Hint added to API error messages: "Use 'rm' command to reset your API key"
  - Works for Unsplash and Pexels sources
- **Auto-Update System**: New `update` command with GitHub Releases integration
  - Downloads from GitHub Releases with Runtime-style progress bar
  - Self-replacement mechanism (rename current → download new → restart)
  - **UAC elevation support**: Auto-detects if admin rights needed (Program Files)
    - Prompts user: "Would you like to continue with elevated permissions? (yes/no)"
    - Relaunches with PowerShell `Start-Process -Verb RunAs` on confirmation
  - Startup version check: Shows "[ INFO ] New version available" only when update is released
  - Silent on startup if no update (no message)
  - Automatic cleanup of old backup on next launch

### Fixed
- **Auto-Change Service**: Fixed bug where Wallhaven/Pexels source would silently fetch from Bing
  - `fetch_silent()` now correctly routes to `fetch_wallhaven_silent()` and `fetch_pexels_silent()`

---
## [1.2.0] - 2026-01-04

### Added
- **NEW SOURCE: Wallhaven** (`src/wallhaven.rs`)
  - No API key required for SFW content![alt text](image.png)
  - Rate limit tracking: 45 requests/minute
  - Default settings: `ratios=16x9`, `atleast=1920x1080`
  - **Sorting options**: Toplist (most favorited), Hot (trending), Random, Relevance
  - Empty query support for global popular wallpapers
  - Template words for silent auto-fetch
  - Theme selection and image count prompts
  - **Categories**: General + Anime (matches Wallhaven homepage Toplist)
- **NEW SOURCE: Pexels** (`src/pexels.rs`)  
  - API key required (free signup)
  - Rate limit tracking: 200 requests/hour
  - Default settings: `orientation=landscape`, `size=large`
  - Downloads from `src.large2x` (1880px width)
  - Template words for silent auto-fetch

### Changed
- **Source Selection** (`set_source` in `main.rs`)
  - Now shows 4 sources: Bing, Wallhaven, Unsplash, Pexels
  - Categorized as DEFAULT (no key) and PREMIUM (key required)
  - Auto-prompts for API key when selecting Pexels
- **Fetch Dispatcher** (`fetch` in `main.rs`)
  - Routes to `fetch_wallhaven()` and `fetch_pexels()`
- **Config Struct**
  - Added `WallhavenConfig` and `PexelsConfig` with rate limit tracking
  - Uses `#[serde(default)]` for backward compatibility

### Fixed
- **Pexels Rate Limit Bug**: Fixed u32 underflow causing `4294942498/200` display
  - Implementation now uses `saturating_sub` to prevent integer underflow
  - Added sanity check to auto-reset corrupted values (> 200) to 0
- **Help & UI**
  - Updated help display with all 4 sources and rate limits
  - Updated source descriptions throughout

### Documentation
- **README.md**: Added source comparison table and workflow documentation for new sources
- **Docs**: Comprehensive API research added to `docs/planned`


## [1.1.6] - 2026-01-03

### Added
- **Animated Spinner + Progress Bar**: Downloads now show BOTH animated spinner AND growing progress dashes
  - Format: `⠋ [1/5] [------    ] 30% Mountain landscape`
  - **Smooth animation**: Spinner advances every ~100ms (like old RuntimeLoader) using time-based logic
  - Uses Unicode spinner on Windows 11+ and ASCII spinner (|/-\) on Windows 10
  - Thread-local state tracks spinner frame and last update time

### Fixed
- **Unsplash Downloads**: Fixed progress indicator to use Runtime-style per-image progress bars
  - Each image now shows individual download progress from 0% → 100%
  - Progress bar grows as bytes are downloaded (e.g., `--` → `------` → `----------`)
  - Progress line clears after each image completes (only ✓ message remains)
  - Matches behavior with Bing downloads for consistency
- **Already Exists Detection**: Unsplash downloads now show a skip message for already-downloaded files instead of silently continuing
- **Line Wrapping Glitch**: Fixed multi-line printing bug when descriptions are too long
  - Long descriptions are now truncated to ~35 chars with "..." to prevent terminal line wrap
  - Ensures progress bar always stays on a single line

### Changed
- **fetch_unsplash**: Migrated from global progress counter to chunked streaming with per-image progress
  - Downloads in 8KB chunks with live progress updates per image
  - Added terminal echo control to prevent keyboard glitch during downloads
  - Each image gets a fresh 0% → 100% progress cycle
- **print_progress_bar**: Enhanced to include animated spinner alongside progress visualization

---
any
## [1.1.5] - 2026-01-03

### Added
- **Synchronous Progress Bar**: 
  - Format: `  [2/8] [----------    ] 35% Description...`
  - Clears after each image completion, then starts fresh for next image
  - Grows smoothly as chunks download (8KB chunks)
- **Terminal Echo Control**: `disable_terminal_echo()` and `enable_terminal_echo()` functions
  - Prevents keyboard glitch where keypresses appear on screen during downloads
  - Auto-enabled during fetch operations

### Changed
- **fetch_bing**: Now uses streaming download with real-time progress bar
  - Downloads in 8KB chunks with live progress updates
  - Each image gets its own progress bar that resets to 0%
  - Progress bar clears after completion (only ✓ message remains)
- **Error Handling**: Fixed infinite loop bug in download error handling

### Fixed
- **Keyboard Glitch**: User keypresses no longer echo to terminal during downloads
- **Progress Display**: Progress bar now updates smoothly per image, not across all images

### Technical
- Removed threaded progress bar (RuntimeLoader.start_with_progress) - was slow/buggy
- Added Windows console mode manipulation for echo control
- Using `std::io::Read` for chunked streaming instead of `.bytes()`

## [1.1.4] - 2026-01-03

### Added
- **Auto Daily Scheduling**: New default option to schedule wallpaper changes at 8:00 AM daily with zero configuration
- **Interval Submenu**: Organized hourly scheduling options into a cleaner submenu structure
- **Smart Retry System**: Time/interval inputs now retry with helpful hints instead of exiting on error
- **Cancel Options**: Users can type 'cancel' or '0' to exit input prompts at any time
- **PC Wake Support**: Added notification that scheduled tasks run even when PC is off at scheduled time

### Changed
- **Simplified Schedule Menu**: Reduced from 5 options to 3 clear categories:
  - Option 1: Auto Daily (8:00 AM, zero-config)
  - Option 2: Daily at specific time (user-chosen time)
  - Option 3: Interval-based (submenu with 1hr, 3hrs, 6hrs, custom)
- **Better Error Messages**: Clearer validation feedback with examples (e.g., "Example: 09:00 for 9 AM")
- **scheduler.rs**: Added `AutoDaily` variant to `ScheduleFrequency` enum with full serialization support

### Fixed
- **Input Validation UX**: Users no longer need to restart the entire command after a typo - they can retry immediately
- **Format Confusion**: Added clear examples for HH:MM format to prevent user mistakes

## [1.1.3] - 2026-01-03

### Added
- **Main Menu Shortcuts**: Added `v`, `visuals`, `m`, `menu` shortcuts to quickly access the main menu control panel
- **Command Feedback**: Added a brief (0.5s) animated spinner "Running..." to provide visual feedback that a command is executing
- **Default Action**: Running `visuals` without arguments now opens the Main Menu instead of Help
- **Windows Version Detection**: Auto-detects Windows version to use Unicode spinners on Win11+ and ASCII spinners on Win10/below for compatibility

### Changed
- `interactive_prompt`: Updated to redirect `v` and `menu` references to the new Main Menu
- Error messages now suggest `v` for main menu instead of just `h` for help
- `RuntimeLoader`: Now adapts spinner characters based on Windows version (Unicode ⠋⠙⠹ on Win11+, ASCII |/-\ on Win10/below)

## [1.1.2] - 2026-01-03

### Changed
- **Command Naming**: Renamed schedule commands for clarity
  - `schedule` → `set` (shortcut: `s`)
  - `unschedule` → `unset` (shortcut: `un`)
  - `schedule-status` → `status` (shortcut: `st`)
  - Old names still work for backward compatibility
- **Case-Insensitive Commands**: All commands now work regardless of case
- **Redesigned Help Section**: Box-style table with Command/Shortcut/Description columns

### Updated
- `README.md` with full command reference table

## [1.1.1] - 2026-01-03

### Fixed
- **Forever Index Tracking**: Index now increments continuously (0→1→2→...→fetch→4→5) instead of resetting to 0
- **Bing Config Sync**: Config now syncs with actual folder files before fetch/auto-change
  - Detects deleted images and removes orphaned hashes
  - Prevents "already have all images" when files are deleted

### Added
- **Unsplash Silent Fetch Templates**: 10 curated high-quality themes for auto-fetch
  - Themes: deep space, nature, mountains, sunset, sunrise, dark aesthetic, macro nature, flowers, abstract, sand dunes
  - Random theme selection with relevance sort for best quality
- **Testing Commands Guide**: New `docs/COMMANDS.md` with PowerShell test commands

## [1.1.0] - 2026-01-03

### Added
- **Auto-Change Feature**: Automatically rotate wallpapers on a schedule
- New commands: `schedule` (s), `unschedule`, `schedule-status` (ss)
- Windows Task Scheduler integration for zero-resource background updates
- Scheduling options: Daily, Hourly, 3 Hours, 6 Hours, Custom
- Silent fallback: Automatically fetches from Bing if no wallpapers exist
- Sequential wallpaper selection (oldest to newest)
- Installer update: Automatically cleans up scheduled tasks on uninstall

## [1.0.0] - 2026-01-02

### Added
- Multi-source wallpaper support (Bing and Unsplash)
- Interactive CLI mode with shortcuts
- Commands: fetch, change, open, source, reset, help, exit
- Unsplash API integration with theme search
- Sort options: relevance, latest, random
- Rate limit tracking for Unsplash API
- Windows installer (MSI) via WiX v5
- System PATH integration

### Technical
- Built with Rust
- Uses reqwest for HTTP requests
- Windows API integration for wallpaper setting
- Hash-based duplicate detection for  images

---

## How to Update This File

When releasing a new version:

1. Add new section with version number and date
2. Organize changes under these categories:
   - Added - new features
   - Changed - changes in existing functionality
   - Fixed - bug fixes
   - Removed - removed features
   - Security - security fixes
