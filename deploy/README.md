# Deployment Guide

This directory contains service configuration files for running Weather Warning Checker as a background service on various operating systems.

## Prerequisites

1. Build the release binary:
   ```bash
   cd src-tauri
   cargo build --release
   ```

2. Copy the binary and configuration files to the installation directory (default: `/opt/weather-checker/`):
   ```bash
   sudo mkdir -p /opt/weather-checker/data
   sudo cp target/release/tauri-weather-checker /opt/weather-checker/
   sudo cp .env config.yaml /opt/weather-checker/
   ```

3. **Fix `.env` paths for deployment:**

   The development `.env` uses paths relative to `src-tauri/` (e.g., `../config.yaml`, `../data/xml`).
   For deployment where the working directory is `/opt/weather-checker/`, these `../` prefixes must be removed:

   ```bash
   # Check current paths
   grep -E '(DATADIR|DELETED_DIR|DB_PATH|CONFIG_PATH)' /opt/weather-checker/.env

   # Fix paths (remove ../ prefixes)
   sudo sed -i 's|=\.\./|=|g' /opt/weather-checker/.env

   # Verify
   grep -E '(DATADIR|DELETED_DIR|DB_PATH|CONFIG_PATH)' /opt/weather-checker/.env
   ```

   Expected values for deployment:
   | Variable | Development (`src-tauri/`) | Deployment (`/opt/weather-checker/`) |
   |----------|--------------------------|--------------------------------------|
   | CONFIG_PATH | `../config.yaml` | `config.yaml` |
   | DATADIR | `../data/xml` | `data/xml` |
   | DELETED_DIR | `../data/deleted` | `data/deleted` |
   | DB_PATH | `../data/weather.sqlite3` | `data/weather.sqlite3` |

---

## Ubuntu / systemd

### Prerequisites

Tauri/GTK requires a display server. For headless environments, install Xvfb:

```bash
sudo apt install xvfb
```

### Install

```bash
# Create service user
sudo useradd -r -s /usr/sbin/nologin weather-checker
sudo chown -R weather-checker:weather-checker /opt/weather-checker

# Install unit files
sudo cp deploy/systemd/weather-checker.service /etc/systemd/system/
sudo cp deploy/systemd/weather-checker-watchdog.service /etc/systemd/system/
sudo cp deploy/systemd/weather-checker-watchdog.timer /etc/systemd/system/

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable --now weather-checker.service
sudo systemctl enable --now weather-checker-watchdog.timer
```

### Manual Test (before systemd)

```bash
cd /opt/weather-checker
sudo -u weather-checker xvfb-run --auto-servernum --server-args="-screen 0 1x1x8" \
  env RUST_LOG=tauri_weather_checker=debug ./tauri-weather-checker
# Ctrl+C to stop
```

Verify:
- Logs show "Weather check completed in XXXms" (Step 1: Enhanced Logging)
- `data/heartbeat` file exists with UTC timestamp (Step 2: Heartbeat)
- Email received with subject "test:weather-checker: started" (Step 3: Startup Notification)

### Management

```bash
# Status
sudo systemctl status weather-checker

# Logs (follow)
journalctl -u weather-checker -f

# Restart
sudo systemctl restart weather-checker

# Stop (triggers graceful shutdown)
sudo systemctl stop weather-checker

# Verify graceful shutdown in logs:
# "Shutdown signal received, stopping gracefully..."
# "Shutting down scheduler..."
# "Scheduler stopped"
# "Weather Checker stopped"

# Watchdog status
systemctl list-timers weather-checker-watchdog.timer

# Manual watchdog check
sudo systemctl start weather-checker-watchdog.service
journalctl -u weather-checker-watchdog --no-pager | tail -5
```

### Crash Recovery Test

```bash
# Start service
sudo systemctl start weather-checker

# Force kill (simulates crash)
sudo kill -9 $(systemctl show weather-checker --property=MainPID --value)

# Wait for auto-restart (RestartSec=30)
sleep 35
sudo systemctl status weather-checker
# Should show: active (running)

# Check restart count
systemctl show weather-checker --property=NRestarts
```

### Configuration

- Edit `/opt/weather-checker/.env` for environment variables
- Edit `/opt/weather-checker/config.yaml` for monitored regions
- Changes take effect on the next 10-minute check cycle (no restart needed)

### Update Binary

```bash
cd ~/projects/weather-warning-checker/src-tauri
cargo build --release
sudo systemctl stop weather-checker
sudo cp target/release/tauri-weather-checker /opt/weather-checker/
sudo systemctl start weather-checker
```

---

## macOS / launchd

### Install

```bash
# Copy plist (edit paths in plist if not using /opt/weather-checker/)
sudo cp deploy/launchd/com.aktnk.weather-checker.plist /Library/LaunchDaemons/

# Load and start
sudo launchctl load /Library/LaunchDaemons/com.aktnk.weather-checker.plist
```

### Management

```bash
# Status
sudo launchctl list | grep weather-checker

# Stop
sudo launchctl unload /Library/LaunchDaemons/com.aktnk.weather-checker.plist

# Start
sudo launchctl load /Library/LaunchDaemons/com.aktnk.weather-checker.plist

# Logs
tail -f /opt/weather-checker/data/stdout.log
tail -f /opt/weather-checker/data/stderr.log
```

### Note

Environment variables from `.env` are loaded by the application itself (via `dotenvy`), so place `.env` in the working directory (`/opt/weather-checker/`). The `EnvironmentVariables` in the plist only sets `RUST_LOG`; add other variables there if needed.

---

## Windows / NSSM

### Prerequisites

Install [NSSM](https://nssm.cc/) and add it to your system PATH.

### Install

Run PowerShell as Administrator:

```powershell
# Default paths (C:\opt\weather-checker\)
.\deploy\windows\install.ps1

# Custom paths
.\deploy\windows\install.ps1 -ExePath "D:\services\weather-checker\tauri-weather-checker.exe" -WorkDir "D:\services\weather-checker"
```

### Management

```powershell
# Status
nssm status weather-checker

# Restart
nssm restart weather-checker

# Stop
nssm stop weather-checker

# Edit service configuration (GUI)
nssm edit weather-checker

# Logs
Get-Content C:\opt\weather-checker\data\stdout.log -Tail 50
```

### Uninstall

Run PowerShell as Administrator:

```powershell
.\deploy\windows\uninstall.ps1
```

---

## Heartbeat Monitoring

All platforms: The application writes a heartbeat timestamp to `data/heartbeat` after each successful weather check (every 10 minutes). On systemd, the watchdog timer automatically checks this file every 30 minutes and restarts the service if the heartbeat is stale.

For macOS and Windows, you can set up an external cron job or scheduled task to check the heartbeat file age and restart the service if needed.

---

## Troubleshooting

### GTK initialization error

```
Failed to initialize gtk backend!: BoolError { message: "Failed to initialize GTK" ... }
```

Tauri requires a display server. Use `xvfb-run` to provide a virtual display:
- systemd: Already configured in `weather-checker.service`
- Manual: `xvfb-run --auto-servernum --server-args="-screen 0 1x1x8" ./tauri-weather-checker`

### config.yaml not found

```
Failed to read config file '../config.yaml': No such file or directory
```

The `.env` file contains development paths with `../` prefixes. Fix paths for the deployment directory (see Prerequisites step 3).

### xvfb-run mktemp failure

```
mktemp: failed to create directory via template '/tmp/xvfb-run.XXXXXX'
```

The systemd service needs `PrivateTmp=true` (already included in the provided service file) to allow `/tmp` access under `ProtectSystem=strict`.
