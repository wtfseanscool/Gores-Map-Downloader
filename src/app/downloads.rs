//! Download logic

use super::App;
use crate::types::*;
use eframe::egui;
use futures::StreamExt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use tracing::info;

/// Download a single map file with progress tracking and cancellation support.
async fn download_map(
    idx: usize,
    url: String,
    dest: PathBuf,
    map_size: i64,
    skip_existing: bool,
    state: Arc<Mutex<DownloadState>>,
    client: &reqwest::Client,
    ctx: &egui::Context,
    token: &CancellationToken,
) {
    if token.is_cancelled() {
        let mut s = state.lock().unwrap();
        if matches!(s.downloads.get(&idx), Some(DownloadStatus::Pending)) {
            s.downloads.insert(idx, DownloadStatus::Cancelled);
            s.cancelled_count += 1;
        }
        ctx.request_repaint();
        return;
    }

    if skip_existing && dest.exists() {
        let mut s = state.lock().unwrap();
        s.downloads.insert(idx, DownloadStatus::Skipped);
        s.skipped_count += 1;
        s.downloaded_bytes += map_size as u64;
        ctx.request_repaint();
        return;
    }

    {
        let mut s = state.lock().unwrap();
        s.downloads.insert(idx, DownloadStatus::Downloading(0, 0));
        s.active_count += 1;
    }
    ctx.request_repaint();

    let result = client.get(&url).send().await;

    match result {
        Ok(response) if response.status().is_success() => {
            let total_size = response.content_length().unwrap_or(0);
            let mut downloaded: u64 = 0;
            let mut bytes_vec = Vec::with_capacity(total_size as usize);
            let mut stream = response.bytes_stream();
            let mut last_repaint = std::time::Instant::now();

            loop {
                tokio::select! {
                    _ = token.cancelled() => {
                        let mut s = state.lock().unwrap();
                        s.downloads.insert(idx, DownloadStatus::Cancelled);
                        s.cancelled_count += 1;
                        s.active_count -= 1;
                        ctx.request_repaint();
                        return;
                    }
                    chunk = stream.next() => {
                        match chunk {
                            Some(Ok(data)) => {
                                downloaded += data.len() as u64;
                                bytes_vec.extend_from_slice(&data);
                                let mut s = state.lock().unwrap();
                                s.downloads.insert(idx, DownloadStatus::Downloading(downloaded, total_size));
                                drop(s);
                                if last_repaint.elapsed() >= std::time::Duration::from_millis(100) {
                                    ctx.request_repaint();
                                    last_repaint = std::time::Instant::now();
                                }
                            }
                            Some(Err(e)) => {
                                let mut s = state.lock().unwrap();
                                s.downloads.insert(idx, DownloadStatus::Failed(e.to_string()));
                                s.failed_count += 1;
                                s.active_count -= 1;
                                ctx.request_repaint();
                                return;
                            }
                            None => break,
                        }
                    }
                }
            }

            if std::fs::write(&dest, &bytes_vec).is_ok() {
                let mut s = state.lock().unwrap();
                s.downloads.insert(idx, DownloadStatus::Complete);
                s.completed_count += 1;
                s.active_count -= 1;
                s.downloaded_bytes += map_size as u64;
            } else {
                let mut s = state.lock().unwrap();
                s.downloads.insert(idx, DownloadStatus::Failed("Write failed".into()));
                s.failed_count += 1;
                s.active_count -= 1;
            }
        }
        Ok(response) => {
            let mut s = state.lock().unwrap();
            s.downloads.insert(idx, DownloadStatus::Failed(format!("HTTP {}", response.status())));
            s.failed_count += 1;
            s.active_count -= 1;
        }
        Err(e) => {
            let mut s = state.lock().unwrap();
            s.downloads.insert(idx, DownloadStatus::Failed(e.to_string()));
            s.failed_count += 1;
            s.active_count -= 1;
        }
    }
    ctx.request_repaint();
}

/// Spawn a batch of download tasks with a shared semaphore.
fn spawn_download_batch(
    maps: Vec<(usize, String, PathBuf, i64, bool)>,
    state: Arc<Mutex<DownloadState>>,
    cancel_token: CancellationToken,
    ctx: egui::Context,
    runtime: &tokio::runtime::Runtime,
) {
    runtime.spawn(async move {
        let semaphore = Arc::new(tokio::sync::Semaphore::new(4));
        let client = reqwest::Client::new();
        let mut handles = vec![];

        for (idx, url, dest, map_size, skip_existing) in maps {
            let sem = semaphore.clone();
            let state = state.clone();
            let client = client.clone();
            let ctx = ctx.clone();
            let token = cancel_token.clone();

            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.unwrap();
                download_map(idx, url, dest, map_size, skip_existing, state, &client, &ctx, &token).await;
            }));
        }

        for handle in handles {
            let _ = handle.await;
        }
    });
}

impl App {
    pub fn download_selected(&mut self, ctx: &egui::Context) {
        let selected: Vec<usize> = self.selected_indices.iter().copied().collect();
        if selected.is_empty() {
            return;
        }

        std::fs::create_dir_all(&self.download_path).ok();

        let maps: Vec<(usize, String, PathBuf, i64, bool)> = selected
            .iter()
            .filter_map(|&idx| {
                let map = self.maps.get(idx)?;
                let url = Self::get_map_url(map);
                let dest = self.download_path.join(format!("{}.map", map.name));
                Some((idx, url, dest, map.size, true)) // skip_existing = true
            })
            .collect();

        info!(count = maps.len(), path = %self.download_path.display(), "Starting download batch");

        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());

        {
            let mut s = self.download_state.lock().unwrap();
            s.total_queued = maps.len();
            s.completed_count = 0;
            s.failed_count = 0;
            s.skipped_count = 0;
            s.cancelled_count = 0;
            s.total_bytes = maps.iter().map(|(_, _, _, size, _)| *size as u64).sum();
            s.downloaded_bytes = 0;
            s.download_order = maps.iter().map(|(idx, _, _, _, _)| *idx).collect();
            for &(idx, _, _, _, _) in &maps {
                s.downloads.insert(idx, DownloadStatus::Pending);
            }
        }

        self.show_download_modal = true;

        spawn_download_batch(maps, self.download_state.clone(), cancel_token, ctx.clone(), &self.runtime);
    }

    pub fn retry_failed_downloads(&mut self, ctx: &egui::Context) {
        let failed_maps: Vec<(usize, String, PathBuf, i64, bool)> = {
            let s = self.download_state.lock().unwrap();
            s.download_order
                .iter()
                .filter_map(|&idx| {
                    if matches!(s.downloads.get(&idx), Some(DownloadStatus::Failed(_))) {
                        let map = self.maps.get(idx)?;
                        let url = Self::get_map_url(map);
                        let dest = self.download_path.join(format!("{}.map", map.name));
                        Some((idx, url, dest, map.size, false)) // skip_existing = false
                    } else {
                        None
                    }
                })
                .collect()
        };

        if failed_maps.is_empty() {
            return;
        }

        let cancel_token = CancellationToken::new();
        self.cancel_token = Some(cancel_token.clone());

        {
            let mut s = self.download_state.lock().unwrap();
            s.failed_count = 0;
            for &(idx, _, _, _, _) in &failed_maps {
                s.downloads.insert(idx, DownloadStatus::Pending);
            }
        }

        spawn_download_batch(failed_maps, self.download_state.clone(), cancel_token, ctx.clone(), &self.runtime);
    }
}
