package org.opentapunlock.app.service

import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.app.Service
import android.content.Context
import android.content.Intent
import android.hardware.SensorManager
import android.os.Build
import android.os.IBinder
import android.os.VibrationEffect
import android.os.Vibrator
import android.os.VibratorManager
import android.util.Log
import androidx.core.app.NotificationCompat
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.SupervisorJob
import kotlinx.coroutines.launch
import org.opentapunlock.app.gesture.GestureAction
import org.opentapunlock.app.gesture.GestureConfigManager
import org.opentapunlock.app.gesture.GestureDetectionListener
import org.opentapunlock.app.gesture.GestureType
import org.opentapunlock.app.gesture.TapDetector
import org.opentapunlock.app.jni.OpentapJni
import org.opentapunlock.app.network.UnlockTransmitter
import org.opentapunlock.app.security.BiometricAuthManager
import org.opentapunlock.app.ui.MainActivity

/**
 * Android Foreground Service that runs continuously in the background.
 * Monitors accelerometer impulses for Double, Triple, or Long Taps while you text,
 * browse, or use other apps. Unlocks your laptop instantly without opening any app UI!
 */
class TapBackgroundService : Service(), GestureDetectionListener {
    companion object {
        private const val TAG = "TapBackgroundService"
        private const val NOTIFICATION_ID = 3034
        private const val CHANNEL_ID = "opentap_bg_monitoring"
    }

    private lateinit var sensorManager: SensorManager
    private lateinit var tapDetector: TapDetector
    private lateinit var gestureConfigManager: GestureConfigManager
    private lateinit var biometricAuthManager: BiometricAuthManager
    private val serviceScope = CoroutineScope(Dispatchers.Default + SupervisorJob())
    private var counter: Long = 1

    override fun onCreate() {
        super.onCreate()
        Log.i(TAG, "Initializing OpenTap Background Sensor Monitoring Service...")
        
        sensorManager = getSystemService(Context.SENSOR_SERVICE) as SensorManager
        tapDetector = TapDetector(this)
        gestureConfigManager = GestureConfigManager(this)
        biometricAuthManager = BiometricAuthManager(this)

        try {
            createNotificationChannel()
            startForeground(NOTIFICATION_ID, buildForegroundNotification())
        } catch (e: Throwable) {
            Log.w(TAG, "startForeground deferred or permission pending: ${e.message}")
        }

        try {
            tapDetector.register(sensorManager)
        } catch (e: Throwable) {
            Log.w(TAG, "Sensor registration deferred: ${e.message}")
        }
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        Log.i(TAG, "TapBackgroundService started and monitoring taps system-wide.")
        return START_STICKY
    }

    override fun onBind(intent: Intent?): IBinder? = null

    override fun onDestroy() {
        Log.i(TAG, "Destroying TapBackgroundService and unregistering sensors...")
        tapDetector.unregister(sensorManager)
        super.onDestroy()
    }

    override fun onGestureDetected(gesture: GestureType) {
        Log.i(TAG, "System-wide background gesture triggered: ${gesture.displayName}")
        
        // 1. Trigger haptic vibration confirmation so user feels tactile feedback in hand!
        triggerHapticConfirmation()

        // 2. Check if phone is currently unlocked by the user (Fingerprint / FaceID / PIN)
        if (!biometricAuthManager.isDeviceCurrentlyUnlocked()) {
            Log.w(TAG, "Phone is currently locked! Please unlock phone first before tapping to unlock PC.")
            return
        }

        // 3. Lookup user's customized action mapping for this gesture
        val action = gestureConfigManager.getAction(gesture)
        if (action == GestureAction.NONE) {
            Log.i(TAG, "Gesture ${gesture.name} is mapped to NONE. No action taken.")
            return
        }

        Log.i(TAG, "Executing customized action: ${action.displayName} (${action.rustAction})")

        // 4. Execute action over wireless link without interrupting current phone app!
        serviceScope.launch {
            executeWirelessCommand(action)
        }
    }

    private suspend fun executeWirelessCommand(action: GestureAction) {
        val prefs = getSharedPreferences("opentap_vault", Context.MODE_PRIVATE)
        val hostIp = prefs.getString("host_ip", "10.150.10.41") ?: "10.150.10.41"
        val port = prefs.getInt("tls_port", 8765)
        val targetPcId = prefs.getString("target_pc_id", "chara") ?: "chara"
        val mobileUuid = prefs.getString("mobile_uuid", "mobile-device-uuid") ?: "mobile-device-uuid"
        val privateKeyHex = prefs.getString("private_key_hex", "") ?: ""
        val activeKeyHex = if (privateKeyHex.isEmpty()) "112233445566778899001122334455667788990011223344556677889900aabb" else privateKeyHex

        counter++
        val packetBytes = OpentapJni.signUnlockPayload(
            mobileUuid,
            activeKeyHex,
            targetPcId,
            action.rustAction,
            counter
        )

        if (packetBytes != null) {
            val success = UnlockTransmitter.transmitOverWifi(hostIp, port, packetBytes)
            if (!success) {
                // Try fallback BLE GATT transmission
                UnlockTransmitter.transmitOverBle("6f70656e-7461-702d-756e-6c6f636b3031", packetBytes)
            }
        } else {
            Log.e(TAG, "Native Rust JNI failed to sign unlock payload packet.")
        }
    }

    private fun triggerHapticConfirmation() {
        try {
            val vibrator = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.S) {
                val mgr = getSystemService(Context.VIBRATOR_MANAGER_SERVICE) as VibratorManager
                mgr.defaultVibrator
            } else {
                @Suppress("DEPRECATION")
                getSystemService(Context.VIBRATOR_SERVICE) as Vibrator
            }

            if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
                vibrator.vibrate(VibrationEffect.createOneShot(140, VibrationEffect.DEFAULT_AMPLITUDE))
            } else {
                @Suppress("DEPRECATION")
                vibrator.vibrate(140)
            }
        } catch (e: Exception) {
            Log.w(TAG, "Haptic vibration failed: ${e.message}")
        }
    }

    private fun createNotificationChannel() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            val channel = NotificationChannel(
                CHANNEL_ID,
                "OpenTap Background Monitoring",
                NotificationManager.IMPORTANCE_LOW
            ).apply {
                description = "Monitors Back Tap gestures while phone screen is unlocked."
            }
            val manager = getSystemService(NotificationManager::class.java)
            manager?.createNotificationChannel(channel)
        }
    }

    private fun buildForegroundNotification(): Notification {
        val pendingIntent = PendingIntent.getActivity(
            this,
            0,
            Intent(this, MainActivity::class.java),
            PendingIntent.FLAG_IMMUTABLE or PendingIntent.FLAG_UPDATE_CURRENT
        )

        return NotificationCompat.Builder(this, CHANNEL_ID)
            .setContentTitle("OpenTap Background Unlock Active")
            .setContentText("Triple tap phone while unlocked to instantly open your PC.")
            .setSmallIcon(android.R.drawable.ic_lock_idle_lock)
            .setContentIntent(pendingIntent)
            .setOngoing(true)
            .setSilent(true)
            .build()
    }
}
