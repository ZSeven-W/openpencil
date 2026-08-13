package dev.openpencil.player

import kotlin.math.ln

/**
 * Converts consecutive finger distances into the wheel delta consumed by the
 * editor ABI. The engine applies `exp(delta * 0.0015)`, so this preserves the
 * platform pinch ratio instead of depending on distances measured in pixels.
 */
internal object PinchZoomDelta {
    private const val ZOOM_EXPONENT_PER_WHEEL_UNIT = 0.0015
    private const val MINIMUM_DISTANCE = 0.001f

    fun wheelDelta(previousDistance: Float, currentDistance: Float): Float {
        if (!previousDistance.isFinite() ||
            !currentDistance.isFinite() ||
            previousDistance <= MINIMUM_DISTANCE ||
            currentDistance <= MINIMUM_DISTANCE
        ) {
            return 0f
        }

        val ratio = currentDistance.toDouble() / previousDistance.toDouble()
        if (!ratio.isFinite() || ratio <= 0.0) return 0f
        val delta = ln(ratio) / ZOOM_EXPONENT_PER_WHEEL_UNIT
        return if (delta.isFinite()) delta.toFloat() else 0f
    }
}
