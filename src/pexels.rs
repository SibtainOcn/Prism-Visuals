// ============================================================================
// Pexels API Integration
// ============================================================================
// Base URL: https://api.pexels.com/v1
// Rate Limit: 200 requests/hour, 20,000 requests/month
// API Key: REQUIRED (free signup at pexels.com/api)
// ============================================================================

use serde::{Deserialize, Serialize};

// ============================================================================
// Configuration
// ============================================================================
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PexelsConfig {
    pub api_key: String,
    pub theme: String,
    pub last_fetch_time: Option<String>,
    pub requests_this_hour: u32,
    pub hour_window_start: Option<String>,  // Track when the current hour started
}

impl Default for PexelsConfig {
    fn default() -> Self {
        PexelsConfig {
            api_key: String::new(),
            theme: "nature".to_string(),
            last_fetch_time: None,
            requests_this_hour: 0,
            hour_window_start: None,
        }
    }
}

// ============================================================================
// API Response Structures
// ============================================================================
#[derive(Debug, Deserialize)]
pub struct PexelsResponse {
    pub page: u32,
    pub per_page: u32,
    pub total_results: u32,
    pub photos: Vec<PexelsPhoto>,
}

#[derive(Debug, Deserialize)]
pub struct PexelsPhoto {
    pub id: u64,
    pub width: u32,
    pub height: u32,
    pub url: String,
    pub photographer: String,
    pub photographer_url: String,
    pub avg_color: Option<String>,
    pub src: PexelsSrc,
    pub alt: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PexelsSrc {
    pub original: String,
    pub large2x: String,
    pub large: String,
    pub medium: String,
    pub small: String,
    pub portrait: String,
    pub landscape: String,
    pub tiny: String,
}

// ============================================================================
// These are curated keywords that produce high-quality desktop wallpapers
// ============================================================================
pub const PEXELS_TEMPLATES: [&str; 20] = [
    // Nature & Landscapes
    "nature landscape wallpaper",
    "mountain scenery 4k",
    "ocean waves beach",
    "forest trees green",
    "lake reflection water",
    "waterfall jungle tropical",
    // Sky & Space
    "night sky stars",
    "galaxy stars nebula",
    "aurora borealis northern lights",
    "sunset clouds orange",
    "sunrise golden hour",
    // Urban & Architecture
    "city architecture skyline",
    "city night lights",
    "modern architecture building",
    // Seasonal & Climate
    "snow winter peaks",
    "desert sand dunes",
    "autumn leaves forest",
    // Abstract & Minimal
    "abstract art colorful",
    "minimal background gradient",
    "clouds atmosphere dramatic",
];

// ============================================================================
// Default API Parameters
// ============================================================================
pub const DEFAULT_ORIENTATION: &str = "landscape";
pub const DEFAULT_SIZE: &str = "large";  // 24MP minimum filter
pub const DEFAULT_PER_PAGE: u32 = 30;    // Max useful per request

// ============================================================================
// Helper Functions
// ============================================================================

/// Build the search URL with proper parameters
pub fn build_search_url(query: &str, per_page: u32) -> String {
    format!(
        "https://api.pexels.com/v1/search?query={}&orientation={}&size={}&per_page={}",
        urlencoding::encode(query),
        DEFAULT_ORIENTATION,
        DEFAULT_SIZE,
        per_page
    )
}

/// Get a random template word for silent fetch
pub fn get_random_template() -> &'static str {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;
    PEXELS_TEMPLATES[seed % PEXELS_TEMPLATES.len()]
}

/// Get the best download URL based on screen size
/// Default: large2x (1880px) for 1080p monitors
/// Optional: original for 4K
pub fn get_download_url(src: &PexelsSrc, use_original: bool) -> &str {
    if use_original {
        &src.original
    } else {
        &src.large2x
    }
}
