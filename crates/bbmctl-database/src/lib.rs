pub mod entities;
pub mod migrations;
pub mod repositories;

use std::path::PathBuf;

use anyhow::{Context, Result};
use sea_orm::{Database as SeaDatabase, DatabaseConnection};
use sea_orm_migration::MigratorTrait;

use migrations::Migrator;

pub use repositories::{CampaignRepo, MeasurementRepo, SettingsRepo};

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

        Migrator::up(&conn, None).await
            .context("database migration failed")?;

        Ok(Self { conn })
    }

    /// Connect to an in-memory SQLite database (for testing).
    #[cfg(any(test, feature = "test-utils"))]
    pub async fn connect_in_memory() -> Result<Self> {
        let conn = SeaDatabase::connect("sqlite::memory:")
            .await
            .context("could not open in-memory database")?;

        Migrator::up(&conn, None).await
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
