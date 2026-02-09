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

    // Start scheduler in background
    tokio::spawn(async move {
        if let Err(e) = scheduler::start_scheduler().await {
            tracing::error!("Scheduler error: {}", e);
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

    Ok(())
}
