//! Utility functions

use crate::constants::{APP_VERSION, CACHE_REFRESH};
use std::path::PathBuf;

// With stroke — for sidebar logo (large display)
pub const LOGO_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 251.93 167.71"><defs><style>.c1{fill:#fff;stroke:#09090b;stroke-width:1px}.c2{fill:#2dd4bf;stroke:#09090b;stroke-width:1px}</style></defs><path class="c1" d="m104.54,84.12h-19.01c-2.88,0-4.96.64-6.25,1.93-1.29,1.29-1.93,3.37-1.93,6.24v35.46h-22.04c-3.48,0-6.1-.91-7.84-2.74-1.74-1.81-2.61-4.61-2.61-8.39v-55.8c0-3.79,1.14-16.34,3.41-18.23,2.27-1.89,5.76-2.84,10.45-2.84h47.26c2.88,0,4.96-.64,6.25-1.93,1.29-1.29,1.93-3.37,1.93-6.25V8.18c0-2.88-.64-4.96-1.93-6.25C110.94.64,108.86,0,105.98,0h-56.81C30.24,0,22.91,3.79,13.75,11.36,4.58,18.94,0,26.49,0,42.25v82.08c0,15.77,4.58,24.44,13.75,32.02,9.16,7.57,16.49,11.36,35.43,11.36h66.35c2.88,0,4.96-.64,6.25-1.93,1.29-1.29,1.93-3.37,1.93-6.25v-45.95l-19.16-29.45Z"/><path class="c2" d="m128.23,113.58v45.95c0,2.88.64,4.96,1.93,6.25,1.29,1.29,3.37,1.93,6.25,1.93h66.35c18.94,0,26.26-3.79,35.43-11.36,9.16-7.57,13.75-16.25,13.75-32.02V42.25c0-15.75-4.58-23.31-13.75-30.88C229.02,3.79,221.69,0,202.75,0h-56.81c-2.88,0-4.96.64-6.25,1.93-1.29,1.29-1.93,3.37-1.93,6.25v23.39c0,2.88.64,4.96,1.93,6.25,1.29,1.29,3.37,1.93,6.25,1.93h47.26c4.7,0,8.18.95,10.45,2.84,2.27,1.89,3.41,14.44,3.41,18.23v55.8c0,3.79-.87,6.59-2.61,8.39-1.74,1.83-4.36,2.74-7.84,2.74h-22.04v-35.46c0-2.87-.64-4.95-1.93-6.24-1.29-1.29-3.37-1.93-6.25-1.93h-19.01s-19.16,29.45-19.16,29.45Z"/></svg>"#;

// No stroke, square viewBox — for window/taskbar icons
pub const ICON_SVG: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 251.93 251.93"><defs><style>.c1{fill:#fff}.c2{fill:#2dd4bf}</style></defs><g transform="translate(0,42.11)"><path class="c1" d="m104.54,84.12h-19.01c-2.88,0-4.96.64-6.25,1.93-1.29,1.29-1.93,3.37-1.93,6.24v35.46h-22.04c-3.48,0-6.1-.91-7.84-2.74-1.74-1.81-2.61-4.61-2.61-8.39v-55.8c0-3.79,1.14-16.34,3.41-18.23,2.27-1.89,5.76-2.84,10.45-2.84h47.26c2.88,0,4.96-.64,6.25-1.93,1.29-1.29,1.93-3.37,1.93-6.25V8.18c0-2.88-.64-4.96-1.93-6.25C110.94.64,108.86,0,105.98,0h-56.81C30.24,0,22.91,3.79,13.75,11.36,4.58,18.94,0,26.49,0,42.25v82.08c0,15.77,4.58,24.44,13.75,32.02,9.16,7.57,16.49,11.36,35.43,11.36h66.35c2.88,0,4.96-.64,6.25-1.93,1.29-1.29,1.93-3.37,1.93-6.25v-45.95l-19.16-29.45Z"/><path class="c2" d="m128.23,113.58v45.95c0,2.88.64,4.96,1.93,6.25,1.29,1.29,3.37,1.93,6.25,1.93h66.35c18.94,0,26.26-3.79,35.43-11.36,9.16-7.57,13.75-16.25,13.75-32.02V42.25c0-15.75-4.58-23.31-13.75-30.88C229.02,3.79,221.69,0,202.75,0h-56.81c-2.88,0-4.96.64-6.25,1.93-1.29,1.29-1.93,3.37-1.93,6.25v23.39c0,2.88.64,4.96,1.93,6.25,1.29,1.29,3.37,1.93,6.25,1.93h47.26c4.7,0,8.18.95,10.45,2.84,2.27,1.89,3.41,14.44,3.41,18.23v55.8c0,3.79-.87,6.59-2.61,8.39-1.74,1.83-4.36,2.74-7.84,2.74h-22.04v-35.46c0-2.87-.64-4.95-1.93-6.24-1.29-1.29-3.37-1.93-6.25-1.93h-19.01s-19.16,29.45-19.16,29.45Z"/></g></svg>"#;

/// Rasterize the logo SVG at the given width, preserving aspect ratio.
pub fn rasterize_logo(width: u32) -> (Vec<u8>, u32, u32) {
    let tree = resvg::usvg::Tree::from_str(LOGO_SVG, &resvg::usvg::Options::default()).unwrap();
    let svg_size = tree.size();
    let scale = width as f32 / svg_size.width();
    let height = (svg_size.height() * scale).ceil() as u32;
    let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        resvg::usvg::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    (premul_to_straight(&pixmap), width, height)
}

/// Rasterize the icon SVG to a square image (for window/taskbar icons).
pub fn rasterize_logo_square(size: u32) -> (Vec<u8>, u32, u32) {
    let tree = resvg::usvg::Tree::from_str(ICON_SVG, &resvg::usvg::Options::default()).unwrap();
    let scale = size as f32 / tree.size().width();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size, size).unwrap();
    resvg::render(
        &tree,
        resvg::usvg::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    (premul_to_straight(&pixmap), size, size)
}

fn premul_to_straight(pixmap: &resvg::tiny_skia::Pixmap) -> Vec<u8> {
    pixmap
        .pixels()
        .iter()
        .flat_map(|p| {
            let a = p.alpha();
            if a == 0 {
                [0, 0, 0, 0]
            } else {
                let r = (p.red() as u16 * 255 / a as u16) as u8;
                let g = (p.green() as u16 * 255 / a as u16) as u8;
                let b = (p.blue() as u16 * 255 / a as u16) as u8;
                [r, g, b, a]
            }
        })
        .collect()
}

/// Get the cache directory path
pub fn get_cache_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("Gores Map Downloader")
        .join("cache")
}

/// Format bytes into human-readable string (B, KB, MB)
pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Compare two version strings, returns true if a > b
pub fn version_greater_than(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> Vec<u32> { v.split('.').filter_map(|s| s.parse().ok()).collect() };
    parse(a) > parse(b)
}

/// Process cache refresh on version upgrade - clears outdated cached files
pub fn process_cache_refresh(cache_dir: &std::path::Path) {
    let version_file = cache_dir.join("version.txt");
    let stored = std::fs::read_to_string(&version_file)
        .unwrap_or_default()
        .trim()
        .to_string();

    for (ver, files) in CACHE_REFRESH {
        if stored.is_empty() || version_greater_than(ver, &stored) {
            for name in *files {
                let _ = std::fs::remove_file(
                    cache_dir.join("thumbnails").join(format!("{}.png", name)),
                );
                let _ = std::fs::remove_file(cache_dir.join("full").join(format!("{}.png", name)));
            }
        }
    }

    let _ = std::fs::write(&version_file, APP_VERSION);
}
