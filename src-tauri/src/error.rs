use thiserror::Error;

#[derive(Error, Debug)]
pub enum WeatherCheckerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Email error: {0}")]
    Email(#[from] lettre::error::Error),

    #[error("XML parsing error: {0}")]
    XmlParse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Scheduler error: {0}")]
    Scheduler(String),

    #[error("Other error: {0}")]
    Other(String),
}

impl From<tokio_cron_scheduler::JobSchedulerError> for WeatherCheckerError {
    fn from(err: tokio_cron_scheduler::JobSchedulerError) -> Self {
        WeatherCheckerError::Scheduler(err.to_string())
    }
}

impl From<lettre::address::AddressError> for WeatherCheckerError {
    fn from(err: lettre::address::AddressError) -> Self {
        WeatherCheckerError::Other(format!("Address error: {}", err))
    }
}

impl From<lettre::transport::smtp::Error> for WeatherCheckerError {
    fn from(err: lettre::transport::smtp::Error) -> Self {
        WeatherCheckerError::Other(format!("SMTP error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, WeatherCheckerError>;
