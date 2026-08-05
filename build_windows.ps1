# PowerShell script to build Windows release binary and output it to the \release folder.

$ErrorActionPreference = "Stop"

# Settings
$OutputName = "AARustSerialMonitor"

# Ensure the output release folder exists
$ReleaseDir = Join-Path $PSScriptRoot "release"
if (-not (Test-Path $ReleaseDir)) {
    New-Item -ItemType Directory -Path $ReleaseDir | Out-Null
}

Write-Host "`n=== Building Windows Release Binary ===" -ForegroundColor Cyan
try {
    cargo build --release
    $WinSource = Join-Path $PSScriptRoot "target\release\aa_rust_serial_monitor.exe"
    $WinDest = Join-Path $ReleaseDir "$OutputName.exe"
    Copy-Item -Path $WinSource -Destination $WinDest -Force
    Write-Host "[SUCCESS] Windows binary compiled and copied to: release/$OutputName.exe" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to build Windows binary: $_" -ForegroundColor Red
    Exit 1
}

Write-Host "`n=== Build Completed Successfully! ===" -ForegroundColor Green
Write-Host "Output available in the 'release' directory:"
Get-ChildItem $ReleaseDir | Select-Object Name, Length, LastWriteTime | Format-Table
