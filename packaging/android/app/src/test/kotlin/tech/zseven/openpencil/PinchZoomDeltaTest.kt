package tech.zseven.openpencil

import kotlin.math.abs
import kotlin.math.exp
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class PinchZoomDeltaTest {
    @Test
    fun wheelDeltaReproducesPinchRatio() {
        assertScale(previous = 100f, current = 200f, expected = 2.0)
        assertScale(previous = 200f, current = 100f, expected = 0.5)
        assertScale(previous = 120f, current = 150f, expected = 1.25)
    }

    @Test
    fun consecutiveUpdatesCompose() {
        val first = PinchZoomDelta.wheelDelta(80f, 100f)
        val second = PinchZoomDelta.wheelDelta(100f, 160f)
        val combined = PinchZoomDelta.wheelDelta(80f, 160f)

        assertTrue(abs((first + second) - combined) < 0.001f)
    }

    @Test
    fun invalidOrTinyDistancesAreIgnored() {
        assertEquals(0f, PinchZoomDelta.wheelDelta(0f, 100f), 0f)
        assertEquals(0f, PinchZoomDelta.wheelDelta(0.001f, 100f), 0f)
        assertEquals(0f, PinchZoomDelta.wheelDelta(100f, 0f), 0f)
        assertEquals(0f, PinchZoomDelta.wheelDelta(Float.NaN, 100f), 0f)
        assertEquals(0f, PinchZoomDelta.wheelDelta(100f, Float.POSITIVE_INFINITY), 0f)
    }

    private fun assertScale(previous: Float, current: Float, expected: Double) {
        val delta = PinchZoomDelta.wheelDelta(previous, current)
        val actual = exp(delta.toDouble() * 0.0015)
        assertEquals(expected, actual, 0.000_001)
    }
}
