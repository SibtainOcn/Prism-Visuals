// ============================================================================
// Wallhaven API Integration
// ============================================================================
// Base URL: https://wallhaven.cc/api/v1
// Rate Limit: 45 requests/minute (tracked locally)
// API Key: NOT required for SFW content
// ============================================================================

use serde::{Deserialize, Serialize};

// ============================================================================
// Configuration
// ============================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WallhavenConfig {
    pub theme: String,
    pub last_fetch_time: Option<String>,
    pub requests_this_minute: u32,
    pub minute_window_start: Option<String>,  // Track when the current minute started
}

impl Default for WallhavenConfig {
    fn default() -> Self {
        WallhavenConfig {
            theme: "nature".to_string(),
            last_fetch_time: None,
            requests_this_minute: 0,
            minute_window_start: None,
        }
    }
}

// ============================================================================
// API Response Structures
// ============================================================================
#[derive(Debug, Deserialize)]
pub struct WallhavenResponse {
    pub data: Vec<WallhavenWallpaper>,
    pub meta: Option<WallhavenMeta>,
}

#[derive(Debug, Deserialize)]
pub struct WallhavenWallpaper {
    pub id: String,
    pub url: String,
    pub resolution: String,
    pub file_size: u64,
    pub file_type: String,
    pub path: String,  // Direct download URL
    pub thumbs: WallhavenThumbs,
    pub purity: String,
    pub category: String,
}

#[derive(Debug, Deserialize)]
pub struct WallhavenThumbs {
    pub large: String,
    pub original: String,
    pub small: String,
}

#[derive(Debug, Deserialize)]
pub struct WallhavenMeta {
    pub current_page: u32,
    pub last_page: u32,
    pub per_page: u32,
    pub total: u32,
}

// ============================================================================
// Template Words for Silent Fetch
// These are curated keywords that produce high-quality desktop wallpapers
// ============================================================================
pub const WALLHAVEN_TEMPLATES: [&str; 12] = [
    "nature landscape wallpaper",
    "mountain scenery 4k",
    "ocean waves beach",
    "forest trees green",
    "galaxy stars nebula",
    "city night lights",
    "aurora borealis",
    "sunset clouds orange",
    "lake reflection",
    "snow winter peaks",
    "desert sand dunes",
    "waterfall jungle",
];

// ============================================================================
// Default API Parameters
// ============================================================================
pub const DEFAULT_CATEGORIES: &str = "100";  // General only (no Anime - may contain suggestive content)
pub const DEFAULT_PURITY: &str = "100";      // SFW only
pub const DEFAULT_SORTING: &str = "relevance";
pub const DEFAULT_ATLEAST: &str = "1920x1080";
pub const DEFAULT_RATIOS: &str = "16x9";

// ============================================================================
// Helper Functions
// ============================================================================

/// Build the search URL with proper parameters
pub fn build_search_url(query: &str, sorting: &str, page: u32) -> String {
    format!(
        "https://wallhaven.cc/api/v1/search?q={}&categories={}&purity={}&sorting={}&atleast={}&ratios={}&page={}",
        urlencoding::encode(query),
        DEFAULT_CATEGORIES,
        DEFAULT_PURITY,
        sorting,
        DEFAULT_ATLEAST,
        DEFAULT_RATIOS,
        page
    )
}

/// Get a random template word for silent fetch
pub fn get_random_template() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    WALLHAVEN_TEMPLATES[seed % WALLHAVEN_TEMPLATES.len()]
}

/// Build SAFE search URL for silent/auto fetch (General category only, no Anime)
/// This avoids any suggestive poses or revealing artwork in auto-fetched images
pub fn build_search_url_safe(query: &str, sorting: &str, page: u32) -> String {
    format!(
        "https://wallhaven.cc/api/v1/search?q={}&categories=100&purity=100&sorting={}&atleast={}&ratios={}&page={}",
        urlencoding::encode(query),
        sorting,
        DEFAULT_ATLEAST,
        DEFAULT_RATIOS,
        page
    )
}
