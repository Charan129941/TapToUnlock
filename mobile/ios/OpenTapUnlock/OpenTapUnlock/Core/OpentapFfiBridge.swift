//
//  OpentapFfiBridge.swift
//  OpenTapUnlock
//
//  Swift wrapper around native Rust C-ABI library (libopentap_ffi.a).
//

import Foundation

enum FfiError: Error, LocalizedError {
    case bufferOverflow
    case invalidHex
    case serializationFailed
    case nativeError(code: Int32)

    var errorDescription: String? {
        switch self {
        case .bufferOverflow: return "Native buffer capacity exceeded."
        case .invalidHex: return "Invalid hex encoded cryptographic key."
        case .serializationFailed: return "Postcard binary serialization failed."
        case .nativeError(let code): return "Rust native execution error code: \(code)"
        }
    }
}

final class OpentapFfiBridge {
    static let shared = OpentapFfiBridge()
    private init() {}

    /**
     * Generates an Ed25519 keypair via Rust.
     */
    func generateKeyPair() -> Result<(publicKeyHex: String, privateKeyHex: String), FfiError> {
        var pubBuf = [CChar](repeating: 0, count: 128)
        var privBuf = [CChar](repeating: 0, count: 128)

        let status = opentap_ffi_generate_keypair(&pubBuf, pubBuf.count, &privBuf, privBuf.count)
        guard status == 0 else {
            return .failure(.nativeError(code: status))
        }

        let pubStr = String(cString: pubBuf)
        let privStr = String(cString: privBuf)
        return .success((pubStr, privStr))
    }

    /**
     * Signs an unlock payload using Ed25519 secret key and postcard serialization.
     */
    func signUnlockPayload(
        mobileUuid: String,
        privateKeyHex: String,
        targetPcId: String,
        action: String,
        counter: UInt64
    ) -> Result<Data, FfiError> {
        var outBuf = [UInt8](repeating: 0, count: 2048)
        var actualLen: Int = 0

        let status = mobileUuid.withCString { uuidPtr in
            privateKeyHex.withCString { privPtr in
                targetPcId.withCString { pcPtr in
                    action.withCString { actPtr in
                        opentap_ffi_sign_payload(
                            uuidPtr,
                            privPtr,
                            pcPtr,
                            actPtr,
                            counter,
                            &outBuf,
                            outBuf.count,
                            &actualLen
                        )
                    }
                }
            }
        }

        guard status == 0 else {
            return .failure(.nativeError(code: status))
        }

        let packetData = Data(bytes: outBuf, count: actualLen)
        return .success(packetData)
    }

    /**
     * Parses an Out-Of-Band QR Code challenge URI from desktop opentapd.
     */
    func parseQrUri(uri: String) -> Result<[String: Any], FfiError> {
        var jsonBuf = [CChar](repeating: 0, count: 4096)

        let status = uri.withCString { uriPtr in
            opentap_ffi_parse_qr_uri(uriPtr, &jsonBuf, jsonBuf.count)
        }

        guard status == 0 else {
            return .failure(.nativeError(code: status))
        }

        let jsonStr = String(cString: jsonBuf)
        guard let data = jsonStr.data(using: .utf8),
              let dict = try? JSONSerialization.jsonObject(with: data, options: []) as? [String: Any] else {
            return .failure(.serializationFailed)
        }

        return .success(dict)
    }
}
