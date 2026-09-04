package tech.zseven.openpencil

/** Side effects requested by one pure background-work state transition. */
internal data class BackgroundWorkTransition(
    val startServiceEpoch: Long = NO_SERVICE_EPOCH,
    val stopServiceEpoch: Long = NO_SERVICE_EPOCH,
    val becameActive: Boolean = false,
    val completed: Boolean = false,
    val notifyCompletion: Boolean = false,
    val notifyPaused: Boolean = false,
    val scheduleRetainedCleanup: Boolean = false,
    val destroyEngine: Long = 0L,
)

internal const val NO_SERVICE_EPOCH = 0L

/**
 * Process-local ownership state for the single native editor engine.
 *
 * All methods are called under [BackgroundGenerationController]'s monitor.
 * Keeping the transition logic Android-free makes the destroy/adopt/service
 * races executable as ordinary JVM unit tests.
 */
internal class BackgroundWorkState {
    private var engine = 0L
    private var editorMode = false
    private var activityVisible = false
    private var viewAttached = false
    private var surfaceSuspended = true
    private var workActive = false
    private var generationEpoch = NO_SERVICE_EPOCH
    private var serviceEpoch = NO_SERVICE_EPOCH
    /** Process-monotonic: never reset when an engine is removed/recreated. */
    private var lastIssuedEpoch = NO_SERVICE_EPOCH
    private var serviceBlocked = false
    private var surfaceWasAttached = false
    private var retainedResult = false
    private var pausedForForeground = false

    fun registerEngine(handle: Long, mode: Boolean) {
        check(handle != 0L) { "cannot register a null engine" }
        check(engine == 0L) { "an engine is already registered" }
        engine = handle
        editorMode = mode
        viewAttached = true
        surfaceSuspended = true
        workActive = false
        generationEpoch = NO_SERVICE_EPOCH
        serviceEpoch = NO_SERVICE_EPOCH
        serviceBlocked = false
        surfaceWasAttached = false
        retainedResult = false
        pausedForForeground = false
    }

    fun setActivityVisible(visible: Boolean): BackgroundWorkTransition {
        activityVisible = visible
        return maybeStartService()
    }

    fun observe(handle: Long, active: Boolean): BackgroundWorkTransition {
        if (handle == 0L || handle != engine) return BackgroundWorkTransition()
        if (active) {
            val becameActive = !workActive
            if (becameActive) {
                generationEpoch = issueEpoch()
                serviceBlocked = false
                pausedForForeground = false
            }
            workActive = true
            retainedResult = false
            val start = maybeStartService()
            return start.copy(becameActive = becameActive)
        }
        if (!workActive) return BackgroundWorkTransition()

        workActive = false
        val completedServiceEpoch = serviceEpoch
        serviceEpoch = NO_SERVICE_EPOCH
        serviceBlocked = false
        retainedResult = true
        pausedForForeground = false
        return BackgroundWorkTransition(
            completed = true,
            stopServiceEpoch = completedServiceEpoch,
            notifyCompletion = !activityVisible || surfaceSuspended,
            scheduleRetainedCleanup = !viewAttached,
        )
    }

    fun markSurfaceAttached(handle: Long) {
        if (handle == engine) surfaceWasAttached = true
    }

    fun markSurfaceResuming(handle: Long) {
        if (handle == engine && viewAttached) surfaceSuspended = false
    }

    fun markSurfaceSuspended(handle: Long) {
        if (handle == engine) surfaceSuspended = true
    }

    fun backgroundPumpHandle(epoch: Long): Long =
        if (
            engine != 0L && workActive && serviceEpoch == epoch &&
            epoch != NO_SERVICE_EPOCH && surfaceSuspended
        ) {
            engine
        } else {
            0L
        }

    fun hasActiveWork(epoch: Long): Boolean =
        engine != 0L && workActive && serviceEpoch == epoch && epoch != NO_SERVICE_EPOCH

    fun currentServiceEpoch(): Long = serviceEpoch

    fun adopt(mode: Boolean): Pair<Long, Boolean>? {
        val activeLease = workActive && serviceEpoch != NO_SERVICE_EPOCH && !serviceBlocked
        if (
            engine == 0L || viewAttached ||
            (!activeLease && !retainedResult && !pausedForForeground) || editorMode != mode
        ) {
            return null
        }
        viewAttached = true
        retainedResult = false
        // Close the background-pump gate before the caller resumes EGL.
        surfaceSuspended = false
        return engine to surfaceWasAttached
    }

    fun detachView(handle: Long): BackgroundWorkTransition {
        if (handle == 0L || handle != engine || !viewAttached) {
            return BackgroundWorkTransition()
        }
        viewAttached = false
        return when {
            workActive && serviceEpoch != NO_SERVICE_EPOCH && !serviceBlocked ->
                BackgroundWorkTransition()
            retainedResult || pausedForForeground ->
                BackgroundWorkTransition(scheduleRetainedCleanup = true)
            else -> BackgroundWorkTransition(destroyEngine = removeEngine())
        }
    }

    fun serviceStartFailed(epoch: Long): BackgroundWorkTransition {
        if (epoch == NO_SERVICE_EPOCH || epoch != serviceEpoch) {
            return BackgroundWorkTransition()
        }
        serviceEpoch = NO_SERVICE_EPOCH
        serviceBlocked = true
        pausedForForeground = workActive
        return BackgroundWorkTransition(
            stopServiceEpoch = epoch,
            notifyPaused = pausedForForeground && (!activityVisible || surfaceSuspended),
            scheduleRetainedCleanup = !viewAttached,
        )
    }

    fun serviceTimedOut(epoch: Long): BackgroundWorkTransition {
        if (epoch == NO_SERVICE_EPOCH || epoch != serviceEpoch) {
            return BackgroundWorkTransition()
        }
        serviceEpoch = NO_SERVICE_EPOCH
        serviceBlocked = true
        pausedForForeground = workActive
        return BackgroundWorkTransition(
            stopServiceEpoch = epoch,
            notifyPaused = pausedForForeground && (!activityVisible || surfaceSuspended),
            scheduleRetainedCleanup = !viewAttached,
        )
    }

    fun expireRetainedResult(): Long =
        if (
            engine != 0L && !viewAttached &&
            ((retainedResult && !workActive) || pausedForForeground)
        ) {
            removeEngine()
        } else {
            0L
        }

    private fun maybeStartService(): BackgroundWorkTransition {
        val shouldStart = engine != 0L && activityVisible && workActive &&
            serviceEpoch == NO_SERVICE_EPOCH && !serviceBlocked
        if (!shouldStart) return BackgroundWorkTransition()
        serviceEpoch = generationEpoch
        return BackgroundWorkTransition(startServiceEpoch = serviceEpoch)
    }

    private fun issueEpoch(): Long {
        lastIssuedEpoch = if (lastIssuedEpoch == Long.MAX_VALUE) 1L else lastIssuedEpoch + 1L
        return lastIssuedEpoch
    }

    private fun removeEngine(): Long {
        val removed = engine
        engine = 0L
        editorMode = false
        viewAttached = false
        surfaceSuspended = true
        workActive = false
        generationEpoch = NO_SERVICE_EPOCH
        serviceEpoch = NO_SERVICE_EPOCH
        serviceBlocked = false
        surfaceWasAttached = false
        retainedResult = false
        pausedForForeground = false
        return removed
    }
}

/** Active Android Service start-id paired with its process-monotonic epoch. */
internal data class BackgroundServiceRun(val epoch: Long, val startId: Int)

/**
 * Pure token gate used by the Service. A delayed stop/timeout can consume only
 * the exact run that created it; a newer run remains untouched.
 */
internal class BackgroundServiceEpochState {
    private var current: BackgroundServiceRun? = null

    @Synchronized
    fun activate(epoch: Long, startId: Int): BackgroundServiceRun? {
        if (epoch == NO_SERVICE_EPOCH || startId <= 0) return null
        val previous = current
        current = BackgroundServiceRun(epoch, startId)
        return previous
    }

    @Synchronized
    fun isCurrent(epoch: Long): Boolean = current?.epoch == epoch

    @Synchronized
    fun takeEpoch(epoch: Long): BackgroundServiceRun? {
        val run = current?.takeIf { it.epoch == epoch } ?: return null
        current = null
        return run
    }

    @Synchronized
    fun takeStartId(startId: Int): BackgroundServiceRun? {
        val run = current?.takeIf { it.startId == startId } ?: return null
        current = null
        return run
    }

    @Synchronized
    fun clear(): BackgroundServiceRun? = current.also { current = null }
}
