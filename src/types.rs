//! Common types and data structures

use std::collections::HashMap;

/// Download status for individual map downloads
#[derive(Clone, PartialEq)]
pub enum DownloadStatus {
    Pending,
    Downloading(u64, u64), // (downloaded_bytes, total_bytes)
    Complete,
    Skipped,
    Cancelled,
    Failed(String),
}

/// State tracking for batch downloads
pub struct DownloadState {
    pub downloads: HashMap<usize, DownloadStatus>, // map_idx -> status
    pub download_order: Vec<usize>,                // Preserve order for display
    pub active_count: usize,
    pub total_queued: usize,
    pub completed_count: usize,
    pub failed_count: usize,
    pub skipped_count: usize,
    pub cancelled_count: usize,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
}

impl Default for DownloadState {
    fn default() -> Self {
        Self {
            downloads: HashMap::new(),
            download_order: Vec::new(),
            active_count: 0,
            total_queued: 0,
            completed_count: 0,
            failed_count: 0,
            skipped_count: 0,
            cancelled_count: 0,
            total_bytes: 0,
            downloaded_bytes: 0,
        }
    }
}

/// Column to sort by in list view
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Name,
    Category,
    Stars,
    Points,
    Author,
    ReleaseDate,
}

/// Sort direction for list view
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Ascending,
    Descending,
}

/// Marker for indexed scrollbar - represents a jump point
#[derive(Clone)]
pub struct ScrollIndexMarker {
    pub label: String,
    pub row_index: usize,
}

/// Manifest structure from remote JSON
#[derive(serde::Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(alias = "count")]
    pub map_count: usize,
    pub maps: Vec<ManifestMap>,
}

/// Individual map entry in manifest
#[derive(serde::Deserialize)]
pub struct ManifestMap {
    pub name: String,
    pub category: String,
    pub stars: i32,
    pub points: i32,
    pub author: String,
    pub release_date: String,
    #[serde(default)]
    pub size: i64,
}
