use crate::cleanup::Cleanup;
use crate::config::Config;
use crate::error::Result;
use crate::notification::EmailNotifier;
use crate::weather_checker::WeatherChecker;
use std::sync::atomic::{AtomicU32, Ordering};
use tokio_cron_scheduler::{Job, JobScheduler};
use tokio_util::sync::CancellationToken;

static CONSECUTIVE_FAILURES: AtomicU32 = AtomicU32::new(0);
const FAILURE_WARNING_THRESHOLD: u32 = 3;

pub async fn start_scheduler(cancel_token: CancellationToken) -> Result<()> {
    tracing::info!("Starting scheduler...");

    // Send startup notification (non-fatal)
    match Config::from_env() {
        Ok(config) => {
            let notifier = EmailNotifier::new(config);
            if let Err(e) = notifier
                .send_system_notification("started", "Service started successfully")
                .await
            {
                tracing::warn!("Failed to send startup notification: {}", e);
            }
        }
        Err(e) => {
            tracing::warn!("Failed to load config for startup notification: {}", e);
        }
    }

    let mut scheduler = JobScheduler::new().await?;

    // Run weather check immediately on startup
    tracing::info!("Running initial weather check...");
    match run_weather_check().await {
        Ok(()) => {
            CONSECUTIVE_FAILURES.store(0, Ordering::Relaxed);
            write_heartbeat();
        }
        Err(e) => {
            let count = CONSECUTIVE_FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
            tracing::error!("Initial weather check failed: {}", e);
            if count >= FAILURE_WARNING_THRESHOLD {
                tracing::warn!(
                    "Weather check has failed {} consecutive times",
                    count
                );
            }
        }
    }

    // Schedule weather check every 10 minutes
    let weather_job = Job::new_async("0 */10 * * * *", |_uuid, _lock| {
        Box::pin(async {
            match run_weather_check().await {
                Ok(()) => {
                    let prev = CONSECUTIVE_FAILURES.swap(0, Ordering::Relaxed);
                    if prev >= FAILURE_WARNING_THRESHOLD {
                        tracing::info!(
                            "Weather check recovered after {} consecutive failures",
                            prev
                        );
                    }
                    write_heartbeat();
                }
                Err(e) => {
                    let count = CONSECUTIVE_FAILURES.fetch_add(1, Ordering::Relaxed) + 1;
                    tracing::error!("Weather check failed: {}", e);
                    if count >= FAILURE_WARNING_THRESHOLD {
                        tracing::warn!(
                            "Weather check has failed {} consecutive times",
                            count
                        );
                    }
                }
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

    // Wait for cancellation signal
    cancel_token.cancelled().await;
    tracing::info!("Shutting down scheduler...");
    scheduler.shutdown().await?;
    tracing::info!("Scheduler stopped");

    Ok(())
}

async fn run_weather_check() -> Result<()> {
    let start = std::time::Instant::now();
    let checker = WeatherChecker::new().await?;
    checker.run_check().await?;
    let elapsed = start.elapsed();
    tracing::info!("Weather check completed in {}ms", elapsed.as_millis());
    Ok(())
}

async fn run_cleanup() -> Result<()> {
    let cleanup = Cleanup::new().await?;
    cleanup.run_cleanup().await
}

fn write_heartbeat() {
    let heartbeat_path = std::path::Path::new("data/heartbeat");
    if let Some(parent) = heartbeat_path.parent() {
        if !parent.exists() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                tracing::warn!("Failed to create heartbeat directory: {}", e);
                return;
            }
        }
    }
    let timestamp = chrono::Utc::now().to_rfc3339();
    if let Err(e) = std::fs::write(heartbeat_path, timestamp) {
        tracing::warn!("Failed to write heartbeat file: {}", e);
    }
}
