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
        private const val TAP_THRESHOLD_MS2 = 3.2f // Sensitive threshold for comfortable average finger taps
        private const val LONG_TAP_THRESHOLD_MS2 = 2.8f
        private const val MIN_TAP_INTERVAL_MS = 100L // Debounce window
        private const val MAX_TAP_WINDOW_MS = 1200L // Widen window for relaxed multi-taps
    }

    private val tapTimestamps = mutableListOf<Long>()
    private var lastTapTime = 0L

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

        val magnitude = sqrt((x * x + y * y + z * z).toDouble()).toFloat()
        val now = System.currentTimeMillis()

        if (magnitude >= TAP_THRESHOLD_MS2) {
            if (now - lastTapTime > MIN_TAP_INTERVAL_MS) {
                lastTapTime = now
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
                if (interval > 600) {
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
                    }, 350)
                }
            }
        }
    }
}
