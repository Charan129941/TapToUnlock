# OpenTapUnlock Production Build and Integration Test Runner (Windows PowerShell)
$ErrorActionPreference = "Stop"

# Automatically switch to the project root directory regardless of where the script is called from
Set-Location -Path "$PSScriptRoot\.."

Write-Host "===========================================================" -ForegroundColor Cyan
Write-Host "[+] OPENTAPUNLOCK FULL ECOSYSTEM BUILD AND VERIFICATION" -ForegroundColor Cyan
Write-Host "===========================================================" -ForegroundColor Cyan

# 1. Check required toolchains
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[ERROR] Rust toolchain (cargo) is required. Please install via https://rustup.rs" -ForegroundColor Red
    exit 1
}

# 2. Build Core and Daemon
Write-Host "`n[1/4] Building OpenTap Core Cryptographic Engine and Windows COM Daemon..." -ForegroundColor Yellow
cargo build --release --package opentap-core
cargo build --release --package opentapd

# 3. Run 100-Case E2E Integration and Stress Test Matrix
Write-Host "`n[2/4] Executing 100-Case E2E Integration and Security Test Matrix..." -ForegroundColor Yellow
cargo run --release --package e2e-matrix

# 4. Build Desktop Control Center UI
Write-Host "`n[3/4] Building Tauri Control Center UI..." -ForegroundColor Yellow
if (Get-Command npm -ErrorAction SilentlyContinue) {
    Push-Location "desktop\control-center"
    npm install
    npm run tauri build
    Pop-Location
} else {
    Write-Host "[WARNING] Node.js (npm) not found. Skipping UI desktop build." -ForegroundColor DarkYellow
}

Write-Host "`n===========================================================" -ForegroundColor Green
Write-Host "[SUCCESS] BUILD AND 100-CASE VERIFICATION COMPLETED SUCCESSFULLY!" -ForegroundColor Green
Write-Host "Binaries located in: target\release\opentapd.exe" -ForegroundColor Green
Write-Host "===========================================================" -ForegroundColor Green
