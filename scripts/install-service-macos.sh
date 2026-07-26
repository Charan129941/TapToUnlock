#!/usr/bin/env bash
# ==============================================================================
# OpenTapUnlock: macOS launchd Daemon Installer for opentapd
# ==============================================================================
set -e

if [ "$EUID" -ne 0 ]; then
  echo "[ERROR] This script must be run as root (sudo ./install-service-macos.sh)."
  exit 1
fi

echo "============================================================================"
echo "          OpenTapUnlock macOS launchd Daemon Installer (opentapd)           "
echo "============================================================================"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SCRIPT_DIR/../desktop/daemon"
TARGET_BIN="/usr/local/bin/opentapd"
PLIST_FILE="/Library/LaunchDaemons/org.opentapunlock.daemon.plist"

echo "[1/4] Building opentapd release binary..."
if ! command -v cargo &> /dev/null; then
  echo "[ERROR] Cargo/Rust toolchain not found."
  exit 1
fi

cargo build --release --manifest-path "$DAEMON_DIR/Cargo.toml"

BUILT_BIN="$DAEMON_DIR/../../target/release/opentapd"
if [ ! -f "$BUILT_BIN" ]; then
  echo "[ERROR] Build failed: $BUILT_BIN not found."
  exit 1
fi

echo "[2/4] Installing binary to $TARGET_BIN..."
mkdir -p /usr/local/bin
cp -f "$BUILT_BIN" "$TARGET_BIN"
chmod 755 "$TARGET_BIN"
chown root:wheel "$TARGET_BIN"

echo "[3/4] Creating LaunchDaemon plist at $PLIST_FILE..."
cat << 'EOF' > "$PLIST_FILE"
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>org.opentapunlock.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/opentapd</string>
        <string>--daemon</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardErrorPath</key>
    <string>/var/log/opentapd.err.log</string>
    <key>StandardOutPath</key>
    <string>/var/log/opentapd.out.log</string>
</dict>
</plist>
EOF

chmod 644 "$PLIST_FILE"
chown root:wheel "$PLIST_FILE"

echo "[4/4] Loading launchd daemon..."
if command -v launchctl &> /dev/null; then
  launchctl unload "$PLIST_FILE" 2>/dev/null || true
  launchctl load -w "$PLIST_FILE"
  echo "      -> LaunchDaemon loaded and started!"
else
  echo "      -> [NOTE] launchctl not detected (not on native macOS host)."
fi

echo "============================================================================"
echo "  [SUCCESS] opentapd installed as macOS background LaunchDaemon!"
echo "  To pair your mobile phone, run: sudo opentapd --pair"
echo "============================================================================"
