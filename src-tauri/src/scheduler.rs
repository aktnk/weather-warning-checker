use tokio_cron_scheduler::{JobScheduler, Job};
use crate::weather_checker::WeatherChecker;
use crate::cleanup::Cleanup;
use crate::error::Result;

pub async fn start_scheduler() -> Result<()> {
    tracing::info!("Starting scheduler...");

    let scheduler = JobScheduler::new().await?;

    // Run weather check immediately on startup
    tracing::info!("Running initial weather check...");
    if let Err(e) = run_weather_check().await {
        tracing::error!("Initial weather check failed: {}", e);
    }

    // Schedule weather check every 10 minutes
    let weather_job = Job::new_async("0 */10 * * * *", |_uuid, _lock| {
        Box::pin(async {
            if let Err(e) = run_weather_check().await {
                tracing::error!("Weather check failed: {}", e);
            }
        })
    })?;

    scheduler.add(weather_job).await?;
    tracing::info!("Scheduled weather check every 10 minutes");

    // Schedule cleanup daily at 01:00
    let cleanup_job = Job::new_async("0 0 1 * * *", |_uuid, _lock| {
        Box::pin(async {
            if let Err(e) = run_cleanup().await {
                tracing::error!("Cleanup failed: {}", e);
            }
        })
    })?;

    scheduler.add(cleanup_job).await?;
    tracing::info!("Scheduled cleanup daily at 01:00");

    scheduler.start().await?;

    // Keep scheduler alive
    loop {
        tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
    }
}

async fn run_weather_check() -> Result<()> {
    let checker = WeatherChecker::new().await?;
    checker.run_check().await
}

async fn run_cleanup() -> Result<()> {
    let cleanup = Cleanup::new().await?;
    cleanup.run_cleanup().await
}
