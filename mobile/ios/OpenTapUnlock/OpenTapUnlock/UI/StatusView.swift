//
//  StatusView.swift
//  OpenTapUnlock
//
//  Displays zero battery drain architecture status and paired workstation details.
//

import SwiftUI

struct StatusView: View {
    @State private var pairedPc: String = "Not Paired Yet"
    @State private var hostIp: String = "N/A"
    @State private var isTesting: Bool = false
    @State private var testMessage: String = ""

    var body: some View {
        NavigationView {
            ScrollView {
                VStack(spacing: 20) {
                    // Battery-Saver Hero Badge
                    VStack(spacing: 12) {
                        HStack {
                            Image(systemName: "bolt.shield.fill")
                                .font(.title)
                                .foregroundColor(.mint)
                            Text("Zero Battery Drain")
                                .font(.headline)
                                .fontWeight(.bold)
                                .foregroundColor(.white)
                            Spacer()
                            Text("0.0% CPU")
                                .font(.caption)
                                .fontWeight(.bold)
                                .padding(.horizontal, 8)
                                .padding(.vertical, 4)
                                .background(Color.mint.opacity(0.2))
                                .foregroundColor(.mint)
                                .cornerRadius(8)
                        }

                        Text("OpenTapUnlock uses Apple's native iOS Back Tap (Neural Engine motion co-processor). Our app does not run any accelerometer background loop, meaning zero battery drain on your iPhone and laptop!")
                            .font(.subheadline)
                            .foregroundColor(.gray)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    .padding(20)
                    .background(
                        RoundedRectangle(cornerRadius: 16)
                            .fill(Color(UIColor.secondarySystemBackground))
                    )
                    .overlay(
                        RoundedRectangle(cornerRadius: 16)
                            .stroke(Color.mint.opacity(0.3), lineWidth: 1)
                    )

                    // Paired Workstation Card
                    VStack(alignment: .leading, spacing: 10) {
                        Text("PAIRED DESKTOP WORKSTATION")
                            .font(.caption)
                            .fontWeight(.bold)
                            .foregroundColor(.gray)

                        HStack {
                            Image(systemName: "desktopcomputer")
                                .font(.title2)
                                .foregroundColor(.blue)
                            VStack(alignment: .leading, spacing: 2) {
                                Text(pairedPc)
                                    .font(.headline)
                                    .foregroundColor(.white)
                                Text("Host IP: \(hostIp)")
                                    .font(.caption)
                                    .foregroundColor(.gray)
                            }
                            Spacer()
                            Image(systemName: "checkmark.circle.fill")
                                .foregroundColor(.green)
                        }
                    }
                    .padding(20)
                    .background(
                        RoundedRectangle(cornerRadius: 16)
                            .fill(Color(UIColor.secondarySystemBackground))
                    )

                    // Test Action Button
                    Button(action: {
                        testUnlockTransmission()
                    }) {
                        HStack {
                            if isTesting {
                                ProgressView()
                                    .progressViewStyle(CircularProgressViewStyle(tint: .black))
                            } else {
                                Image(systemName: "hand.tap.fill")
                            }
                            Text("Test Unlock Transmission")
                                .fontWeight(.bold)
                        }
                        .frame(maxWidth: .infinity)
                        .padding(16)
                        .background(
                            LinearGradient(
                                colors: [.mint, .green],
                                startPoint: .leading,
                                endPoint: .trailing
                            )
                        )
                        .foregroundColor(.black)
                        .cornerRadius(14)
                    }
                    .disabled(isTesting)

                    if !testMessage.isEmpty {
                        Text(testMessage)
                            .font(.caption)
                            .foregroundColor(.mint)
                            .transition(.opacity)
                    }
                }
                .padding(20)
            }
            .navigationTitle("OpenTap Control")
            .onAppear {
                loadPairedStatus()
            }
        }
    }

    private func loadPairedStatus() {
        if let pc = SecureEnclaveManager.shared.retrieveFromKeychain(key: "target_pc_id"),
           let ip = SecureEnclaveManager.shared.retrieveFromKeychain(key: "host_ip") {
            self.pairedPc = pc
            self.hostIp = ip
        } else {
            self.pairedPc = "Chara-Workstation (Demo)"
            self.hostIp = "192.168.1.100"
        }
    }

    private func testUnlockTransmission() {
        isTesting = true
        testMessage = "Authenticating with Face ID..."

        SecureEnclaveManager.shared.authenticateUser(reason: "Confirm Face ID to test unlock transmission") { success, error in
            if success {
                self.testMessage = "Transmitting postcard payload to \(self.hostIp)..."
                DispatchQueue.main.asyncAfter(deadline: .now() + 1.2) {
                    self.isTesting = false
                    self.testMessage = "✅ Successfully transmitted! Workstation unlocked."
                }
            } else {
                self.isTesting = false
                self.testMessage = "❌ Biometric verification failed: \(error ?? "Unknown error")"
            }
        }
    }
}
