package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class BackgroundWorkStateTest {
    @Test
    fun foreground_completion_stops_service_without_result_notification() {
        val state = readyForegroundState(41L)

        val started = state.observe(41L, true)
        val epoch = started.startServiceEpoch
        assertTrue(epoch != NO_SERVICE_EPOCH)
        assertTrue(started.becameActive)
        assertEquals(0L, state.backgroundPumpHandle(epoch))

        val completed = state.observe(41L, false)
        assertTrue(completed.completed)
        assertEquals(epoch, completed.stopServiceEpoch)
        assertFalse(completed.notifyCompletion)
        assertFalse(completed.scheduleRetainedCleanup)
    }

    @Test
    fun background_completion_notifies_and_retains_result_for_recreation() {
        val state = readyForegroundState(42L)
        val epoch = state.observe(42L, true).startServiceEpoch
        state.setActivityVisible(false)
        state.markSurfaceSuspended(42L)
        assertEquals(42L, state.backgroundPumpHandle(epoch))
        assertEquals(0L, state.detachView(42L).destroyEngine)

        val completed = state.observe(42L, false)
        assertTrue(completed.completed)
        assertTrue(completed.notifyCompletion)
        assertTrue(completed.scheduleRetainedCleanup)

        val lease = state.adopt(mode = true)
        assertNotNull(lease)
        assertEquals(42L, lease?.first)
        assertEquals(0L, state.expireRetainedResult())
    }

    @Test
    fun re_adopt_wins_a_queued_tick_and_closes_the_background_pump_gate() {
        val state = readyForegroundState(43L)
        val epoch = state.observe(43L, true).startServiceEpoch
        state.markSurfaceSuspended(43L)
        state.detachView(43L)

        assertEquals(43L, state.backgroundPumpHandle(epoch))
        assertNotNull(state.adopt(mode = true))
        assertEquals(0L, state.backgroundPumpHandle(epoch))

        val completed = state.observe(43L, false)
        assertTrue(completed.completed)
        assertFalse(completed.notifyCompletion)
        assertFalse(completed.scheduleRetainedCleanup)
    }

    @Test
    fun completed_detached_result_expires_only_when_it_was_not_adopted() {
        val state = readyForegroundState(44L)
        state.observe(44L, true)
        state.markSurfaceSuspended(44L)
        state.detachView(44L)
        state.observe(44L, false)

        assertEquals(44L, state.expireRetainedResult())
        assertNull(state.adopt(mode = true))
    }

    @Test
    fun timeout_or_start_failure_retains_a_bounded_foreground_resume_lease() {
        val timedOut = readyForegroundState(45L)
        val timeoutEpoch = timedOut.observe(45L, true).startServiceEpoch
        timedOut.markSurfaceSuspended(45L)
        timedOut.detachView(45L)
        val timeout = timedOut.serviceTimedOut(timeoutEpoch)
        assertTrue(timeout.scheduleRetainedCleanup)
        assertTrue(timeout.notifyPaused)
        assertNotNull(timedOut.adopt(mode = true))

        val failed = readyForegroundState(46L)
        val failedEpoch = failed.observe(46L, true).startServiceEpoch
        failed.markSurfaceSuspended(46L)
        failed.detachView(46L)
        assertTrue(failed.serviceStartFailed(failedEpoch).scheduleRetainedCleanup)
        assertEquals(46L, failed.expireRetainedResult())
    }

    @Test
    fun foreground_timeout_stops_service_without_a_stale_pause_notification() {
        val state = readyForegroundState(53L)
        val epoch = state.observe(53L, true).startServiceEpoch

        val timeout = state.serviceTimedOut(epoch)

        assertFalse(timeout.notifyPaused)
        assertEquals(epoch, timeout.stopServiceEpoch)
    }

    @Test
    fun delayed_timeout_from_completed_epoch_cannot_block_next_generation() {
        val state = readyForegroundState(48L)
        val firstEpoch = state.observe(48L, true).startServiceEpoch
        val completed = state.observe(48L, false)
        assertEquals(firstEpoch, completed.stopServiceEpoch)

        val secondEpoch = state.observe(48L, true).startServiceEpoch
        assertTrue(secondEpoch > firstEpoch)
        state.markSurfaceSuspended(48L)

        val staleTimeout = state.serviceTimedOut(firstEpoch)
        assertFalse(staleTimeout.notifyPaused)
        assertEquals(NO_SERVICE_EPOCH, staleTimeout.stopServiceEpoch)
        assertEquals(secondEpoch, state.currentServiceEpoch())
        assertEquals(48L, state.backgroundPumpHandle(secondEpoch))
    }

    @Test
    fun stale_stop_and_timeout_tokens_cannot_consume_newer_service_run() {
        val gate = BackgroundServiceEpochState()
        gate.activate(epoch = 11L, startId = 101)
        gate.activate(epoch = 12L, startId = 102)

        assertNull(gate.takeEpoch(11L))
        assertNull(gate.takeStartId(101))
        assertTrue(gate.isCurrent(12L))
        assertEquals(102, gate.takeEpoch(12L)?.startId)
    }

    @Test
    fun delayed_completed_transition_cannot_stop_the_next_generation() {
        val state = readyForegroundState(51L)
        val service = BackgroundServiceEpochState()
        val firstStart = state.observe(51L, true)
        service.activate(firstStart.startServiceEpoch, startId = 201)

        // Hold this side effect while a second generation starts and takes
        // ownership of a newer Android start-id.
        val delayedCompletion = state.observe(51L, false)
        val secondStart = state.observe(51L, true)
        service.activate(secondStart.startServiceEpoch, startId = 202)

        assertNull(service.takeEpoch(delayedCompletion.stopServiceEpoch))
        assertTrue(service.isCurrent(secondStart.startServiceEpoch))
        assertEquals(202, service.takeEpoch(secondStart.startServiceEpoch)?.startId)
    }

    @Test
    fun delayed_timeout_report_cannot_pause_or_stop_the_next_generation() {
        val state = readyForegroundState(52L)
        val service = BackgroundServiceEpochState()
        val firstEpoch = state.observe(52L, true).startServiceEpoch
        service.activate(firstEpoch, startId = 301)

        // Android consumes the timed-out run immediately, but the controller
        // report may arrive later on its reporting thread.
        assertEquals(firstEpoch, service.takeStartId(301)?.epoch)
        state.observe(52L, false)
        val secondEpoch = state.observe(52L, true).startServiceEpoch
        service.activate(secondEpoch, startId = 302)

        val delayedTimeout = state.serviceTimedOut(firstEpoch)
        assertFalse(delayedTimeout.notifyPaused)
        assertEquals(NO_SERVICE_EPOCH, delayedTimeout.stopServiceEpoch)
        assertEquals(secondEpoch, state.currentServiceEpoch())
        assertTrue(service.isCurrent(secondEpoch))
    }

    @Test
    fun service_epochs_do_not_repeat_when_an_engine_is_recreated() {
        val state = readyForegroundState(49L)
        val firstEpoch = state.observe(49L, true).startServiceEpoch
        state.observe(49L, false)
        assertTrue(state.detachView(49L).scheduleRetainedCleanup)
        assertEquals(49L, state.expireRetainedResult())

        state.registerEngine(50L, mode = true)
        state.markSurfaceResuming(50L)
        val secondEpoch = state.observe(50L, true).startServiceEpoch
        assertTrue(secondEpoch > firstEpoch)
        assertFalse(state.serviceTimedOut(firstEpoch).notifyPaused)
        assertEquals(secondEpoch, state.currentServiceEpoch())
    }

    @Test
    fun inactive_engine_is_destroyed_when_its_only_view_goes_away() {
        val state = readyForegroundState(47L)
        assertEquals(47L, state.detachView(47L).destroyEngine)
    }

    private fun readyForegroundState(handle: Long = 41L): BackgroundWorkState =
        BackgroundWorkState().apply {
            registerEngine(handle, mode = true)
            markSurfaceAttached(handle)
            markSurfaceResuming(handle)
            setActivityVisible(true)
        }
}
