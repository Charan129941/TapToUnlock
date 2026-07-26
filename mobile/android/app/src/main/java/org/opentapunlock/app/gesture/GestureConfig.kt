package org.opentapunlock.app.gesture

import android.content.Context
import android.content.SharedPreferences

enum class GestureType(val displayName: String) {
    DOUBLE_TAP("Double Tap"),
    TRIPLE_TAP("Triple Tap (Default)"),
    TWO_LONG_TAPS("Two Long Taps")
}

enum class GestureAction(val displayName: String, val rustAction: String) {
    UNLOCK_PC("Unlock PC Screen", "UNLOCK"),
    LOCK_PC("Lock PC Screen", "LOCK"),
    SLEEP_PC("Put PC to Sleep", "SLEEP"),
    MUTE_AUDIO("Mute PC Audio", "MUTE"),
    NONE("No Action", "NONE")
}

data class GestureMapping(
    val gesture: GestureType,
    val action: GestureAction,
    val targetPcId: String
)

/**
 * Manages user-customizable gesture-to-action mappings saved in SharedPreferences.
 */
class GestureConfigManager(context: Context) {
    private val prefs: SharedPreferences = context.getSharedPreferences("opentap_gestures", Context.MODE_PRIVATE)

    init {
        // Initialize default mappings if empty
        if (!prefs.contains(GestureType.TRIPLE_TAP.name)) {
            setMapping(GestureType.TRIPLE_TAP, GestureAction.UNLOCK_PC)
            setMapping(GestureType.DOUBLE_TAP, GestureAction.LOCK_PC)
            setMapping(GestureType.TWO_LONG_TAPS, GestureAction.SLEEP_PC)
        }
    }

    fun setMapping(gesture: GestureType, action: GestureAction) {
        prefs.edit().putString(gesture.name, action.name).apply()
    }

    fun getAction(gesture: GestureType): GestureAction {
        val actionName = prefs.getString(gesture.name, GestureAction.NONE.name) ?: GestureAction.NONE.name
        return try {
            GestureAction.valueOf(actionName)
        } catch (e: Exception) {
            GestureAction.NONE
        }
    }

    fun getAllMappings(defaultPcId: String = "default-pc"): List<GestureMapping> {
        return GestureType.values().map { gesture ->
            GestureMapping(gesture, getAction(gesture), defaultPcId)
        }
    }
}
