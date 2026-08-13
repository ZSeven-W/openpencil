package dev.openpencil.player

/** Engine-thread upcalls (forward-only). Every method runs ON the engine thread. */
interface OpCallbacks {
    /** A mutation (pointer / resize / attach / resume / text edit / caret
     *  blink) requested a redraw. `hasNextWake` schedules a timed frame
     *  (the caret blink). */
    fun onNeedsRedraw(hasNextWake: Boolean, nextWakeMs: Long)

    /** A runtime diagnostic (document load / layout / GPU failures). */
    fun onRuntimeError(kind: Int, message: String, source: String?)

    /** Inline text-edit focus transition: show/hide the system keyboard. */
    fun onInputFocusChanged(focused: Boolean, inputKind: Int, returnKeyHint: Int)

    /** A paint pass recorded a remote image miss; fetch the URL and push
     *  the bytes back via `OpNative.nativeRemoteImageResult`. */
    fun onRemoteImageRequest(requestId: Long, url: String)
}
