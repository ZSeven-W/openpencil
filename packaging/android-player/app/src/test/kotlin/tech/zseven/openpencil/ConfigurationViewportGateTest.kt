package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ConfigurationViewportGateTest {
    @Test
    fun densityOnlyChangeWithoutSurfaceCallbackUsesSecondPreDrawFallback() {
        val gate = ConfigurationViewportGate()
        gate.begin(widthPx = 1080, heightPx = 2400)

        assertEquals(
            ViewportGateDecision.RETRY_NEXT_PRE_DRAW,
            gate.evaluatePreDraw(1080, 2400, 1080, 2400),
        )
        assertTrue(gate.awaitingInsets)
        assertEquals(
            ViewportGateDecision.APPLY,
            gate.evaluatePreDraw(1080, 2400, 1080, 2400),
        )
        assertFalse(gate.awaitingInsets)
    }

    @Test
    fun lateRotationInsetsCannotPublishNewBoundsWithOldSafeArea() {
        val gate = ConfigurationViewportGate()
        gate.begin(widthPx = 1080, heightPx = 2400)

        // The first pre-draw may still see the old geometry. It only arms the
        // fallback; it must not publish yet.
        assertEquals(
            ViewportGateDecision.RETRY_NEXT_PRE_DRAW,
            gate.evaluatePreDraw(1080, 2400, 1080, 2400),
        )
        // Traversal then installs landscape bounds before the requested inset
        // redispatch. The gate must keep waiting, however many frames pass.
        assertEquals(
            ViewportGateDecision.WAIT_FOR_INSETS,
            gate.evaluatePreDraw(2400, 1080, 2400, 1080),
        )
        assertEquals(
            ViewportGateDecision.WAIT_FOR_INSETS,
            gate.evaluatePreDraw(2400, 1080, 2400, 1080),
        )
        assertTrue(gate.awaitingInsets)

        gate.onInsetsDispatched()
        assertEquals(
            ViewportGateDecision.APPLY,
            gate.evaluatePreDraw(2400, 1080, 2400, 1080),
        )
    }

    @Test
    fun recreatedSurfaceDoesNotBypassRotationInsetBarrier() {
        val gate = ConfigurationViewportGate()
        gate.begin(widthPx = 1080, heightPx = 2400)

        assertEquals(
            ViewportGateDecision.WAIT_FOR_INSETS,
            gate.evaluatePreDraw(2400, 1080, 2400, 1080),
        )
        assertTrue(gate.awaitingInsets)
    }

    @Test
    fun surfaceAndViewMustAgreeBeforeDensityFallback() {
        val gate = ConfigurationViewportGate()
        gate.begin(widthPx = 1080, heightPx = 2400)

        assertEquals(
            ViewportGateDecision.WAIT_FOR_INSETS,
            gate.evaluatePreDraw(1080, 2400, 1080, 2200),
        )
        assertTrue(gate.awaitingInsets)
    }
}
