// Prevents additional console window on Windows in release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cleanup;
mod config;
mod database;
mod error;
mod jma_feed;
mod notification;
mod scheduler;
mod weather_checker;

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables FIRST
    dotenvy::dotenv().ok();

    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tauri_weather_checker=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting Weather Checker...");

    // Initialize database
    let db = database::Database::new().await?;
    db.init_schema().await?;
    tracing::info!("Database initialized");

    // Create cancellation token for graceful shutdown
    let cancel_token = tokio_util::sync::CancellationToken::new();
    let scheduler_token = cancel_token.clone();

    // Start scheduler in background and capture handle for crash detection
    let scheduler_handle = tokio::spawn(async move {
        scheduler::start_scheduler(scheduler_token).await
    });

    // Spawn shutdown signal handler
    let shutdown_token = cancel_token.clone();
    tokio::spawn(async move {
        wait_for_shutdown_signal().await;
        tracing::info!("Shutdown signal received, stopping gracefully...");
        shutdown_token.cancel();
    });

    // Monitor scheduler task for crashes
    let monitor_token = cancel_token.clone();
    tokio::spawn(async move {
        match scheduler_handle.await {
            Ok(Ok(())) => {
                // Scheduler exited cleanly (e.g. via cancellation)
                tracing::info!("Scheduler task completed normally");
            }
            Ok(Err(e)) => {
                // Scheduler returned an error
                if !monitor_token.is_cancelled() {
                    tracing::error!("Scheduler crashed with error: {}", e);
                    std::process::exit(1);
                }
            }
            Err(e) => {
                // Scheduler task panicked
                if !monitor_token.is_cancelled() {
                    tracing::error!("Scheduler task panicked: {}", e);
                    std::process::exit(1);
                }
            }
        }
    });

    // Build Tauri app with system tray
    tauri::Builder::default()
        .setup(|_app| {
            // System tray can be added later with proper icons
            tracing::info!(
                "Application initialized (system tray disabled until icons are configured)"
            );
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");

    // Tauri has exited, ensure scheduler is cancelled
    cancel_token.cancel();
    // Give scheduler a moment to clean up
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    tracing::info!("Weather Checker stopped");

    Ok(())
}

async fn wait_for_shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");
        let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
        tokio::select! {
            _ = sigterm.recv() => tracing::info!("Received SIGTERM"),
            _ = sigint.recv() => tracing::info!("Received SIGINT"),
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
        tracing::info!("Received Ctrl+C");
    }
}
