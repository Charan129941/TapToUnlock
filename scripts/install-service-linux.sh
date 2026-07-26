#!/usr/bin/env bash
# ==============================================================================
# OpenTapUnlock: Linux systemd Service Installer for opentapd
# ==============================================================================
set -e

if [ "$EUID" -ne 0 ]; then
  echo "[ERROR] This script must be run as root (sudo ./install-service-linux.sh)."
  exit 1
fi

echo "============================================================================"
echo "          OpenTapUnlock Linux systemd Service Installer (opentapd)          "
echo "============================================================================"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DAEMON_DIR="$SCRIPT_DIR/../desktop/daemon"
TARGET_BIN="/usr/local/bin/opentapd"
SERVICE_FILE="/etc/systemd/system/opentapd.service"

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
cp -f "$BUILT_BIN" "$TARGET_BIN"
chmod 755 "$TARGET_BIN"
chown root:root "$TARGET_BIN"

echo "[3/4] Creating systemd unit file at $SERVICE_FILE..."
cat << 'EOF' > "$SERVICE_FILE"
[Unit]
Description=OpenTapUnlock Zero-Trust Desktop Authentication Daemon
After=network.target bluetooth.target
Wants=bluetooth.target

[Service]
Type=simple
ExecStart=/usr/local/bin/opentapd --daemon
Restart=always
RestartSec=3
User=root
Group=root
ProtectSystem=full
PrivateTmp=true
AmbientCapabilities=CAP_NET_ADMIN CAP_NET_BIND_SERVICE CAP_NET_RAW

[Install]
WantedBy=multi-user.target
EOF

chmod 644 "$SERVICE_FILE"

echo "[4/4] Reloading systemd daemon and enabling opentapd service..."
if command -v systemctl &> /dev/null; then
  systemctl daemon-reload
  systemctl enable opentapd.service
  systemctl restart opentapd.service
  echo "      -> Service started! Status:"
  systemctl status opentapd.service --no-pager || true
else
  echo "      -> [NOTE] systemctl not detected (not on native systemd Linux host)."
fi

echo "============================================================================"
echo "  [SUCCESS] opentapd installed and running as background system service!"
echo "  To pair a mobile device, run: sudo opentapd --pair"
echo "============================================================================"
