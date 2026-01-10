use std::env;
use crate::error::{Result, WeatherCheckerError};

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
