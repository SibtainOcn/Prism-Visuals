// ============================================================================
// PICKER ARCHIVE MODULE - Universal Image Picker
// URL parsers for: Spotlight Archive, Unsplash, Pexels, Wallhaven
// ============================================================================

use reqwest::blocking::Client;
use std::time::Duration;

/// Base URL for the archive site
pub const BASE_URL: &str = "https://windows10spotlight.com";

/// Get full resolution URL from various URL formats
/// 
/// Supports:
/// - Direct full-res: .../wp-content/uploads/2025/12/abc123.jpg
/// - Thumbnail: .../abc123-1024x576.jpg → strips suffix
/// - Page URL: /images/40412 → fetches and extracts
pub fn get_full_res_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Case 1: Already a direct full-res image URL
    if url.contains("/wp-content/uploads/") 
        && !url.contains("-1024x576") 
        && !url.contains("-300x169")
        && url.ends_with(".jpg") {
        return Ok(url.to_string());
    }
    
    // Case 2: Thumbnail URL - strip size suffix
    if url.contains("-1024x576.jpg") {
        return Ok(url.replace("-1024x576", ""));
    }
    if url.contains("-300x169.jpg") {
        return Ok(url.replace("-300x169", ""));
    }
    
    // Case 3: Page URL - need to fetch and parse
    if url.contains("/images/") || url.contains("windows10spotlight.com/") {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let html = client.get(url).send()?.text()?;
        
        // Look for srcset with 1920w (full resolution)
        // Pattern: https://windows10spotlight.com/wp-content/uploads/YYYY/MM/{hash}.jpg 1920w
        if let Some(pos) = html.find("1920w") {
            // Go backwards to find the URL start
            let before = &html[..pos];
            if let Some(url_start) = before.rfind("https://windows10spotlight.com/wp-content/uploads/") {
                let url_chunk = &html[url_start..pos];
                // Extract just the URL (ends before space)
                if let Some(url_end) = url_chunk.rfind(".jpg") {
                    let full_url = &url_chunk[..url_end + 4];
                    // Make sure it's not a thumbnail
                    if !full_url.contains("-1024x576") && !full_url.contains("-300x169") {
                        return Ok(full_url.to_string());
                    }
                }
            }
        }
        
        // Fallback: look for any full-res jpg
        if let Some(start) = html.find("https://windows10spotlight.com/wp-content/uploads/") {
            let chunk = &html[start..];
            if let Some(end) = chunk.find(".jpg") {
                let img_url = &chunk[..end + 4];
                // Convert thumbnail to full-res if needed
                let full_url = img_url
                    .replace("-1024x576", "")
                    .replace("-300x169", "");
                return Ok(full_url);
            }
        }
        
        return Err("Could not find image URL in page".into());
    }
    
    Err(format!("Unsupported URL format: {}", url).into())
}

/// Extract a unique ID from image URL for deduplication
/// Takes the hash portion of the filename
pub fn extract_image_id(url: &str) -> String {
    // URL format: .../wp-content/uploads/2025/12/abc123def456.jpg
    // Extract: abc123def456
    url.split('/')
        .last()
        .unwrap_or("unknown")
        .replace(".jpg", "")
        .replace(".jpeg", "")
        .replace(".png", "")
}

/// Fetch the latest image URL from the homepage
/// Returns (image_url, title) tuple
pub fn fetch_latest_image_url() -> Result<(String, String), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
        .timeout(Duration::from_secs(30))
        .build()?;
    
    // Fetch homepage (page 1 has the latest images)
    let html = client.get(format!("{}/page/1", BASE_URL)).send()?.text()?;
    
    // Find the first full-res image URL (1920w in srcset)
    if let Some(pos) = html.find("1920w") {
        let before = &html[..pos];
        if let Some(url_start) = before.rfind("https://windows10spotlight.com/wp-content/uploads/") {
            let url_chunk = &html[url_start..pos];
            if let Some(url_end) = url_chunk.rfind(".jpg") {
                let image_url = &url_chunk[..url_end + 4];
                if !image_url.contains("-1024x576") && !image_url.contains("-300x169") {
                    // Try to extract title from entry-title
                    let title = extract_title_from_html(&html).unwrap_or_else(|| "Spotlight".to_string());
                    return Ok((image_url.to_string(), title));
                }
            }
        }
    }
    
    Err("Could not fetch latest image from homepage".into())
}

/// Extract title from HTML page
fn extract_title_from_html(html: &str) -> Option<String> {
    // Look for: <span class="entry-title hidden">Title Here</span>
    if let Some(start) = html.find("entry-title hidden\">") {
        let after = &html[start + 20..];
        if let Some(end) = after.find("</span>") {
            let title = &after[..end];
            // Clean up the title
            let clean_title: String = title
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == ' ' || *c == ',')
                .take(50)
                .collect();
            if !clean_title.is_empty() {
                return Some(clean_title.trim().to_string());
            }
        }
    }
    None
}

// ============================================================================
// UNSPLASH URL PARSER
// ============================================================================

/// Get ORIGINAL resolution URL from Unsplash (no size limits!)
/// Supports: unsplash.com/photos/{id} or direct image URLs
pub fn get_unsplash_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Case 1: Direct image URL from images.unsplash.com
    if url.contains("images.unsplash.com") {
        // Get ORIGINAL quality - remove size params, keep only quality
        let base = url.split('?').next().unwrap_or(url);
        return Ok(format!("{}?q=100", base));  // Original size, max quality
    }
    
    // Case 2: Photo page URL - unsplash.com/photos/{id}
    if url.contains("unsplash.com/photos/") {
        // Extract photo ID
        let parts: Vec<&str> = url.split("/photos/").collect();
        if parts.len() >= 2 {
            let id = parts[1].split(['/', '?']).next().unwrap_or("");
            if !id.is_empty() {
                // Use download endpoint for ORIGINAL quality
                return Ok(format!("https://unsplash.com/photos/{}/download?force=true", id));
            }
        }
    }
    
    Err(format!("Unsupported Unsplash URL: {}", url).into())
}

// ============================================================================
// PEXELS URL PARSER
// ============================================================================

/// Get ORIGINAL resolution URL from Pexels (no size limits!)
/// Supports: pexels.com/photo/{name}-{id}/ or direct image URLs
pub fn get_pexels_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Case 1: Direct image URL - get original without size constraints
    if url.contains("images.pexels.com") {
        // Get ORIGINAL quality - remove size params
        let base = url.split('?').next().unwrap_or(url);
        return Ok(base.to_string());  // No params = original size
    }
    
    // Case 2: Photo page URL
    if url.contains("pexels.com/photo/") {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
            .timeout(Duration::from_secs(30))
            .build()?;
        
        let html = client.get(url).send()?.text()?;
        
        // Look for og:image meta tag
        if let Some(start) = html.find("og:image\" content=\"") {
            let after = &html[start + 19..];
            if let Some(end) = after.find("\"") {
                let img_url = &after[..end];
                // Get ORIGINAL - no size params
                let base = img_url.split('?').next().unwrap_or(img_url);
                return Ok(base.to_string());  // Original quality
            }
        }
        
        // Fallback: look for any pexels image URL
        if let Some(start) = html.find("https://images.pexels.com/photos/") {
            let chunk = &html[start..];
            if let Some(end) = chunk.find("\"") {
                let img_url = &chunk[..end];
                let base = img_url.split('?').next().unwrap_or(img_url);
                return Ok(base.to_string());  // Original quality
            }
        }
    }
    
    Err(format!("Unsupported Pexels URL: {}", url).into())
}

// ============================================================================
// WALLHAVEN URL PARSER  
// ============================================================================

/// Get full resolution URL from Wallhaven
/// Supports: wallhaven.cc/w/{id} or direct image URLs
pub fn get_wallhaven_url(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Case 1: Direct image URL from w.wallhaven.cc
    if url.contains("w.wallhaven.cc/full/") {
        return Ok(url.to_string());
    }
    
    // Case 2: Thumbnail URL - convert to full
    if url.contains("th.wallhaven.cc/") {
        // Pattern: th.wallhaven.cc/small/ab/abc123.jpg → w.wallhaven.cc/full/ab/wallhaven-abc123.jpg
        let url = url.replace("th.wallhaven.cc/small/", "w.wallhaven.cc/full/");
        let url = url.replace("th.wallhaven.cc/orig/", "w.wallhaven.cc/full/");
        // Add wallhaven- prefix to filename
        if let Some(last_slash) = url.rfind('/') {
            let (path, filename) = url.split_at(last_slash + 1);
            if !filename.starts_with("wallhaven-") {
                return Ok(format!("{}wallhaven-{}", path, filename));
            }
        }
        return Ok(url);
    }
    
    // Case 3: Wallpaper page URL - wallhaven.cc/w/{id}
    if url.contains("wallhaven.cc/w/") {
        // Extract ID
        let parts: Vec<&str> = url.split("/w/").collect();
        if parts.len() >= 2 {
            let id = parts[1].split(['/', '?']).next().unwrap_or("");
            if !id.is_empty() && id.len() >= 2 {
                // Construct direct URL
                // Pattern: w.wallhaven.cc/full/{first2}/{wallhaven-{id}.jpg/png}
                let prefix = &id[..2];
                
                // Try jpg first, then png
                let jpg_url = format!("https://w.wallhaven.cc/full/{}/wallhaven-{}.jpg", prefix, id);
                let client = Client::builder()
                    .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                    .timeout(Duration::from_secs(10))
                    .build()?;
                
                // Quick check if jpg exists
                if let Ok(resp) = client.head(&jpg_url).send() {
                    if resp.status().is_success() {
                        return Ok(jpg_url);
                    }
                }
                
                // Try png
                let png_url = format!("https://w.wallhaven.cc/full/{}/wallhaven-{}.png", prefix, id);
                return Ok(png_url);
            }
        }
    }
    
    Err(format!("Unsupported Wallhaven URL: {}", url).into())
}

// ============================================================================
// UNIVERSAL URL DISPATCHER
// ============================================================================

/// Universal image URL parser - routes to correct parser based on source
pub fn get_image_url(url: &str, source: &str) -> Result<String, Box<dyn std::error::Error>> {
    match source {
        "spotlight" => get_full_res_url(url),
        "unsplash" => get_unsplash_url(url),
        "pexels" => get_pexels_url(url),
        "wallhaven" => get_wallhaven_url(url),
        _ => Err(format!("Unknown source: {}", source).into()),
    }
}

/// Validate URL belongs to the expected source
pub fn validate_url(url: &str, source: &str) -> bool {
    match source {
        "spotlight" => url.contains("windows10spotlight.com"),
        "unsplash" => url.contains("unsplash.com") || url.contains("images.unsplash.com"),
        "pexels" => url.contains("pexels.com") || url.contains("images.pexels.com"),
        "wallhaven" => url.contains("wallhaven.cc"),
        _ => false,
    }
}

/// Get website URL for a source
pub fn get_website_url(source: &str) -> &'static str {
    match source {
        "spotlight" => "https://windows10spotlight.com",
        "unsplash" => "https://unsplash.com",
        "pexels" => "https://www.pexels.com",
        "wallhaven" => "https://wallhaven.cc",
        _ => "https://google.com",
    }
}

/// Format bytes to human-readable string
pub fn format_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.2} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_image_id() {
        let url = "https://windows10spotlight.com/wp-content/uploads/2025/12/abc123def.jpg";
        assert_eq!(extract_image_id(url), "abc123def");
    }

    #[test]
    fn test_thumbnail_to_full_res() {
        let thumb = "https://windows10spotlight.com/wp-content/uploads/2025/12/abc123-1024x576.jpg";
        let full = get_full_res_url(thumb).unwrap();
        assert!(!full.contains("-1024x576"));
    }
}
