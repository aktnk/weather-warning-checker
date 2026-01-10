# Tauri Weather Checker

Rust + Tauri implementation of the JMA Weather Warning Checker.

## Project Status

**Phase 1: Project Setup - COMPLETE**

This is a skeleton project with all core modules defined but not fully implemented. The following components are ready:

- ✅ Database layer (SQLite with sqlx)
- ✅ Configuration management
- ✅ Email notification system
- ✅ Scheduler (10-minute weather checks, daily cleanup)
- ✅ Error handling
- ⚠️ JMA Feed client (skeleton only - needs XML parsing implementation)
- ⚠️ Weather checker logic (skeleton only - needs integration)

**Next Steps:**
1. Implement XML parsing for JMA feeds (extra.xml and VPWW54 format)
2. Complete JMA feed client integration
3. Test the complete workflow
4. Add system tray menu items
5. Optional: Add GUI for configuration

## Architecture

This is a **background service** application that runs in the system tray:

```
Tauri App (System Tray)
├── Scheduler (tokio-cron-scheduler)
│   ├── Weather check: Every 10 minutes
│   └── Cleanup: Daily at 01:00
├── Database (SQLite via sqlx)
│   ├── Extra (Last-Modified tracking)
│   ├── VPWW54xml (XML file cache)
│   └── CityReport (Warning state)
├── JMA Feed Client
│   ├── Fetch extra.xml
│   ├── Parse VPWW54 entries
│   └── Download warning data
├── Weather Checker
│   └── Compare and detect changes
└── Notification (Gmail SMTP)
    └── Send email on status change
```

## Prerequisites

- **Rust** 1.70+ (installed: 1.91.1)
- **Node.js** 18+ (installed: 22.19.0)
- **System dependencies** (for Tauri):
  - Linux: `libwebkit2gtk-4.1-dev`, `build-essential`, `curl`, `wget`, `file`, `libssl-dev`, `libgtk-3-dev`, `librsvg2-dev`
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft Visual Studio C++ Build Tools

## Setup

1. **Clone and navigate**:
   ```bash
   cd tauri-weather-checker
   ```

2. **Install system dependencies** (Ubuntu/Debian):
   ```bash
   sudo apt update
   sudo apt install libwebkit2gtk-4.1-dev \
     build-essential \
     curl \
     wget \
     file \
     libssl-dev \
     libayatana-appindicator3-dev \
     librsvg2-dev
   ```

3. **Create environment file**:
   ```bash
   cp .env.example .env
   # Edit .env and add your Gmail credentials
   ```

4. **Build the project**:
   ```bash
   cd src-tauri
   cargo build
   ```

## Development

### Run in development mode

```bash
cd src-tauri
cargo run
```

This will:
- Initialize the SQLite database
- Start the scheduler (weather check + cleanup)
- Show a system tray icon
- Output logs to console

### Build for production

```bash
cd src-tauri
cargo build --release
```

The binary will be in `src-tauri/target/release/tauri-weather-checker`.

### Run with logging

```bash
RUST_LOG=tauri_weather_checker=debug cargo run
```

Log levels: `error`, `warn`, `info`, `debug`, `trace`

## Configuration

All configuration is done via environment variables (`.env` file):

| Variable | Description | Example |
|----------|-------------|---------|
| `DATADIR` | XML cache directory | `data/xml` |
| `DELETED_DIR` | Deleted XML directory | `data/deleted` |
| `DB_PATH` | SQLite database path | `data/weather.sqlite3` |
| `GMAIL_APP_PASS` | Gmail app password | `abcd efgh ijkl mnop` |
| `GMAIL_FROM` | Sender email | `you@gmail.com` |
| `EMAIL_TO` | Recipient email | `recipient@example.com` |
| `EMAIL_BCC` | BCC email (optional) | `bcc@example.com` |

### Gmail Setup

1. Enable 2-factor authentication in Google Account
2. Generate an app password: https://myaccount.google.com/apppasswords
3. Use the app password in `GMAIL_APP_PASS`

## Project Structure

```
tauri-weather-checker/
├── src-tauri/
│   ├── src/
│   │   ├── main.rs           # Entry point, Tauri setup
│   │   ├── config.rs         # Environment configuration
│   │   ├── database.rs       # SQLite operations
│   │   ├── jma_feed.rs       # JMA XML fetching/parsing (TODO)
│   │   ├── weather_checker.rs # Core warning logic
│   │   ├── notification.rs   # Email notifications
│   │   ├── cleanup.rs        # Data cleanup tasks
│   │   ├── scheduler.rs      # Cron-like scheduling
│   │   └── error.rs          # Error types
│   ├── Cargo.toml            # Rust dependencies
│   ├── tauri.conf.json       # Tauri configuration
│   └── build.rs              # Build script
├── .env.example              # Example environment file
├── .gitignore
└── README.md
```

## Monitoring Regions

To add or modify monitored regions, edit `src-tauri/src/weather_checker.rs`:

```rust
pub async fn run_check(&self) -> Result<()> {
    // Add your regions here
    self.check_warnings("静岡地方気象台", &["裾野市", "御殿場市"]).await?;
    self.check_warnings("東京管区気象台", &["千代田区"]).await?;
    // Add more as needed
    Ok(())
}
```

Then rebuild: `cargo build`

## Troubleshooting

### Database errors
- Ensure `data/` directory exists and is writable
- Delete `data/weather.sqlite3` to reinitialize

### Email not sending
- Verify `.env` file exists with correct credentials
- Check Gmail app password is valid
- Ensure 2FA is enabled on Google account

### Build errors
- Update Rust: `rustup update`
- Clean build: `cargo clean && cargo build`

## TODO: Implementation Tasks

The following features need to be implemented:

1. **JMA Feed XML Parsing** (`src-tauri/src/jma_feed.rs`):
   - [ ] Parse `extra.xml` feed to extract VPWW54 entries
   - [ ] Parse VPWW54 format XML files
   - [ ] Extract city, warning kind, and status
   - [ ] Handle different LMO (Local Meteorological Observatory) formats

2. **Weather Checker Integration**:
   - [ ] Complete `get_latest_vpww54_for_lmo` implementation
   - [ ] Test warning status change detection
   - [ ] Handle XML file caching properly

3. **System Tray Menu**:
   - [ ] Add menu items (Start/Stop, View Logs, Settings, Quit)
   - [ ] Handle menu events
   - [ ] Show status in tooltip

4. **Optional: GUI**:
   - [ ] Settings screen for email config
   - [ ] Add/remove monitored regions
   - [ ] Log viewer
   - [ ] Notification history

## Comparison with Python Version

| Feature | Python + Docker | Rust + Tauri |
|---------|----------------|--------------|
| Memory | ~100-200 MB | ~10-50 MB |
| Binary size | ~500 MB (Docker image) | ~5-15 MB |
| Startup time | 3-5 seconds | <1 second |
| Dependencies | Docker required | None (single binary) |
| Distribution | Docker image | Cross-platform executable |
| Configuration | .env + code edit | .env (+ GUI later) |
| Platform | Docker-supported OS | Windows/Mac/Linux native |

## License

Same as the original Python project.
