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
     * Generates a new Ed25519 keypair in Rust.
     * @return JSON string containing {"public_key_hex": "...", "private_key_hex": "..."}
     */
    external fun generateKeyPair(): String

    /**
     * Signs an unlock challenge payload using Ed25519 secret key and postcard serialization.
     */
    external fun signUnlockPayload(
        mobileDeviceUuid: String,
        privateKeyHex: String,
        targetPcId: String,
        action: String,
        counter: Long
    ): ByteArray?

    /**
     * Parses an Out-Of-Band QR Code challenge URI from desktop opentapd.
     * @return JSON representation of pairing parameters.
     */
    external fun parseQrUri(uri: String): String
}
