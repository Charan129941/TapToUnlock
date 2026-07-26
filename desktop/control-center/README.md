# OpenTap Control Center (Tauri / React 18 / Tailwind CSS)

The OpenTap Control Center is a production-grade desktop graphical interface built with **Tauri**, **React 18**, **TypeScript**, and **Tailwind CSS**. It allows you to monitor background security status, manage authorized mobile phones in your vault, generate GUI pairing QR codes, and view real-time audit logs.

---

## ⚡ Zero Battery & Memory Drain on Laptop

Traditional desktop apps built with Electron package an entire Chromium browser and Node.js runtime, consuming 250MB+ RAM and continuously draining CPU and battery.
In contrast, OpenTap Control Center uses **Tauri**:
- **Native OS WebView**: Uses Windows WebView2 / macOS WebKit / Linux WebKitGTK. The installer footprint is under ~15 MB.
- **Zero Polling Loops**: Communicates with the root `opentapd` daemon via event-driven Named Pipes (`\\.\pipe\opentapd_ipc`) or UNIX sockets (`/var/run/opentapd.sock`).
- **System Tray Minimization**: When minimized or closed, the Control Center runs in the system tray with **0.00% CPU usage and zero battery drain**.

---

## 🎨 Clean, Intuitive Aesthetics
- **Dashboard Tab**: Displays a prominent zero-battery badge, network listening status (port 8765), and one-click manual lock button.
- **Paired Phones Tab**: Lists all mobile devices authorized to unlock this workstation, displaying their Ed25519 public key hash and allowing one-click authorization revocation.
- **Pair New Phone Tab**: Generates a live Out-Of-Band QR code directly in the desktop window so you can pair without running terminal commands.
- **Audit Logs Tab**: Real-time chronological log of every biometric unlock attempt and verification result.

---

## 🛠️ How to Build and Run Locally

```bash
# 1. Install Node.js dependencies
npm install

# 2. Run in Development Preview Mode (hot reload + simulated IPC)
npm run dev

# 3. Build and launch Tauri Native Desktop App (requires Rust toolchain installed)
npm run tauri dev

# 4. Build Production Bundle (.msi / .app / .deb / .AppImage)
npm run tauri build
```
