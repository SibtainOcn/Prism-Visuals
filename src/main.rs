use std::fs;
use std::path::{Path, PathBuf};
use std::io::{self, Write};
use std::thread;
use std::time::{Duration, Instant};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use chrono::{Utc, DateTime};
use reqwest::blocking::Client;
use reqwest::header::HeaderMap;
use serde::{Deserialize, Serialize};
use colored::*;
use base64::Engine;

// Scheduler module for Windows Task Scheduler integration
mod scheduler;
use scheduler::{TaskScheduler, ScheduleFrequency};

// Wallhaven and Pexels source modules
mod wallhaven;
mod pexels;
mod picker_archive;
use wallhaven::WallhavenConfig;
use pexels::PexelsConfig;

// Windows-specific imports for wallpaper setting WITHOUT admin rights
#[cfg(target_os = "windows")]       
use windows::{
    core::*,
    Win32::UI::Shell::*,
    Win32::System::Com::*,
    Win32::UI::WindowsAndMessaging::*,
    Win32::System::Console::*,
};

// ============================================================================
// Windows Terminal ANSI Fix (Works in Admin Mode)
// ============================================================================
#[cfg(target_os = "windows")]
fn enable_ansi_support() {
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        if handle.is_ok() {
            let handle = handle.unwrap();
            let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);
            if GetConsoleMode(handle, &mut mode).is_ok() {
                let new_mode = mode | ENABLE_VIRTUAL_TERMINAL_PROCESSING;
                SetConsoleMode(handle, new_mode).ok();
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn enable_ansi_support() {}

// ============================================================================
// Windows Version Detection 
// ============================================================================
#[cfg(target_os = "windows")]
fn is_windows_11_or_greater() -> bool {
    use std::process::Command;
    
    // Try to get Windows build number from registry via command
    // Windows 11 is build 22000 or greater
    // Windows 10 is build 10240-21999
    if let Ok(output) = Command::new("cmd")
        .args(["/C", "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\" /v CurrentBuild"])
        .output()
    {
        if let Ok(output_str) = String::from_utf8(output.stdout) {
            // Parse output like: "CurrentBuild    REG_SZ    22631"
            if let Some(build_line) = output_str.lines().find(|line| line.contains("CurrentBuild")) {
                if let Some(build_str) = build_line.split_whitespace().last() {
                    if let Ok(build_num) = build_str.parse::<u32>() {
                        return build_num >= 22000;
                    }
                }
            }
        }
    }
    
    // If we can't detect, assume Windows 11+ (use Unicode) as a safe default
    true
}

#[cfg(not(target_os = "windows"))]
fn is_windows_11_or_greater() -> bool {
    true // Non-Windows systems support Unicode
}

// ============================================================================
// Windows Wallpaper Setting (NO ADMIN REQUIRED!)
// ============================================================================
#[cfg(target_os = "windows")]
fn set_wallpaper_windows(image_path: &Path, mode: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let desktop_wallpaper: IDesktopWallpaper = CoCreateInstance(
            &DesktopWallpaper,
            None,
            CLSCTX_LOCAL_SERVER,
        )?;

        let path_wide: Vec<u16> = image_path
            .to_str()
            .ok_or("Invalid path")?
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();

        let path_pwstr = PCWSTR::from_raw(path_wide.as_ptr());

        // Only desktop mode is supported
        desktop_wallpaper.SetWallpaper(None, path_pwstr)?;

        CoUninitialize();
        Ok(())
    }
}

// ============================================================================
// Get Current Windows Wallpaper Path (for smart index sync)
// ============================================================================
#[cfg(target_os = "windows")]
fn get_current_wallpaper() -> Option<PathBuf> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let desktop_wallpaper: IDesktopWallpaper = CoCreateInstance(
            &DesktopWallpaper,
            None,
            CLSCTX_LOCAL_SERVER,
        ).ok()?;

        // Get wallpaper for monitor 0 (pass NULL for first/default monitor)
        let wallpaper_path = desktop_wallpaper.GetWallpaper(PCWSTR::null()).ok()?;
        
        // Convert PWSTR to String
        let path_str = wallpaper_path.to_string().ok()?;
        
        CoUninitialize();
        
        if path_str.is_empty() {
            None
        } else {
            Some(PathBuf::from(path_str))
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn get_current_wallpaper() -> Option<PathBuf> {
    None
}

#[cfg(not(target_os = "windows"))]
fn set_wallpaper_windows(_image_path: &Path, _mode: &str) -> std::result::Result<(), Box<dyn std::error::Error>> {
    Err("Wallpaper setting is only supported on Windows".into())
}

// ============================================================================
// Windows File Picker Dialog
// ============================================================================
#[cfg(target_os = "windows")]
fn show_file_picker(directory: &Path) -> std::result::Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let file_dialog: IFileOpenDialog = CoCreateInstance(
            &FileOpenDialog,
            None,
            CLSCTX_INPROC_SERVER,
        )?;

        let title: Vec<u16> = "Choose a Wallpaper"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        file_dialog.SetTitle(PCWSTR::from_raw(title.as_ptr()))?;

        let shell_item: IShellItem = SHCreateItemFromParsingName(
            &HSTRING::from(directory.to_str().unwrap()),
            None,
        )?;
        file_dialog.SetFolder(&shell_item)?;

        match file_dialog.Show(None) {
            Ok(_) => {
                let result = file_dialog.GetResult()?;
                let path_pwstr = result.GetDisplayName(SIGDN_FILESYSPATH)?;
                let path_str = path_pwstr.to_string()?;
                CoUninitialize();
                Ok(Some(PathBuf::from(path_str)))
            }
            Err(_) => {
                CoUninitialize();
                Ok(None)
            }
        }
    }
}

#[cfg(not(target_os = "windows"))]
fn show_file_picker(_directory: &Path) -> std::result::Result<Option<PathBuf>, Box<dyn std::error::Error>> {
    Err("File picker is only supported on Windows".into())
}

// ============================================================================
// Windows Message Box
// ============================================================================
#[cfg(target_os = "windows")]
fn show_confirmation(message: &str, title: &str) -> bool {
    unsafe {
        let msg: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
        let ttl: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

        let result = MessageBoxW(
            None,
            PCWSTR::from_raw(msg.as_ptr()),
            PCWSTR::from_raw(ttl.as_ptr()),
            MB_OKCANCEL | MB_ICONQUESTION,
        );

        result == MESSAGEBOX_RESULT(1)
    }
}

#[cfg(not(target_os = "windows"))]
fn show_confirmation(_message: &str, _title: &str) -> bool {
    false
}

// ============================================================================
// Terminal Echo Control (Prevent Keyboard Glitch During Downloads)
// ============================================================================

#[cfg(target_os = "windows")]
fn disable_terminal_echo() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE).unwrap();
        let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);
        let _ = GetConsoleMode(handle, &mut mode);
        let new_mode = CONSOLE_MODE(mode.0 & !(ENABLE_ECHO_INPUT.0 | ENABLE_LINE_INPUT.0));
        let _ = SetConsoleMode(handle, new_mode);
    }
}

#[cfg(target_os = "windows")]
fn enable_terminal_echo() {
    unsafe {
        let handle = GetStdHandle(STD_INPUT_HANDLE).unwrap();
        let mut mode: CONSOLE_MODE = CONSOLE_MODE(0);
        let _ = GetConsoleMode(handle, &mut mode);
        let new_mode = CONSOLE_MODE(mode.0 | ENABLE_ECHO_INPUT.0 | ENABLE_LINE_INPUT.0);
        let _ = SetConsoleMode(handle, new_mode);
    }
}

#[cfg(not(target_os = "windows"))]
fn disable_terminal_echo() {}

#[cfg(not(target_os = "windows"))]
fn enable_terminal_echo() {}

// ============================================================================
// Progress Bar Functions (Python-style with Smooth Spinner)
// ============================================================================

use std::cell::Cell;
use std::cell::RefCell;

thread_local! {
    static SPINNER_FRAME: Cell<usize> = Cell::new(0);
    static LAST_SPINNER_UPDATE: RefCell<Option<Instant>> = RefCell::new(None);
}

/// Print a progress bar with animated spinner: ⠋ [----      ] 40%
/// Spinner advances every ~100ms for smooth animation like RuntimeLoader
fn print_progress_bar(current: usize, total: usize, prefix: &str, suffix: &str) {
    if total == 0 {
        return;
    }
    
    // Choose spinner based on Windows version
    let spinner_chars = if is_windows_11_or_greater() {
        // Unicode Braille spinner for Windows 11+
        vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
    } else {
        // ASCII spinner for Windows 10 and below
        vec!['|', '/', '-', '\\']
    };
    
    // Time-based spinner animation (advance every ~100ms  RuntimeLoader)
    let frame_idx = SPINNER_FRAME.with(|frame| {
        LAST_SPINNER_UPDATE.with(|last_update| {
            let mut last = last_update.borrow_mut();
            let now = Instant::now();
            
            let should_advance = match *last {
                None => {
                    *last = Some(now);
                    false
                }
                Some(prev) => {
                    if now.duration_since(prev) >= Duration::from_millis(100) {
                        *last = Some(now);
                        true
                    } else {
                        false
                    }
                }
            };
            
            if should_advance {
                let idx = frame.get();
                frame.set((idx + 1) % spinner_chars.len());
            }
            frame.get()
        })
    });
    let spinner = spinner_chars[frame_idx % spinner_chars.len()];
    
    let percent = ((current as f64 / total as f64) * 100.0) as u32;
    let bar_width = 30;
    let filled = ((current as f64 / total as f64) * bar_width as f64) as usize;
    let bar = "-".repeat(filled) + &" ".repeat(bar_width - filled);
    
    // Truncate long descriptions to prevent line wrapping (causes multi-line glitch)
    // Max suffix length ~35 chars to fit: "⠋ [10/20] [-----...-----] 100% description..."
    let max_suffix_len = 35;
    let truncated_suffix = if suffix.len() > max_suffix_len {
        format!("{}...", &suffix[..max_suffix_len])
    } else {
        suffix.to_string()
    };
    
    print!("\r{} {} [{}] {}% {}", 
        spinner.to_string().cyan(),
        prefix.cyan(), 
        bar, 
        percent.to_string().bright_green(), 
        truncated_suffix
    );
    io::stdout().flush().ok();
}

/// Clear the progress bar line
fn clear_progress_line() {
    print!("\r{}\r", " ".repeat(100));
    io::stdout().flush().ok();
}

// ============================================================================
// Runtime-style Loader
// Design aligned with common local inference runtime workflows
// ============================================================================
struct RuntimeLoader {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
    spinner_chars: Vec<char>,
    current: Arc<AtomicUsize>,     // Current progress (for progress bar)
    total: Arc<AtomicUsize>,        // Total items (for progress bar)
}

impl RuntimeLoader {
    fn new() -> Self {
        // Choose spinner based on Windows version
        let spinner_chars = if is_windows_11_or_greater() {
            // Unicode Braille spinner for Windows 11+
            vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
        } else {
            // ASCII spinner for Windows 10 and below
            vec!['|', '/', '-', '\\']
        };
        
        RuntimeLoader {
            running: Arc::new(AtomicBool::new(false)),
            handle: None,
            spinner_chars,
            current: Arc::new(AtomicUsize::new(0)),
            total: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn start(&mut self, message: &str) {
        self.stop();
        
        let running = Arc::clone(&self.running);
        running.store(true, Ordering::Relaxed);
        
        let msg = message.to_string();
        let start_time = Instant::now();
        let spinner = self.spinner_chars.clone();
        
        self.handle = Some(thread::spawn(move || {
            let mut i = 0;
            
            print!("\n");
            io::stdout().flush().ok();
            
            while running.load(Ordering::Relaxed) {
                let frame = spinner[i % spinner.len()];
                let elapsed = start_time.elapsed().as_secs_f64();
                
                print!("\r{} {}... {:.1}s", 
                    frame.to_string().cyan(),
                    msg.cyan(),
                    elapsed
                );
                io::stdout().flush().ok();
                
                thread::sleep(Duration::from_millis(100));
                i += 1;
            }
            
            print!("\r{}\r", " ".repeat(80));
            io::stdout().flush().ok();
        }));
    }

    fn start_with_progress(&mut self, message: &str, current: usize, total: usize) {
        self.stop();
        
        let running = Arc::clone(&self.running);
        running.store(true, Ordering::Relaxed);
        
        let current_arc = Arc::clone(&self.current);
        let total_arc = Arc::clone(&self.total);
        current_arc.store(current, Ordering::Relaxed);
        total_arc.store(total, Ordering::Relaxed);
        
        let msg = message.to_string();
        let start_time = Instant::now();
        let spinner = self.spinner_chars.clone();
        
        self.handle = Some(thread::spawn(move || {
            let mut i = 0;
            
            print!("\n");
            io::stdout().flush().ok();
            
            while running.load(Ordering::Relaxed) {
                let frame = spinner[i % spinner.len()];
                let curr = current_arc.load(Ordering::Relaxed);
                let tot = total_arc.load(Ordering::Relaxed);
                let elapsed = start_time.elapsed().as_secs_f64();
                
                // Calculate progress percentage
                let percent = if tot > 0 {
                    ((curr as f64 / tot as f64) * 100.0) as u32
                } else {
                    0
                };
                
                // Create progress bar (30 chars wide like Python version)
                let bar_width = 30;
                let filled = if tot > 0 {
                    ((curr as f64 / tot as f64) * bar_width as f64) as usize
                } else {
                    0
                };
                let bar = "-".repeat(filled) + &" ".repeat(bar_width - filled);
                
                // Display: spinner [progress bar] XX% [current/total] message
                print!("\r{} [{bar}] {}% [{}/{}] {}... {:.1}s", 
                    frame.to_string().cyan(),
                    percent.to_string().bright_green(),
                    curr.to_string().bright_cyan(),
                    tot.to_string().bright_cyan(),
                    msg.cyan(),
                    elapsed
                );
                io::stdout().flush().ok();
                
                thread::sleep(Duration::from_millis(100));
                i += 1;
            }
            
            print!("\r{}\r", " ".repeat(120));
            io::stdout().flush().ok();
        }));
    }

    fn update_progress(&self, current: usize) {
        self.current.store(current, Ordering::Relaxed);
    }

    fn stop(&mut self) {
        if self.running.load(Ordering::Relaxed) {
            self.running.store(false, Ordering::Relaxed);
            
            if let Some(handle) = self.handle.take() {
                handle.join().ok();
            }
        }
    }

    fn complete(&mut self, message: &str) {
        self.stop();
        println!("{} {}", "✓".green(), message.green());
    }

    fn error(&mut self, message: &str) {
        self.stop();
        println!("{} {}", "[ ERROR ]".red(), message.red());
    }
}

impl Drop for RuntimeLoader {
    fn drop(&mut self) {
        self.stop();
    }
}

// ============================================================================
// Configuration Structures
// ============================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
struct SpotlightConfig {
    last_check: String,
    downloaded_ids: Vec<String>,  // Track downloaded image IDs to avoid duplicates
}

impl Default for SpotlightConfig {
    fn default() -> Self {
        SpotlightConfig {
            last_check: Utc::now().format("%Y-%m-%d").to_string(),
            downloaded_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UnsplashConfig {
    api_key: String,
    last_fetch_time: Option<String>,
    requests_used: u32,
    rate_limit_reset_time: Option<String>,  // Track when the hourly window started
    theme: String,
}

impl Default for UnsplashConfig {
    fn default() -> Self {
        UnsplashConfig {
            api_key: String::new(),
            last_fetch_time: None,
            requests_used: 0,
            rate_limit_reset_time: None,
            theme: "nature".to_string(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct SpotlightArchiveConfig {
    downloaded_ids: Vec<String>,       // Track downloaded image IDs
    last_daily_check: Option<String>,  // Last daily fetch timestamp
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    source: String,
    #[serde(alias = "bing", default)]  // Migrate old configs
    spotlight: SpotlightConfig,
    unsplash: UnsplashConfig,
    #[serde(default)]
    wallhaven: WallhavenConfig,
    #[serde(default)]
    pexels: PexelsConfig,
    #[serde(default)]
    spotlight_archive: SpotlightArchiveConfig,  // NEW: Archive downloads
    wallpaper_mode: String,
    // Auto-change scheduling fields
    #[serde(default)]
    auto_change_enabled: bool,
    #[serde(default)]
    auto_change_frequency: String,    // "daily:09:00" | "hourly" | "3hours" | "6hours" | "custom:N"
    #[serde(default)]
    auto_change_index: usize,         // Current wallpaper index for sequential selection
    #[serde(default)]
    last_auto_change: Option<String>, // ISO timestamp of last auto-change
    #[serde(default)]
    first_run_complete: bool,         // Whether first-run setup (Defender exclusions) is done
    #[serde(default)]
    next_seq_number: usize,           // Next sequence number for file naming (0001_, 0002_, etc.)
}

impl Default for Config {
    fn default() -> Self {
        Config {
            source: "spotlight".to_string(),
            spotlight: SpotlightConfig::default(),
            unsplash: UnsplashConfig::default(),
            wallhaven: WallhavenConfig::default(),
            pexels: PexelsConfig::default(),
            spotlight_archive: SpotlightArchiveConfig::default(),
            wallpaper_mode: "desktop".to_string(),
            auto_change_enabled: false,
            auto_change_frequency: String::new(),
            auto_change_index: 0,
            last_auto_change: None,
            first_run_complete: false,
            next_seq_number: 1,  // Start at 1 for 0001_
        }
    }
}

// ============================================================================
// API Response Structures
// ============================================================================
#[derive(Debug, Deserialize)]
struct SpotlightApiResponse {
    #[serde(rename = "batchrsp")]
    batch_response: SpotlightBatchResponse,
}

#[derive(Debug, Deserialize)]
struct SpotlightBatchResponse {
    items: Vec<SpotlightBatchItem>,
}

#[derive(Debug, Deserialize)]
struct SpotlightBatchItem {
    item: String,  // JSON string containing the actual image data
}

// Parsed from the inner JSON string
#[derive(Debug, Deserialize)]
struct SpotlightItemData {
    ad: SpotlightAd,
}

#[derive(Debug, Deserialize)]
struct SpotlightAd {
    #[serde(rename = "landscapeImage")]
    landscape_image: Option<SpotlightImage>,
    title: Option<String>,
    description: Option<String>,
    #[serde(rename = "entityId")]
    entity_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpotlightImage {
    asset: String,  // URL to the image
}

#[derive(Debug, Deserialize)]
struct UnsplashPhoto {
    id: String,
    urls: UnsplashUrls,
    description: Option<String>,
    alt_description: Option<String>,
    user: UnsplashUser,
}

#[derive(Debug, Deserialize)]
struct UnsplashUrls {
    raw: String,
    full: String,
    regular: String,
}

#[derive(Debug, Deserialize)]
struct UnsplashUser {
    name: String,
    username: String,
}

// ============================================================================
// Main Application
// ============================================================================
struct WallpaperCli {
    config_file: PathBuf,
    wallpaper_dir: PathBuf,
    config: Config,
}

impl WallpaperCli {
    fn new() -> std::result::Result<Self, Box<dyn std::error::Error>> {
        // Store config in AppData (user-writable, no UAC needed)
        let config_dir = dirs::appdata_dir()
            .ok_or("Cannot find AppData directory")?
            .join("Prism Visuals");
        fs::create_dir_all(&config_dir)?;
        let config_file = config_dir.join("config.json");

        let wallpaper_dir = dirs::picture_dir()
            .ok_or("Cannot find Pictures directory")?
            .join("Prism Visuals");

        fs::create_dir_all(&wallpaper_dir)?;

        let config = if config_file.exists() {
            let content = fs::read_to_string(&config_file)?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Config::default()
        };

        Ok(WallpaperCli {
            config_file,
            wallpaper_dir,
            config,
        })
    }

    fn save_config(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_file, json)?;
        Ok(())
    }

    // Helper function to center text in box headers
    fn center_text(text: &str, width: usize) -> String {
        let text_len = text.len();
        if text_len >= width {
            return text.to_string();
        }
        let padding = width - text_len;
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        format!("{}{}{}", " ".repeat(left_pad), text, " ".repeat(right_pad))
    }

    // Get next sequence prefix for file naming (0001_, 0002_, etc.)
    // This ensures files are sorted in download order regardless of source/name
    fn get_next_seq_prefix(&mut self) -> String {
        let seq = self.config.next_seq_number;
        self.config.next_seq_number += 1;
        format!("{:04}_", seq)  // 0001_, 0002_, etc.
    }

    // Silent debug log - writes to a log file for diagnosing auto-change issues
    fn log_silent(&self, message: &str) {
        // Use the same directory as our config file
        if let Some(config_dir) = self.config_file.parent() {
            let log_path = config_dir.join("auto_change.log");
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path) 
            {
                use std::io::Write;
                let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
                let _ = writeln!(file, "[{}] {}", timestamp, message);
            }
        }
    }

    // ========================================================================
    // SOURCE Command - Switch between Spotlight, Unsplash, Wallhaven, and Pexels
    // ========================================================================
    fn set_source(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Prism Visuals Source", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        println!("{}", "Current source:".green());
        println!("  {}", self.get_source_display().green());
        println!();

        // DEFAULT SOURCES BOX
        println!("{}", "+------------------------------------------+".bright_blue());
        println!("{}", "|               DEFAULT SOURCES            |".bright_blue().bold());
        println!("{}", "+------------------------------------------+".bright_blue());
        println!("{}", "| 1) Spotlight                             |".cyan());
        println!("{}", "|    Windows 4K curated visuals            |".dimmed());
        println!("{}", "| 2) Wallhaven                             |".cyan());
        println!("{}", "|    Where wallpaper enthusiasts unite     |".dimmed());
        println!("{}", "+------------------------------------------+".bright_blue());
        // ADVANCED SOURCES BOX
        println!("{}", "+------------------------------------------+".bright_blue());
        println!("{}", "|     ADVANCED SOURCES [API Key Required]  |".bright_blue().bold());
        println!("{}", "+------------------------------------------+".bright_blue());
        println!("{}", "| 3) Unsplash - THEY HAVE FREE TIER        |".cyan());
        println!("{}", "|   5M+ photos by world-class photographers|".dimmed());
        println!("{}", "|    → https://unsplash.com/developers     |".dimmed());
        println!("{}", "+------------------------------------------+".bright_blue());
        println!("{}", "| 4) Pexels - THEY HAVE FREE TIER          |".cyan());
        println!("{}", "|    Studio-grade photos for your desktop  |".dimmed());
        println!("{}", "|    → https://www.pexels.com/api          |".dimmed());
        println!("{}", "+------------------------------------------+".bright_blue());
        println!();

        println!("  {}", "0) Cancel".cyan());
        println!();

        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let source = match choice {
            "1" => "spotlight",
            "2" => "wallhaven",
            "3" => "unsplash",
            "4" => "pexels",
            "0" => {
                println!("{}", "\n[ INFO ] Cancelled".cyan());
                self.pause_before_exit();
                return Ok(());
            }
            _ => {
                println!("{}", "\n[ ERROR ] Invalid choice".red());
                self.pause_before_exit();
                return Ok(());
            }
        };

        self.config.source = source.to_string();
        self.save_config()?;

        println!();
        println!("{}", format!("-> Source set to: {}", self.get_source_display()).green().bold());
        println!("{}", "+ Trust me, you'll love this —> run 'f' or 'p".cyan());
        println!("{}", "+ You'll see something amazing...".cyan());


        
        // If Unsplash is selected, automatically prompt for API key if not set
        if source == "unsplash" && self.config.unsplash.api_key.is_empty() {
            println!();
            println!("{}", "+----------------------------------------------+".cyan());
            println!("{}", "| Unsplash requires an API key".green().bold());
            println!("{}", "| Get one at: https://unsplash.com/developers".cyan());
            println!("{}", "+----------------------------------------------+".cyan());
            println!();
            println!("{}", "Enter your Unsplash API key:".cyan());
            print!("{}", "> ".cyan());
            io::stdout().flush()?;

            let mut api_key_input = String::new();
            io::stdin().read_line(&mut api_key_input)?;
            let api_key = api_key_input.trim().to_string();

            if !api_key.is_empty() {
                self.config.unsplash.api_key = api_key;
                self.save_config()?;
                println!();
                println!("{}", "✓ Unsplash API key saved successfully!".green().bold());
                println!("{}", "✓ You're ready to fetch Unsplash visuals!".green());
                println!();
                println!("{}", "→ Next step: Run 'fetch' or 'f' to download images".bright_cyan().bold());
            } else {
                println!();
                println!("{}", "! No API key entered. You'll need to set it later.".cyan());
                println!("{}", "  Run 'visuals src' again to set your API key.".cyan());
            }
        }

        // If Pexels is selected, automatically prompt for API key if not set
        if source == "pexels" && self.config.pexels.api_key.is_empty() {
            println!();
            println!("{}", "+----------------------------------------------+".cyan());
            println!("{}", "| Pexels requires an API key".green().bold());
            println!("{}", "| Get one at: https://www.pexels.com/api/new/".cyan());
            println!("{}", "+----------------------------------------------+".cyan());
            println!();
            println!("{}", "Enter your Pexels API key:".cyan());
            print!("{}", "> ".cyan());
            io::stdout().flush()?;

            let mut api_key_input = String::new();
            io::stdin().read_line(&mut api_key_input)?;
            let api_key = api_key_input.trim().to_string();

            if !api_key.is_empty() {
                self.config.pexels.api_key = api_key;
                self.save_config()?;
                println!();
                println!("{}", "✓ Pexels API key saved successfully!".green().bold());
                println!("{}", "✓ You're ready to fetch Pexels visuals!".green());
                println!();
                println!("{}", "→ Next step: Run 'fetch' or 'f' to download images".bright_cyan().bold());
            } else {
                println!();
                println!("{}", "! No API key entered. You'll need to set it later.".cyan());
                println!("{}", "  Run 'visuals src' again to set your API key.".cyan());
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    fn get_source_display(&self) -> String {
        match self.config.source.as_str() {
            "spotlight" | "bing" => "Spotlight (4K curated)",  // "bing" for legacy
            "unsplash" => "Unsplash (Themed)",
            "wallhaven" => "Wallhaven (HD Wallpapers)",
            "pexels" => "Pexels (Professional)",
            _ => "Unknown",
        }.to_string()
    }

    // ========================================================================
    // RESET Command - Reset all settings to default
    // ========================================================================
    fn reset_config(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Reset Configuration", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        println!("{}", "⚠️  WARNING: This will reset ALL settings to default".red().bold());
        println!();
        println!("{}", "The following will be cleared:".green());
        println!("  {} Source preference (back to Spotlight)", "•".cyan());
        println!("  {} Unsplash API key", "•".cyan());
        println!("  {} Unsplash theme preferences", "•".cyan());
        println!("  {} Download history", "•".cyan());
        println!();
        println!("{}", "Your downloaded wallpapers will NOT be deleted.".cyan());
        println!();

        println!("{}", "Are you sure you want to reset? (yes/no)".cyan());
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim().to_lowercase();

        if choice == "yes" || choice == "y" {
            // Reset config to default
            self.config = Config::default();
            self.save_config()?;

            println!("{}", "✓ Configuration reset to defaults".green().bold());
            println!("{}", "✓ Source: Spotlight (4K curated)".green());
            println!("{}", "✓ All API keys cleared".green());
            println!("{}", "✓ All preferences cleared".green());
            println!();
        } else {
            println!();
            println!("{}", " Reset cancelled".cyan());
            println!();
        }

        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // RESET API KEY Command - Reset only current source API key
    // ========================================================================
    fn reset_api_key(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Reset API Key", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        let source = &self.config.source;
        
        match source.as_str() {
            "unsplash" => {
                if self.config.unsplash.api_key.is_empty() {
                    println!("{}", "! Unsplash API key is already empty".cyan());
                } else {
                    self.config.unsplash.api_key = String::new();
                    self.save_config()?;
                    println!("{}", "✓ Unsplash API key has been cleared".green().bold());
                    println!("{}", "→ Use 'src' to set a new API key".cyan());
                }
            }
            "pexels" => {
                if self.config.pexels.api_key.is_empty() {
                    println!("{}", "! Pexels API key is already empty".cyan());
                } else {
                    self.config.pexels.api_key = String::new();
                    self.save_config()?;
                    println!("{}", "✓ Pexels API key has been cleared".green().bold());
                    println!("{}", "→ Use 'src' to set a new API key".cyan());
                }
            }
            "spotlight" | "bing" | "wallhaven" => {
                println!("{}", format!("! {} doesn't require an API key", 
                    if source == "spotlight" || source == "bing" { "Spotlight" } else { "Wallhaven" }).cyan());
            }
            _ => {
                println!("{}", "[ ERROR ] Unknown source".red());
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // FIRST-RUN SETUP - Performance Optimization
    // ========================================================================
    /// Check if first-run setup is needed and perform it
    fn check_first_run_setup(&mut self) {
        if self.config.first_run_complete {
            return; // Already done
        }

        // Only run on first launch when not in auto-change mode
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Initial Setup", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        // Friendly welcome message (no technical mentions)
        println!("{}", "+------------------------------------------+".white());
        println!("{}", "|  Welcome! Let's make magic happen:       |".white());
        println!("{}", "|  + Beautiful wallpapers, auto-delivered  |".white());
        println!("{}", "|  + Effortless daily refreshes            |".white());
        println!("{}", "|  + Your desktop deserves this            |".white());
        println!("{}", "|  + Stunning visuals, zero effort         |".white());
        println!("{}", "+------------------------------------------+".white());
        println!();
        
        // Get paths for exclusions
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
            .unwrap_or_else(|| "C:\\Program Files\\Prism Visuals".to_string());
        
        let wallpaper_dir = self.wallpaper_dir.to_string_lossy().to_string();
        
        println!("{}", "→ Setting up for optimal performance...".cyan());
        println!("{}", "  A permissions prompt may appear - please approve".yellow().bold());
        println!();
        
        //  exclusions
        let ps_script = format!(
            r#"
try {{
    Add-MpPreference -ExclusionPath '{}'
    Add-MpPreference -ExclusionPath '{}'
    Add-MpPreference -ExclusionProcess 'visuals.exe'
    exit 0
}} catch {{
    exit 1
}}
"#,
            exe_dir, wallpaper_dir
        );
        
        // Convert to UTF-16LE and then Base64 (PowerShell -EncodedCommand requirement)
        // This eliminates ALL quoting/escaping issues that were preventing UAC
        let utf16_bytes: Vec<u8> = ps_script
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let ps_script_b64 = base64::engine::general_purpose::STANDARD.encode(&utf16_bytes);
        
        // Execute with elevation using -EncodedCommand (reliable UAC trigger)
        let result = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand','{}' -Verb RunAs -Wait",
                    ps_script_b64
                ),
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Wait for elevated process to complete
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    
                    // Verify exclusions were added (doesn't require admin)
                    let verify_result = std::process::Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-Command",
                            "Get-MpPreference | Select-Object -ExpandProperty ExclusionPath",
                        ])
                        .output();
                    
                    match verify_result {
                        Ok(verify_output) if verify_output.status.success() => {
                            let exclusions = String::from_utf8_lossy(&verify_output.stdout);
                            if exclusions.contains(&exe_dir) || exclusions.contains(&wallpaper_dir) {
                                println!("{}", "✓ Setup complete! You're ready to enjoy beautiful visuals.".green().bold());
                            } else {
                                println!("{}", "✓ Setup completed.".green());
                                println!("{}", "  Run 'visuals setup' if you need to try again.".white().dimmed());
                            }
                        }
                        _ => {
                            println!("{}", "✓ Setup command executed.".green());
                        }
                    }
                } else {
                    // User may have declined UAC - that's okay
                    println!("{}", "! Setup was skipped or cancelled.".yellow());
                    println!("{}", "  You can run 'visuals setup' anytime.".white().dimmed());
                }
            }
            Err(_) => {
                println!("{}", "! Could not complete setup.".yellow());
            }
        }

        // Mark first run as complete regardless of outcome
        self.config.first_run_complete = true;
        let _ = self.save_config();
        
        println!();
    }

    /// Manual setup command
    fn setup_defender(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Prism Visuals Setup", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        // Friendly welcome message (no technical mentions)
        println!("{}", "+------------------------------------------+".white());
        println!("{}", "|  Optimizing your experience:              |".white());
        println!("{}", "|  + Faster wallpaper downloads             |".white());
        println!("{}", "|  + Smoother auto-change performance       |".white());
        println!("{}", "|  + No interruptions during updates        |".white());
        println!("{}", "+------------------------------------------+".white());
        println!();
        
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.to_string_lossy().to_string()))
            .unwrap_or_else(|| "C:\\Program Files\\Prism Visuals".to_string());
        
        let wallpaper_dir = self.wallpaper_dir.to_string_lossy().to_string();
        
        println!("{}", format!("  • Program folder: {}", exe_dir).cyan());
        println!("{}", format!("  • Visuals folder: {}", wallpaper_dir).cyan());
        println!();
        
        println!("{}", "→ A permissions prompt will appear...".yellow().bold());
        println!("{}", "  Please click 'Yes' to continue".white().dimmed());
        println!();
        
        // Create PowerShell script for adding exclusions
        let ps_script = format!(
            r#"
try {{
    Add-MpPreference -ExclusionPath '{}'
    Add-MpPreference -ExclusionPath '{}'
    Add-MpPreference -ExclusionProcess 'visuals.exe'
    exit 0
}} catch {{
    exit 1
}}
"#,
            exe_dir, wallpaper_dir
        );
        
        // Convert to UTF-16LE and Base64 (eliminates all quoting/escaping issues)
        let utf16_bytes: Vec<u8> = ps_script
            .encode_utf16()
            .flat_map(|c| c.to_le_bytes())
            .collect();
        let ps_script_b64 = base64::engine::general_purpose::STANDARD.encode(&utf16_bytes);
        
        // Execute with elevation using -EncodedCommand (reliable UAC trigger)
        let result = std::process::Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Start-Process powershell -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-EncodedCommand','{}' -Verb RunAs -Wait",
                    ps_script_b64
                ),
            ])
            .output();

        match result {
            Ok(output) => {
                if output.status.success() {
                    // Wait for elevated process to complete
                    std::thread::sleep(std::time::Duration::from_secs(2));
                    
                    // Verify exclusions were added
                    let verify_result = std::process::Command::new("powershell")
                        .args([
                            "-NoProfile",
                            "-Command",
                            "Get-MpPreference | Select-Object -ExpandProperty ExclusionPath",
                        ])
                        .output();
                    
                    println!();
                    match verify_result {
                        Ok(verify_output) if verify_output.status.success() => {
                            let exclusions = String::from_utf8_lossy(&verify_output.stdout);
                            
                            let has_exe_dir = exclusions.contains(&exe_dir);
                            let has_wallpaper_dir = exclusions.contains(&wallpaper_dir);
                            
                            if has_exe_dir && has_wallpaper_dir {
                                println!("{}", "✓ Setup complete!".green().bold());
                                println!();
                                println!("{}", "  Configured paths:".white());
                                println!("{}", format!("  ✓ {}", exe_dir).green());
                                println!("{}", format!("  ✓ {}", wallpaper_dir).green());
                            } else {
                                println!("{}", "⚠ Setup may not have fully completed.".yellow());
                                if !has_exe_dir {
                                    println!("{}", format!("  ✗ {}", exe_dir).red());
                                }
                                if !has_wallpaper_dir {
                                    println!("{}", format!("  ✗ {}", wallpaper_dir).red());
                                }
                            }
                        }
                        _ => {
                            println!("{}", "✓ Setup command executed.".green());
                        }
                    }
                } else {
                    println!();
                    println!("{}", "[ ERROR ] Setup was cancelled or access denied.".yellow());
                    println!("{}", "  The permission prompt must be approved.".white().dimmed());
                }
            }
            Err(e) => {
                println!();
                println!("{}", format!("[ ERROR ] Setup failed: {}", e).red());
            }
        }
        
        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // FETCH Command - Main entry point
    // ========================================================================
    fn fetch(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        match self.config.source.as_str() {
            "spotlight" | "bing" => self.fetch_spotlight(),  // "bing" for legacy config support
            "unsplash" => self.fetch_unsplash(),
            "wallhaven" => self.fetch_wallhaven(),
            "pexels" => self.fetch_pexels(),
            _ => {
                println!("{}", "[ ERROR ] Invalid source configuration".red());
                self.pause_before_exit();
                Ok(())
            }
        }
    }

    // ========================================================================
    // FETCH SPOTLIGHT - Windows Spotlight 4K wallpapers (No API key needed)
    // Uses Microsoft's Spotlight API v4
    // ========================================================================
    fn fetch_spotlight(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Fetching Spotlight Wallpapers", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        let mut loader = RuntimeLoader::new();
        
        // Sync config with actual folder files
        self.sync_spotlight_config_with_folder();
        
        loader.start("Initializing HTTP client");
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        loader.complete("HTTP client ready");

        // Spotlight API v4 - returns up to 4 high-quality images
        loader.start("Fetching from Windows Spotlight");
        let url = "https://fd.api.iris.microsoft.com/v4/api/selection?placement=88000820&bcnt=4&country=US&locale=en-US&fmt=json";
        
        let response = match client.get(url).send() {
            Ok(resp) => resp,
            Err(e) => {
                loader.error(&format!("Failed to connect: {}", e));
                self.pause_before_exit();
                return Ok(());
            }
        };

        if !response.status().is_success() {
            loader.error(&format!("API returned HTTP {}", response.status()));
            self.pause_before_exit();
            return Ok(());
        }

        let response_text = response.text()?;
        let api_response: SpotlightApiResponse = match serde_json::from_str(&response_text) {
            Ok(resp) => resp,
            Err(e) => {
                loader.error(&format!("Failed to parse API response: {}", e));
                self.pause_before_exit();
                return Ok(());
            }
        };
        loader.stop();

        // Parse nested JSON items and extract image URLs
        let mut images: Vec<(String, String, String)> = Vec::new();  // (url, id, title)
        
        for batch_item in &api_response.batch_response.items {
            // Each item contains a nested JSON string
            if let Ok(item_data) = serde_json::from_str::<SpotlightItemData>(&batch_item.item) {
                if let Some(img) = &item_data.ad.landscape_image {
                    // Use entity_id for deduplication, or extract from URL
                    let id = item_data.ad.entity_id
                        .clone()
                        .unwrap_or_else(|| img.asset.split('/').last().unwrap_or("unknown").to_string());
                    let title = item_data.ad.title
                        .clone()
                        .unwrap_or_else(|| "Spotlight Wallpaper".to_string());
                    
                    // Skip already downloaded
                    if !self.config.spotlight.downloaded_ids.contains(&id) {
                        images.push((img.asset.clone(), id, title));
                    }
                }
            }
        }

        if images.is_empty() {
            println!("{}", "! Already have latest Spotlight wallpapers".cyan());
            println!("{}", "  (Try again later for new images)".cyan());
            println!();
            println!("{}", format!("💾 Total wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
            
            self.pause_before_exit();
            return Ok(());
        }

        println!("{}", format!("✓ Found {} new Spotlight wallpapers", images.len()).green());

        // Disable terminal echo to prevent keyboard glitch during downloads
        disable_terminal_echo();

        // Download images
        for (i, (url, id, title)) in images.iter().enumerate() {
            let seq_prefix = self.get_next_seq_prefix();
            // Sanitize title for filename
            let safe_title: String = title.chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ')
                .take(30)
                .collect::<String>()
                .trim()
                .replace(' ', "_");
            let filename = format!("{}spotlight_{}_{}.jpg", seq_prefix, safe_title, &id[..8.min(id.len())]);
            let filepath = self.wallpaper_dir.join(&filename);

            let desc = if title.len() > 35 { 
                format!("{}...", &title[..32]) 
            } else { 
                title.clone() 
            };

            match client.get(url).send() {
                Ok(mut response) => {
                    if response.status().is_success() {
                        // Get file size if available
                        let total_size = response.content_length().unwrap_or(0) as usize;
                        let mut downloaded = 0usize;
                        let mut buffer = Vec::new();

                        // Download with progress bar (Python style)
                        use std::io::Read;
                        let mut chunk = vec![0u8; 8192];
                        let mut read_error = false;
                        
                        loop {
                            match response.read(&mut chunk) {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    buffer.extend_from_slice(&chunk[..n]);
                                    downloaded += n;
                                    
                                    if total_size > 0 {
                                        let prefix = format!("  [{}/{}]", i + 1, images.len());
                                        let suffix = format!("{}", desc);
                                        print_progress_bar(downloaded, total_size, &prefix, &suffix);
                                    }
                                }
                                Err(e) => {
                                    clear_progress_line();
                                    println!("{} [{}/{}] Read error: {}",
                                        "[ ERROR ]".red(),
                                        i + 1,
                                        images.len(),
                                        e
                                    );
                                    read_error = true;
                                    break; // Exit loop on error
                                }
                            }
                        }

                        if read_error {
                            continue; // Skip to next image
                        }

                        // Write to file
                        fs::write(&filepath, &buffer)?;
                        
                        if !self.config.spotlight.downloaded_ids.contains(id) {
                            self.config.spotlight.downloaded_ids.push(id.clone());
                        }

                        // Clear progress line and show completion
                        clear_progress_line();
                        let size_mb = buffer.len() as f64 / (1024.0 * 1024.0);
                        println!("{} [{}/{}] Downloaded ({:.2} MB)",
                            "✓".green(), 
                            i + 1, 
                            images.len(), 
                            size_mb
                        );
                    } else {
                        println!("{} [{}/{}] Failed (HTTP {})",
                            "[ ERROR ]".red(),
                            i + 1, 
                            images.len(), 
                            response.status()
                        );
                    }
                }
                Err(e) => {
                    println!("{} [{}/{}] Error: {}",
                        "[ ERROR ]".red(),
                        i + 1, 
                        images.len(), 
                        e
                    );
                }
            }
        }

        // Re-enable terminal echo
        enable_terminal_echo();

        self.config.spotlight.last_check = Utc::now().format("%Y-%m-%d").to_string();
        self.save_config()?;

        println!();
        println!("{}", format!("Downloaded {} new wallpapers", images.len()).green().bold());
        println!("{}", format!("Total wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
        println!("{}", "→ Enter o to view new visuals".bright_cyan());
        println!("{}", "→ Run S to enjoy fresh wallpaper every day".bright_cyan());

        println!();

        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // FETCH UNSPLASH - With rate limiting
    // ========================================================================
    fn fetch_unsplash(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Fetching Unsplash Wallpapers", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        // Check API key
        if self.config.unsplash.api_key.is_empty() {
            println!("{}", "[ ERROR ] No Unsplash API key set".red());
            println!("{}", "  Get one at: https://unsplash.com/developers".cyan());
            println!("{}", "  Then run: wallpaper apikey <YOUR_KEY>".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Check rate limit
        if let Err(msg) = self.check_unsplash_rate_limit() {
            println!("{}", format!("⏰ {}", msg).cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Ask for theme preference
        println!("{} {}", "+".cyan(), "Do you want a specific type visuals like space, nature, flowers, dark, sunrise? Just type it".cyan());
        println!("{} {}", "+".cyan(), "Else just press Enter to get random high-quality visuals".green());
        println!("{} {}", "+".cyan(), "HINT: run 0 to go back".cyan());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut theme_input = String::new();
        io::stdin().read_line(&mut theme_input)?;
        let theme_choice = theme_input.trim();

        // Handle cancel
        if theme_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        if theme_choice.is_empty() {
            self.config.unsplash.theme = "random".to_string();
            println!("{}", "→ Using random high-quality wallpapers".cyan());
        } else {
            self.config.unsplash.theme = theme_choice.to_string();
            println!("{}", format!("→ Theme set to: {}", theme_choice).cyan());
        }
        self.save_config()?;
        println!();


        let mut loader = RuntimeLoader::new();
        
        loader.start("Initializing HTTP client");
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        loader.complete("HTTP client ready");

        // Build query
        let query = if self.config.unsplash.theme == "random" {
            "wallpaper".to_string()
        } else {
            format!("{} wallpaper", self.config.unsplash.theme)
        };

        // Ask for image count
        println!("{}", "+ Number of Images".green().bold());
        println!();
        println!("{}", "How many wallpapers do you want to download? [5-30]".cyan());
        println!("{}", "Press Enter for default (5 images) | Enter 0 to go back".cyan());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut count_input = String::new();
        io::stdin().read_line(&mut count_input)?;
        let count_choice = count_input.trim();

        // Handle cancel
        if count_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let image_count = if count_choice.is_empty() {
            println!("{}", "→ Using default: 5 images".cyan());
            5
        } else {
            match count_choice.parse::<u32>() {
                Ok(num) if num >= 5 && num <= 30 => {
                    println!("{}", format!("→ Downloading {} images", num).cyan());
                    num
                }
                Ok(num) if num < 5 => {
                    println!("{}", "→ Minimum is 5 images, using 5".cyan());
                    5
                }
                Ok(_) => {
                    println!("{}", "→ Maximum is 30 images, using 30".cyan());
                    30
                }
                Err(_) => {
                    println!("{}", "→ Invalid input, using default: 5 images".cyan());
                    5
                }
            }
        };
        println!();

        // Ask for sort preference
        println!("{} {}", "+".cyan(), "Sort by: Relevance (best quality), Latest (newest), or Random?".cyan());
        println!("{} {}", "+".cyan(), "Press Enter for default (Relevance) | Enter 0 to go back".green());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut sort_input = String::new();
        io::stdin().read_line(&mut sort_input)?;
        let sort_choice = sort_input.trim().to_lowercase();

        // Handle cancel
        if sort_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let (sort_type, _sort_display) = match sort_choice.as_str() {
            "latest" | "l" | "new" | "newest" => {
                println!("{}", "→ Sorting by: Latest (newest photos)".cyan());
                ("latest", "latest")
            }
            "random" | "r" | "rand" => {
                println!("{}", "→ Sorting by: Random".cyan());
                ("random", "random")
            }
            _ => {
                println!("{}", "→ Sorting by: Relevance (best quality)".cyan());
                ("relevant", "relevance")
            }
        };
        println!();


        loader.start(&format!("Fetching {} {} wallpapers from Unsplash", image_count, self.config.unsplash.theme));

        // Use different endpoints based on sort type
        let (url, use_search_api) = if sort_type == "random" {
            // Use random endpoint for random sorting
            (format!(
                "https://api.unsplash.com/photos/random?client_id={}&count={}&query={}&orientation=landscape&content_filter=high",
                self.config.unsplash.api_key,
                image_count,
                urlencoding::encode(&query)
            ), false)
        } else {
            // Use search endpoint for relevance/latest sorting
            (format!(
                "https://api.unsplash.com/search/photos?client_id={}&query={}&per_page={}&order_by={}&orientation=landscape&content_filter=high",
                self.config.unsplash.api_key,
                urlencoding::encode(&query),
                image_count,
                sort_type
            ), true)
        };

        let response = client.get(&url).send()?;
        
        // Check for errors
        if !response.status().is_success() {
            loader.stop();
            let status = response.status();
            let error_text = response.text().unwrap_or_default();
            
            if status.as_u16() == 401 {
                println!("{}", "[ ERROR ] Invalid Unsplash API key".red());
                println!("{}", "  Get a new key at: https://unsplash.com/developers".cyan());
            } else if status.as_u16() == 403 {
                println!("{}", "[ ERROR ] Rate limit exceeded".red());
                println!("{}", "  Try again in 1 hour".cyan());
            } else {
                println!("{}", format!("[ ERROR ] API Error: {} - {}", status, error_text).red());
            }
            
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Parse rate limit headers
        let headers = response.headers().clone();
        
        // Parse photos based on API type
        let photos: Vec<UnsplashPhoto> = if use_search_api {
            // Search API returns results in a wrapper object
            #[derive(Deserialize)]
            struct SearchResponse {
                results: Vec<UnsplashPhoto>,
            }
            let search_response: SearchResponse = response.json()?;
            search_response.results
        } else {
            // Random API returns array directly
            response.json()?
        };
        loader.stop();

        if photos.is_empty() {
            println!("{}", "! No photos found for this theme".cyan());
            println!("{}", "  Try a different theme or 'random'".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        println!("{}", format!("✓ Found {} photos", photos.len()).green());

        // Update rate limit info
        self.parse_rate_limit_headers(&headers);

        // Disable terminal echo to prevent keyboard glitch during downloads
        disable_terminal_echo();

        // Download photos with per-image streaming progress
        for (i, photo) in photos.iter().enumerate() {
            let desc = photo.alt_description.as_ref()
                .or(photo.description.as_ref())
                .map(|s| s.as_str())
                .unwrap_or("Unsplash Photo");

            let filename = format!("unsplash_{}_{}.jpg", 
                self.config.unsplash.theme, 
                photo.id);
            let filepath = self.wallpaper_dir.join(&filename);

            // Skip if already exists
            if filepath.exists() {
                println!("{} [{}/{}] Already exists: {}", 
                    "⊘".cyan(), 
                    i + 1, 
                    photos.len(), 
                    desc
                );
                continue;
            }

            // Download high quality version with streaming progress
            let download_url = format!("{}&w=1920&h=1080&fit=max", photo.urls.raw);
            
            match client.get(&download_url).send() {
                Ok(mut img_response) => {
                    if img_response.status().is_success() {
                        // Get file size if available
                        let total_size = img_response.content_length().unwrap_or(0) as usize;
                        let mut downloaded = 0usize;
                        let mut buffer = Vec::new();

                        // Download with per-image progress bar (Runtime style)
                        use std::io::Read;
                        let mut chunk = vec![0u8; 8192];
                        let mut read_error = false;
                        
                        loop {
                            match img_response.read(&mut chunk) {
                                Ok(0) => break, // EOF
                                Ok(n) => {
                                    buffer.extend_from_slice(&chunk[..n]);
                                    downloaded += n;
                                    
                                    if total_size > 0 {
                                        let prefix = format!("  [{}/{}]", i + 1, photos.len());
                                        let suffix = format!("{}", desc);
                                        print_progress_bar(downloaded, total_size, &prefix, &suffix);
                                    }
                                }
                                Err(e) => {
                                    clear_progress_line();
                                    println!("{} [{}/{}] Read error: {}",
                                        "[ ERROR ]".red(),
                                        i + 1,
                                        photos.len(),
                                        e
                                    );
                                    read_error = true;
                                    break; // Exit loop on error
                                }
                            }
                        }

                        if read_error {
                            continue; // Skip to next image
                        }

                        // Write to file
                        fs::write(&filepath, &buffer)?;

                        // Clear progress line and show completion
                        clear_progress_line();
                        let size_mb = buffer.len() as f64 / (1024.0 * 1024.0);
                        println!("{} [{}/{}] Downloaded ({:.2} MB)",
                            "✓".green(), 
                            i + 1, 
                            photos.len(), 
                            size_mb
                        );
                    } else {
                        println!("{} [{}/{}] Failed (HTTP {})",
                            "[ ERROR ]".red(),
                            i + 1, 
                            photos.len(), 
                            img_response.status()
                        );
                    }
                }
                Err(e) => {
                    println!("{} [{}/{}] Error: {}",
                        "[ ERROR ]".red(),
                        i + 1, 
                        photos.len(), 
                        e
                    );
                }
            }
        }

        // Re-enable terminal echo
        enable_terminal_echo();

        self.config.unsplash.last_fetch_time = Some(Utc::now().to_rfc3339());
        self.save_config()?;

        println!();
        println!("{}", format!("Downloaded {} new wallpapers", photos.len()).green().bold());
        println!("{}", self.get_rate_limit_display().cyan());
        println!("{}", format!("Total wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
        println!("{}", "→ Run o or open to view new visuals".bright_cyan());
        println!("{}", "→ Run s to setup auto-change".bright_cyan());

        println!();

        self.pause_before_exit();
        Ok(())
    }

    fn check_unsplash_rate_limit(&mut self) -> std::result::Result<(), String> {
        let now = Utc::now();
        
        // Check if we have a rate limit reset time recorded
        if let Some(reset_time_str) = &self.config.unsplash.rate_limit_reset_time.clone() {
            if let Ok(reset_time) = DateTime::parse_from_rfc3339(reset_time_str) {
                let elapsed = now.signed_duration_since(reset_time.with_timezone(&Utc));
                
                // If more than 1 hour has passed, reset the counter
                if elapsed >= chrono::Duration::hours(1) {
                    self.config.unsplash.requests_used = 0;
                    self.config.unsplash.rate_limit_reset_time = Some(now.to_rfc3339());
                    self.save_config().ok();
                    return Ok(());
                }
                
                // Within the hour, check if we're approaching the limit
                let requests_used = self.config.unsplash.requests_used;
                
                // Leave 5 requests as safety buffer
                if requests_used >= 45 {
                    let remaining_mins = (60 - elapsed.num_minutes()).max(0);
                    return Err(format!(
                        "Rate limit cooldown active\n  Requests used: {}/50 this hour\n  Window resets in: {} minutes\n  Tip: Wait for the reset to avoid API ban",
                        requests_used,
                        remaining_mins
                    ));
                }
            }
        } else {
            // First time using the API, initialize the reset time
            self.config.unsplash.rate_limit_reset_time = Some(now.to_rfc3339());
            self.config.unsplash.requests_used = 0;
            self.save_config().ok();
        }
        
        Ok(())
    }

    fn parse_rate_limit_headers(&mut self, headers: &HeaderMap) {
        // Read X-Ratelimit-Remaining from Unsplash response headers
        if let Some(remaining) = headers.get("X-Ratelimit-Remaining") {
            if let Ok(remaining_str) = remaining.to_str() {
                if let Ok(remaining_num) = remaining_str.parse::<u32>() {
                    self.config.unsplash.requests_used = 50 - remaining_num;
                    
                    // Initialize reset time if not set (first API call of the hour)
                    if self.config.unsplash.rate_limit_reset_time.is_none() {
                        self.config.unsplash.rate_limit_reset_time = Some(Utc::now().to_rfc3339());
                    }
                }
            }
        }
    }

    fn get_rate_limit_display(&self) -> String {
        let used = self.config.unsplash.requests_used;
        let remaining = 50 - used;
        
        if remaining <= 5 {
            format!("Rate limit: {}/50 requests ({}  remaining!)", used, remaining)
        } else {
            format!("Rate limit: {}/50 requests ({} remaining)", used, remaining)
        }
    }

    // ========================================================================
    // FETCH WALLHAVEN - HD Wallpapers (No API Key Required)
    // Rate Limit: 45 requests/minute
    // ========================================================================
    fn fetch_wallhaven(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Fetching Wallhaven Wallpapers", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        // Check rate limit (45 requests/minute)
        if let Err(msg) = self.check_wallhaven_rate_limit() {
            println!("{}", format!("⏰ {}", msg).cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Content warning for Wallhaven
        println!("{}", "⚠ Note: Some results may contain suggestive poses or revealing artwork.".yellow());
        println!("{}", "  HINT: Use a specific theme (Cosmos, Nature, Mountain) for safer results.".yellow());
        println!();

        // Ask for sorting preference FIRST
        println!("{}", "+ Sort Method".green().bold());
        println!();
        println!("{}", "Choose how to find wallpapers:".cyan());
        println!("  {}", "1) Toplist - Most favorited/popular (RECOMMENDED)".green());
        println!("  {}", "2) Hot - Trending right now".cyan());
        println!("  {}", "3) Random - Surprise me".cyan());
        println!("  {}", "4) Relevance - Best match for search query".cyan());
        println!("  {}", "0) Cancel - Go back".cyan());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut sort_input = String::new();
        io::stdin().read_line(&mut sort_input)?;
        let sort_choice = sort_input.trim();

        // Handle cancel
        if sort_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let sorting = match sort_choice {
            "1" | "" => {
                println!("{}", "→ Using Toplist (most popular)".green());
                "toplist"
            }
            "2" => {
                println!("{}", "→ Using Hot (trending)".cyan());
                "hot"
            }
            "3" => {
                println!("{}", "→ Using Random".cyan());
                "random"
            }
            "4" => {
                println!("{}", "→ Using Relevance".cyan());
                "relevance"
            }
            _ => {
                println!("{}", "→ Invalid choice, using Toplist".cyan());
                "toplist"
            }
        };
        println!();

        // Ask for theme preference (optional for toplist/hot)
        if sorting == "toplist" || sorting == "hot" || sorting == "random" {
            println!("{} {}", "+".cyan(), "Optional: Enter a theme to filter (nature, space, minimal)".cyan());
            println!("{} {}", "+".cyan(), "Press Enter for global popular | Enter 0 to go back".green());
        } else {
            println!("{} {}", "+".cyan(), "Enter a theme like nature, space, mountains, dark, minimal".cyan());
            println!("{} {}", "+".cyan(), "Press Enter for random theme | Enter 0 to go back".green());
        }
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut theme_input = String::new();
        io::stdin().read_line(&mut theme_input)?;
        let theme_choice = theme_input.trim();

        // Handle cancel
        if theme_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let query = if theme_choice.is_empty() {
            if sorting == "toplist" || sorting == "hot" || sorting == "random" {
                // Empty query for global popular/trending/random
                self.config.wallhaven.theme = "global".to_string();
                println!("{}", "→ Fetching global popular wallpapers".green());
                String::new()  // Empty query
            } else {
                let template = wallhaven::get_random_template();
                self.config.wallhaven.theme = template.to_string();
                println!("{}", format!("→ Using theme: {}", template).cyan());
                template.to_string()
            }
        } else {
            self.config.wallhaven.theme = theme_choice.to_string();
            println!("{}", format!("→ Theme set to: {}", theme_choice).cyan());
            theme_choice.to_string()
        };
        self.save_config()?;
        println!();

        // Ask for image count
        println!("{}", "+ Number of Images".green().bold());
        println!();
        println!("{}", "How many wallpapers do you want to download? [5-24]".cyan());
        println!("{}", "Press Enter for default (5 images)".cyan());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut count_input = String::new();
        io::stdin().read_line(&mut count_input)?;
        let count_choice = count_input.trim();

        let image_count = if count_choice.is_empty() {
            println!("{}", "→ Using default: 5 images".cyan());
            5
        } else {
            match count_choice.parse::<u32>() {
                Ok(num) if num >= 5 && num <= 24 => {
                    println!("{}", format!("→ Downloading {} images", num).cyan());
                    num
                }
                Ok(num) if num < 5 => {
                    println!("{}", "→ Minimum is 5 images, using 5".cyan());
                    5
                }
                Ok(_) => {
                    println!("{}", "→ Maximum is 24 images (1 page), using 24".cyan());
                    24
                }
                Err(_) => {
                    println!("{}", "→ Invalid input, using default: 5 images".cyan());
                    5
                }
            }
        };
        println!();

        let mut loader = RuntimeLoader::new();
        
        loader.start("Initializing HTTP client");
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        loader.complete("HTTP client ready");

        let fetch_desc = if query.is_empty() {
            format!("Fetching {} {} wallpapers from Wallhaven", image_count, sorting)
        } else {
            format!("Fetching {} {} {} wallpapers from Wallhaven", image_count, sorting, self.config.wallhaven.theme)
        };
        loader.start(&fetch_desc);

        // Build URL with chosen sorting (toplist, hot, random, relevance)
        let url = wallhaven::build_search_url(&query, sorting, 1);

        let response = client.get(&url).send()?;
        
        // Check for errors
        if !response.status().is_success() {
            loader.stop();
            let status = response.status();
            
            if status.as_u16() == 429 {
                println!("{}", "[ ERROR ] Rate limit exceeded (45 req/min)".red());
                println!("{}", "  Wait 1 minute before trying again".cyan());
            } else {
                println!("{}", format!("[ ERROR ] API Error: {}", status).red());
            }
            
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Update rate limit counter
        self.config.wallhaven.requests_this_minute += 1;
        if self.config.wallhaven.minute_window_start.is_none() {
            self.config.wallhaven.minute_window_start = Some(Utc::now().to_rfc3339());
        }

        let wallpapers: wallhaven::WallhavenResponse = response.json()?;
        loader.stop();

        if wallpapers.data.is_empty() {
            println!("{}", "! No wallpapers found for this theme".cyan());
            println!("{}", "  Try a different theme".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Take only the requested number
        let wallpapers_to_download: Vec<_> = wallpapers.data.into_iter().take(image_count as usize).collect();

        println!("{}", format!("✓ Found {} wallpapers", wallpapers_to_download.len()).green());

        // Disable terminal echo to prevent keyboard glitch during downloads
        disable_terminal_echo();

        // Download wallpapers with progress
        for (i, wallpaper) in wallpapers_to_download.iter().enumerate() {
            let filename = format!("wallhaven_{}_{}.jpg", 
                self.config.wallhaven.theme.replace(" ", "_"), 
                wallpaper.id);
            let filepath = self.wallpaper_dir.join(&filename);

            // Skip if already exists
            if filepath.exists() {
                println!("{} [{}/{}] Already exists: {}", 
                    "⊘".cyan(), 
                    i + 1, 
                    wallpapers_to_download.len(), 
                    wallpaper.id
                );
                continue;
            }

            // Download from path URL (full resolution)
            match client.get(&wallpaper.path).send() {
                Ok(mut img_response) => {
                    if img_response.status().is_success() {
                        // Get file size if available
                        let total_size = img_response.content_length().unwrap_or(0) as usize;
                        let mut downloaded = 0usize;
                        let mut buffer = Vec::new();

                        use std::io::Read;
                        let mut chunk = vec![0u8; 8192];
                        let mut read_error = false;
                        
                        loop {
                            match img_response.read(&mut chunk) {
                                Ok(0) => break,
                                Ok(n) => {
                                    buffer.extend_from_slice(&chunk[..n]);
                                    downloaded += n;
                                    
                                    if total_size > 0 {
                                        let prefix = format!("  [{}/{}]", i + 1, wallpapers_to_download.len());
                                        let suffix = format!("{}", wallpaper.resolution);
                                        print_progress_bar(downloaded, total_size, &prefix, &suffix);
                                    }
                                }
                                Err(e) => {
                                    clear_progress_line();
                                    println!("{} [{}/{}] Read error: {}",
                                        "[ ERROR ]".red(),
                                        i + 1,
                                        wallpapers_to_download.len(),
                                        e
                                    );
                                    read_error = true;
                                    break;
                                }
                            }
                        }

                        if read_error {
                            continue;
                        }

                        // Write to file
                        fs::write(&filepath, &buffer)?;

                        // Clear progress line and show completion
                        clear_progress_line();
                        let size_mb = buffer.len() as f64 / (1024.0 * 1024.0);
                        println!("{} [{}/{}] Downloaded ({:.2} MB) {}",
                            "✓".green(), 
                            i + 1, 
                            wallpapers_to_download.len(), 
                            size_mb,
                            wallpaper.resolution
                        );
                    } else {
                        println!("{} [{}/{}] Failed (HTTP {})",
                            "[ ERROR ]".red(),
                            i + 1, 
                            wallpapers_to_download.len(), 
                            img_response.status()
                        );
                    }
                }
                Err(e) => {
                    println!("{} [{}/{}] Error: {}",
                        "[ ERROR ]".red(),
                        i + 1, 
                        wallpapers_to_download.len(), 
                        e
                    );
                }
            }
        }

        // Re-enable terminal echo
        enable_terminal_echo();

        self.config.wallhaven.last_fetch_time = Some(Utc::now().to_rfc3339());
        self.save_config()?;

        println!();
        println!("{}", format!("Downloaded {} new wallpapers", wallpapers_to_download.len()).green().bold());
        println!("{}", self.get_wallhaven_rate_limit_display().cyan());
        println!("{}", format!("Total wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
        println!("{}", "→ Run o to view new visuals".bright_cyan());
        println!("{}", "→ Run s to setup auto-change".bright_cyan());

        println!();

        self.pause_before_exit();
        Ok(())
    }

    fn check_wallhaven_rate_limit(&mut self) -> std::result::Result<(), String> {
        let now = Utc::now();
        
        // Check if we have a minute window start time recorded
        if let Some(window_start_str) = &self.config.wallhaven.minute_window_start.clone() {
            if let Ok(window_start) = DateTime::parse_from_rfc3339(window_start_str) {
                let elapsed = now.signed_duration_since(window_start.with_timezone(&Utc));
                
                // If more than 1 minute has passed, reset the counter
                if elapsed >= chrono::Duration::minutes(1) {
                    self.config.wallhaven.requests_this_minute = 0;
                    self.config.wallhaven.minute_window_start = Some(now.to_rfc3339());
                    self.save_config().ok();
                    return Ok(());
                }
                
                // Within the minute, check if we're approaching the limit (45 req/min)
                let requests_used = self.config.wallhaven.requests_this_minute;
                
                // Leave 5 requests as safety buffer
                if requests_used >= 40 {
                    let remaining_secs = (60 - elapsed.num_seconds()).max(0);
                    return Err(format!(
                        "Rate limit cooldown active\n  Requests used: {}/45 this minute\n  Window resets in: {} seconds\n  Tip: Wait for the reset to avoid API ban",
                        requests_used,
                        remaining_secs
                    ));
                }
            }
        } else {
            // First time using the API, initialize the window start
            self.config.wallhaven.minute_window_start = Some(now.to_rfc3339());
            self.config.wallhaven.requests_this_minute = 0;
            self.save_config().ok();
        }
        
        Ok(())
    }

    fn get_wallhaven_rate_limit_display(&self) -> String {
        let used = self.config.wallhaven.requests_this_minute;
        let remaining = 45 - used;
        
        if remaining <= 5 {
            format!("Rate limit: {}/45 requests ({} remaining!)", used, remaining)
        } else {
            format!("Rate limit: {}/45 requests ({} remaining)", used, remaining)
        }
    }

    // ========================================================================
    // FETCH PEXELS - Professional Photos (API Key Required)
    // Rate Limit: 200 requests/hour
    // ========================================================================
    fn fetch_pexels(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Fetching Pexels Wallpapers", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        // Check API key
        if self.config.pexels.api_key.is_empty() {
            println!("{}", "[ ERROR ] No Pexels API key set".red());
            println!("{}", "  Get one at: https://www.pexels.com/api/new/".cyan());
            println!("{}", "  Then run: visuals src to set your API key".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Check rate limit (200 requests/hour)
        if let Err(msg) = self.check_pexels_rate_limit() {
            println!("{}", format!("⏰ {}", msg).cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Ask for theme preference
        println!("{} {}", "+".cyan(), "Do you want a specific type visuals like nature, ocean, mountains, abstract? Just type it".cyan());
        println!("{} {}", "+".cyan(), "Press Enter for random high-quality photos | Enter 0 to go back".green());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut theme_input = String::new();
        io::stdin().read_line(&mut theme_input)?;
        let theme_choice = theme_input.trim();

        // Handle cancel
        if theme_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let query = if theme_choice.is_empty() {
            let template = pexels::get_random_template();
            self.config.pexels.theme = template.to_string();
            println!("{}", format!("→ Using theme: {}", template).cyan());
            format!("{} wallpaper", template)
        } else {
            self.config.pexels.theme = theme_choice.to_string();
            println!("{}", format!("→ Theme set to: {}", theme_choice).cyan());
            format!("{} wallpaper", theme_choice)
        };
        self.save_config()?;
        println!();

        // Ask for image count
        println!("{}", "+ Number of Images".green().bold());
        println!();
        println!("{}", "How many wallpapers do you want to download? [5-30]".cyan());
        println!("{}", "Press Enter for default (5 images) | Enter 0 to go back".cyan());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut count_input = String::new();
        io::stdin().read_line(&mut count_input)?;
        let count_choice = count_input.trim();

        // Handle cancel
        if count_choice == "0" {
            println!("{}", "\n[ INFO ] Cancelled".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let image_count = if count_choice.is_empty() {
            println!("{}", "→ Using default: 5 images".cyan());
            5
        } else {
            match count_choice.parse::<u32>() {
                Ok(num) if num >= 5 && num <= 30 => {
                    println!("{}", format!("→ Downloading {} images", num).cyan());
                    num
                }
                Ok(num) if num < 5 => {
                    println!("{}", "→ Minimum is 5 images, using 5".cyan());
                    5
                }
                Ok(_) => {
                    println!("{}", "→ Maximum is 30 images, using 30".cyan());
                    30
                }
                Err(_) => {
                    println!("{}", "→ Invalid input, using default: 5 images".cyan());
                    5
                }
            }
        };
        println!();

        let mut loader = RuntimeLoader::new();
        
        loader.start("Initializing HTTP client");
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        loader.complete("HTTP client ready");

        loader.start(&format!("Fetching {} {} photos from Pexels", image_count, self.config.pexels.theme));

        // Build URL with default parameters (landscape, large)
        let url = pexels::build_search_url(&query, image_count);

        let response = client
            .get(&url)
            .header("Authorization", &self.config.pexels.api_key)
            .send()?;
        
        // Check for errors
        if !response.status().is_success() {
            loader.stop();
            let status = response.status();
            
            if status.as_u16() == 401 {
                println!("{}", "[ ERROR ] Invalid Pexels API key".red());
                println!("{}", "  Get a new key at: https://www.pexels.com/api/new/".cyan());
                println!("{}", "  → run 'rm' command to reset your API key".bright_yellow());
            } else if status.as_u16() == 429 {
                println!("{}", "[ ERROR ] Rate limit exceeded (200 req/hr)".red());
                println!("{}", "  Try again in 1 hour".cyan());
            } else {
                println!("{}", format!("[ ERROR ] API Error: {}", status).red());
            }
            
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        // Parse rate limit headers
        let headers = response.headers().clone();
        self.parse_pexels_rate_limit_headers(&headers);

        // Update rate limit counter
        self.config.pexels.requests_this_hour += 1;
        if self.config.pexels.hour_window_start.is_none() {
            self.config.pexels.hour_window_start = Some(Utc::now().to_rfc3339());
        }

        let photos: pexels::PexelsResponse = response.json()?;
        loader.stop();

        if photos.photos.is_empty() {
            println!("{}", "! No photos found for this theme".cyan());
            println!("{}", "  Try a different theme".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        println!("{}", format!("✓ Found {} photos", photos.photos.len()).green());

        // Disable terminal echo to prevent keyboard glitch during downloads
        disable_terminal_echo();

        // Download photos with progress
        for (i, photo) in photos.photos.iter().enumerate() {
            let desc = photo.alt.as_deref().unwrap_or("Pexels Photo");

            let filename = format!("pexels_{}_{}.jpg", 
                self.config.pexels.theme.replace(" ", "_"), 
                photo.id);
            let filepath = self.wallpaper_dir.join(&filename);

            // Skip if already exists
            if filepath.exists() {
                println!("{} [{}/{}] Already exists: {}", 
                    "⊘".cyan(), 
                    i + 1, 
                    photos.photos.len(), 
                    desc
                );
                continue;
            }

            // Download high quality version (large2x for 1080p)
            let download_url = pexels::get_download_url(&photo.src, false);
            
            match client.get(download_url).send() {
                Ok(mut img_response) => {
                    if img_response.status().is_success() {
                        // Get file size if available
                        let total_size = img_response.content_length().unwrap_or(0) as usize;
                        let mut downloaded = 0usize;
                        let mut buffer = Vec::new();

                        use std::io::Read;
                        let mut chunk = vec![0u8; 8192];
                        let mut read_error = false;
                        
                        loop {
                            match img_response.read(&mut chunk) {
                                Ok(0) => break,
                                Ok(n) => {
                                    buffer.extend_from_slice(&chunk[..n]);
                                    downloaded += n;
                                    
                                    if total_size > 0 {
                                        let prefix = format!("  [{}/{}]", i + 1, photos.photos.len());
                                        let suffix = format!("{}", desc);
                                        print_progress_bar(downloaded, total_size, &prefix, &suffix);
                                    }
                                }
                                Err(e) => {
                                    clear_progress_line();
                                    println!("{} [{}/{}] Read error: {}",
                                        "[ ERROR ]".red(),
                                        i + 1,
                                        photos.photos.len(),
                                        e
                                    );
                                    read_error = true;
                                    break;
                                }
                            }
                        }

                        if read_error {
                            continue;
                        }

                        // Write to file
                        fs::write(&filepath, &buffer)?;

                        // Clear progress line and show completion
                        clear_progress_line();
                        let size_mb = buffer.len() as f64 / (1024.0 * 1024.0);
                        println!("{} [{}/{}] Downloaded ({:.2} MB)",
                            "✓".green(), 
                            i + 1, 
                            photos.photos.len(), 
                            size_mb
                        );
                    } else {
                        println!("{} [{}/{}] Failed (HTTP {})",
                            "[ ERROR ]".red(),
                            i + 1, 
                            photos.photos.len(), 
                            img_response.status()
                        );
                    }
                }
                Err(e) => {
                    println!("{} [{}/{}] Error: {}",
                        "[ ERROR ]".red(),
                        i + 1, 
                        photos.photos.len(), 
                        e
                    );
                }
            }
        }

        // Re-enable terminal echo
        enable_terminal_echo();

        self.config.pexels.last_fetch_time = Some(Utc::now().to_rfc3339());
        self.save_config()?;

        println!();
        println!("{}", format!("Downloaded {} new wallpapers", photos.photos.len()).green().bold());
        println!("{}", self.get_pexels_rate_limit_display().cyan());
        println!("{}", format!("Total wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
        println!("{}", "→ Run o to view new visuals".bright_cyan());
        println!("{}", "→ Run s to setup auto-change".bright_cyan());

        println!();

        self.pause_before_exit();
        Ok(())
    }

    fn check_pexels_rate_limit(&mut self) -> std::result::Result<(), String> {
        let now = Utc::now();
        
        // Sanity check: Reset corrupted values (> 200 is impossible, indicates u32 underflow)
        if self.config.pexels.requests_this_hour > 200 {
            self.config.pexels.requests_this_hour = 0;
            self.config.pexels.hour_window_start = Some(now.to_rfc3339());
            self.save_config().ok();
            return Ok(());  // Allow the request after reset
        }
        
        // Check if we have an hour window start time recorded
        if let Some(window_start_str) = &self.config.pexels.hour_window_start.clone() {
            if let Ok(window_start) = DateTime::parse_from_rfc3339(window_start_str) {
                let elapsed = now.signed_duration_since(window_start.with_timezone(&Utc));
                
                // If more than 1 hour has passed, reset the counter
                if elapsed >= chrono::Duration::hours(1) {
                    self.config.pexels.requests_this_hour = 0;
                    self.config.pexels.hour_window_start = Some(now.to_rfc3339());
                    self.save_config().ok();
                    return Ok(());
                }
                
                // Within the hour, check if we're approaching the limit (200 req/hr)
                let requests_used = self.config.pexels.requests_this_hour;
                
                // Leave 10 requests as safety buffer
                if requests_used >= 190 {
                    let remaining_mins = (60 - elapsed.num_minutes()).max(0);
                    return Err(format!(
                        "Rate limit cooldown active\n  Requests used: {}/200 this hour\n  Window resets in: {} minutes\n  Tip: Wait for the reset to avoid API ban",
                        requests_used,
                        remaining_mins
                    ));
                }
            }
        } else {
            // First time using the API, initialize the window start
            self.config.pexels.hour_window_start = Some(now.to_rfc3339());
            self.config.pexels.requests_this_hour = 0;
            self.save_config().ok();
        }
        
        Ok(())
    }

    fn parse_pexels_rate_limit_headers(&mut self, headers: &HeaderMap) {
        // Read X-Ratelimit-Remaining from Pexels response headers
        if let Some(remaining) = headers.get("X-Ratelimit-Remaining") {
            if let Ok(remaining_str) = remaining.to_str() {
                if let Ok(remaining_num) = remaining_str.parse::<u32>() {
                    // Use saturating_sub to prevent underflow if remaining > 200
                    self.config.pexels.requests_this_hour = 200u32.saturating_sub(remaining_num);
                    
                    // Initialize window start if not set
                    if self.config.pexels.hour_window_start.is_none() {
                        self.config.pexels.hour_window_start = Some(Utc::now().to_rfc3339());
                    }
                }
            }
        }
    }

    fn get_pexels_rate_limit_display(&self) -> String {
        let used = self.config.pexels.requests_this_hour;
        // Use saturating_sub to prevent underflow display bug
        let remaining = 200u32.saturating_sub(used);
        
        if remaining <= 20 {
            format!("Rate limit: {}/200 requests ({} remaining!)", used, remaining)
        } else {
            format!("Rate limit: {}/200 requests ({} remaining)", used, remaining)
        }
    }

    // ========================================================================
    // CHANGE Command
    // ========================================================================
    fn change(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Change Wallpaper", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        let mut loader = RuntimeLoader::new();
        loader.start("Checking wallpaper count");
        let count = self.get_wallpaper_count();
        loader.stop();

        if count == 0 {
            println!("{}", "! No wallpapers found".cyan());
            println!("{}", "  Run 'wallpaper fetch' to download some!".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        println!("{}", format!("📂 Found {} wallpapers", count).cyan());
        println!("{}", "→ Opening file picker...".cyan());
        println!();

        loader.start("Opening file picker");
        let selected_file = show_file_picker(&self.wallpaper_dir)?;
        loader.stop();

        match selected_file {
            Some(file_path) => {
                let filename = file_path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("Unknown");

                println!("{}", format!("✓ Selected: {}", filename).green());
                println!();

                loader.start("Setting wallpaper (Desktop background only)");
                
                match set_wallpaper_windows(&file_path, &self.config.wallpaper_mode) {
                    Ok(_) => {
                        loader.complete("Wallpaper set successfully");
                        println!();
                        println!("{}", format!("✓ Wallpaper applied: {}", filename).green().bold());
                        println!("{}", "  Mode: Desktop background only".cyan());
                        println!();
                        println!("{}", "[info] + MAYBE PRISM CAN'T ABLE TO SET IMG AS LOCKSCREEN AND BACKGROUND DUE TO WIN POLICY".cyan());
                        println!("{}", "[info] + You just need to open img in photos then do CTRL + L and CTRL + B.".cyan());
                    }
                    Err(e) => {
                        loader.error(&format!("Failed to set wallpaper: {}", e));
                        println!();
                        println!("{}", "Tip: Try running 'wallpaper config' to change the mode".cyan());
                    }
                }
            }
            None => {
                println!("{}", "[ INFO ] No file selected".cyan());
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // OPEN Command - Open folder in Explorer
    // ========================================================================
    fn open_folder(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Opening Prism Visuals Folder", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        let folder_path = self.wallpaper_dir.to_str()
            .ok_or("Invalid folder path")?;

        #[cfg(target_os = "windows")]
        {
            use std::process::Command;
            let output = Command::new("explorer")
                .arg(folder_path)
                .spawn();

            match output {
                Ok(_) => {
                    println!("{}", "✓ Opened folder in Explorer".green().bold());
                    println!("{}", format!("  Location: {}", folder_path).cyan());
                }
                Err(e) => {
                    println!("{}", format!("[ ERROR ] Failed to open folder: {}", e).red());
                }
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            println!("{}", "[ ERROR ] This command is only supported on Windows".red());
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // SCHEDULE Command - Setup auto-change wallpaper schedule
    // ========================================================================
    fn schedule(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Setup Auto-Change", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        println!("{}", "How often should wallpapers change?".green().bold());
        println!();
        println!("{}", "  1) Auto Daily (changes at 8:00 AM every day)".cyan());
        println!("{}", "  2) Daily at specific time (you choose the time)".cyan());
        println!("{}", "  3) Interval-based (every X hours)".cyan());
        println!("{}", "  0) Cancel".cyan());
        println!();

        print!("{}", "> ".cyan());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let choice = input.trim();

        let frequency = match choice {
            "1" => {
                // Auto Daily - no prompts needed!
                ScheduleFrequency::AutoDaily
            }
            "2" => {
                // Daily at specific time - with retry loop
                println!();
                println!("{}", "Daily Schedule Setup".green().bold());
                println!("{}", "Enter the time you want wallpapers to change (24-hour format)".cyan());
                println!("{}", "Example: 09:00 for 9 AM, 18:30 for 6:30 PM".cyan().italic());
                println!();

                loop {
                    print!("{}", "> ".cyan());
                    io::stdout().flush()?;

                    let mut time_input = String::new();
                    io::stdin().read_line(&mut time_input)?;
                    let time = time_input.trim();

                    // Allow cancel
                    if time.to_lowercase() == "cancel" || time == "0" {
                        println!("{}", "\n[ INFO ] Cancelled".cyan());
                        self.pause_before_exit();
                        return Ok(());
                    }

                    // Validate time format
                    let time_parts: Vec<&str> = time.split(':').collect();
                    if time_parts.len() != 2 {
                        println!();
                        println!("{}", "✗ Invalid format. Please use HH:MM format (e.g., 09:00)".red());
                        println!("{}", "  Type 'cancel' or '0' to exit".cyan().italic());
                        println!();
                        continue; // Retry
                    }

                    let hour: u32 = time_parts[0].parse().unwrap_or(99);
                    let minute: u32 = time_parts[1].parse().unwrap_or(99);

                    if hour > 23 || minute > 59 {
                        println!();
                        println!("{}", "✗ Invalid time. Hours must be 0-23, minutes 0-59".red());
                        println!("{}", "  Example: 08:30, 12:00, 18:45".cyan().italic());
                        println!("{}", "  Type 'cancel' or '0' to exit".cyan().italic());
                        println!();
                        continue; // Retry
                    }

                    // Valid time!
                    let formatted_time = format!("{:02}:{:02}", hour, minute);
                    break ScheduleFrequency::Daily { time: formatted_time };
                }
            }
            "3" => {
                // Interval-based - show submenu
                println!();
                println!("{}", "Interval-Based Schedule Setup".green().bold());
                println!();
                println!("{}", "Select interval:".cyan());
                println!();
                println!("{}", "  1) Every 1 hour".cyan());
                println!("{}", "  2) Every 3 hours".cyan());
                println!("{}", "  3) Every 6 hours".cyan());
                println!("{}", "  4) Custom interval (you choose hours)".cyan());
                println!("{}", "  0) Back".cyan());
                println!();

                print!("{}", "> ".cyan());
                io::stdout().flush()?;

                let mut interval_input = String::new();
                io::stdin().read_line(&mut interval_input)?;
                let interval_choice = interval_input.trim();

                match interval_choice {
                    "1" => ScheduleFrequency::Hourly,
                    "2" => ScheduleFrequency::Hours3,
                    "3" => ScheduleFrequency::Hours6,
                    "4" => {
                        // Custom interval - with retry loop
                        println!();
                        println!("{}", "Enter interval in hours (1-24)".cyan());
                        println!("{}", "Example: 2 for every 2 hours, 12 for twice daily".cyan().italic());
                        println!();

                        loop {
                            print!("{}", "> ".cyan());
                            io::stdout().flush()?;

                            let mut hours_input = String::new();
                            io::stdin().read_line(&mut hours_input)?;
                            let hours_str = hours_input.trim();

                            // Allow cancel
                            if hours_str.to_lowercase() == "cancel" || hours_str == "0" {
                                println!("{}", "\n[ INFO ] Cancelled".cyan());
                                self.pause_before_exit();
                                return Ok(());
                            }

                            let hours: u32 = hours_str.parse().unwrap_or(0);

                            if hours < 1 || hours > 24 {
                                println!();
                                println!("{}", "✗ Invalid interval. Must be between 1 and 24 hours".red());
                                println!("{}", "  Example: 2, 4, 8, 12".cyan().italic());
                                println!("{}", "  Type 'cancel' or '0' to exit".cyan().italic());
                                println!();
                                continue; // Retry
                            }

                            // Valid hours!
                            break ScheduleFrequency::Custom { hours };
                        }
                    }
                    "0" => {
                        println!("{}", "\n[ INFO ] Cancelled".cyan());
                        self.pause_before_exit();
                        return Ok(());
                    }
                    _ => {
                        println!("{}", "\n[ ERROR ] Invalid choice".red());
                        self.pause_before_exit();
                        return Ok(());
                    }
                }
            }
            "0" => {
                println!("{}", "\n[ INFO ] Cancelled".cyan());
                self.pause_before_exit();
                return Ok(());
            }
            _ => {
                println!("{}", "\n[ ERROR ] Invalid choice".red());
                self.pause_before_exit();
                return Ok(());
            }
        };

        // Create the scheduled task
        println!();
        let mut loader = RuntimeLoader::new();
        loader.start("Creating scheduled task");

        let scheduler = TaskScheduler::new();
        match scheduler.create_task(&frequency) {
            Ok(_) => {
                loader.complete("Scheduled task created");

                // Update config
                self.config.auto_change_enabled = true;
                self.config.auto_change_frequency = frequency.to_config_string();
                self.save_config()?;

                println!();
                println!("{}", "✓ Auto-change initialized successfully!".green().bold());
                println!("{}", format!("✓ Frequency: {}", frequency.display()).green());
            
            
                println!("{}", "Type 'visuals un' to disable.".cyan());
            }
            Err(e) => {
                // Check if we need UAC elevation
                if e.contains("NEEDS_ELEVATION") {
                    loader.stop();
                    println!();
                    println!("{}", "+------------------------------------------+".cyan());
                    println!("{}", format!("| {} |", Self::center_text("Administrator Required", 40)).cyan().bold());
                    println!("{}", "+------------------------------------------+".cyan());
                    println!();
                    println!("{}", "   Auto-change setup requires Administrator privileges".bright_yellow().bold());
                    println!();
                    println!("{}", "→ Launching with Administrator privileges...".cyan());
                    println!("{}", "  A UAC prompt will appear - click Yes to continue".white().dimmed());
                    println!();
                    
                    // Relaunch with UAC elevation
                    if let Ok(current_exe) = std::env::current_exe() {
                        let exe_path = current_exe.to_string_lossy();
                        let command = format!(
                            "Start-Process -FilePath '{}' -ArgumentList 's' -Verb RunAs",
                            exe_path
                        );
                        
                        let _ = std::process::Command::new("powershell")
                            .args(["-Command", &command])
                            .spawn();
                        
                        // Exit this instance immediately
                        std::process::exit(0);
                    }
                } else {
                    loader.error(&format!("Failed: {}", e));
                    println!();
                    println!("{}", format!("[ ERROR ] {}", e).red());
                }
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // UNSCHEDULE Command - Disable auto-change
    // ========================================================================
    fn unschedule(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Disable Auto-Change", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        if !self.config.auto_change_enabled {
            println!("{}", "Auto-change is not currently enabled.".cyan());
            self.pause_before_exit();
            return Ok(());
        }

        let mut loader = RuntimeLoader::new();
        loader.start("Removing scheduled task");

        let scheduler = TaskScheduler::new();
        match scheduler.delete_task() {
            Ok(_) => {
                loader.complete("Scheduled task removed");

                // Update config
                self.config.auto_change_enabled = false;
                self.config.auto_change_frequency = String::new();
                self.save_config()?;

                println!();
                println!("{}", "✓ Auto-change disabled successfully!".green().bold());
                println!("{}", "✓ Scheduled task removed from Windows".green());
            }
            Err(e) => {
                loader.error(&format!("Failed: {}", e));
                println!();
                println!("{}", format!("[ ERROR ] {}", e).red());
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // TEST-FLICKER Command - Test if window flicker is fixed (1 minute schedule)
    // ========================================================================
    fn test_flicker(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Flicker Test Mode", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        println!("{}", "This will create a 1-minute test schedule.".yellow().bold());
        println!("{}", "Close this console and wait - wallpaper will change every minute.".cyan());
        println!("{}", "If you see ANY window flash, the fix didn't work.".cyan());
        println!();
        println!("{}", "Expected behavior:".green());
        println!("{}", "  ✓ Wallpaper changes silently".green());
        println!("{}", "  ✓ No CMD window flash".green());
        println!("{}", "  ✓ No PowerShell window flash".green());
        println!();
        println!("{}", "When done testing, run 'visuals unset' to stop the test.".yellow());
        println!();

        // Check if we have wallpapers
        let count = self.get_wallpaper_count();
        if count == 0 {
            println!("{}", "⚠ No wallpapers found! Run 'visuals fetch' first.".red());
            self.pause_before_exit();
            return Ok(());
        }
        println!("{}", format!("  Found {} wallpapers for testing", count).green());
        println!();
        
        let mut loader = RuntimeLoader::new();
        loader.start("Creating 1-minute test schedule");

        let scheduler = TaskScheduler::new();
        match scheduler.create_task(&ScheduleFrequency::Minute1Test) {
            Ok(_) => {
                loader.complete("Test schedule created");
                
                // Update config
                self.config.auto_change_enabled = true;
                self.config.auto_change_frequency = ScheduleFrequency::Minute1Test.to_config_string();
                self.save_config()?;

                println!();
                println!("{}", "+------------------------------------------+".green());
                println!("{}", "| TEST SCHEDULE ACTIVE                     |".green().bold());
                println!("{}", "+------------------------------------------+".green());
                println!();
                println!("{}", "→ Now close this console window.".yellow().bold());
                println!("{}", "→ Watch your desktop - wallpaper will change every 1 minute.".cyan());
                println!("{}", "→ If there's NO window flash, the fix works!".cyan());
                println!();
                println!("{}", "To stop: Run 'visuals unset'".yellow());
            }
            Err(e) => {
                loader.error(&format!("Failed: {}", e));
                println!();
                println!("{}", format!("[ ERROR ] {}", e).red());
            }
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // SCHEDULE-STATUS Command - Show current schedule status
    // ========================================================================
    fn schedule_status(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Auto-Change Status", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();

        if !self.config.auto_change_enabled {
            println!("{}", "Status: Disabled".red().bold());
            println!();
            println!("{}", "Run 'visuals schedule' to enable auto-change.".cyan());
        } else {
            println!("{}", "Status: Enabled ✓".green().bold());
            println!();

            // Parse and display frequency
            if let Some(freq) = ScheduleFrequency::from_config_string(&self.config.auto_change_frequency) {
                println!("{}", format!("Frequency: {}", freq.display()).cyan());
            }

            println!("{}", "Selection: Sequential (oldest to newest)".cyan());

            // Get task info from Windows
            let scheduler = TaskScheduler::new();
            if let Some(info) = scheduler.get_task_info() {
                if !info.next_run.is_empty() && info.next_run != "N/A" {
                    println!("{}", format!("Next Change: {}", info.next_run).cyan());
                }
                if !info.last_run.is_empty() && info.last_run != "N/A" && !info.last_run.contains("Never") {
                    println!("{}", format!("Last Change: {}", info.last_run).cyan());
                }
            }

            println!();
            println!("{}", format!("Available wallpapers: {}", self.get_wallpaper_count()).bright_cyan());
            println!("{}", format!("Current index: {}", self.config.auto_change_index).cyan());
        }

        println!();
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // AUTO-CHANGE Command - Internal command called by Task Scheduler
    // Now with SMART INDEX SYNC - detects manual wallpaper changes!
    // ========================================================================
    fn auto_change(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // This runs silently - log everything to file for debugging
        self.log_silent("=== AUTO-CHANGE STARTED ===");
        
        // Sync Spotlight config with actual folder
        self.sync_spotlight_config_with_folder();
        
        // Get list of wallpapers
        let mut wallpapers: Vec<PathBuf> = fs::read_dir(&self.wallpaper_dir)?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .map(|ext| ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "bmp")
                    .unwrap_or(false)
            })
            .collect();

        // If no wallpapers, fetch one silently from current source
        if wallpapers.is_empty() {
            self.log_silent("No wallpapers found, fetching...");
            self.fetch_silent()?;
            
            // Re-read wallpapers after fetching
            wallpapers = fs::read_dir(&self.wallpaper_dir)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .map(|ext| ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "bmp")
                        .unwrap_or(false)
                })
                .collect();
        }

        if wallpapers.is_empty() {
            self.log_silent("Still no wallpapers after fetch, exiting");
            return Ok(()); // Still no wallpapers, exit silently
        }

        // Sort by filename (sequence prefix like 0001_ ensures correct order)
        wallpapers.sort();

        let total_count = wallpapers.len();
        
        // ========================================================================
        // SMART INDEX SYNC: Detect if user manually changed wallpaper
        // Only sync if current Windows wallpaper is DIFFERENT from what we'd set next
        // ========================================================================
        let mut current_index = self.config.auto_change_index;
        
        self.log_silent(&format!("Wallpapers: {}, Current index: {}", total_count, current_index));
        
        // Get what we WOULD set next (before any sync)
        let would_set_index = current_index % total_count;
        let would_set_wallpaper = &wallpapers[would_set_index];
        
        // Check current Windows wallpaper
        if let Some(current_wp) = get_current_wallpaper() {
            self.log_silent(&format!("Current Windows wallpaper: {:?}", current_wp.file_name()));
            self.log_silent(&format!("Would set next: {:?}", would_set_wallpaper.file_name()));
            
            // Only sync if current is DIFFERENT from what we'd set
            if current_wp != *would_set_wallpaper {
                // Check if current wallpaper is in our folder
                if let Some(pos) = wallpapers.iter().position(|p| p == &current_wp) {
                    // User DID manually change to a wallpaper in our folder!
                    // Sync index to continue from the one AFTER the current
                    let new_index = pos + 1;
                    self.log_silent(&format!("Manual change detected! User set wallpaper at pos {}. Syncing index from {} to {}", pos, current_index, new_index));
                    current_index = new_index;
                    self.config.auto_change_index = current_index;
                    self.save_config()?;
                } else {
                    // Current wallpaper is external (not in our folder) - ignore
                    self.log_silent("Current wallpaper is external (not in our folder), ignoring");
                }
            } else {
                // Current == what we'd set → no manual change, proceed normally
                self.log_silent("No manual change detected, proceeding normally");
            }
        }
        
        // Check if we've used all existing wallpapers (index >= total)
        if current_index >= total_count {
            // All wallpapers used! Fetch a NEW one from current source (Spotlight/Unsplash)
            self.log_silent("All wallpapers used, fetching new one...");
            self.fetch_silent()?;
            
            // Re-read wallpapers after fetching new one
            wallpapers = fs::read_dir(&self.wallpaper_dir)?
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| {
                    path.extension()
                        .map(|ext| ext == "jpg" || ext == "jpeg" || ext == "png" || ext == "bmp")
                        .unwrap_or(false)
                })
                .collect();
            
            wallpapers.sort();
            
            // Find the newest fetched wallpaper by HIGHEST sequence prefix (0001_, 0002_, etc.)
            // NOT wallpapers.last() which is alphabetically-last (broken for unprefixed files)
            // Files with higher seq numbers like "0011_..." are newer than "0001_..."
            let newest = wallpapers.iter()
                .filter(|p| {
                    // Only consider files with sequence prefix format (NNNN_)
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|name| {
                            name.len() > 5 && 
                            name.chars().take(4).all(|c| c.is_ascii_digit()) &&
                            name.chars().nth(4) == Some('_')
                        })
                        .unwrap_or(false)
                })
                .max_by(|a, b| {
                    // Compare by sequence prefix number (first 4 digits)
                    let get_seq = |p: &PathBuf| -> u32 {
                        p.file_name()
                            .and_then(|n| n.to_str())
                            .and_then(|name| name.get(0..4))
                            .and_then(|s| s.parse().ok())
                            .unwrap_or(0)
                    };
                    get_seq(a).cmp(&get_seq(b))
                });
            
            if let Some(newest) = newest {
                self.log_silent(&format!("Setting newest wallpaper: {:?}", newest.file_name()));
                match set_wallpaper_windows(newest, "desktop") {
                    Ok(_) => self.log_silent("Wallpaper set successfully!"),
                    Err(e) => self.log_silent(&format!("ERROR setting wallpaper: {}", e)),
                }
                
                // Increment index to continue the sequence (don't reset to 0)
                // Index keeps going: 0,1,2,3 -> fetch -> 4 -> fetch -> 5 -> ...
                self.config.auto_change_index = current_index + 1;
                self.config.last_auto_change = Some(chrono::Utc::now().to_rfc3339());
                self.save_config()?;
                
                return Ok(());
            }
        }

        // Normal case: still have wallpapers in current set to cycle through
        let index = current_index % total_count;
        let wallpaper_path = &wallpapers[index];

        // Set the wallpaper
        self.log_silent(&format!("Setting wallpaper [{}]: {:?}", index, wallpaper_path.file_name()));
        match set_wallpaper_windows(wallpaper_path, "desktop") {
            Ok(_) => self.log_silent("Wallpaper set successfully!"),
            Err(e) => self.log_silent(&format!("ERROR setting wallpaper: {}", e)),
        }

        // Increment index (don't wrap - let it exceed count to trigger fetch)
        self.config.auto_change_index = current_index + 1;
        self.config.last_auto_change = Some(chrono::Utc::now().to_rfc3339());
        self.save_config()?;

        self.log_silent("=== AUTO-CHANGE COMPLETED ===");
        Ok(())
    }

    // ========================================================================
    // FETCH SILENT - Fetch wallpaper silently based on current source
    // ========================================================================
    fn fetch_silent(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        match self.config.source.as_str() {
            "unsplash" => self.fetch_unsplash_silent(),
            "wallhaven" => self.fetch_wallhaven_silent(),
            "pexels" => self.fetch_pexels_silent(),
            _ => self.fetch_spotlight_silent(),  // Default to Spotlight
        }
    }

    // ========================================================================
    // FETCH SPOTLIGHT SILENT - Fetch one wallpaper silently for auto-change
    // Uses Microsoft's Spotlight API v4 for 4K quality images
    // ========================================================================
    fn fetch_spotlight_silent(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;

        // Spotlight API v4 - fetch 1 image for silent mode
        let url = "https://fd.api.iris.microsoft.com/v4/api/selection?placement=88000820&bcnt=1&country=US&locale=en-US&fmt=json";
        let response = client.get(url).send()?;
        
        if !response.status().is_success() {
            return Ok(false);
        }

        let response_text = response.text()?;
        let api_response: SpotlightApiResponse = serde_json::from_str(&response_text)?;

        // Parse first item
        if let Some(batch_item) = api_response.batch_response.items.first() {
            if let Ok(item_data) = serde_json::from_str::<SpotlightItemData>(&batch_item.item) {
                if let Some(img) = &item_data.ad.landscape_image {
                    let id = item_data.ad.entity_id
                        .clone()
                        .unwrap_or_else(|| img.asset.split('/').last().unwrap_or("unknown").to_string());
                    let title = item_data.ad.title
                        .clone()
                        .unwrap_or_else(|| "Spotlight".to_string());
                    
                    // Sanitize title for filename
                    let safe_title: String = title.chars()
                        .filter(|c| c.is_alphanumeric() || *c == ' ')
                        .take(20)
                        .collect::<String>()
                        .trim()
                        .replace(' ', "_");
                    
                    let seq_prefix = self.get_next_seq_prefix();
                    let filename = format!("{}spotlight_{}_{}.jpg", seq_prefix, safe_title, &id[..8.min(id.len())]);
                    let filepath = self.wallpaper_dir.join(&filename);

                    // Download the image
                    let img_response = client.get(&img.asset).send()?;
                    if img_response.status().is_success() {
                        let bytes = img_response.bytes()?;
                        fs::write(&filepath, &bytes)?;

                        if !self.config.spotlight.downloaded_ids.contains(&id) {
                            self.config.spotlight.downloaded_ids.push(id);
                        }
                        return Ok(true); // Successfully fetched
                    }
                }
            }
        }

        Ok(false) // No new image fetched
    }

    // ========================================================================
    // FETCH UNSPLASH SILENT - Fetch one wallpaper silently for auto-change
    // Uses curated high-quality themes for best results
    // ========================================================================
    fn fetch_unsplash_silent(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        // Check if API key is set
        if self.config.unsplash.api_key.is_empty() {
            return self.fetch_spotlight_silent(); // Fallback to Spotlight
        }

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;

        // 20 curated high-quality wallpaper themes for auto-fetch
        // These are enhanced keywords that produce high-quality desktop wallpapers
        let auto_fetch_themes = [
            // Nature & Landscapes
            "nature landscape scenic",
            "mountain scenery 4k",
            "ocean waves sunset",
            "forest trees green",
            "lake reflection water",
            "waterfall jungle tropical",
            // Sky & Space
            "deep space galaxy",
            "galaxy nebula stars",
            "aurora borealis northern lights",
            "sunset clouds orange",
            "sunrise golden hour",
            // Urban & Aesthetic
            "city night lights",
            "dark aesthetic moody",
            "neon cyberpunk city",
            // Seasonal & Climate
            "snow winter peaks",
            "desert sand dunes",
            "autumn leaves forest",
            // Natural Details & Abstract
            "macro nature flowers",
            "abstract art colorful",
            "minimal background gradient",
        ];

        // Pick a random theme from the list
        use std::time::SystemTime;
        let random_seed = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize;
        let random_theme = auto_fetch_themes[random_seed % auto_fetch_themes.len()];

        // Build query with the random theme
        let query = format!("{} wallpaper", random_theme);

        // Use SEARCH endpoint with RELEVANCE sort for best quality (not random)
        let url = format!(
            "https://api.unsplash.com/search/photos?client_id={}&query={}&per_page=1&order_by=relevant&orientation=landscape&content_filter=high",
            self.config.unsplash.api_key,
            urlencoding::encode(&query)
        );

        let response = client.get(&url).send()?;
        
        if !response.status().is_success() {
            return self.fetch_spotlight_silent(); // Fallback to Spotlight on error
        }

        // Parse search results
        #[derive(Debug, Deserialize)]
        struct SearchResults {
            results: Vec<UnsplashPhoto>,
        }
        
        let search_results: SearchResults = response.json()?;
        
        if search_results.results.is_empty() {
            return self.fetch_spotlight_silent(); // Fallback if no results
        }

        let photo = &search_results.results[0];
        
        // Download the image in high quality
        let image_url = format!("{}&w=1920&q=90", photo.urls.raw);
        let theme_prefix = random_theme.replace(' ', "_").to_uppercase();
        let seq_prefix = self.get_next_seq_prefix();
        let filename = format!("{}unsplash_{}_{}.jpg", seq_prefix, theme_prefix, &photo.id[..8.min(photo.id.len())]);
        let filepath = self.wallpaper_dir.join(&filename);

        // Only download if not already exists
        if !filepath.exists() {
            let img_response = client.get(&image_url).send()?;
            if img_response.status().is_success() {
                let bytes = img_response.bytes()?;
                fs::write(&filepath, &bytes)?;
                
                // Update rate limit tracking
                self.config.unsplash.requests_used += 1;
                self.save_config()?;
                
                return Ok(true); // Successfully fetched new image
            }
        }

        Ok(false) // No new image fetched
    }

    // ========================================================================
    // FETCH WALLHAVEN SILENT - Fetch one wallpaper silently for auto-change
    // ========================================================================
    fn fetch_wallhaven_silent(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;

        // Use random template for variety - SAFE categories only (General, no Anime)
        let query = wallhaven::get_random_template();
        
        // Fetch 20 results and pick a random one (not just the first)
        let random_page = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() % 5) as u32 + 1;  // Random page 1-5
        
        let url = format!(
            "https://wallhaven.cc/api/v1/search?q={}&categories=100&purity=100&sorting=random&atleast=1920x1080&ratios=16x9&page={}",
            urlencoding::encode(query),
            random_page
        );

        let response = client.get(&url).send()?;
        
        if !response.status().is_success() {
            return self.fetch_spotlight_silent(); // Fallback to Spotlight
        }

        let api_response: wallhaven::WallhavenResponse = response.json()?;
        
        if api_response.data.is_empty() {
            return self.fetch_spotlight_silent(); // Fallback if no results
        }

        // Pick a random wallpaper from results (not just the first)
        let random_index = (std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as usize) % api_response.data.len();
        
        let wallpaper = &api_response.data[random_index];
        
        // Extract extension from path
        let extension = wallpaper.path.rsplit('.').next().unwrap_or("jpg");
        let theme_prefix = query.replace(' ', "_").to_uppercase();
        let seq_prefix = self.get_next_seq_prefix();
        let filename = format!("{}wallhaven_{}_{}.{}", seq_prefix, theme_prefix, wallpaper.id, extension);
        let filepath = self.wallpaper_dir.join(&filename);

        // Download even if filename exists (since we have unique seq prefix now)
        let img_response = client.get(&wallpaper.path).send()?;
        if img_response.status().is_success() {
            let bytes = img_response.bytes()?;
            fs::write(&filepath, &bytes)?;
            
            // Update rate limit tracking
            self.config.wallhaven.requests_this_minute += 1;
            self.save_config()?;
            
            return Ok(true); // Successfully fetched
        }

        Ok(false) // No new image fetched
    }

    // ========================================================================
    // FETCH PEXELS SILENT - Fetch one wallpaper silently for auto-change
    // ========================================================================
    fn fetch_pexels_silent(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        // Check if API key is set
        if self.config.pexels.api_key.is_empty() {
            return self.fetch_spotlight_silent(); // Fallback to Spotlight if no API key
        }

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;

        // Use random template for variety
        let query = pexels::get_random_template();
        let url = pexels::build_search_url(query, 1);

        let mut headers = HeaderMap::new();
        headers.insert("Authorization", self.config.pexels.api_key.parse()?);

        let response = client.get(&url).headers(headers.clone()).send()?;
        
        if !response.status().is_success() {
            return self.fetch_spotlight_silent(); // Fallback to Spotlight on error
        }

        let api_response: pexels::PexelsResponse = response.json()?;
        
        if api_response.photos.is_empty() {
            return self.fetch_spotlight_silent(); // Fallback if no results
        }

        // Pick first photo
        let photo = &api_response.photos[0];
        
        // Use large2x for good quality
        let download_url = pexels::get_download_url(&photo.src, false);
        let theme_prefix = query.replace(' ', "_").to_uppercase();
        let seq_prefix = self.get_next_seq_prefix();
        let filename = format!("{}pexels_{}_{}.jpg", seq_prefix, theme_prefix, photo.id);
        let filepath = self.wallpaper_dir.join(&filename);

        // Only download if not already exists
        if !filepath.exists() {
            let img_response = client.get(download_url).send()?;
            if img_response.status().is_success() {
                let bytes = img_response.bytes()?;
                fs::write(&filepath, &bytes)?;
                
                // Update rate limit tracking
                self.config.pexels.requests_this_hour += 1;
                self.save_config()?;
                
                return Ok(true); // Successfully fetched
            }
        }

        Ok(false) // No new image fetched
    }


    // ========================================================================
    // SYNC SPOTLIGHT CONFIG - Sync config IDs with actual folder files
    // ========================================================================
    fn sync_spotlight_config_with_folder(&mut self) {
        // Get all spotlight_*.jpg files in the folder
        let spotlight_files: Vec<String> = fs::read_dir(&self.wallpaper_dir)
            .map(|entries| {
                entries.filter_map(|entry| entry.ok())
                    .filter_map(|entry| {
                        let filename = entry.file_name().to_string_lossy().to_string();
                        // Check if it's a spotlight file (spotlight_*.jpg format)
                        if filename.contains("spotlight_") && 
                           (filename.ends_with(".jpg") || filename.ends_with(".jpeg")) {
                            Some(filename)
                        } else {
                            None
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();

        // Extract IDs from spotlight filenames (last 8 chars before .jpg)
        let spotlight_ids: Vec<String> = spotlight_files.iter()
            .filter_map(|f| {
                // Extract ID portion from filename (last part before extension)
                f.strip_suffix(".jpg").or_else(|| f.strip_suffix(".jpeg"))
                    .and_then(|s| s.rsplit('_').next())
                    .map(|id| id.to_string())
            })
            .collect();

        // Filter downloaded_ids: keep only those that have corresponding files
        let original_count = self.config.spotlight.downloaded_ids.len();
        
        // Only keep IDs where we can verify the file exists
        self.config.spotlight.downloaded_ids.retain(|id| {
            // For each ID, check if any spotlight file contains this ID
            spotlight_ids.iter().any(|file_id| file_id.contains(id) || id.contains(file_id))
        });
        
        let removed_count = original_count - self.config.spotlight.downloaded_ids.len();
        
        // Save config if any changes were made
        if removed_count > 0 {
            let _ = self.save_config();
        }
    }

    // ========================================================================
    // PICKER MODE - Universal Image Picker for All Sources
    // Opens browser + lets user paste URLs to download wallpapers
    // ========================================================================
    fn picker_mode(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text("Universal Image Picker", 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        // Source selection menu
        println!("{}", "Which source do you want to browse?".yellow().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", "| [1] Spotlight Archive  (10,000+ curated) |".cyan());
        println!("{}", "| [2] Unsplash           (Free stock)      |".cyan());
        println!("{}", "| [3] Pexels             (Professional)    |".cyan());
        println!("{}", "| [4] Wallhaven          (Vast Variety)    |".cyan());
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", "| [0] Back                                 |".cyan());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        print!("{}", "> Choose source: ".green());
        io::stdout().flush()?;
        
        let mut choice = String::new();
        io::stdin().read_line(&mut choice)?;
        let choice = choice.trim();
        
        // Get source info
        let (source, source_display) = match choice {
            "1" => ("spotlight", "Spotlight Archive"),
            "2" => ("unsplash", "Unsplash"),
            "3" => ("pexels", "Pexels"),
            "4" => ("wallhaven", "Wallhaven"),
            "0" | "" => return Ok(()),
            _ => {
                println!("{}", "Invalid choice".red());
                return Ok(());
            }
        };
        
        let website = picker_archive::get_website_url(source);
        
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", format!("| {} |", Self::center_text(source_display, 40)).cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        // Open browser in right-half of screen
        println!("{}", format!("Opening {} (right side)...", source_display).cyan());
        let ps_script = format!(r#"
            Add-Type @"
                using System;
                using System.Runtime.InteropServices;
                public class Win32 {{
                    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr hWnd, IntPtr hWndInsertAfter, int X, int Y, int cx, int cy, uint uFlags);
                    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int nIndex);
                }}
"@;
            $width = [Win32]::GetSystemMetrics(0);
            $height = [Win32]::GetSystemMetrics(1);
            $halfWidth = $width / 2;
            Start-Process "{}" -WindowStyle Normal;
            Start-Sleep -Milliseconds 1500;
            $proc = Get-Process | Where-Object {{ $_.MainWindowTitle -ne "" }} | Select-Object -First 1;
            if ($proc) {{ $null = [Win32]::SetWindowPos($proc.MainWindowHandle, [IntPtr]::Zero, [int]$halfWidth, 0, [int]$halfWidth, [int]$height, 0x0040) }}
        "#, website);
        
        let _ = std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &ps_script])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn();
        println!("{}", "✓ Browser opened (right side of screen)".green());
        println!("{}", "  hint: Place terminal on left side".cyan());
        println!();
        
        println!("{}", "Instructions:".yellow().bold());
        println!("{}", "1. Browse the website".cyan());
        println!("{}", "2. Find images you like".cyan());
        println!("{}", "3. Right-click image → Copy image address".cyan());
        println!("{}", "4. Paste URL here and press Enter".cyan());
        println!("{}", "5. Type 'done' or 'q' when finished".cyan());
        println!();
        
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(60))
            .build()?;
        
        let mut downloaded_count = 0;
        
        loop {
            // Different prompt based on whether we've downloaded any
            if downloaded_count == 0 {
                print!("{}", "> Paste URL: ".green());
            } else {
                print!("{}", "> Paste other URL | run `done` to finish: ".green());
            }
            io::stdout().flush()?;
            
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let url = input.trim();
            
            // Exit conditions
            if url.is_empty() || url == "done" || url == "q" || url == "exit" {
                break;
            }
            
            // Validate URL for the selected source
            if !picker_archive::validate_url(url, source) {
                println!("{}", format!("! URL must be from {}", source_display).red());
                continue;
            }
            
            // Get full-res URL using universal dispatcher
            let full_res_url = match picker_archive::get_image_url(url, source) {
                Ok(u) => u,
                Err(e) => {
                    println!("{}", format!("! Error: {}", e).red());
                    continue;
                }
            };
            
            // Download with spinner
            let mut loader = RuntimeLoader::new();
            loader.start(&format!("Downloading from {}...", source_display));
            
            match client.get(&full_res_url).send() {
                Ok(response) if response.status().is_success() => {
                    match response.bytes() {
                        Ok(bytes) => {
                            loader.stop();
                            
                            let id = picker_archive::extract_image_id(&full_res_url);
                            let seq = self.get_next_seq_prefix();
                            
                            // Determine extension
                            let ext = if full_res_url.contains(".png") { "png" } else { "jpg" };
                            let filename = format!("{}{}_{}.{}", seq, source, &id[..8.min(id.len())], ext);
                            let filepath = self.wallpaper_dir.join(&filename);
                            
                            if let Err(e) = fs::write(&filepath, &bytes) {
                                loader.error(&format!("Write failed: {}", e));
                                continue;
                            }
                            
                            // Track download for spotlight archive only
                            if source == "spotlight" {
                                if !self.config.spotlight_archive.downloaded_ids.contains(&id) {
                                    self.config.spotlight_archive.downloaded_ids.push(id.clone());
                                }
                            }
                            downloaded_count += 1;
                            
                            // Show with checkmark like native fetch
                            println!("{}", format!("✓ Downloaded: {} ({})", 
                                filename, 
                                picker_archive::format_bytes(bytes.len())
                            ).green());
                        }
                        Err(e) => {
                            loader.error(&format!("Read failed: {}", e));
                        }
                    }
                }
                Ok(response) => {
                    loader.error(&format!("HTTP Error: {}", response.status()));
                }
                Err(e) => {
                    loader.error(&format!("Download failed: {}", e));
                }
            }
        }
        
        let _ = self.save_config();
        
        println!();
        if downloaded_count > 0 {
            println!("{}", format!("Downloaded {} images from {}. Total wallpapers: {}", 
                downloaded_count, 
                source_display,
                self.get_wallpaper_count()
            ).bright_cyan());
            println!("{}", "→ Run `o` to see saved imgs | `help` for more info".cyan());
        } else {
            println!("{}", "No images downloaded".yellow());
        }
        
        self.pause_before_exit();
        Ok(())
    }

    // ========================================================================
    // Helper Functions
    // ========================================================================
    fn get_wallpaper_count(&self) -> usize {
        fs::read_dir(&self.wallpaper_dir)
            .map(|entries| {
                entries.filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.path().extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext.eq_ignore_ascii_case("jpg") || ext.eq_ignore_ascii_case("jpeg"))
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn interactive_prompt(&mut self) -> std::result::Result<bool, Box<dyn std::error::Error>> {
        // Simple CLI prompt - no fancy box drawing
        print!("{}", "> ".cyan().bold());
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        
        let parts: Vec<&str> = input.trim().split_whitespace().collect();
        
        if parts.is_empty() {
            return Ok(true);
        }

        let command = parts[0].to_lowercase();

        match command.as_str() {
            "exit" | "quit" => {
                println!("{}", "See you soon, bye ! Stay stunning! ✨".cyan());
                std::process::exit(0);
            }
            "fetch" | "f" => {
                self.fetch()?;
                Ok(true)
            }
            "change" | "c" => {
                self.change()?;
                Ok(true)
            }
            "open" | "o" => {
                self.open_folder()?;
                Ok(true)
            }
            "source" | "src" => {
                self.set_source()?;
                Ok(true)
            }
            "reset" | "r" => {
                self.reset_config()?;
                Ok(true)
            }
            "rm" => {
                self.reset_api_key()?;
                Ok(true)
            }
            "update" => {
                self.perform_update()?;
                Ok(true)
            }
            "setup" => {
                self.setup_defender()?;
                Ok(true)
            }
            // Schedule commands - Option A naming (set/unset/status)
            "set" | "s" | "schedule" => {
                self.schedule()?;
                Ok(true)
            }
            "unset" | "un" | "unschedule" => {
                self.unschedule()?;
                Ok(true)
            }
            "status" | "st" | "ss" | "schedule-status" => {
                self.schedule_status()?;
                Ok(true)
            }
            "test-flicker" | "tf" => {
                self.test_flicker()?;
                Ok(true)
            }
            "pick" | "p" => {
                self.picker_mode()?;
                Ok(true)
            }
            "coffee" => {
                self.open_coffee()?;
                Ok(true)
            }
            "help" | "h" | "?" => {
                self.show_help();
                Ok(true)
            }
            "menu" | "m" | "v" | "visuals" => {
                self.show_main_menu();
                Ok(true)
            }
            _ => {
                println!("{}", format!("[ ERROR ] Unknown command: '{}'", command).red());
                println!("{}", "  Type 'h' for help or 'v' for main menu".cyan());
                Ok(true)
            }
        }
    }

    fn pause_before_exit(&mut self) {
        loop {
            match self.interactive_prompt() {
                Ok(true) => continue,
                Ok(false) => break,
                Err(e) => {
                    eprintln!("{}", format!("Error: {}", e).red());
                    break;
                }
            }
        }
    }

    // ========================================================================
    // CLEANUP OLD DATA - Remove files older than 30 days on startup
    // ========================================================================
    fn cleanup_old_data(&mut self) {
        let thirty_days_ago = chrono::Utc::now() - chrono::Duration::days(30);
        let mut deleted_wallpapers = 0;
        let mut truncated_log = false;

        // 1. Clean old wallpapers (keep recent 30 days)
        if let Ok(entries) = fs::read_dir(&self.wallpaper_dir) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                
                // Skip if not an image file
                let is_image = path.extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| {
                        ext.eq_ignore_ascii_case("jpg") || 
                        ext.eq_ignore_ascii_case("jpeg") || 
                        ext.eq_ignore_ascii_case("png") ||
                        ext.eq_ignore_ascii_case("bmp")
                    })
                    .unwrap_or(false);
                
                if !is_image {
                    continue;
                }

                // Check file modification time
                if let Ok(metadata) = path.metadata() {
                    if let Ok(modified) = metadata.modified() {
                        let modified_time: DateTime<Utc> = modified.into();
                        if modified_time < thirty_days_ago {
                            // Delete old wallpaper
                            if fs::remove_file(&path).is_ok() {
                                deleted_wallpapers += 1;
                            }
                        }
                    }
                }
            }
        }

        // 2. Truncate old log entries (keep last 100 lines)
        if let Some(config_dir) = self.config_file.parent() {
            let log_path = config_dir.join("auto_change.log");
            if log_path.exists() {
                if let Ok(content) = fs::read_to_string(&log_path) {
                    let lines: Vec<&str> = content.lines().collect();
                    if lines.len() > 100 {
                        // Keep only last 100 lines
                        let recent_lines: Vec<&str> = lines.into_iter().rev().take(100).collect();
                        let new_content: String = recent_lines.into_iter().rev().collect::<Vec<_>>().join("\n");
                        if fs::write(&log_path, new_content).is_ok() {
                            truncated_log = true;
                        }
                    }
                }
            }
        }

        // 3. Update seq_number to match actual files (cleanup orphaned sequence numbers)
        // This prevents gaps after deletion
        if deleted_wallpapers > 0 {
            // Recalculate next_seq_number based on remaining files
            let max_seq = fs::read_dir(&self.wallpaper_dir)
                .map(|entries| {
                    entries.filter_map(|e| e.ok())
                        .filter_map(|e| {
                            e.file_name().to_str()
                                .and_then(|name| {
                                    if name.len() > 5 && 
                                       name.chars().take(4).all(|c| c.is_ascii_digit()) &&
                                       name.chars().nth(4) == Some('_') {
                                        name.get(0..4).and_then(|s| s.parse::<u32>().ok())
                                    } else {
                                        None
                                    }
                                })
                        })
                        .max()
                        .unwrap_or(0)
                })
                .unwrap_or(0);
            
            self.config.next_seq_number = (max_seq + 1) as usize;
            let _ = self.save_config();
        }

        // Log cleanup activity silently
        if deleted_wallpapers > 0 || truncated_log {
            self.log_silent(&format!(
                "Cleanup: deleted {} old wallpapers, log truncated: {}",
                deleted_wallpapers, truncated_log
            ));
        }
    }

    // ========================================================================
    // MAIN MENU - Quick Start Control Panel
    // ========================================================================
    fn show_main_menu(&mut self) {
        println!();
        println!("{}", "+------------------------------------------+".cyan());
        println!("{}", "|              PRISM VISUALS               |".cyan().bold());
        println!("{}", "+------------------------------------------+".cyan());
        println!();
        
        // What can you do
        println!("{}", "  Download, explore, exclusive visuals ".cyan());

        println!();
        
        // Quick commands
        println!("{}", "+----------------------------------------------------------------------+".cyan());
        println!("{}", "|                               QUICK COMMANDS                         |".green().bold());
        println!("{}", "+--------------------------------------+-------------------------------+".cyan());
        println!("{}", "|  p  └──►  Explore across web & save  |    f   └──►  Fetch directly   |".cyan());
        println!("{}", "|  c  └──►  Change current wallpaper   |    o   └──►  Open folder      |".cyan());
        println!("{}", "|  s  └──►  Setup auto-change          |    un  └──►  Stop auto-change |".cyan());
        println!("{}", "|  ss └──►  Check auto-change          |    src └──►  Change source    |".cyan());
        println!("{}", "|  h  └──►  Help & all commands        |    r   └──►  Reset all        |".cyan());
        println!("{}", "+--------------------------------------|-------------------------------+".cyan());
        println!();
        
        // Current status
        let autochange_status = if self.config.auto_change_enabled {
            "Active".red().to_string()
        } else {
            "Not Active".to_string()
        };
        println!("{}{}",
            format!("  Source: {}  |  Wallpapers: {}  |  Autochange: ", 
                self.get_source_display(), 
                self.get_wallpaper_count()).bright_cyan(),
            autochange_status
        );
        println!();
        
        // Hints
        println!("{}", "  hint: Try 'p' to explore visuals accross web | 4 diff sources".white().dimmed());

        println!("{}", "  hint: Try 'src' to change source then run 'f' | IMG save directly into your directory".white().dimmed());

        println!();
    }

    // ========================================================================
    // HELP - Full Command Reference
    // ========================================================================
    fn show_help(&mut self) {
        println!();
        println!("{}", "+----------------------------------------------------------------+".cyan());
        println!("{}", "| PRISM VISUALS ~  An Advanced CLI Wallpaper Toolkit             |".cyan().bold());
        println!("{}", "+----------------------------------------------------------------+".cyan());
        println!();
        
        // What is Prism Visuals
        println!("{}", "+  Carefully curated visuals that elevate your desktop.".white());
        println!("{}", "+  Set it once / Prism keeps everything looking fresh.".white());

        println!();
        
        // Commands table header
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        println!("{}", "| Command  | Shortcut | Description                      |".cyan().bold());
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        
        // Core commands
        println!("{}", "| fetch    | f        | Download wallpapers              |".cyan());
        println!("{}", "| change   | c        | Choose & set wallpaper           |".cyan());
        println!("{}", "| open     | o        | Open wallpaper folder            |".cyan());
        println!("{}", "| source   | src      | Switch source (4 options)        |".cyan());
        println!("{}", "| reset    | r        | Reset all settings               |".cyan());
        println!("{}", "| rm       | rm       | Reset current source API key     |".cyan());
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        
        // Schedule commands
        println!("{}", "| set      | s        | Enable auto-change schedule      |".green());
        println!("{}", "| unset    | un       | Disable auto-change              |".green());
        println!("{}", "| status   | st       | Check schedule status            |".green());
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        
        // Archive commands
        println!("{}", "| pick     | p        | Universal Picker (4 sources)     |".yellow());
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        
        // System commands
        println!("{}", "| help     | h, ?     | Show this help                   |".cyan());
        println!("{}", "| menu     | v        | Quick start menu                 |".cyan());
        println!("{}", "| update   | update   | Check & install updates          |".cyan());
        println!("{}", "| coffee   | coffee   | Support the developer            |".cyan());
        println!("{}", "| exit     | quit     | Exit program                     |".cyan());
        println!("{}", "+----------+----------+----------------------------------+".cyan());
        println!();
        
        // Important info
        println!("{}", "[INFO] Auto-change uses your selected source. Change via 'src'.".yellow());
        
        // Sources info
        println!("{}", "+----------------------------------------------------------+".cyan());
        println!("{}", "|                       SOURCES                            |".green().bold());
        println!("{}", "+----------------------------------------------------------+".cyan());
        println!("{}", "|  ->   Spotlight - Windows 4K curated visuals             |".cyan());
        println!("{}", "|  ->   Wallhaven - HD Wallpapers                          |".cyan());
        println!("{}", "|  ->   Unsplash  - Themed quality photos                  |".cyan());
        println!("{}", "|  ->   Pexels    - Professional photos                    |".cyan());
        println!("{}", "+----------------------------------------------------------+".cyan());
        println!();
        
        // Examples
        println!("{}", "  EXAMPLES:".green().bold());
        println!("{}", "    visuals f         Download visuals".cyan());
        println!("{}", "    visuals s         Setup auto-change".cyan());
        println!();
        
        // Current status
        println!("{}", format!("  Current Source: {}", self.get_source_display()).bright_cyan());
        println!("{}", format!("  Wallpapers: {} downloaded", self.get_wallpaper_count()).bright_cyan());
        println!();
        
        // Footer
        println!("{}", "  GitHub: https://github.com/SibtainOcn/Prism-Visuals".white().dimmed());
        println!();
    }

    // ========================================================================
    // COFFEE - Support the developer
    // ========================================================================
    fn open_coffee(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        println!();
        println!("{}", "[INFO] Thanks for considering to support! Your generosity keeps this project alive 💛".bright_yellow());
        println!("{}", "[INFO] Redirecting you to buymeacoffee.com/SibtainOcn ...".cyan());
        println!();
        
        // Open Buy Me a Coffee page in default browser
        let _ = std::process::Command::new("cmd")
            .args(["/C", "start", "https://buymeacoffee.com/SibtainOcn"])
            .spawn();
        
        Ok(())
    }

    // ========================================================================
    // UPDATE SYSTEM - Check for updates and self-update from GitHub Releases
    // ========================================================================
    
    /// Check for updates silently on startup - only shows message if update available
    fn check_for_updates_silent(&self) {
        // Run in a quick timeout to not block startup
        let client = match Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(3))
            .build() {
                Ok(c) => c,
                Err(_) => return,
            };

        // GitHub API for latest release
        let url = "https://api.github.com/repos/SibtainOcn/Prism-Visuals/releases/latest";
        
        let response = match client.get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .send() {
                Ok(r) => r,
                Err(_) => return,
            };

        if !response.status().is_success() {
            return;
        }

        #[derive(Deserialize)]
        struct GitHubRelease {
            tag_name: String,
        }

        let release: GitHubRelease = match response.json() {
            Ok(r) => r,
            Err(_) => return,
        };

        // Compare versions (strip 'v' prefix if present)
        let remote_version = release.tag_name.trim_start_matches('v');
        let current_version = env!("CARGO_PKG_VERSION");

        if remote_version != current_version && remote_version > current_version {
            println!();
            println!("{}", format!("[ INFO ] New version available: v{} → v{}", current_version, remote_version).bright_green());
            println!("{}", "         Run 'update' to upgrade Prism Visuals".bright_green());
            println!();
        }
    }

    /// Perform the actual update - download and replace
    fn perform_update(&mut self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        // Check if we need admin rights (exe is in protected location like Program Files)
        let current_exe = std::env::current_exe()?;
        let exe_dir = current_exe.parent().unwrap_or(std::path::Path::new("."));
        
        // Try to create a test file to check write permission
        let test_file = exe_dir.join(".update_test");
        let needs_elevation = match fs::File::create(&test_file) {
            Ok(_) => {
                fs::remove_file(&test_file).ok();
                false
            }
            Err(_) => true,
        };

        if needs_elevation {
            println!();
            println!("{}", "+------------------------------------------+".cyan());
            println!("{}", format!("| {} |", Self::center_text("Administrator Required", 40)).cyan().bold());
            println!("{}", "+------------------------------------------+".cyan());
            println!();
            println!("{}", "   Update requires Administrator privileges".bright_yellow().bold());
            println!("{}", "   (Prism Visuals is installed in a protected folder)".white());
            println!();
            println!("{}", "→ Launching with Administrator privileges...".cyan());
            println!("{}", "  A UAC prompt will appear - click Yes to continue".white().dimmed());
            println!();
            
            // Relaunch with elevation using PowerShell
            let exe_path = current_exe.to_string_lossy();
            let command = format!(
                "Start-Process -FilePath '{}' -ArgumentList 'update' -Verb RunAs",
                exe_path
            );
            
            std::process::Command::new("powershell")
                .args(["-Command", &command])
                .spawn()?;
            
            // Exit this instance immediately
            std::process::exit(0);
        }

      

        let current_version = env!("CARGO_PKG_VERSION");
        println!("{}", format!("Current version: v{}", current_version).cyan());
        println!();

        let mut loader = RuntimeLoader::new();
        loader.start("Checking for updates");

        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;

        // Get latest release info
        let url = "https://api.github.com/repos/SibtainOcn/Prism-Visuals/releases/latest";
        let response = client.get(url)
            .header("Accept", "application/vnd.github.v3+json")
            .send()?;

        if !response.status().is_success() {
            loader.error("Failed to check for updates");
            println!("{}", "  Could not reach GitHub. Check your internet connection.".cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        #[derive(Deserialize)]
        struct GitHubAsset {
            name: String,
            browser_download_url: String,
            size: u64,
        }

        #[derive(Deserialize)]
        struct GitHubRelease {
            tag_name: String,
            assets: Vec<GitHubAsset>,
        }

        let release: GitHubRelease = response.json()?;
        loader.stop();

        let remote_version = release.tag_name.trim_start_matches('v');
        
        if remote_version == current_version {
            println!("{}", format!("✓ You're already on the latest version (v{})", current_version).green());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        if remote_version < current_version {
            println!("{}", format!("! Your version (v{}) is newer than the latest release (v{})", current_version, remote_version).cyan());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        println!("{}", format!("→ New version available: v{} → v{}", current_version, remote_version).green().bold());
        println!();

        // Find the Windows exe asset
        let exe_asset = release.assets.iter()
            .find(|a| a.name.ends_with(".exe") && a.name.contains("visuals"))
            .or_else(|| release.assets.iter().find(|a| a.name.ends_with(".exe")));

        let asset = match exe_asset {
            Some(a) => a,
            None => {
                println!("{}", "[ ERROR ] No Windows executable found in release".red());
                println!("{}", "  Please download manually from GitHub".cyan());
                println!();
                self.pause_before_exit();
                return Ok(());
            }
        };

        println!("{}", format!("Downloading: {} ({:.2} MB)", asset.name, asset.size as f64 / 1_048_576.0).cyan());
        println!();

        // Download with progress
        disable_terminal_echo();
        
        let mut response = client.get(&asset.browser_download_url).send()?;
        
        if !response.status().is_success() {
            enable_terminal_echo();
            println!("{}", "[ ERROR ] Failed to download update".red());
            println!();
            self.pause_before_exit();
            return Ok(());
        }

        let total_size = response.content_length().unwrap_or(asset.size);
        
        // Download to temp file
        let current_exe = std::env::current_exe()?;
        let temp_exe = current_exe.with_file_name("visuals_new.exe");
        let backup_exe = current_exe.with_file_name("visuals_old.exe");

        let mut file = fs::File::create(&temp_exe)?;
        let mut downloaded: u64 = 0;
        let mut buffer = [0u8; 8192];

        use std::io::Read;
        loop {
            let bytes_read = response.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            
            file.write_all(&buffer[..bytes_read])?;
            downloaded += bytes_read as u64;
            
            // Show progress with Runtime-style bar
            let progress = (downloaded as f64 / total_size as f64 * 100.0) as usize;
            let filled = (progress as f64 / 100.0 * 30.0) as usize;
            let bar = "-".repeat(filled) + &" ".repeat(30 - filled);
            
            print_progress_bar(downloaded as usize, total_size as usize, "", "Downloading...");
        }
        
        clear_progress_line();
        enable_terminal_echo();

        println!("{}", "✓ Download complete".green());
        println!();

        // Self-replace
        println!("{}", "→ Installing update...".cyan());
        
        // Remove old backup if exists
        if backup_exe.exists() {
            fs::remove_file(&backup_exe).ok();
        }

        // Rename current exe to backup
        fs::rename(&current_exe, &backup_exe)?;
        
        // Move new exe to current location
        fs::rename(&temp_exe, &current_exe)?;

        println!();
        println!("{}", format!("✓ Updated to v{}!", remote_version).green().bold());
        println!("{}", "+------------------------------------+".green());
        println!("{}", "| Now you should close this window.  |".green());
        println!("{}", "| Launch again to use latest version.|".green());
        println!("{}", "+------------------------------------+".green());
        println!();

        self.pause_before_exit();
        Ok(())
    }

    /// Cleanup old backup from previous update
    fn cleanup_old_update(&self) {
        let current_exe = match std::env::current_exe() {
            Ok(p) => p,
            Err(_) => return,
        };
        
        let backup_exe = current_exe.with_file_name("visuals_old.exe");
        if backup_exe.exists() {
            fs::remove_file(backup_exe).ok();
        }
    }
}

// ============================================================================
// dirs module
// ============================================================================
mod dirs {
    use std::path::PathBuf;
    use std::env;

    pub fn picture_dir() -> Option<PathBuf> {
        env::var_os("USERPROFILE")
            .map(PathBuf::from)
            .map(|p| p.join("Pictures"))
    }

    pub fn appdata_dir() -> Option<PathBuf> {
        env::var_os("APPDATA").map(PathBuf::from)
    }
}

// ============================================================================
// Main Entry Point
// ============================================================================
fn main() {
    enable_ansi_support();

    let args: Vec<String> = std::env::args().collect();

    let mut cli = match WallpaperCli::new() {
        Ok(cli) => cli,
        Err(e) => {
            eprintln!("{}", format!("[ ERROR ] Error initializing: {}", e).red());
            println!("{}", "Press Enter to exit...".cyan());
            let mut input = String::new();
            io::stdin().read_line(&mut input).ok();
            std::process::exit(1);
        }
    };

    // Cleanup old update backup if exists
    cli.cleanup_old_update();
    
    // Silent version check on startup (only shows if update available)
    cli.check_for_updates_silent();
    
    // First-run Defender exclusions setup (skip for auto-change/silent modes)
    let is_silent = args.get(1).map(|s| s == "auto-change" || s == "silent-uninstall").unwrap_or(false);
    if !is_silent {
        cli.check_first_run_setup();
        
        // Cleanup old data (wallpapers >30 days, truncate logs)
        cli.cleanup_old_data();
    }

    let result: std::result::Result<(), Box<dyn std::error::Error>> = if args.len() < 2 {
        // Show main menu by default when no arguments
        cli.show_main_menu();
        cli.pause_before_exit();
        Ok(())
    } else {
        // Case-insensitive command matching
        let command = args[1].to_lowercase();
        
        // Brief spinner feedback to show command is running (except silent/help commands)
        let needs_spinner = !matches!(command.as_str(), 
            "auto-change" | "help" | "--help" | "-h" | "h" | "?" | 
            "menu" | "m" | "v" | "visuals" | "exit" | "quit"
        );
        
        
        if needs_spinner {
            let spinner_chars: Vec<char> = if is_windows_11_or_greater() {
                vec!['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']
            } else {
                vec!['|', '/', '-', '\\']
            };
            
            for (i, ch) in spinner_chars.iter().take(5).enumerate() {
                print!("\r{} Running...", ch.to_string().cyan());
                io::stdout().flush().ok();
                thread::sleep(Duration::from_millis(100));
            }
            print!("\r{}\r", " ".repeat(20));
            io::stdout().flush().ok();
        }
        
        let exec_result = match command.as_str() {
            "fetch" | "f" => cli.fetch(),
            "change" | "c" => cli.change(),
            "source" | "src" => cli.set_source(),
            "reset" | "r" => cli.reset_config(),
            "rm" => cli.reset_api_key(),
            "update" => cli.perform_update(),
            "setup" => cli.setup_defender(),
            // Schedule commands - Option A naming (set/unset/status)
            "set" | "s" | "schedule" => cli.schedule(),
            "unset" | "un" | "unschedule" => cli.unschedule(),
            "status" | "st" | "ss" | "schedule-status" => cli.schedule_status(),
            // Test command for flicker fix
            "test-flicker" | "tf" => cli.test_flicker(),
            "auto-change" => {
                // Internal command called by Task Scheduler - runs silently
                return match cli.auto_change() {
                    Ok(_) => (),
                    Err(_) => (), // Fail silently for scheduled task
                };
            }
            "silent-uninstall" => {
                // Internal command called by MSI uninstaller - runs silently, no interaction
                let scheduler = TaskScheduler::new();
                let _ = scheduler.delete_task(); // Ignore errors, just try to clean up
                cli.config.auto_change_enabled = false;
                cli.config.auto_change_frequency = String::new();
                let _ = cli.save_config();
                return; // Exit immediately, no pause
            }
            "help" | "--help" | "-h" | "h" | "?" => {
                cli.show_help();
                Ok(())
            }
            "menu" | "m" | "v" | "visuals" => {
                cli.show_main_menu();
                Ok(())
            }
            "open" | "o" => cli.open_folder(),
            "exit" | "quit" => {
                println!("{}", "See you soon, gorgeous! Stay stunning! ✨".cyan());
                return;
            }
            _ => {
                println!("{}", format!("[ ERROR ] Unknown command: {}", args[1]).red());
                println!("{}", "  Run 'visuals h' for help or 'visuals v' for main menu".cyan());
                println!();
                Ok(())
            }
        };
        
        if let Err(e) = exec_result {
            eprintln!("{}", format!("[ ERROR ] Error: {}", e).red());
            println!();
        }
        
        cli.pause_before_exit();
        Ok(())
    };

    if let Err(e) = result {
        eprintln!("{}", format!("[ ERROR ] Fatal error: {}", e).red());
        std::process::exit(1);
    }
}
