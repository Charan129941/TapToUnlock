# ==============================================================================
# OpenTapUnlock: Windows System Service Installer for opentapd
# ==============================================================================
param (
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$ServiceName = "OpenTapDaemon"
$ServiceDisplayName = "OpenTapUnlock Biometric Desktop Service"
$TargetBinPath = "$env:ProgramFiles\OpenTapUnlock\opentapd.exe"

$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "[ERROR] This script must be run as Administrator (Right-click -> Run as Administrator)."
    exit 1
}

if ($Uninstall) {
    Write-Host "============================================================================" -ForegroundColor Yellow
    Write-Host "         OpenTapUnlock Windows Service Uninstaller ($ServiceName)           " -ForegroundColor Yellow
    Write-Host "============================================================================"
    if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
        Write-Host "[1/2] Stopping and deleting Windows Service $ServiceName..."
        Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
        sc.exe delete $ServiceName | Out-Null
        Write-Host "      -> Removed Windows Service" -ForegroundColor Green
    }
    if (Test-Path "$env:ProgramFiles\OpenTapUnlock") {
        Write-Host "[2/2] Removing installation directory..."
        Remove-Item -Path "$env:ProgramFiles\OpenTapUnlock" -Recurse -Force
        Write-Host "      -> Cleaned $env:ProgramFiles\OpenTapUnlock" -ForegroundColor Green
    }
    Write-Host "============================================================================" -ForegroundColor Green
    Write-Host "  [SUCCESS] $ServiceName uninstalled cleanly!" -ForegroundColor Green
    Write-Host "============================================================================"
    exit 0
}

Write-Host "============================================================================" -ForegroundColor Cyan
Write-Host "        OpenTapUnlock Windows Service Installer ($ServiceName)              " -ForegroundColor Cyan
Write-Host "============================================================================"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CrateDir = Join-Path $ScriptDir "..\desktop\daemon"

Write-Host "[1/4] Building opentapd release binary using Cargo..." -ForegroundColor White
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "[ERROR] Cargo toolchain not found. Please install Rust from rustup.rs."
    exit 1
}

Push-Location $CrateDir
try {
    cargo build --release
} finally {
    Pop-Location
}

$BuiltBin = Join-Path $CrateDir "..\..\target\release\opentapd.exe"
if (-not (Test-Path $BuiltBin)) {
    Write-Error "[ERROR] Build failed: $BuiltBin was not generated."
    exit 1
}

Write-Host "[2/4] Installing executable to $TargetBinPath..." -ForegroundColor White
New-Item -Path "$env:ProgramFiles\OpenTapUnlock" -ItemType Directory -Force | Out-Null
Copy-Item -Path $BuiltBin -Destination $TargetBinPath -Force
Write-Host "      -> Installed to $TargetBinPath" -ForegroundColor Green

Write-Host "[3/4] Registering as Windows System Service..." -ForegroundColor White
if (Get-Service -Name $ServiceName -ErrorAction SilentlyContinue) {
    Stop-Service -Name $ServiceName -Force -ErrorAction SilentlyContinue
    sc.exe delete $ServiceName | Out-Null
    Start-Sleep -Seconds 1
}

New-Service -Name $ServiceName -DisplayName $ServiceDisplayName `
    -BinaryPathName "\"$TargetBinPath\" --daemon" `
    -Description "Zero-Trust authentication coordinator for mobile biometric PC unlocking." `
    -StartupType Automatic | Out-Null

Write-Host "      -> Registered Service $ServiceName (Automatic Startup)" -ForegroundColor Green

Write-Host "[4/4] Starting Service..." -ForegroundColor White
Start-Service -Name $ServiceName
Get-Service -Name $ServiceName | Format-Table -AutoSize

Write-Host "============================================================================" -ForegroundColor Green
Write-Host "  [SUCCESS] opentapd is running as a Windows System Service!" -ForegroundColor Green
Write-Host "  To pair your mobile phone, open an Admin PowerShell terminal and run:" -ForegroundColor White
Write-Host "      & \"$TargetBinPath\" --pair" -ForegroundColor Yellow
Write-Host "============================================================================"
