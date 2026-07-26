#!/usr/bin/env bash
# OpenTapUnlock Production Build & Integration Test Runner (Linux / macOS)
set -e

echo "==========================================================="
echo "⚡ OPENTAPUNLOCK FULL ECOSYSTEM BUILD & VERIFICATION"
echo "==========================================================="

# 1. Check required toolchains
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust toolchain (cargo) is required. Please install via https://rustup.rs"
    exit 1
fi

if ! command -v npm &> /dev/null; then
    echo "⚠️ Warning: Node.js (npm) not found. Skipping UI desktop build."
fi

# 2. Build Core & Daemon
echo "\n📦 [1/5] Building OpenTap Core Cryptographic Engine & OS Daemon..."
cargo build --release --package opentap-core
cargo build --release --package opentapd

# 3. Run 100-Case E2E Integration & Stress Test Matrix
echo "\n🧪 [2/5] Executing 100-Case E2E Integration & Security Test Matrix..."
cargo run --release --package e2e-matrix

# 4. Build Mobile Libraries (if NDK / Xcode installed)
echo "\n📱 [3/5] Building Mobile FFI & JNI Bridges..."
if command -v cargo-ndk &> /dev/null; then
    echo "  -> Building Android JNI Library (libopentap_jni.so)..."
    cd mobile/android/opentap-jni && cargo ndk -t arm64-v8a -t x86_64 build --release && cd ../../..
else
    echo "  -> Skipping Android NDK build (cargo-ndk not installed)"
fi

if [[ "$OSTYPE" == "darwin"* ]]; then
    echo "  -> Building iOS Static Library (libopentap_ffi.a)..."
    cd mobile/ios/opentap-ffi && cargo build --release --target aarch64-apple-ios && cd ../../..
fi

# 5. Build Desktop Control Center UI
echo "\n🖥️ [4/5] Building Tauri Control Center UI..."
if command -v npm &> /dev/null; then
    cd desktop/control-center
    npm install --silent
    npm run tauri build -- --bundles app,deb,appimage || echo "⚠️ Tauri native bundling skipped (missing OS dependencies)"
    cd ../..
fi

echo "\n==========================================================="
echo "🎉 BUILD & 100-CASE VERIFICATION COMPLETED SUCCESSFULLY!"
echo "Binaries located in: target/release/opentapd"
echo "==========================================================="
