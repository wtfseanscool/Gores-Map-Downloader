//! Thumbnail and preview loading

use super::App;
use crate::constants::*;
use eframe::egui;
use futures::StreamExt;
use tracing::{debug, warn};

impl App {
    pub fn start_thumbnail_prefetch(&mut self, ctx: &egui::Context) {
        let cache_dir = self.cache_dir.clone();
        let ctx_clone = ctx.clone();
        let map_names: Vec<String> = self.maps.iter().map(|m| m.name.clone()).collect();

        debug!(count = map_names.len(), "Starting thumbnail prefetch");

        self.runtime.spawn(async move {
            let client = reqwest::Client::new();
            let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(8));

            let thumb_dir = cache_dir.join("thumbnails");
            std::fs::create_dir_all(&thumb_dir).ok();

            let mut handles = vec![];

            for name in map_names {
                let thumb_path = thumb_dir.join(format!("{}.png", name));
                if thumb_path.exists() {
                    continue;
                }

                let sem = semaphore.clone();
                let client = client.clone();
                let ctx = ctx_clone.clone();
                let url = format!("{}/thumbnails/{}.png", PREVIEWS_BASE_URL, name);

                let handle = tokio::spawn(async move {
                    let _permit = sem.acquire().await.ok();
                    if let Ok(response) = client.get(&url).send().await {
                        if response.status().is_success() {
                            if let Ok(bytes) = response.bytes().await {
                                std::fs::write(&thumb_path, &bytes).ok();
                                ctx.request_repaint();
                            }
                        }
                    }
                });
                handles.push(handle);
            }

            for handle in handles {
                handle.await.ok();
            }
        });
    }

    pub fn load_thumbnail(
        &mut self,
        ctx: &egui::Context,
        map_name: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(cached) = self.thumbnail_cache.get(map_name) {
            return cached.clone();
        }

        let thumb_path = self
            .cache_dir
            .join("thumbnails")
            .join(format!("{}.png", map_name));

        if thumb_path.exists() {
            let texture = image::open(&thumb_path).ok().map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                ctx.load_texture(
                    map_name,
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    egui::TextureOptions::LINEAR,
                )
            });
            self.thumbnail_cache
                .insert(map_name.to_string(), texture.clone());
            return texture;
        }

        None
    }

    pub fn load_full_preview(&mut self, ctx: &egui::Context, map_name: &str) {
        if self.preview_textures.contains_key(map_name) || self.preview_loading.contains(map_name) {
            return;
        }

        let full_path = self
            .cache_dir
            .join("full")
            .join(format!("{}.png", map_name));

        if full_path.exists() {
            let tex = image::open(&full_path).ok().map(|img| {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels = rgba.into_raw();
                ctx.load_texture(
                    format!("{}_full", map_name),
                    egui::ColorImage::from_rgba_unmultiplied(size, &pixels),
                    egui::TextureOptions::LINEAR,
                )
            });
            self.preview_textures.insert(map_name.to_string(), tex);
            return;
        }

        self.preview_loading.insert(map_name.to_string());
        let url = format!("{}/full/{}.png", PREVIEWS_BASE_URL, map_name);
        let cache_path = full_path.clone();
        let ctx_clone = ctx.clone();

        self.runtime.spawn(async move {
            if let Ok(response) = reqwest::get(&url).await {
                if response.status().is_success() {
                    if let Ok(bytes) = response.bytes().await {
                        std::fs::create_dir_all(cache_path.parent().unwrap()).ok();
                        std::fs::write(&cache_path, &bytes).ok();
                    }
                }
            }
            ctx_clone.request_repaint();
        });
    }

    pub fn open_preview_multi(&mut self, ctx: &egui::Context, map_names: Vec<String>) {
        self.preview_maps = map_names;
        self.preview_active_tab = 0;
        self.preview_zoom = 1.0;
        self.preview_offset = egui::Vec2::ZERO;
        self.preview_needs_fit = true;
        for name in &self.preview_maps.clone() {
            self.load_full_preview(ctx, name);
        }
    }
}
