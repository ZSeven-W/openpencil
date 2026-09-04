package tech.zseven.openpencil

import android.os.SystemClock
import android.util.Log
import java.lang.ref.WeakReference

private const val TAG = "OpenPencilPlayer"

/**
 * Engine → shell upcalls. Most UI methods run on the engine thread and post
 * View / Choreographer work to the main thread. Collaboration redraw wakes
 * and credential methods may run on workers; both paths are thread-safe.
 */
class OpCallbacksImpl(context: android.content.Context) : OpCallbacks {
    private val credentialStore =
        AndroidCollaborationCredentialStore(context.applicationContext)

    @Volatile
    private var viewRef = WeakReference<OpSurfaceView>(null)

    @Volatile
    private var engine = 0L

    fun attach(view: OpSurfaceView) {
        viewRef = WeakReference(view)
    }

    fun detach(view: OpSurfaceView) {
        if (viewRef.get() === view) viewRef = WeakReference(null)
    }

    fun attachEngine(handle: Long) {
        engine = handle
    }

    fun clearEngine(handle: Long) {
        if (engine == handle) engine = 0L
    }

    fun engineHandle(): Long = engine

    override fun onNeedsRedraw(hasNextWake: Boolean, nextWakeMs: Long) {
        // Mutations and collaboration workers draw promptly. Timed wakes
        // carry caret/collaboration deadlines without keeping a hot loop.
        if (!hasNextWake) {
            viewRef.get()?.requestFrame()
            return
        }
        val delay = nextWakeMs - SystemClock.uptimeMillis()
        viewRef.get()?.scheduleFrame(delay)
    }

    override fun onRuntimeError(kind: Int, message: String, source: String?) {
        Log.e(TAG, "runtime error kind=$kind: $message${source?.let { " ($it)" } ?: ""}")
    }

    override fun onInputFocusChanged(focused: Boolean, inputKind: Int, returnKeyHint: Int) {
        // Editor mode: IME focus is polled via nativeEditorImeFocused after
        // every interaction; this callback serves the viewer-mode text ABI.
        Log.i(TAG, "input focus changed: focused=$focused kind=$inputKind hint=$returnKeyHint")
    }

    override fun onRemoteImageRequest(requestId: Long, url: String) {
        // Remote image enrichment is part of generation and must survive an
        // Activity/View recreation. Keep this fetch on the stable callback
        // receiver instead of routing it through the currently attached view.
        Thread {
            val bytes = runCatching {
                val connection =
                    java.net.URL(url).openConnection() as java.net.HttpURLConnection
                connection.connectTimeout = 10_000
                connection.readTimeout = 15_000
                connection.instanceFollowRedirects = true
                try {
                    if (connection.responseCode in 200..299) {
                        connection.inputStream.use { it.readBytes() }
                    } else {
                        ByteArray(0)
                    }
                } finally {
                    connection.disconnect()
                }
            }.getOrElse { ByteArray(0) }
            val current = engine
            if (current != 0L) {
                OpNative.nativeRemoteImageResult(current, requestId, bytes)
            }
        }.apply { name = "OpenPencilRemoteImage" }.start()
    }

    override fun onCredentialLoad(): ByteArray? = credentialStore.load()

    override fun onCredentialStoreIfAbsent(value: ByteArray): Boolean = try {
        credentialStore.storeIfAbsent(value)
        true
    } finally {
        value.fill(0)
    }
}
