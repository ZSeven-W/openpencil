package dev.openpencil.player

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ViewportInputStateTest {
    @Test
    fun pendingDensityDoesNotChangeInputScaleBeforeAtomicCommit() {
        val state = ViewportInputState(initialDensity = 2f)
        assertTrue(state.commitIfSuccessful(status = 0))

        assertTrue(state.stageDensity(3f))
        assertTrue(state.beginGeometryUpdate())

        assertTrue(state.inputBlocked)
        assertEquals(3f, state.pendingDensity)
        assertEquals(2f, state.committedDensity)
    }

    @Test
    fun successfulAtomicCommitPublishesDensityAndUnblocksInputTogether() {
        val state = ViewportInputState(initialDensity = 2f)
        state.commitIfSuccessful(status = 0)
        state.stageDensity(3f)
        state.beginGeometryUpdate()

        assertTrue(state.commitIfSuccessful(status = 0))
        assertEquals(3f, state.committedDensity)
        assertFalse(state.inputBlocked)
    }

    @Test
    fun failedAtomicCommitKeepsOldInputScaleBlocked() {
        val state = ViewportInputState(initialDensity = 2f)
        state.commitIfSuccessful(status = 0)
        state.stageDensity(3f)
        state.beginGeometryUpdate()

        assertFalse(state.commitIfSuccessful(status = 4))
        assertEquals(2f, state.committedDensity)
        assertTrue(state.inputBlocked)
        assertFalse(state.beginGeometryUpdate())
    }

    @Test
    fun commitDoesNotRouteTailOfInterruptedPhysicalStream() {
        val state = ViewportInputState(initialDensity = 2f)
        state.commitIfSuccessful(status = 0)
        state.beginGeometryUpdate()

        assertFalse(state.acceptsTouch(ViewportInputState.ACTION_DOWN, pointerCount = 1))
        state.commitIfSuccessful(status = 0)
        assertFalse(state.acceptsTouch(actionMasked = 2, pointerCount = 1))
        assertFalse(state.acceptsTouch(actionMasked = 5, pointerCount = 2))
        assertFalse(state.acceptsTouch(ViewportInputState.ACTION_UP, pointerCount = 1))
        assertTrue(state.acceptsTouch(ViewportInputState.ACTION_DOWN, pointerCount = 1))
    }

    @Test
    fun freshDownCanRecoverWhenPlatformDropsInterruptedStreamTerminal() {
        val state = ViewportInputState(initialDensity = 2f)
        state.commitIfSuccessful(status = 0)
        state.beginGeometryUpdate()
        state.commitIfSuccessful(status = 0)

        assertTrue(state.acceptsTouch(ViewportInputState.ACTION_DOWN, pointerCount = 1))
    }
}
