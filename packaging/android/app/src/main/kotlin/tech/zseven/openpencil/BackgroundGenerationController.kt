package tech.zseven.openpencil

import android.content.Context
import android.content.Intent
import android.os.SystemClock
import android.util.Log
import androidx.core.content.ContextCompat
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

/** A retained engine and its stable native callback receiver. */
internal data class BackgroundEngineLease(
    val engine: Long,
    val surfaceWasAttached: Boolean,
)

/**
 * Serializes foreground/suspended pump ownership and retains the engine while
 * its foreground service finishes user-started generation work.
 */
internal object BackgroundGenerationController {
    private const val TAG = "OpenPencilPlayer"
    private val monitor = Any()
    private val state = BackgroundWorkState()
    private var callbacks: OpCallbacksImpl? = null
    private val cleanupWorker = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "OpenPencilRetainedEngineCleanup").apply { isDaemon = true }
    }
    private var cleanupTask: ScheduledFuture<*>? = null

    private data class PendingTransition(
        val transition: BackgroundWorkTransition,
        val receiverToDestroy: OpCallbacksImpl? = null,
    )

    fun registerEngine(handle: Long, receiver: OpCallbacksImpl, editorMode: Boolean) {
        synchronized(monitor) {
            cancelCleanupLocked()
            state.registerEngine(handle, editorMode)
            callbacks = receiver
            receiver.attachEngine(handle)
        }
    }

    fun markSurfaceAttached(handle: Long) = synchronized(monitor) {
        state.markSurfaceAttached(handle)
    }

    fun markSurfaceResuming(context: Context, handle: Long) {
        synchronized(monitor) {
            // This waits for an in-flight background tick, then closes its gate
            // before OpSurfaceView resumes or attaches a render surface.
            state.markSurfaceResuming(handle)
        }
        BackgroundGenerationService.releaseWakeLockForForeground()
        BackgroundGenerationNotifications.dismissPaused(context.applicationContext)
    }

    fun markSurfaceSuspended(handle: Long) = synchronized(monitor) {
        state.markSurfaceSuspended(handle)
    }

    fun setActivityVisible(context: Context, visible: Boolean) {
        if (visible) {
            BackgroundGenerationNotifications.dismissPaused(context.applicationContext)
        }
        val transition = synchronized(monitor) { state.setActivityVisible(visible) }
        applyTransition(context.applicationContext, PendingTransition(transition))
    }

    /**
     * Polls after a foreground frame or immediately before leaving the app.
     * Returns true only on the inactive -> active edge so the Activity can
     * make one contextual notification-permission request.
     */
    fun observeForeground(context: Context, handle: Long): Boolean {
        if (handle == 0L) return false
        val active = OpNative.nativeHasBackgroundWork(handle)
        val pending = synchronized(monitor) {
            pendingTransitionLocked(state.observe(handle, active))
        }
        applyTransition(context.applicationContext, pending)
        return pending.transition.becameActive
    }

    /** Called only by the service's single non-main worker. */
    fun pumpBackground(context: Context, serviceEpoch: Long): Boolean {
        val pending: PendingTransition
        val remainsActive: Boolean
        synchronized(monitor) {
            val handle = state.backgroundPumpHandle(serviceEpoch)
            if (handle == 0L) return state.hasActiveWork(serviceEpoch)
            remainsActive = OpNative.nativeBackgroundTick(handle, SystemClock.uptimeMillis())
            pending = pendingTransitionLocked(state.observe(handle, remainsActive))
        }
        applyTransition(context.applicationContext, pending)
        return remainsActive
    }

    fun hasActiveServiceWork(serviceEpoch: Long): Boolean = synchronized(monitor) {
        state.hasActiveWork(serviceEpoch)
    }

    fun needsBackgroundPump(serviceEpoch: Long): Boolean = synchronized(monitor) {
        state.backgroundPumpHandle(serviceEpoch) != 0L
    }

    fun currentServiceEpoch(): Long = synchronized(monitor) { state.currentServiceEpoch() }

    fun adoptEngine(view: OpSurfaceView, editorMode: Boolean): BackgroundEngineLease? =
        synchronized(monitor) {
            val receiver = callbacks ?: return@synchronized null
            val adopted = state.adopt(editorMode) ?: return@synchronized null
            cancelCleanupLocked()
            receiver.attach(view)
            BackgroundEngineLease(adopted.first, adopted.second)
        }

    /**
     * Detaches the old callback target. The service retains active work; an
     * inactive or unavailable engine is destroyed after leaving the monitor.
     */
    fun releaseView(context: Context, handle: Long, view: OpSurfaceView) {
        val pending: PendingTransition
        synchronized(monitor) {
            callbacks?.detach(view)
            pending = pendingTransitionLocked(state.detachView(handle))
        }
        applyTransition(context.applicationContext, pending)
    }

    /** API 35+ quota/error callback scoped to the Service run that observed it. */
    fun onServiceTimeout(context: Context, serviceEpoch: Long) {
        val pending = synchronized(monitor) {
            pendingTransitionLocked(state.serviceTimedOut(serviceEpoch))
        }
        applyTransition(context.applicationContext, pending)
    }

    /** Service launch/foreground promotion failure, scoped to its epoch. */
    fun onServiceStartFailed(context: Context, serviceEpoch: Long) {
        val pending = synchronized(monitor) {
            pendingTransitionLocked(state.serviceStartFailed(serviceEpoch))
        }
        applyTransition(context.applicationContext, pending)
    }

    private fun applyTransition(context: Context, pending: PendingTransition) {
        val transition = pending.transition
        if (transition.destroyEngine != 0L) {
            destroyEngine(transition.destroyEngine, pending.receiverToDestroy)
        }
        if (transition.scheduleRetainedCleanup) scheduleRetainedCleanup()
        if (transition.notifyPaused) {
            BackgroundGenerationNotifications.showPaused(context)
        }
        if (transition.completed) {
            if (transition.notifyCompletion) {
                BackgroundGenerationNotifications.showCompleted(context)
            }
        }
        if (transition.stopServiceEpoch != NO_SERVICE_EPOCH) {
            BackgroundGenerationService.requestStop(transition.stopServiceEpoch)
        }
        if (transition.startServiceEpoch != NO_SERVICE_EPOCH) {
            startService(context, transition.startServiceEpoch)
        }
    }

    private fun startService(context: Context, serviceEpoch: Long) {
        val shouldStart = synchronized(monitor) {
            callbacks?.engineHandle() != null && state.currentServiceEpoch() == serviceEpoch
        }
        if (!shouldStart) return
        try {
            ContextCompat.startForegroundService(context, serviceIntent(context, serviceEpoch))
        } catch (error: RuntimeException) {
            Log.e(TAG, "could not start background generation service", error)
            val pending = synchronized(monitor) {
                pendingTransitionLocked(state.serviceStartFailed(serviceEpoch))
            }
            applyTransition(context, pending)
        }
    }

    /** Must be called with [monitor] held. */
    private fun pendingTransitionLocked(
        transition: BackgroundWorkTransition,
    ): PendingTransition {
        val receiver = if (transition.destroyEngine != 0L) {
            callbacks.also { callbacks = null }
        } else {
            null
        }
        return PendingTransition(transition, receiver)
    }

    private fun scheduleRetainedCleanup() {
        synchronized(monitor) {
            cancelCleanupLocked()
            cleanupTask = cleanupWorker.schedule(
                ::expireRetainedEngine,
                RETAINED_RESULT_TIMEOUT_MINUTES,
                TimeUnit.MINUTES,
            )
        }
    }

    private fun expireRetainedEngine() {
        val receiver: OpCallbacksImpl?
        val handle: Long
        synchronized(monitor) {
            cleanupTask = null
            handle = state.expireRetainedResult()
            receiver = if (handle != 0L) callbacks.also { callbacks = null } else null
        }
        destroyEngine(handle, receiver)
    }

    private fun cancelCleanupLocked() {
        cleanupTask?.cancel(false)
        cleanupTask = null
    }

    private fun destroyEngine(handle: Long, receiver: OpCallbacksImpl?) {
        if (handle == 0L) return
        receiver?.clearEngine(handle)
        OpNative.nativeDestroy(handle)
    }

    private fun serviceIntent(context: Context, serviceEpoch: Long): Intent =
        Intent(context.applicationContext, BackgroundGenerationService::class.java).putExtra(
            BackgroundGenerationService.EXTRA_SERVICE_EPOCH,
            serviceEpoch,
        )

    private const val RETAINED_RESULT_TIMEOUT_MINUTES = 30L
}
