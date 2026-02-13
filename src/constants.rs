//! Application constants and configuration

pub const MAPS_BASE_URL: &str = "https://raw.githubusercontent.com/wtfseanscool/kog-maps/main";
pub const PREVIEWS_BASE_URL: &str =
    "https://raw.githubusercontent.com/wtfseanscool/kog-maps-previews/main";
pub const MANIFEST_URL: &str =
    "https://raw.githubusercontent.com/wtfseanscool/kog-maps/main/manifest.json";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const REPO_OWNER: &str = "wtfseanscool";
pub const REPO_NAME: &str = "Gores-Map-Downloader";

/// Cache refresh - maps to clear when upgrading to/past each version
pub const CACHE_REFRESH: &[(&str, &[&str])] = &[];
