# PowerShell script to build both Windows and Linux release binaries and output them to the \release folder.

$ErrorActionPreference = "Stop"

# Ensure the output release folder exists
$ReleaseDir = Join-Path $PSScriptRoot "release"
if (-not (Test-Path $ReleaseDir)) {
    New-Item -ItemType Directory -Path $ReleaseDir | Out-Null
}

Write-Host "`n=== Step 1: Building Windows Release Binary ===" -ForegroundColor Cyan
try {
    cargo build --release
    $WinSource = Join-Path $PSScriptRoot "target\release\aa_rust_serial_monitor.exe"
    $WinDest = Join-Path $ReleaseDir "aa_rust_serial_monitor.exe"
    Copy-Item -Path $WinSource -Destination $WinDest -Force
    Write-Host "[SUCCESS] Windows binary compiled and copied to: release/aa_rust_serial_monitor.exe" -ForegroundColor Green
} catch {
    Write-Host "[ERROR] Failed to build Windows binary: $_" -ForegroundColor Red
    Exit 1
}

# Determine if we should use Docker/Cross or WSL Ubuntu
$UseWsl = $false
$DockerAvailable = $false

Write-Host "`n=== Step 2: Checking for Docker / Cross build capability ===" -ForegroundColor Cyan
if (Get-Command docker -ErrorAction SilentlyContinue) {
    try {
        $dockerCheck = docker ps 2>&1
        if ($LASTEXITCODE -eq 0) {
            $DockerAvailable = $true
            Write-Host "[INFO] Docker is running. Will use 'cross' for Linux build." -ForegroundColor Green
        }
    } catch {
        # Docker is installed but not running
    }
}

if (-not $DockerAvailable) {
    Write-Host "[INFO] Docker is not available or not running. Checking for WSL Ubuntu..." -ForegroundColor Yellow
    try {
        # Directly test executing whoami in Ubuntu distribution
        $null = wsl -d Ubuntu whoami 2>&1
        if ($LASTEXITCODE -eq 0) {
            $UseWsl = $true
            Write-Host "[SUCCESS] WSL Ubuntu detected! Will use WSL to build the Linux binary." -ForegroundColor Green
        }
    } catch {
        # WSL or Ubuntu not available
    }
}

if (-not $DockerAvailable -and -not $UseWsl) {
    Write-Host "[ERROR] Neither Docker (running) nor WSL Ubuntu was detected." -ForegroundColor Red
    Write-Host "[ADVICE] To build for Linux, please either start Docker Desktop or install WSL Ubuntu." -ForegroundColor Yellow
    Exit 1
}

if ($DockerAvailable) {
    # Option 2: Docker/Cross build
    Write-Host "`n=== Step 3: Checking cross tool installation ===" -ForegroundColor Cyan
    if (-not (Get-Command cross -ErrorAction SilentlyContinue)) {
        Write-Host "[INFO] Installing 'cross'..." -ForegroundColor Yellow
        cargo install cross --git https://github.com/cross-rs/cross
    }
    
    Write-Host "`n=== Step 4: Building Linux Release Binary via cross ===" -ForegroundColor Cyan
    try {
        cross build --target x86_64-unknown-linux-gnu --release
        $LinuxSource = Join-Path $PSScriptRoot "target\x86_64-unknown-linux-gnu\release\aa_rust_serial_monitor"
        $LinuxDest = Join-Path $ReleaseDir "aa_rust_serial_monitor"
        Copy-Item -Path $LinuxSource -Destination $LinuxDest -Force
        Write-Host "[SUCCESS] Linux binary compiled and copied to: release/aa_rust_serial_monitor" -ForegroundColor Green
    } catch {
        Write-Host "[ERROR] Failed to build Linux binary via cross: $_" -ForegroundColor Red
        Exit 1
    }
}
elseif ($UseWsl) {
    # Option 1: WSL Ubuntu build
    Write-Host "`n=== Step 3: Setting up WSL Ubuntu dependencies ===" -ForegroundColor Cyan
    
    # 1. Install Ubuntu package dependencies as root (does not require sudo password prompting in WSL)
    Write-Host "[INFO] Ensuring Linux dependencies are installed in WSL Ubuntu (running apt-get)..." -ForegroundColor Yellow
    wsl -d Ubuntu -u root apt-get update
    wsl -d Ubuntu -u root apt-get install -y pkg-config libudev-dev libx11-dev libxcb1-dev libxkbcommon-dev libegl1-mesa-dev libasound2-dev build-essential curl
    Write-Host "[SUCCESS] Linux system dependencies verified." -ForegroundColor Green

    # 2. Check if Cargo/Rust is installed for default user
    $wslUser = (wsl -d Ubuntu whoami).Trim()
    Write-Host "[INFO] Checking for Rust/Cargo in WSL Ubuntu for user '$wslUser'..." -ForegroundColor Yellow
    $hasCargo = $false
    try {
        $cargoCheck = wsl -d Ubuntu -u $wslUser bash -c "source ~/.cargo/env 2>/dev/null; cargo --version" 2>&1
        if ($LASTEXITCODE -eq 0) {
            $hasCargo = $true
            Write-Host "[SUCCESS] Rust/Cargo is already installed in WSL." -ForegroundColor Green
        }
    } catch {}

    if (-not $hasCargo) {
        Write-Host "[INFO] Rust/Cargo not found in WSL. Installing now..." -ForegroundColor Yellow
        wsl -d Ubuntu -u $wslUser bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"
        Write-Host "[SUCCESS] Rust/Cargo installed in WSL Ubuntu." -ForegroundColor Green
    }

    # 3. Convert path and run build
    Write-Host "`n=== Step 4: Building Linux Release Binary inside WSL Ubuntu ===" -ForegroundColor Cyan
    $linuxPath = (wsl wslpath ($PSScriptRoot -replace '\\', '/')).Trim()
    
    try {
        Write-Host "[INFO] Compiling Linux binary inside WSL (this may take a few minutes)..." -ForegroundColor Yellow
        wsl -d Ubuntu -u $wslUser bash -c "source ~/.cargo/env; cd '$linuxPath'; cargo build --release"
        
        $LinuxSource = Join-Path $PSScriptRoot "target\release\aa_rust_serial_monitor"
        $LinuxDest = Join-Path $ReleaseDir "aa_rust_serial_monitor"
        Copy-Item -Path $LinuxSource -Destination $LinuxDest -Force
        Write-Host "[SUCCESS] Linux binary compiled and copied to: release/aa_rust_serial_monitor" -ForegroundColor Green
    } catch {
        Write-Host "[ERROR] Failed to build Linux binary inside WSL: $_" -ForegroundColor Red
        Exit 1
    }
}

Write-Host "`n=== Build Completed Successfully! ===" -ForegroundColor Green
Write-Host "Outputs available in the 'release' directory:"
Get-ChildItem $ReleaseDir | Select-Object Name, Length, LastWriteTime | Format-Table
