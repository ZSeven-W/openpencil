package dev.openpencil.player

import android.os.SystemClock
import android.util.Log

private const val TAG = "OpenPencilPlayer"

/**
 * Engine → shell upcalls. All methods run ON the engine thread; anything
 * touching the view / Choreographer is posted to the main thread by the
 * view.
 */
class OpCallbacksImpl(private val view: OpSurfaceView) : OpCallbacks {

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
}
