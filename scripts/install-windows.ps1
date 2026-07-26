# ==============================================================================
# OpenTapUnlock: Windows Custom Credential Provider Installer & Uninstaller
# ==============================================================================
param (
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"
$CLSID = "{6F70656E-7461-702D-756E-6C6F636B3034}"
$ProviderName = "OpenTap Biometric Credential Provider"
$TargetDllPath = "$env:SystemRoot\System32\OpenTapCredProvider.dll"

# 1. Verify Administrator Privileges
$isAdmin = ([Security.Principal.WindowsPrincipal] [Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if (-not $isAdmin) {
    Write-Error "[ERROR] This script must be run as Administrator (Right-click -> Run as Administrator)."
    exit 1
}

if ($Uninstall) {
    Write-Host "============================================================================" -ForegroundColor Yellow
    Write-Host "         OpenTapUnlock Windows Credential Provider Uninstaller              " -ForegroundColor Yellow
    Write-Host "============================================================================"
    
    Write-Host "[1/3] Removing Winlogon Credential Provider registration..."
    if (Test-Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$CLSID") {
        Remove-Item -Path "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$CLSID" -Recurse -Force
        Write-Host "      -> Removed HKLM:\...\Credential Providers\$CLSID" -ForegroundColor Green
    }

    Write-Host "[2/3] Removing COM Class ID registration..."
    if (Test-Path "HKLM:\SOFTWARE\Classes\CLSID\$CLSID") {
        Remove-Item -Path "HKLM:\SOFTWARE\Classes\CLSID\$CLSID" -Recurse -Force
        Write-Host "      -> Removed HKLM:\SOFTWARE\Classes\CLSID\$CLSID" -ForegroundColor Green
    }

    Write-Host "[3/3] Deleting system DLL from System32..."
    if (Test-Path $TargetDllPath) {
        Remove-Item -Path $TargetDllPath -Force -ErrorAction SilentlyContinue
        Write-Host "      -> Removed $TargetDllPath" -ForegroundColor Green
    }

    Write-Host "============================================================================" -ForegroundColor Green
    Write-Host "  [SUCCESS] OpenTapCredProvider uninstalled cleanly!" -ForegroundColor Green
    Write-Host "============================================================================"
    exit 0
}

Write-Host "============================================================================" -ForegroundColor Cyan
Write-Host "     OpenTapUnlock Windows Credential Provider Installer ($CLSID)           " -ForegroundColor Cyan
Write-Host "============================================================================"

$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
$CrateDir = Join-Path $ScriptDir "..\desktop\os-modules\windows-cred-provider"

# 2. Build the Rust COM DLL in Release Mode
Write-Host "[1/5] Building Rust Credential Provider DLL in release mode..." -ForegroundColor White
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Error "[ERROR] Cargo/Rust toolchain not found. Please run 'winget install Rustlang.Rustup' or install from rustup.rs."
    exit 1
}

Push-Location $CrateDir
try {
    cargo build --release
} finally {
    Pop-Location
}

$BuiltDll = Join-Path $CrateDir "..\..\..\target\release\OpenTapCredProvider.dll"
if (-not (Test-Path $BuiltDll)) {
    Write-Error "[ERROR] Build failed: $BuiltDll was not generated."
    exit 1
}

# 3. Copy DLL to C:\Windows\System32\
Write-Host "[2/5] Installing OpenTapCredProvider.dll to $env:SystemRoot\System32\..." -ForegroundColor White
if (Test-Path $TargetDllPath) {
    $Timestamp = Get-Date -Format "yyyyMMdd_HHmmss"
    Copy-Item -Path $TargetDllPath -Destination "$TargetDllPath.bak_$Timestamp" -Force
    Write-Host "      -> Created backup of existing DLL at $TargetDllPath.bak_$Timestamp" -ForegroundColor DarkGray
}
Copy-Item -Path $BuiltDll -Destination $TargetDllPath -Force
Write-Host "      -> Installed to $TargetDllPath" -ForegroundColor Green

# 4. Register COM Class in Windows Registry
Write-Host "[3/5] Registering COM Class in HKLM:\SOFTWARE\Classes\CLSID\$CLSID..." -ForegroundColor White
$ClsidKey = "HKLM:\SOFTWARE\Classes\CLSID\$CLSID"
New-Item -Path $ClsidKey -Force | Out-Null
New-ItemProperty -Path $ClsidKey -Name "(Default)" -Value $ProviderName -PropertyType String -Force | Out-Null

$InprocKey = "$ClsidKey\InprocServer32"
New-Item -Path $InprocKey -Force | Out-Null
New-ItemProperty -Path $InprocKey -Name "(Default)" -Value $TargetDllPath -PropertyType String -Force | Out-Null
New-ItemProperty -Path $InprocKey -Name "ThreadingModel" -Value "Apartment" -PropertyType String -Force | Out-Null
Write-Host "      -> Registered COM InprocServer32 (Apartment Threading)" -ForegroundColor Green

# 5. Register in Winlogon Credential Providers list
Write-Host "[4/5] Registering in Winlogon Credential Providers stack..." -ForegroundColor White
$AuthKey = "HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Authentication\Credential Providers\$CLSID"
New-Item -Path $AuthKey -Force | Out-Null
New-ItemProperty -Path $AuthKey -Name "(Default)" -Value $ProviderName -PropertyType String -Force | Out-Null
Write-Host "      -> Registered Winlogon Credential Provider $CLSID" -ForegroundColor Green

Write-Host "[5/5] Verification Check:" -ForegroundColor White
Get-ItemProperty -Path $AuthKey | Select-Object PSChildName, "(Default)" | Format-Table -AutoSize

Write-Host "============================================================================" -ForegroundColor Green
Write-Host "  [SUCCESS] OpenTapCredProvider installed and registered successfully!" -ForegroundColor Green
Write-Host "  To test: Lock your screen (Win + L). You will see the OpenTap user tile." -ForegroundColor Green
Write-Host "  Note: If opentapd daemon is not running, the tile will indicate offline" -ForegroundColor DarkGray
Write-Host "  and you can seamlessly log in with your standard PIN or Password." -ForegroundColor DarkGray
Write-Host "============================================================================"
