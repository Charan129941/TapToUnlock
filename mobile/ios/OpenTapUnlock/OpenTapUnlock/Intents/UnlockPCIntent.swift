//
//  UnlockPCIntent.swift
//  OpenTapUnlock
//
//  Apple AppIntent & Siri Shortcut handler enabling ZERO BATTERY DRAIN Back Tap automation.
//  Linked directly to iOS Settings -> Accessibility -> Touch -> Back Tap -> Triple Tap!
//

import Foundation
import AppIntents
import UIKit

@available(iOS 16.0, *)
struct UnlockPCIntent: AppIntent {
    static var title: LocalizedStringResource = "Unlock My Workstation"
    static var description = IntentDescription("Instantly sends an Ed25519-signed biometric unlock token to your paired laptop when you perform a Back Tap.")
    static var openAppWhenRun: Bool = false

    @Parameter(title: "Action Shortcut", default: "UNLOCK")
    var actionParam: String

    @MainActor
    func perform() async throws -> some IntentResult {
        // 1. Trigger haptic bump so user feels physical feedback in hand!
        let generator = UIImpactFeedbackGenerator(style: .heavy)
        generator.prepare()
        generator.impactOccurred()

        // 2. Check zero-trust lock screen status: ensure phone is awake & authenticated!
        guard SecureEnclaveManager.shared.isDeviceCurrentlyUnlocked() else {
            throw IntentError.message("Phone is currently locked! Please unlock Face ID first.")
        }

        // 3. Load paired workstation credentials from Keychain
        guard let hostIp = SecureEnclaveManager.shared.retrieveFromKeychain(key: "host_ip"),
              let portStr = SecureEnclaveManager.shared.retrieveFromKeychain(key: "tls_port"),
              let port = UInt16(portStr),
              let pcId = SecureEnclaveManager.shared.retrieveFromKeychain(key: "target_pc_id"),
              let mobileUuid = SecureEnclaveManager.shared.retrieveFromKeychain(key: "mobile_uuid"),
              let privHex = SecureEnclaveManager.shared.retrieveFromKeychain(key: "private_key_hex") else {
            throw IntentError.message("No paired workstation found in Keychain! Please pair in OpenTapUnlock first.")
        }

        // 4. Sign payload in Rust using hardware-backed secret key
        let counter = UInt64(Date().timeIntervalSince1970 * 1000)
        let signResult = OpentapFfiBridge.shared.signUnlockPayload(
            mobileUuid: mobileUuid,
            privateKeyHex: privHex,
            targetPcId: pcId,
            action: actionParam,
            counter: counter
        )

        switch signResult {
        case .success(let payloadData):
            // 5. Transmit packet over Wi-Fi / BLE without opening any UI!
            return try await withCheckedThrowingContinuation { continuation in
                UnlockTransmitter.shared.transmitOverWifi(hostIp: hostIp, port: port, payload: payloadData) { success, errorMsg in
                    if success {
                        continuation.resume(returning: .result())
                    } else {
                        // Fallback to BLE transmission
                        UnlockTransmitter.shared.transmitOverBle(serviceUuid: "6f70656e-7461-702d-756e-6c6f636b3031", payload: payloadData) { bleSuccess, bleErr in
                            if bleSuccess {
                                continuation.resume(returning: .result())
                            } else {
                                continuation.resume(throwing: IntentError.message(bleErr ?? "Transmission failed"))
                            }
                        }
                    }
                }
            }
        case .failure(let error):
            throw IntentError.message("Cryptographic signing failed: \(error.localizedDescription)")
        }
    }
}

enum IntentError: Swift.Error, CustomLocalizedStringResourceConvertible {
    case message(String)

    var localizedStringResource: LocalizedStringResource {
        switch self {
        case .message(let msg): return LocalizedStringResource(stringLiteral: msg)
        }
    }
}

@available(iOS 16.0, *)
struct OpenTapShortcutsProvider: AppShortcutsProvider {
    static var appShortcuts: [AppShortcut] {
        AppShortcut(
            intent: UnlockPCIntent(),
            phrases: [
                "Unlock my workstation with \(.applicationName)",
                "Unlock my PC with \(.applicationName)",
                "Open my laptop with \(.applicationName)"
            ],
            shortTitle: "Unlock PC",
            systemImageName: "lock.open.fill"
        )
    }
}
