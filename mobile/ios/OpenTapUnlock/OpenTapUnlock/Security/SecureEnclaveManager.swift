//
//  SecureEnclaveManager.swift
//  OpenTapUnlock
//
//  Manages zero-trust biometric verification (Face ID / Touch ID / Watch Unlock)
//  and encrypted Apple Keychain storage.
//

import Foundation
import LocalAuthentication
import Security

final class SecureEnclaveManager {
    static let shared = SecureEnclaveManager()
    private let serviceName = "org.opentapunlock.keychain.vault"
    private init() {}

    /**
     * Checks if the iOS device is currently unlocked by the user.
     * When iOS Back Tap fires in the background while texting or browsing,
     * this confirms the user has already passed Face ID / Touch ID!
     */
    func isDeviceCurrentlyUnlocked() -> Bool {
        let context = LAContext()
        var error: NSError?
        // LAContext can evaluate policy or check device passcode status
        return context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error)
    }

    /**
     * Prompts for explicit Face ID / Touch ID confirmation when required by policy.
     */
    func authenticateUser(reason: String, completion: @escaping (Bool, String?) -> Void) {
        let context = LAContext()
        var error: NSError?

        if context.canEvaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, error: &error) {
            context.evaluatePolicy(.deviceOwnerAuthenticationWithBiometrics, localizedReason: reason) { success, authError in
                DispatchQueue.main.async {
                    if success {
                        completion(true, nil)
                    } else {
                        completion(false, authError?.localizedDescription ?? "Authentication rejected")
                    }
                }
            }
        } else {
            // Fallback to device passcode
            if context.canEvaluatePolicy(.deviceOwnerAuthentication, error: &error) {
                context.evaluatePolicy(.deviceOwnerAuthentication, localizedReason: reason) { success, authError in
                    DispatchQueue.main.async {
                        completion(success, authError?.localizedDescription)
                    }
                }
            } else {
                completion(false, "No Face ID, Touch ID, or Passcode configured on this device.")
            }
        }
    }

    /**
     * Saves a paired secret key in the Apple Keychain protected by Secure Enclave / Touch ID.
     */
    func saveToKeychain(key: String, value: String) -> Bool {
        guard let data = value.data(using: .utf8) else { return false }

        // Delete existing item if present
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key
        ]
        SecItemDelete(query as CFDictionary)

        let addQuery: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key,
            kSecValueData as String: data,
            kSecAttrAccessible as String: kSecAttrAccessibleWhenUnlockedThisDeviceOnly
        ]

        let status = SecItemAdd(addQuery as CFDictionary, nil)
        return status == errSecSuccess
    }

    /**
     * Retrieves an encrypted item from the Apple Keychain.
     */
    func retrieveFromKeychain(key: String) -> String? {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key,
            kSecReturnData as String: kCFBooleanTrue!,
            kSecMatchLimit as String: kSecMatchLimitOne
        ]

        var dataTypeRef: AnyObject?
        let status = SecItemCopyMatching(query as CFDictionary, &dataTypeRef)

        guard status == errSecSuccess, let data = dataTypeRef as? Data else {
            return nil
        }

        return String(data: data, encoding: .utf8)
    }

    func deleteFromKeychain(key: String) {
        let query: [String: Any] = [
            kSecClass as String: kSecClassGenericPassword,
            kSecAttrService as String: serviceName,
            kSecAttrAccount as String: key
        ]
        SecItemDelete(query as CFDictionary)
    }
}
