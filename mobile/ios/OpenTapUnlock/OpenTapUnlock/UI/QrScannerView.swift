//
//  QrScannerView.swift
//  OpenTapUnlock
//
//  Camera scanner UI for pairing with desktop opentapd --pair terminal QR code.
//

import SwiftUI

struct QrScannerView: View {
    @State private var isScanning: Bool = false
    @State private var statusMessage: String = "Align the terminal QR code inside the viewfinder"

    var body: some View {
        NavigationView {
            VStack(spacing: 24) {
                Text("Pair Workstation")
                    .font(.title2)
                    .fontWeight(.bold)
                    .foregroundColor(.white)

                Text("Run `sudo opentapd --pair` on your laptop and scan the terminal QR code to establish a zero-trust cryptographic link.")
                    .font(.subheadline)
                    .foregroundColor(.gray)
                    .multilineTextAlignment(.center)
                    .padding(.horizontal, 20)

                // Scanner Viewfinder Simulation Box
                ZStack {
                    RoundedRectangle(cornerRadius: 20)
                        .fill(Color(UIColor.secondarySystemBackground))
                        .frame(height: 280)

                    VStack(spacing: 16) {
                        Image(systemName: "qrcode.viewfinder")
                            .font(.system(size: 70))
                            .foregroundColor(.mint)
                        Text("[ Camera Viewfinder Active ]")
                            .font(.caption)
                            .foregroundColor(.gray)
                    }

                    // Targeted corner brackets for aesthetics
                    RoundedRectangle(cornerRadius: 20)
                        .stroke(Color.mint, style: StrokeStyle(lineWidth: 2, dash: [20, 10]))
                        .frame(height: 280)
                }
                .padding(.horizontal, 20)

                Text(statusMessage)
                    .font(.caption)
                    .foregroundColor(.mint)

                Spacer()

                Button(action: {
                    simulateQrPairing()
                }) {
                    Text("Simulate Scan & Save to Keychain")
                        .fontWeight(.bold)
                        .frame(maxWidth: .infinity)
                        .padding(16)
                        .background(Color.blue)
                        .foregroundColor(.white)
                        .cornerRadius(14)
                }
                .padding(20)
            }
            .navigationTitle("Pairing")
        }
    }

    private func simulateQrPairing() {
        statusMessage = "Generating Ed25519 keypair..."
        
        let keypairRes = OpentapFfiBridge.shared.generateKeyPair()
        switch keypairRes {
        case .success(let keys):
            // Save mock parameters and private key to Apple Keychain
            _ = SecureEnclaveManager.shared.saveToKeychain(key: "target_pc_id", value: "Chara-MacBook-Pro")
            _ = SecureEnclaveManager.shared.saveToKeychain(key: "host_ip", value: "192.168.1.100")
            _ = SecureEnclaveManager.shared.saveToKeychain(key: "tls_port", value: "8765")
            _ = SecureEnclaveManager.shared.saveToKeychain(key: "mobile_uuid", value: "iphone-15-pro-uuid")
            _ = SecureEnclaveManager.shared.saveToKeychain(key: "private_key_hex", value: keys.privateKeyHex)
            
            statusMessage = "✅ Successfully paired with Chara-MacBook-Pro! Keychain secured."
        case .failure(let err):
            statusMessage = "❌ Key generation failed: \(err.localizedDescription)"
        }
    }
}
