package tech.zseven.openpencil

import android.os.SystemClock
import android.util.Log

private const val TAG = "OpenPencilPlayer"

/**
 * Engine → shell upcalls. UI methods run on the engine thread and post view /
 * Choreographer work to the main thread. Credential methods may run on
 * collaboration workers and touch only the thread-safe platform store.
 */
class OpCallbacksImpl(private val view: OpSurfaceView) : OpCallbacks {
    private val credentialStore = AndroidCollaborationCredentialStore(view.context)

    override fun onNeedsRedraw(hasNextWake: Boolean, nextWakeMs: Long) {
        // The viewer engine only fires this from mutations (pointer /
        // resize / attach / resume) — draw promptly. `hasNextWake` is
        // reserved for future animation deadlines.
        if (!hasNextWake) {
            view.requestFrame()
            return
        }
        val delay = nextWakeMs - SystemClock.uptimeMillis()
        view.scheduleFrame(delay)
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
        view.fetchRemoteImage(requestId, url)
    }

    override fun onCredentialLoad(): ByteArray? = credentialStore.load()

    override fun onCredentialStoreIfAbsent(value: ByteArray): Boolean = try {
        credentialStore.storeIfAbsent(value)
        true
    } finally {
        value.fill(0)
    }
}
