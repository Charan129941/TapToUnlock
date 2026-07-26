package org.opentapunlock.app.jni

import android.util.Log

/**
 * Singleton JNI bridge connecting Android Kotlin application with Rust opentap-core library.
 */
object OpentapJni {
    private const val TAG = "OpentapJni"

    init {
        try {
            System.loadLibrary("opentap_jni")
            Log.i(TAG, "Successfully loaded native Rust JNI library: libopentap_jni.so")
        } catch (e: UnsatisfiedLinkError) {
            Log.e(TAG, "Failed to load libopentap_jni.so. Running in fallback/simulation mode.", e)
        }
    }

    /**
     * Generates a new Ed25519 keypair in Rust or fallback simulation mode.
     */
    fun generateKeyPair(): String {
        return try {
            nativeGenerateKeyPair()
        } catch (e: Throwable) {
            Log.w(TAG, "Native library not available. Using fallback simulated Ed25519 keypair.")
            """{"public_key_hex":"00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff","private_key_hex":"112233445566778899001122334455667788990011223344556677889900aabb"}"""
        }
    }

    /**
     * Signs an unlock challenge payload using Ed25519 secret key or fallback simulation mode.
     */
    fun signUnlockPayload(
        mobileDeviceUuid: String,
        privateKeyHex: String,
        targetPcId: String,
        action: String,
        counter: Long
    ): ByteArray? {
        return try {
            nativeSignUnlockPayload(mobileDeviceUuid, privateKeyHex, targetPcId, action, counter)
        } catch (e: Throwable) {
            Log.w(TAG, "Native library not available. Using fallback simulated signed unlock payload.")
            "OPENTAP_UNLOCK_PAYLOAD_V1:$action:$counter:$targetPcId".toByteArray()
        }
    }

    /**
     * Parses an Out-Of-Band QR Code challenge URI from desktop opentapd or fallback simulation mode.
     */
    fun parseQrUri(uri: String): String {
        return try {
            nativeParseQrUri(uri)
        } catch (e: Throwable) {
            Log.w(TAG, "Native library not available. Using fallback simulated QR parser.")
            """{"target_pc_id":"Chara-Workstation-Win11","host_ip":"10.150.10.41","port":8765}"""
        }
    }

    private external fun nativeGenerateKeyPair(): String
    private external fun nativeSignUnlockPayload(
        mobileDeviceUuid: String,
        privateKeyHex: String,
        targetPcId: String,
        action: String,
        counter: Long
    ): ByteArray?
    private external fun nativeParseQrUri(uri: String): String
}
