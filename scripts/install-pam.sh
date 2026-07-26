#!/usr/bin/env bash
# ==============================================================================
# OpenTapUnlock: Automated Linux PAM Module Installer & Configurator
# ==============================================================================
set -e

echo "============================================================================"
echo "          OpenTapUnlock Linux PAM Installer (pam_opentap.so)"
echo "============================================================================"

# 1. Verify Root Privileges
if [ "$EUID" -ne 0 ]; then
  echo "[ERROR] This script must be run as root (sudo ./install-pam.sh)."
  exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PAM_CRATE_DIR="$SCRIPT_DIR/../desktop/os-modules/linux-pam"

# 2. Build the Rust PAM Module in Release Mode
echo "[1/5] Building Rust PAM module in release mode..."
if ! command -v cargo &> /dev/null; then
  echo "[ERROR] Cargo/Rust toolchain not found. Please install Rust via rustup.rs."
  exit 1
fi

cargo build --release --manifest-path "$PAM_CRATE_DIR/Cargo.toml"

BUILT_SO="$PAM_CRATE_DIR/../../target/release/libpam_opentap.so"
if [ ! -f "$BUILT_SO" ]; then
  echo "[ERROR] Build failed: $BUILT_SO was not generated."
  exit 1
fi

# 3. Detect OS Security/PAM Directory
echo "[2/5] Detecting target PAM security directory..."
if [ -d "/lib/x86_64-linux-gnu/security" ]; then
  PAM_DIR="/lib/x86_64-linux-gnu/security"
elif [ -d "/lib64/security" ]; then
  PAM_DIR="/lib64/security"
elif [ -d "/lib/security" ]; then
  PAM_DIR="/lib/security"
else
  PAM_DIR="/lib/x86_64-linux-gnu/security"
  mkdir -p "$PAM_DIR"
fi

echo "      -> Target PAM Directory: $PAM_DIR"

# 4. Install Shared Object Binary
echo "[3/5] Installing pam_opentap.so to $PAM_DIR..."
cp -f "$BUILT_SO" "$PAM_DIR/pam_opentap.so"
chmod 644 "$PAM_DIR/pam_opentap.so"
chown root:root "$PAM_DIR/pam_opentap.so"

# 5. Configure /etc/pam.d/ Stack safely with backup
PAM_TARGET_FILE="/etc/pam.d/common-auth"
if [ ! -f "$PAM_TARGET_FILE" ]; then
  # Fallback to /etc/pam.d/sudo or /etc/pam.d/system-auth on Arch/Fedora/RHEL
  if [ -f "/etc/pam.d/system-auth" ]; then
    PAM_TARGET_FILE="/etc/pam.d/system-auth"
  else
    PAM_TARGET_FILE="/etc/pam.d/sudo"
  fi
fi

echo "[4/5] Backing up and configuring $PAM_TARGET_FILE..."
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
cp -f "$PAM_TARGET_FILE" "${PAM_TARGET_FILE}.bak_${TIMESTAMP}"
echo "      -> Created safety backup at ${PAM_TARGET_FILE}.bak_${TIMESTAMP}"

PAM_LINE="auth    sufficient                      pam_opentap.so socket=/run/opentapd/opentapd.sock timeout=10"

# Check if already installed
if grep -q "pam_opentap.so" "$PAM_TARGET_FILE"; then
  echo "      -> pam_opentap.so is already present in $PAM_TARGET_FILE. Updating line..."
  sed -i "/pam_opentap.so/c\\$PAM_LINE" "$PAM_TARGET_FILE"
else
  echo "      -> Inserting pam_opentap.so at the top of auth chain..."
  # Insert at line 1 or right after comments
  sed -i "1s|^|# OpenTapUnlock Biometric Mobile Unlock\n$PAM_LINE\n|" "$PAM_TARGET_FILE"
fi

echo "[5/5] Verification Check:"
ls -lh "$PAM_DIR/pam_opentap.so"
grep "pam_opentap.so" "$PAM_TARGET_FILE"

echo "============================================================================"
echo "  [SUCCESS] pam_opentap.so installed and configured successfully!"
echo "  Note: If opentapd daemon is stopped, authentication gracefully falls back"
echo "  to your standard password prompt without locking you out."
echo "============================================================================"
