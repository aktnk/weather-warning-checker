# Weather Checker - NSSM Service Installation Script
# Requires: NSSM (https://nssm.cc/) installed and in PATH
# Run as Administrator

param(
    [string]$ExePath = "C:\opt\weather-checker\tauri-weather-checker.exe",
    [string]$WorkDir = "C:\opt\weather-checker",
    [string]$ServiceName = "weather-checker"
)

$ErrorActionPreference = "Stop"

# Check for NSSM
if (-not (Get-Command nssm -ErrorAction SilentlyContinue)) {
    Write-Error "NSSM not found. Install from https://nssm.cc/ and add to PATH."
    exit 1
}

# Check for executable
if (-not (Test-Path $ExePath)) {
    Write-Error "Executable not found: $ExePath"
    exit 1
}

Write-Host "Installing $ServiceName service..."

# Install service
nssm install $ServiceName $ExePath

# Set working directory
nssm set $ServiceName AppDirectory $WorkDir

# Set environment variables
nssm set $ServiceName AppEnvironmentExtra "RUST_LOG=tauri_weather_checker=info"

# Restart on exit
nssm set $ServiceName AppExit Default Restart
nssm set $ServiceName AppRestartDelay 30000

# Logging
$LogDir = Join-Path $WorkDir "data"
if (-not (Test-Path $LogDir)) {
    New-Item -ItemType Directory -Path $LogDir -Force | Out-Null
}
nssm set $ServiceName AppStdout (Join-Path $LogDir "stdout.log")
nssm set $ServiceName AppStderr (Join-Path $LogDir "stderr.log")
nssm set $ServiceName AppRotateFiles 1
nssm set $ServiceName AppRotateBytes 10485760

# Configure Windows service recovery (restart on failure)
sc.exe failure $ServiceName reset=86400 actions=restart/30000/restart/60000/restart/120000

# Start the service
nssm start $ServiceName

Write-Host "$ServiceName service installed and started successfully."
Write-Host "Check status: nssm status $ServiceName"
