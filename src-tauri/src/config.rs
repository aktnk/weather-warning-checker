use std::env;
use std::path::Path;
use serde::Deserialize;
use crate::error::{Result, WeatherCheckerError};

/// Monitored region configuration
#[derive(Debug, Clone, Deserialize)]
pub struct MonitoredRegion {
    /// Local Meteorological Observatory name (e.g., "静岡地方気象台")
    pub lmo: String,
    /// List of cities to monitor (e.g., ["裾野市", "御殿場市"])
    pub cities: Vec<String>,
}

/// Monitor configuration loaded from YAML file
#[derive(Debug, Clone, Deserialize)]
pub struct MonitorConfig {
    /// List of monitored regions
    pub monitored_regions: Vec<MonitoredRegion>,
}

impl MonitorConfig {
    /// Load monitor configuration from YAML file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .map_err(|e| WeatherCheckerError::Config(
                format!("Failed to read config file '{}': {}", path.display(), e)
            ))?;

        let config: MonitorConfig = serde_yaml::from_str(&content)
            .map_err(|e| WeatherCheckerError::Config(
                format!("Failed to parse config file '{}': {}", path.display(), e)
            ))?;

        if config.monitored_regions.is_empty() {
            return Err(WeatherCheckerError::Config(
                "No monitored regions defined in config file".into()
            ));
        }

        tracing::info!(
            "Loaded {} monitored regions from config",
            config.monitored_regions.len()
        );

        for region in &config.monitored_regions {
            tracing::debug!("  {} -> {:?}", region.lmo, region.cities);
        }

        Ok(config)
    }

    /// Get default config file path
    pub fn default_path() -> String {
        env::var("CONFIG_PATH").unwrap_or_else(|_| "config.yaml".to_string())
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub data_dir: String,
    pub deleted_dir: String,
    pub db_path: String,
    pub gmail_app_pass: String,
    pub gmail_from: String,
    pub email_to: String,
    pub email_bcc: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            data_dir: env::var("DATADIR").unwrap_or_else(|_| "data/xml".to_string()),
            deleted_dir: env::var("DELETED_DIR").unwrap_or_else(|_| "data/deleted".to_string()),
            db_path: env::var("DB_PATH").unwrap_or_else(|_| "data/weather.sqlite3".to_string()),
            gmail_app_pass: env::var("GMAIL_APP_PASS")
                .map_err(|_| WeatherCheckerError::Config("GMAIL_APP_PASS not set".into()))?,
            gmail_from: env::var("GMAIL_FROM")
                .map_err(|_| WeatherCheckerError::Config("GMAIL_FROM not set".into()))?,
            email_to: env::var("EMAIL_TO")
                .map_err(|_| WeatherCheckerError::Config("EMAIL_TO not set".into()))?,
            email_bcc: env::var("EMAIL_BCC").ok(),
        })
    }
}
