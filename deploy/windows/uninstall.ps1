# Weather Checker - NSSM Service Removal Script
# Run as Administrator

param(
    [string]$ServiceName = "weather-checker"
)

$ErrorActionPreference = "Stop"

# Check for NSSM
if (-not (Get-Command nssm -ErrorAction SilentlyContinue)) {
    Write-Error "NSSM not found. Install from https://nssm.cc/ and add to PATH."
    exit 1
}

Write-Host "Stopping $ServiceName service..."
nssm stop $ServiceName 2>$null

Write-Host "Removing $ServiceName service..."
nssm remove $ServiceName confirm

Write-Host "$ServiceName service removed successfully."
