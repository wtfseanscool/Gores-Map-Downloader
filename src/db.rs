//! Database module for Gores Map Downloader
//! Handles SQLite storage for map metadata and user settings

use crate::types::ManifestMap;
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tracing::{debug, error};

/// Map metadata stored in database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Map {
    pub id: i64,
    pub name: String,
    pub category: String,
    pub stars: i32,
    pub points: i32,
    pub author: String,
    pub release_date: String,
    pub size: i64,
    pub downloaded: bool,
    pub local_path: Option<String>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open or create database at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        debug!(path = %path.display(), "Database opened");
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS maps (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL UNIQUE,
                category TEXT NOT NULL,
                stars INTEGER NOT NULL,
                points INTEGER NOT NULL,
                author TEXT NOT NULL,
                release_date TEXT NOT NULL,
                size INTEGER NOT NULL DEFAULT 0,
                downloaded INTEGER NOT NULL DEFAULT 0,
                local_path TEXT
            );
            
            CREATE INDEX IF NOT EXISTS idx_maps_category ON maps(category);
            CREATE INDEX IF NOT EXISTS idx_maps_stars ON maps(stars);
            CREATE INDEX IF NOT EXISTS idx_maps_points ON maps(points);
            CREATE INDEX IF NOT EXISTS idx_maps_downloaded ON maps(downloaded);
            
            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            
            CREATE TABLE IF NOT EXISTS metadata (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );",
        )?;
        Ok(())
    }

    /// Clear all maps from database
    pub fn clear_maps(&self) -> Result<()> {
        self.conn.execute("DELETE FROM maps", [])?;
        Ok(())
    }

    /// Import maps from JSON data, preserving download status
    pub fn import_maps(&self, maps: &[ManifestMap]) -> Result<usize> {
        let mut imported = 0;

        for map in maps {
            let result = self.conn.execute(
                "INSERT INTO maps (name, category, stars, points, author, release_date, size)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(name) DO UPDATE SET
                    category = excluded.category,
                    stars = excluded.stars,
                    points = excluded.points,
                    author = excluded.author,
                    release_date = excluded.release_date,
                    size = excluded.size",
                params![
                    map.name,
                    map.category,
                    map.stars,
                    map.points,
                    map.author,
                    map.release_date,
                    map.size
                ],
            );

            match result {
                Ok(_) => imported += 1,
                Err(e) => error!(map = %map.name, error = %e, "Failed to import map"),
            }
        }

        debug!(imported = imported, total = maps.len(), "Maps imported");
        Ok(imported)
    }

    /// Get all maps
    pub fn get_all_maps(&self) -> Result<Vec<Map>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, category, stars, points, author, release_date, size, downloaded, local_path
             FROM maps ORDER BY name COLLATE NOCASE"
        )?;

        let maps = stmt
            .query_map([], |row| {
                Ok(Map {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    category: row.get(2)?,
                    stars: row.get(3)?,
                    points: row.get(4)?,
                    author: row.get(5)?,
                    release_date: row.get(6)?,
                    size: row.get(7)?,
                    downloaded: row.get::<_, i32>(8)? != 0,
                    local_path: row.get(9)?,
                })
            })?
            .collect::<Result<Vec<_>>>()?;

        Ok(maps)
    }

    /// Mark a map as downloaded
    pub fn mark_downloaded(&self, map_id: i64, local_path: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE maps SET downloaded = 1, local_path = ?1 WHERE id = ?2",
            params![local_path, map_id],
        )?;
        Ok(())
    }

    /// Get a setting value
    pub fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Set a setting value
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO settings (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    /// Get database version
    pub fn get_db_version(&self) -> Result<Option<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT value FROM metadata WHERE key = 'version'")?;
        let mut rows = stmt.query([])?;

        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Set database version
    pub fn set_db_version(&self, version: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO metadata (key, value) VALUES ('version', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![version],
        )?;
        Ok(())
    }

    /// Get map count
    pub fn map_count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM maps", [], |r| r.get(0))?;
        Ok(count as usize)
    }
}
