//! Auto-update logic

use super::App;
use crate::constants::*;
use crate::db::Database;
use crate::types::*;
use eframe::egui;
use std::path::PathBuf;
use tracing::{debug, error, info, warn};

impl App {
    pub fn check_for_updates(&mut self, ctx: &egui::Context) {
        if self.update_check_done {
            return;
        }
        self.update_check_done = true;

        let ctx = ctx.clone();
        let current_db_version = self.db.get_db_version().ok().flatten().unwrap_or_default();
        let current_map_count = self.maps.len();
        let current_map_names: std::collections::HashSet<String> = 
            self.maps.iter().map(|m| m.name.clone()).collect();
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Gores Map Downloader")
            .join("maps.db");

        info!(
            db_version = %current_db_version,
            map_count = current_map_count,
            "Starting update check"
        );

        std::thread::spawn(move || {
            // Mock flags: MOCK_APP_UPDATE, MOCK_DB_UPDATE, MOCK_FULL_UPDATE
            let mock_full = std::env::var("MOCK_FULL_UPDATE").is_ok();
            let mock_app = mock_full || std::env::var("MOCK_APP_UPDATE").is_ok();
            let mock_db = mock_full || std::env::var("MOCK_DB_UPDATE").is_ok();

            // App update check
            if mock_app {
                debug!("Mock app update: simulating update dialog");
                let mock_version = "1.0.0".to_string();
                let mock_body = "## What's New\n\n\
                    - Added automatic map database updates\n\
                    - New toast notifications for background updates\n\
                    - Improved search with global keyboard shortcuts\n\n\
                    ## Bug Fixes\n\n\
                    - Fixed manifest field mismatch causing update failures\n\
                    - Fixed sort state not preserving during search\n\n\
                    ## Performance\n\n\
                    - Parallel thumbnail prefetching\n\
                    - Optimized grid view rendering".to_string();
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp("app_update".into(), mock_version);
                    mem.data.insert_temp("app_update_body".into(), mock_body);
                });
                ctx.request_repaint();
            } else if !mock_db {
            debug!("Checking for app updates");
            match self_update::backends::github::ReleaseList::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .build()
                .and_then(|r| r.fetch())
            {
                Ok(releases) => {
                    if let Some(latest) = releases.first() {
                        debug!(latest = %latest.version, current = APP_VERSION, "Fetched latest release");
                        if Self::version_newer(&latest.version, APP_VERSION) {
                            info!(version = %latest.version, "App update available");
                            let body = latest.body.clone().unwrap_or_default();
                            ctx.request_repaint();
                            ctx.memory_mut(|mem| {
                                mem.data
                                    .insert_temp("app_update".into(), latest.version.clone());
                                mem.data
                                    .insert_temp("app_update_body".into(), body);
                            });
                        } else {
                            debug!("App is up to date");
                        }
                    } else {
                        debug!("No GitHub releases found");
                    }
                }
                Err(e) => {
                    warn!(error = %e, "Failed to fetch app releases");
                }
            }
            }

            // DB/manifest update check - auto-update silently
            if mock_db {
                // Mock DB: bypass network, simulate notification
                debug!("Mock DB update: simulating notification");
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp(
                        "db_auto_updated".into(),
                        "MockMap1,MockMap2,MockMap3".to_string(),
                    );
                });
                ctx.request_repaint();
            } else if !mock_app {
            
            debug!(url = MANIFEST_URL, "Fetching manifest");
            match reqwest::blocking::get(MANIFEST_URL) {
                Ok(response) => {
                    debug!(status = %response.status(), "Manifest response received");
                    match response.json::<Manifest>() {
                        Ok(manifest) => {
                            debug!(
                                manifest_version = %manifest.version,
                                manifest_count = manifest.map_count,
                                "Manifest parsed"
                            );
                            
                            if manifest.version != current_db_version
                                || manifest.map_count != current_map_count
                            {
                                info!("Database update available, auto-updating");
                                
                                    let new_maps: Vec<String> = manifest.maps.iter()
                                        .filter(|m| !current_map_names.contains(&m.name))
                                        .map(|m| m.name.clone())
                                        .collect();
                                    
                                    let result: Result<usize, String> = (|| {
                                        let db = Database::open(&db_path).map_err(|e| e.to_string())?;
                                        db.clear_maps().map_err(|e| e.to_string())?;
                                        let count = db.import_maps(&manifest.maps).map_err(|e| e.to_string())?;
                                        db.set_db_version(&manifest.version).map_err(|e| e.to_string())?;
                                        Ok(count)
                                    })();
                                    
                                    match result {
                                        Ok(count) => {
                                            info!(
                                                total = count,
                                                new = new_maps.len(),
                                                names = ?new_maps,
                                                "Database auto-updated"
                                            );
                                            ctx.memory_mut(|mem| {
                                                mem.data.insert_temp(
                                                    "db_auto_updated".into(),
                                                    new_maps.join(","),
                                                );
                                            });
                                        }
                                        Err(e) => {
                                            error!(error = %e, "Database auto-update failed");
                                        }
                                    }
                                    ctx.request_repaint();
                            } else {
                                debug!("Database is up to date");
                            }
                        }
                        Err(e) => {
                            error!(error = %e, "Failed to parse manifest JSON");
                        }
                    }
                }
                Err(e) => {
                    error!(error = %e, "Failed to fetch manifest");
                }
            }
            } // end DB update gate
            info!("Update check complete");
        });
    }

    pub fn version_newer(new: &str, current: &str) -> bool {
        let parse = |s: &str| -> (u32, u32, u32) {
            let parts: Vec<u32> = s
                .trim_start_matches('v')
                .split('.')
                .filter_map(|p| p.parse().ok())
                .collect();
            (
                parts.get(0).copied().unwrap_or(0),
                parts.get(1).copied().unwrap_or(0),
                parts.get(2).copied().unwrap_or(0),
            )
        };
        parse(new) > parse(current)
    }

    pub fn perform_app_update(&mut self, ctx: &egui::Context) {
        self.update_in_progress = true;
        let ctx = ctx.clone();
        let is_mock_retry = std::env::var("MOCK_APP_UPDATE").is_ok() && self.app_update_error.is_some();

        info!("Starting app update download");
        std::thread::spawn(move || {
            if is_mock_retry {
                // Mock: simulate success on retry
                std::thread::sleep(std::time::Duration::from_millis(500));
                ctx.memory_mut(|mem| {
                    mem.data.insert_temp("app_update_done".into(), "1.0.0".to_string());
                });
                ctx.request_repaint();
                return;
            }

            let result = self_update::backends::github::Update::configure()
                .repo_owner(REPO_OWNER)
                .repo_name(REPO_NAME)
                .bin_name("gores-map-downloader")
                .bin_path_in_archive("Gores Map Downloader.exe")
                .current_version(APP_VERSION)
                .build()
                .and_then(|u| u.update());

            ctx.memory_mut(|mem| match result {
                Ok(status) => {
                    info!(version = %status.version(), "App update downloaded");
                    mem.data
                        .insert_temp("app_update_done".into(), status.version().to_string());
                }
                Err(e) => {
                    error!(error = %e, "App update failed");
                    mem.data
                        .insert_temp("app_update_error".into(), e.to_string());
                }
            });
            ctx.request_repaint();
        });
    }

    pub fn perform_db_update(&mut self, ctx: &egui::Context) {
        self.update_in_progress = true;
        let ctx = ctx.clone();
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Gores Map Downloader")
            .join("maps.db");

        info!("Starting manual database update");
        std::thread::spawn(move || {
            let result: Result<(String, usize), String> = (|| {
                let response = reqwest::blocking::get(MANIFEST_URL).map_err(|e| e.to_string())?;
                let manifest: Manifest = response.json().map_err(|e| e.to_string())?;
                let db = Database::open(&db_path).map_err(|e| e.to_string())?;
                db.clear_maps().map_err(|e| e.to_string())?;
                let count = db.import_maps(&manifest.maps).map_err(|e| e.to_string())?;
                db.set_db_version(&manifest.version)
                    .map_err(|e| e.to_string())?;
                Ok((manifest.version, count))
            })();

            ctx.memory_mut(|mem| match result {
                Ok((version, count)) => {
                    info!(version = %version, count = count, "Manual database update complete");
                    mem.data
                        .insert_temp("db_update_done".into(), format!("{}:{}", version, count));
                }
                Err(e) => {
                    error!(error = %e, "Manual database update failed");
                    mem.data.insert_temp("db_update_error".into(), e);
                }
            });
            ctx.request_repaint();
        });
    }
}
