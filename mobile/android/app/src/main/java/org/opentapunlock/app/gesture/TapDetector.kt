package org.opentapunlock.app.gesture

import android.hardware.Sensor
import android.hardware.SensorEvent
import android.hardware.SensorEventListener
import android.hardware.SensorManager
import android.util.Log
import kotlin.math.sqrt

interface GestureDetectionListener {
    fun onGestureDetected(gesture: GestureType)
}

/**
 * Digital Signal Processing (DSP) and threshold engine for recognizing Back Taps using accelerometer sensors.
 * Operates reliably in the background without draining battery.
 */
class TapDetector(private val listener: GestureDetectionListener) : SensorEventListener {
    companion object {
        private const val TAG = "TapDetector"
        // Increased threshold: 5.2 m/s² prevents false triggers from holding, tilting, or moving the phone!
        private const val TAP_THRESHOLD_MS2 = 5.2f
        // Jerk threshold: requires a sharp mechanical collision (finger tap impulse) rather than slow waving/shaking
        private const val JERK_THRESHOLD_MS2 = 4.5f
        private const val MIN_TAP_INTERVAL_MS = 280L // 280ms debounce completely blocks mechanical single-tap aftershock
        private const val MAX_TAP_WINDOW_MS = 1400L // Widen window for relaxed multi-taps
    }

    private val tapTimestamps = mutableListOf<Long>()
    private var lastTapTime = 0L

    // DSP High-Pass Gravity Filter State
    private var gravityX = 0f
    private var gravityY = 0f
    private var gravityZ = 0f
    private var lastDynX = 0f
    private var lastDynY = 0f
    private var lastDynZ = 0f
    private var filterInitialized = false

    fun register(sensorManager: SensorManager): Boolean {
        val sensor = sensorManager.getDefaultSensor(Sensor.TYPE_LINEAR_ACCELERATION)
            ?: sensorManager.getDefaultSensor(Sensor.TYPE_ACCELEROMETER)
        
        return if (sensor != null) {
            sensorManager.registerListener(this, sensor, SensorManager.SENSOR_DELAY_GAME)
            Log.i(TAG, "Registered TapDetector with sensor: ${sensor.name}")
            true
        } else {
            Log.e(TAG, "No suitable accelerometer sensor available on this device!")
            false
        }
    }

    fun unregister(sensorManager: SensorManager) {
        sensorManager.unregisterListener(this)
        Log.i(TAG, "Unregistered TapDetector sensor listener.")
    }

    override fun onSensorChanged(event: SensorEvent?) {
        if (event == null) return

        val x = event.values[0]
        val y = event.values[1]
        val z = event.values[2]

        if (!filterInitialized) {
            gravityX = x
            gravityY = y
            gravityZ = z
            filterInitialized = true
            return
        }

        // 1. High-Pass Filter: Isolate pure dynamic acceleration by filtering out Earth's gravity & slow tilting (alpha = 0.85)
        val alpha = 0.85f
        gravityX = alpha * gravityX + (1 - alpha) * x
        gravityY = alpha * gravityY + (1 - alpha) * y
        gravityZ = alpha * gravityZ + (1 - alpha) * z

        val dynX = x - gravityX
        val dynY = y - gravityY
        val dynZ = z - gravityZ

        // 2. Jerk Calculation (Rate of change of acceleration): Distinguishes sharp mechanical tap impacts from waving/shaking
        val deltaX = dynX - lastDynX
        val deltaY = dynY - lastDynY
        val deltaZ = dynZ - lastDynZ
        
        lastDynX = dynX
        lastDynY = dynY
        lastDynZ = dynZ

        val dynMagnitude = sqrt((dynX * dynX + dynY * dynY + dynZ * dynZ).toDouble()).toFloat()
        val jerk = sqrt((deltaX * deltaX + deltaY * deltaY + deltaZ * deltaZ).toDouble()).toFloat()
        val now = System.currentTimeMillis()

        // 3. Dual-Gate Verification: Must exceed BOTH dynamic magnitude AND mechanical jerk threshold!
        if (dynMagnitude >= TAP_THRESHOLD_MS2 && jerk >= JERK_THRESHOLD_MS2) {
            if (now - lastTapTime > MIN_TAP_INTERVAL_MS) {
                lastTapTime = now
                Log.i(TAG, "DSP Verified Tap Impulse! (Mag: ${dynMagnitude}, Jerk: ${jerk})")
                registerTapImpulse(now)
            }
        }
    }

    override fun onAccuracyChanged(sensor: Sensor?, accuracy: Int) {
        // No-op
    }

    @Synchronized
    private fun registerTapImpulse(timestamp: Long) {
        // Prune stale taps outside the detection window
        tapTimestamps.removeAll { timestamp - it > MAX_TAP_WINDOW_MS }
        tapTimestamps.add(timestamp)

        when (tapTimestamps.size) {
            3 -> {
                Log.i(TAG, "DSP DETECTED GESTURE: Triple Tap!")
                tapTimestamps.clear()
                listener.onGestureDetected(GestureType.TRIPLE_TAP)
            }
            2 -> {
                val interval = tapTimestamps[1] - tapTimestamps[0]
                if (interval > 750) {
                    Log.i(TAG, "DSP DETECTED GESTURE: Two Long Taps!")
                    tapTimestamps.clear()
                    listener.onGestureDetected(GestureType.TWO_LONG_TAPS)
                } else {
                    // Could be start of triple tap; let the window evaluate or trigger double tap if no 3rd arrives
                    Log.d(TAG, "DSP detected 2 taps (interval: ${interval}ms), waiting for potential 3rd tap...")
                    val currentTaps = ArrayList(tapTimestamps)
                    android.os.Handler(android.os.Looper.getMainLooper()).postDelayed({
                        synchronized(this) {
                            if (tapTimestamps == currentTaps && tapTimestamps.size == 2) {
                                Log.i(TAG, "DSP DETECTED GESTURE: Double Tap!")
                                tapTimestamps.clear()
                                listener.onGestureDetected(GestureType.DOUBLE_TAP)
                            }
                        }
                    }, 500)
                }
            }
        }
    }
}
