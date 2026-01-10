use crate::config::Config;
use crate::database::Database;
use crate::error::Result;
use std::fs;
use std::path::PathBuf;
use chrono::{Utc, Duration};

pub struct Cleanup {
    config: Config,
    db: Database,
}

impl Cleanup {
    pub async fn new() -> Result<Self> {
        let config = Config::from_env()?;
        let db = Database::new().await?;

        Ok(Self { config, db })
    }

    pub async fn run_cleanup(&self) -> Result<()> {
        tracing::info!("Starting cleanup task...");

        // Delete old XML files (30+ days)
        self.cleanup_old_files().await?;

        // Delete old database records (30+ days)
        self.cleanup_old_records().await?;

        tracing::info!("Cleanup task completed");
        Ok(())
    }

    async fn cleanup_old_files(&self) -> Result<()> {
        let deleted_dir = PathBuf::from(&self.config.deleted_dir);

        if !deleted_dir.exists() {
            tracing::debug!("Deleted directory does not exist, skipping file cleanup");
            return Ok(());
        }

        let cutoff_date = Utc::now() - Duration::days(30);
        let mut deleted_count = 0;

        for entry in fs::read_dir(&deleted_dir)? {
            let entry = entry?;
            let path = entry.path();

            if !path.is_file() {
                continue;
            }

            let metadata = fs::metadata(&path)?;
            let modified = metadata.modified()?;
            let modified_datetime: chrono::DateTime<Utc> = modified.into();

            if modified_datetime < cutoff_date {
                fs::remove_file(&path)?;
                deleted_count += 1;
                tracing::debug!("Deleted old file: {:?}", path);
            }
        }

        if deleted_count > 0 {
            tracing::info!("Deleted {} old XML files", deleted_count);
        }

        Ok(())
    }

    async fn cleanup_old_records(&self) -> Result<()> {
        self.db.delete_old_records(30).await?;
        tracing::info!("Deleted old database records");
        Ok(())
    }
}
