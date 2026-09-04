package tech.zseven.openpencil

import android.app.Service
import android.content.Intent
import android.content.pm.ServiceInfo
import android.os.Build
import android.os.Handler
import android.os.IBinder
import android.os.Looper
import android.os.PowerManager
import android.os.SystemClock
import android.util.Log
import androidx.annotation.RequiresApi
import androidx.core.app.ServiceCompat
import java.lang.ref.WeakReference
import java.util.concurrent.Executors
import java.util.concurrent.ScheduledFuture
import java.util.concurrent.TimeUnit

/** Same-process foreground service that pumps a suspended native engine. */
class BackgroundGenerationService : Service() {
    private val worker = Executors.newSingleThreadScheduledExecutor { runnable ->
        Thread(runnable, "OpenPencilBackgroundGeneration").apply { isDaemon = true }
    }
    private val mainHandler = Handler(Looper.getMainLooper())
    private val epochState = BackgroundServiceEpochState()
    private var tickTask: ScheduledFuture<*>? = null
    private var tickEpoch = NO_SERVICE_EPOCH
    private var wakeLock: PowerManager.WakeLock? = null
    private var wakeLockRenewAtMs = 0L

    override fun onCreate() {
        super.onCreate()
        activeInstance = WeakReference(this)
        BackgroundGenerationNotifications.createChannels(this)
    }

    override fun onStartCommand(intent: Intent?, flags: Int, startId: Int): Int {
        val requestedEpoch = intent?.getLongExtra(EXTRA_SERVICE_EPOCH, NO_SERVICE_EPOCH)
            ?: NO_SERVICE_EPOCH
        try {
            // Every startForegroundService request must be promoted promptly,
            // including a stale command already queued inside Android.
            ServiceCompat.startForeground(
                this,
                BackgroundGenerationNotifications.ONGOING_ID,
                BackgroundGenerationNotifications.ongoing(this),
                if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
                    ServiceInfo.FOREGROUND_SERVICE_TYPE_DATA_SYNC
                } else {
                    0
                },
            )
        } catch (error: RuntimeException) {
            Log.e(TAG, "could not enter foreground generation mode", error)
            stopSelfResult(startId)
            // The immutable Intent token owns this failure. Retagging a stale
            // command to the controller's newer epoch would poison new work.
            reportStartFailure(requestedEpoch)
            return START_NOT_STICKY
        }

        // Validate AFTER mandatory promotion. A stale command keeps its
        // immutable epoch/start-id; stopSelfResult cannot consume a newer
        // Android start-id that has already been assigned.
        val currentEpoch = BackgroundGenerationController.currentServiceEpoch()
        if (requestedEpoch == NO_SERVICE_EPOCH || requestedEpoch != currentEpoch) {
            if (currentEpoch == NO_SERVICE_EPOCH) {
                ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
            }
            stopSelfResult(startId)
            return START_NOT_STICKY
        }
        val previous = epochState.activate(requestedEpoch, startId)
        if (previous != null && previous.epoch != requestedEpoch) {
            cancelTick(previous.epoch)
            releaseWakeLock()
        }
        ensureTick(requestedEpoch)
        return START_NOT_STICKY
    }

    private fun ensureTick(serviceEpoch: Long) {
        val current = tickTask
        if (tickEpoch == serviceEpoch && current != null && !current.isDone) return
        current?.cancel(true)
        tickEpoch = serviceEpoch
        tickTask = worker.scheduleWithFixedDelay(
            { pumpOnce(serviceEpoch) },
            0L,
            BACKGROUND_TICK_INTERVAL_MS,
            TimeUnit.MILLISECONDS,
        )
    }

    private fun pumpOnce(serviceEpoch: Long) {
        if (!epochState.isCurrent(serviceEpoch)) return
        if (!BackgroundGenerationController.needsBackgroundPump(serviceEpoch)) {
            releaseWakeLock()
            if (!BackgroundGenerationController.hasActiveServiceWork(serviceEpoch)) {
                requestStop(serviceEpoch)
            }
            return
        }

        try {
            renewWakeLockIfNeeded()
        } catch (error: RuntimeException) {
            Log.e(TAG, "could not keep the CPU awake for background generation", error)
            requestPlatformStop(serviceEpoch)
            return
        }
        val remainsActive = try {
            BackgroundGenerationController.pumpBackground(applicationContext, serviceEpoch)
        } catch (error: RuntimeException) {
            Log.e(TAG, "background generation tick failed", error)
            requestPlatformStop(serviceEpoch)
            return
        }
        if (!remainsActive) {
            releaseWakeLock()
            requestStop(serviceEpoch)
        }
    }

    /**
     * Android 15+ dataSync quota callback. A timeout can consume only the
     * exact start-id/epoch that created it; an older callback cannot pause a
     * newer generation.
     */
    @RequiresApi(Build.VERSION_CODES.VANILLA_ICE_CREAM)
    override fun onTimeout(startId: Int, fgsType: Int) {
        val run = epochState.takeStartId(startId) ?: return
        stopRunImmediately(run)
        reportTimeout(run.epoch)
    }

    private fun requestPlatformStop(serviceEpoch: Long) {
        mainHandler.post {
            val run = epochState.takeEpoch(serviceEpoch) ?: return@post
            stopRunImmediately(run)
            reportTimeout(run.epoch)
        }
    }

    private fun requestEpochStop(serviceEpoch: Long) {
        mainHandler.post {
            val run = epochState.takeEpoch(serviceEpoch) ?: return@post
            stopRunImmediately(run)
        }
    }

    /** Runs on the Service main thread and never mutates controller state. */
    private fun stopRunImmediately(run: BackgroundServiceRun) {
        cancelTick(run.epoch)
        releaseWakeLock()
        ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
        // Unlike Context.stopService, this cannot consume a newer start-id
        // that Android already assigned to a later generation.
        stopSelfResult(run.startId)
    }

    private fun cancelTick(serviceEpoch: Long) {
        if (tickEpoch != serviceEpoch) return
        tickTask?.cancel(true)
        tickTask = null
        tickEpoch = NO_SERVICE_EPOCH
    }

    private fun reportTimeout(serviceEpoch: Long) {
        if (serviceEpoch == NO_SERVICE_EPOCH) return
        Thread {
            BackgroundGenerationController.onServiceTimeout(
                applicationContext,
                serviceEpoch,
            )
        }.apply {
            name = "OpenPencilBackgroundTimeout"
            isDaemon = true
        }.start()
    }

    private fun reportStartFailure(serviceEpoch: Long) {
        if (serviceEpoch == NO_SERVICE_EPOCH) return
        Thread {
            BackgroundGenerationController.onServiceStartFailed(
                applicationContext,
                serviceEpoch,
            )
        }.apply {
            name = "OpenPencilBackgroundStartFailure"
            isDaemon = true
        }.start()
    }

    @Synchronized
    private fun renewWakeLockIfNeeded() {
        val now = SystemClock.uptimeMillis()
        val current = wakeLock
        if (current?.isHeld == true && now < wakeLockRenewAtMs) return
        releaseWakeLock()
        val powerManager = getSystemService(PowerManager::class.java)
        wakeLock = powerManager.newWakeLock(
            PowerManager.PARTIAL_WAKE_LOCK,
            "$packageName:background-generation",
        ).apply {
            setReferenceCounted(false)
            acquire(WAKE_LOCK_TIMEOUT_MS)
        }
        wakeLockRenewAtMs = now + WAKE_LOCK_RENEW_INTERVAL_MS
    }

    @Synchronized
    private fun releaseWakeLock() {
        val current = wakeLock
        wakeLock = null
        wakeLockRenewAtMs = 0L
        try {
            if (current?.isHeld == true) current.release()
        } catch (error: RuntimeException) {
            Log.w(TAG, "background generation wake lock was already released", error)
        }
    }

    override fun onDestroy() {
        val abandonedRun = epochState.clear()
        tickTask?.cancel(true)
        tickTask = null
        tickEpoch = NO_SERVICE_EPOCH
        releaseWakeLock()
        worker.shutdownNow()
        if (activeInstance.get() === this) activeInstance = WeakReference(null)
        ServiceCompat.stopForeground(this, ServiceCompat.STOP_FOREGROUND_REMOVE)
        super.onDestroy()
        // Unexpected teardown pauses only the epoch this instance owned.
        abandonedRun?.let { reportTimeout(it.epoch) }
    }

    override fun onBind(intent: Intent?): IBinder? = null

    companion object {
        internal const val EXTRA_SERVICE_EPOCH = "openpencil.background.SERVICE_EPOCH"

        @Volatile
        private var activeInstance = WeakReference<BackgroundGenerationService>(null)

        internal fun requestStop(serviceEpoch: Long) {
            if (serviceEpoch == NO_SERVICE_EPOCH) return
            activeInstance.get()?.requestEpochStop(serviceEpoch)
        }

        internal fun releaseWakeLockForForeground() {
            activeInstance.get()?.releaseWakeLock()
        }

        private const val TAG = "OpenPencilPlayer"
        private const val BACKGROUND_TICK_INTERVAL_MS = 500L
        private const val WAKE_LOCK_TIMEOUT_MS = 2L * 60L * 1_000L
        private const val WAKE_LOCK_RENEW_INTERVAL_MS = 60L * 1_000L
    }
}
