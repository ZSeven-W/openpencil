package tech.zseven.openpencil

/** Native upcalls. Redraw and credential methods may run on collaboration workers. */
interface OpCallbacks {
    /** A mutation or collaboration worker requested a redraw. This method
     *  must be thread-safe and marshal View work to the main thread.
     *  `hasNextWake` schedules a timed frame (the caret blink). */
    fun onNeedsRedraw(hasNextWake: Boolean, nextWakeMs: Long)

    /** A runtime diagnostic (document load / layout / GPU failures). */
    fun onRuntimeError(kind: Int, message: String, source: String?)

    /** Inline text-edit focus transition: show/hide the system keyboard. */
    fun onInputFocusChanged(focused: Boolean, inputKind: Int, returnKeyHint: Int)

    /** A paint pass recorded a remote image miss; fetch the URL and push
     *  the bytes back via `OpNative.nativeRemoteImageResult`. */
    fun onRemoteImageRequest(requestId: Long, url: String)

    /** Returns the decrypted device key, null only when it does not exist. */
    fun onCredentialLoad(): ByteArray?

    /** Atomically stores the first device key; an existing value wins. */
    fun onCredentialStoreIfAbsent(value: ByteArray): Boolean
}
