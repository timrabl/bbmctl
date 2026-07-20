// Copyright (c) 2023-2026 Tim Oliver Rabl
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in
// all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

pub mod entities;
pub mod migrations;
pub mod repositories;

use std::path::PathBuf;

use anyhow::{Context, Result};
use sea_orm::{Database as SeaDatabase, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use migrations::Migrator;

pub use repositories::{CampaignRepo, MeasurementRepo, SettingsRepo};

/// Canonical format for every timestamp written to the database.
///
/// `measurements.timestamp` is a varchar that all queries compare
/// lexicographically, so the representation must be fixed-width and
/// byte-order-equal to chronological order. Millisecond precision keeps
/// same-second measurements correctly ordered; the literal `Z` suffix avoids
/// the `+00:00` form, whose `+` sorts before `Z` and silently inverts ordering.
pub const TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.3fZ";

/// Current UTC time in [`TIMESTAMP_FORMAT`].
pub fn now_timestamp() -> String {
    chrono::Utc::now().format(TIMESTAMP_FORMAT).to_string()
}

pub struct Database {
    conn: DatabaseConnection,
}

impl Database {
    const DB_DIR: &str = ".bbmctl";
    const DB_FILE: &str = "measurements.db";

    pub async fn connect(path: Option<&str>) -> Result<Self> {
        let db_path = if let Some(p) = path {
            PathBuf::from(p)
        } else {
            let dir = dirs::home_dir()
                .context("could not determine home directory")?
                .join(Self::DB_DIR);
            std::fs::create_dir_all(&dir)
                .with_context(|| format!("could not create directory: {}", dir.display()))?;
            dir.join(Self::DB_FILE)
        };

        let url = format!("sqlite:{}?mode=rwc", db_path.display());
        let conn = SeaDatabase::connect(&url)
            .await
            .with_context(|| format!("could not connect to database: {}", db_path.display()))?;

        Migrator::up(&conn, None)
            .await
            .context("database migration failed")?;

        Ok(Self { conn })
    }

    /// Connect to an in-memory SQLite database (for testing).
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn connect_in_memory() -> Result<Self> {
        let conn = SeaDatabase::connect("sqlite::memory:")
            .await
            .context("could not open in-memory database")?;

        Migrator::up(&conn, None)
            .await
            .context("database migration failed")?;

        Ok(Self { conn })
    }

    pub fn measurements(&self) -> MeasurementRepo<'_> {
        MeasurementRepo::new(&self.conn)
    }

    pub fn campaigns(&self) -> CampaignRepo<'_> {
        CampaignRepo::new(&self.conn)
    }

    pub fn settings(&self) -> SettingsRepo<'_> {
        SettingsRepo::new(&self.conn)
    }
}
