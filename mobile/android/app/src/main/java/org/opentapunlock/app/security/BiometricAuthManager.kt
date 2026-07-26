package org.opentapunlock.app.security

import android.app.KeyguardManager
import android.content.Context
import android.util.Log
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity

interface BiometricAuthCallback {
    fun onAuthSuccess()
    fun onAuthFailure(errorMessage: String)
}

/**
 * Manages zero-trust biometric verification (Fingerprint / Face Unlock / Device PIN)
 * before authorizing desktop unlock payload transmission.
 */
class BiometricAuthManager(private val context: Context) {
    companion object {
        private const val TAG = "BiometricAuthManager"
    }

    private val keyguardManager = context.getSystemService(Context.KEYGUARD_SERVICE) as KeyguardManager

    /**
     * Checks if the device lock screen is currently unlocked by the user.
     * When the user performs a Triple Tap while texting or browsing in another app,
     * this confirms they have already passed FaceID/Fingerprint unlock on the phone!
     */
    fun isDeviceCurrentlyUnlocked(): Boolean {
        val isLocked = keyguardManager.isKeyguardLocked
        Log.d(TAG, "Device lockguard status - isLocked: $isLocked")
        return !isLocked && keyguardManager.isDeviceSecure
    }

    /**
     * Prompts for explicit Biometric confirmation if requested or required by policy.
     */
    fun requestBiometricPrompt(activity: FragmentActivity, title: String, callback: BiometricAuthCallback) {
        val biometricManager = BiometricManager.from(context)
        when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL)) {
            BiometricManager.BIOMETRIC_SUCCESS -> {
                val executor = ContextCompat.getMainExecutor(context)
                val promptInfo = BiometricPrompt.PromptInfo.Builder()
                    .setTitle(title)
                    .setSubtitle("Confirm your identity to unlock your laptop")
                    .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL)
                    .build()

                val biometricPrompt = BiometricPrompt(activity, executor, object : BiometricPrompt.AuthenticationCallback() {
                    override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                        super.onAuthenticationSucceeded(result)
                        Log.i(TAG, "Biometric authentication succeeded!")
                        callback.onAuthSuccess()
                    }

                    override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                        super.onAuthenticationError(errorCode, errString)
                        Log.e(TAG, "Biometric error [$errorCode]: $errString")
                        callback.onAuthFailure(errString.toString())
                    }
                })

                biometricPrompt.authenticate(promptInfo)
            }
            else -> {
                Log.w(TAG, "Biometric hardware unavailable or not enrolled. Checking device secure PIN fallback...")
                if (keyguardManager.isDeviceSecure && !keyguardManager.isKeyguardLocked) {
                    callback.onAuthSuccess()
                } else {
                    callback.onAuthFailure("Please set up Lock Screen security (Fingerprint/PIN) in Android Settings.")
                }
            }
        }
    }
}
