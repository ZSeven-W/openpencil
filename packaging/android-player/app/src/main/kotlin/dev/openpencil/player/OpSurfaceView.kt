package dev.openpencil.player

import android.content.Context
import android.text.InputType
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager

private const val TAG = "OpenPencilPlayer"

/** Long-press delay (ms) before a right-click context menu. */
private const val LONG_PRESS_MS = 500L
/** Movement (px) that cancels a long-press candidate. */
private const val LONG_PRESS_SLOP = 12f

/** Pointer phases mirroring the C ABI `OpPointerPhase`. */
private const val PHASE_DOWN = 0
private const val PHASE_MOVE = 1
private const val PHASE_UP = 2
private const val PHASE_CANCEL = 3

/** OpStatus::GpuError discriminant. */
private const val GPU_ERROR = 4

/**
 * Hosts the engine's rendering surface and drives the frame pump. The
 * engine is created ONCE on the first `surfaceCreated`; the shell owns the
 * Surface→ANativeWindow pairing only indirectly — the native layer acquires
 * and releases the window on the engine thread.
 *
 * Gestures are interpreted by the engine: single-finger tap selects the
 * topmost node under the finger, single-finger drag pans, two-finger pinch
 * zooms around the pinch midpoint. The engine paints the document's active
 * page with the exact painter the desktop editor canvas uses.
 */
class OpSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {

    var engine: Long = 0L
        private set

    private var density: Float = resources.displayMetrics.density
    private var attachedOnce = false
    private var docBytes: ByteArray = ByteArray(0)
    private var editorMode = false
    private var fontBytes: List<ByteArray> = emptyList()

    // ---- editor-mode touch tracking --------------------------------------
    private var primaryPointerId = -1
    private var longPressArmed = false
    private var longPressFired = false
    private val longPressRunnable = Runnable { fireLongPress() }
    private var lastMidX = 0f
    private var lastMidY = 0f
    private var lastPinchDist = 0f
    private var twoFingerActive = false
    private var imeShown = false

    private val choreographer = Choreographer.getInstance()
    private var frameScheduled = false

    /** Increments ONLY on platform surfaceCreated; the GpuError recovery
     *  budget is one attempt per generation (never refilled by a successful
     *  frame or a recovery-driven resume). */
    private var surfaceGeneration = 0
    private var lastRecoveredGeneration = -1

    /** Latest insets (logical px), replayed after create/attach and resize. */
    private var safeArea = floatArrayOf(0f, 0f, 0f, 0f) // t, r, b, l
    private var keyboardHeight = 0f
    private var openDocumentHandler: (() -> Unit)? = null

    private val callbacks = OpCallbacksImpl(this)

    init {
        holder.addCallback(this)
        isFocusable = true
        isFocusableInTouchMode = true
    }

    fun configure(doc: ByteArray, editorMode: Boolean, fonts: List<ByteArray> = emptyList()) {
        docBytes = doc
        this.editorMode = editorMode
        fontBytes = fonts
    }

    fun editorMode(): Boolean = editorMode

    /** Registers the main-thread shell action handler owned by the Activity. */
    fun setOpenDocumentHandler(handler: () -> Unit) {
        openDocumentHandler = handler
    }

    private val imm: InputMethodManager
        get() = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

    /** Editor mode: the engine's host owns the IME focus; keep the system
     *  keyboard in sync after every interaction + frame. */
    fun syncIme() {
        if (!editorMode || engine == 0L) return
        val focused = OpNative.nativeEditorImeFocused(engine)
        if (focused && !imeShown) {
            imeShown = true
            requestFocus()
            imm.showSoftInput(this, 0)
        } else if (!focused && imeShown) {
            imeShown = false
            imm.hideSoftInputFromWindow(windowToken, 0)
        }
    }

    override fun onCheckIsTextEditor(): Boolean = editorMode && engine != 0L

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        if (!editorMode || engine == 0L) return null
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        outAttrs.imeOptions = EditorInfo.IME_ACTION_DONE
        return OpInputConnection(this, engine)
    }

    // ---- SurfaceHolder.Callback ------------------------------------------

    override fun surfaceCreated(holder: SurfaceHolder) {
        surfaceGeneration++ // a new surface generation refreshes the recovery budget
        val wLogical = width / density
        val hLogical = height / density
        if (engine == 0L) {
            engine = OpNative.nativeCreate(
                docBytes,
                wLogical,
                hLogical,
                density,
                callbacks,
                if (editorMode) 1 else 0,
            )
            if (engine == 0L) {
                Log.e(TAG, "nativeCreate failed: ${OpNative.nativeLastError(0)}")
                return
            }
            Log.i(TAG, "engine created (${wLogical}x$hLogical dpr=$density)")
            for (bytes in fontBytes) {
                OpNative.nativeRegisterFont(engine, bytes)
            }
        }
        attachOrResume(holder.surface)
        replayInsets()
        requestFrame()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, wPx: Int, hPx: Int) {
        if (engine == 0L) return
        OpNative.nativeResize(engine, wPx / density, hPx / density, density)
        replayInsets()
        requestFrame()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (engine == 0L) return
        // Blocking suspend BEFORE returning — the platform reclaims the
        // Surface after this returns.
        OpNative.nativeSuspend(engine)
    }

    /**
     * Until the FIRST successful attach, GPU mode was never selected and
     * `nativeResume` is invalid: retry `nativeAttachSurface` on each
     * surfaceCreated until it succeeds, then use `nativeResume` thereafter.
     */
    private fun attachOrResume(surface: Surface) {
        if (!attachedOnce) {
            val status = OpNative.nativeAttachSurface(engine, surface)
            if (status == 0) {
                attachedOnce = true
            } else {
                Log.w(TAG, "attach failed status=$status: ${OpNative.nativeLastError(engine)}")
            }
        } else {
            OpNative.nativeResume(engine, surface)
        }
    }

    // ---- Insets (set by MainActivity's OnApplyWindowInsetsListener) -------

    fun updateSafeArea(t: Float, r: Float, b: Float, l: Float) {
        safeArea = floatArrayOf(t, r, b, l)
        if (engine != 0L) OpNative.nativeSetSafeArea(engine, t, r, b, l)
    }

    fun updateKeyboard(h: Float) {
        keyboardHeight = h
        if (engine != 0L) OpNative.nativeSetKeyboard(engine, h)
    }

    private fun replayInsets() {
        if (engine == 0L) return
        OpNative.nativeSetSafeArea(engine, safeArea[0], safeArea[1], safeArea[2], safeArea[3])
        OpNative.nativeSetKeyboard(engine, keyboardHeight)
    }

    // ---- Frame pump (driven by onNeedsRedraw) ----------------------------

    /** Requests a single Choreographer frame; idempotent while one is queued. */
    fun requestFrame() {
        post {
            if (frameScheduled || engine == 0L) return@post
            frameScheduled = true
            choreographer.postFrameCallback(frameCallback)
        }
    }

    private val frameCallback = Choreographer.FrameCallback { frameTimeNanos ->
        frameScheduled = false
        if (engine == 0L) return@FrameCallback
        val status = OpNative.nativeFrame(engine, frameTimeNanos / 1_000_000)
        if (status == GPU_ERROR) {
            recoverGpu()
        } else {
            syncIme()
        }
        pollShellAction()
    }

    /**
     * One suspend → resume recovery per surface generation on a GpuError.
     * A surfaceDestroyed that raced in wins (the surface is invalid → drop
     * the recovery); a spent budget stops the pump and lets the engine's
     * onRuntimeError report it.
     */
    private fun recoverGpu() {
        val gen = surfaceGeneration
        val surface = holder.surface
        if (gen == lastRecoveredGeneration) {
            Log.w(TAG, "GpuError with recovery budget spent (generation $gen) — pump stopped")
            return
        }
        if (surface == null || !surface.isValid) return // raced surfaceDestroyed wins
        lastRecoveredGeneration = gen
        Log.i(TAG, "GpuError → suspend/resume recovery (generation $gen)")
        OpNative.nativeSuspend(engine)
        if (surface.isValid) OpNative.nativeResume(engine, surface)
        requestFrame()
    }

    /** Schedules a frame `delayMs` from now (the engine's next animation wake). */
    fun scheduleFrame(delayMs: Long) {
        if (delayMs <= 0) {
            requestFrame()
        } else {
            postDelayed({ requestFrame() }, delayMs)
        }
    }

    // ---- Touch (logical px, top-left origin — like iOS UITouch) ----------

    override fun onTouchEvent(event: MotionEvent): Boolean {
        if (engine == 0L) return false
        val tMs = event.eventTime
        if (editorMode) {
            return editorTouch(event)
        }
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                val i = event.actionIndex
                sendPointer(event, i, PHASE_DOWN, tMs)
            }
            MotionEvent.ACTION_MOVE -> {
                for (i in 0 until event.pointerCount) sendPointer(event, i, PHASE_MOVE, tMs)
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> {
                val i = event.actionIndex
                sendPointer(event, i, PHASE_UP, tMs)
            }
            MotionEvent.ACTION_CANCEL -> {
                val i = event.actionIndex
                sendPointer(event, i, PHASE_CANCEL, tMs)
            }
            else -> return false
        }
        return true
    }

    private fun sendPointer(event: MotionEvent, index: Int, phase: Int, tMs: Long) {
        val id = event.getPointerId(index)
        OpNative.nativePointer(
            engine,
            id,
            phase,
            event.getX(index) / density,
            event.getY(index) / density,
            tMs,
        )
    }

    // ---- Editor-mode touch: press/move/release + long-press + pan/pinch --

    private fun editorTouch(event: MotionEvent): Boolean {
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                primaryPointerId = event.getPointerId(0)
                longPressArmed = true
                longPressFired = false
                lastKnownX = event.x
                lastKnownY = event.y
                downX = event.x
                downY = event.y
                postDelayed(longPressRunnable, LONG_PRESS_MS)
                OpNative.nativeEditorPress(engine, event.x / density, event.y / density)
                pollShellAction()
                requestFrame()
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (event.pointerCount == 2) {
                    // Two fingers: pan + pinch take over.
                    longPressArmed = false
                    removeCallbacks(longPressRunnable)
                    twoFingerActive = true
                    val (midX, midY) = midpoint(event)
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = distance(event)
                }
            }
            MotionEvent.ACTION_MOVE -> {
                if (twoFingerActive && event.pointerCount >= 2) {
                    val (midX, midY) = midpoint(event)
                    val dx = (midX - lastMidX) / density
                    val dy = (midY - lastMidY) / density
                    val dist = distance(event)
                    val pinchDelta = (dist - lastPinchDist) / density
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = dist
                    if (dx != 0f || dy != 0f) {
                        OpNative.nativeEditorPan(engine, midX / density, midY / density, dx, dy)
                    }
                    if (pinchDelta != 0f) {
                        OpNative.nativeEditorPinch(engine, midX / density, midY / density, pinchDelta)
                    }
                    requestFrame()
                } else if (primaryPointerId >= 0) {
                    val index = event.findPointerIndex(primaryPointerId)
                    if (index >= 0) {
                        val x = event.getX(index) / density
                        val y = event.getY(index) / density
                        lastKnownX = event.getX(index)
                        lastKnownY = event.getY(index)
                        OpNative.nativeEditorMove(engine, x, y)
                        // Movement cancels the long-press candidate.
                        if (longPressArmed) {
                            if (Math.abs(x - downX / density) > LONG_PRESS_SLOP / density ||
                                Math.abs(y - downY / density) > LONG_PRESS_SLOP / density
                            ) {
                                longPressArmed = false
                                removeCallbacks(longPressRunnable)
                            }
                        }
                        requestFrame()
                    }
                }
            }
            MotionEvent.ACTION_POINTER_UP -> {
                if (twoFingerActive) {
                    twoFingerActive = false
                    // Re-arm single-finger tracking on the remaining pointer.
                    val index = if (event.actionIndex == 0) 1 else 0
                    if (index < event.pointerCount) {
                        primaryPointerId = event.getPointerId(index)
                        longPressArmed = false
                    }
                }
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                removeCallbacks(longPressRunnable)
                if (twoFingerActive) {
                    twoFingerActive = false
                } else if (!longPressFired) {
                    val x = event.x / density
                    val y = event.y / density
                    OpNative.nativeEditorRelease(engine, x, y)
                }
                primaryPointerId = -1
                longPressArmed = false
                longPressFired = false
                requestFrame()
            }
            else -> return false
        }
        return true
    }

    private fun fireLongPress() {
        longPressArmed = false
        longPressFired = true
        OpNative.nativeEditorRightPress(engine, lastKnownX / density, lastKnownY / density)
        requestFrame()
    }

    private var lastKnownX = 0f
    private var lastKnownY = 0f
    private var downX = 0f
    private var downY = 0f

    /**
     * Shell actions are consumed only after a blocking JNI call has returned.
     * Posting the handler prevents the Activity Result launcher from running
     * inside an engine callback or touch dispatch stack.
     */
    private fun pollShellAction() {
        if (!editorMode || engine == 0L) return
        val action = OpNative.nativeEditorTakeShellAction(engine)
        when {
            action < 0 || action == OpNative.SHELL_ACTION_NONE -> Unit
            action == OpNative.SHELL_ACTION_OPEN_DOCUMENT -> post {
                if (engine != 0L) openDocumentHandler?.invoke()
            }
            else -> Log.w(TAG, "unknown editor shell action=$action")
        }
    }

    /** Replaces the document atomically in the engine and schedules its first frame. */
    fun openDocument(bytes: ByteArray, displayName: String): Int {
        val current = engine
        if (!editorMode || current == 0L) return OpNative.STATUS_CLOSING
        val status = OpNative.nativeEditorOpenDocument(current, bytes, displayName)
        if (status == 0) {
            syncIme()
            requestFrame()
        }
        return status
    }

    private fun midpoint(event: MotionEvent): Pair<Float, Float> {
        var sx = 0f
        var sy = 0f
        for (i in 0 until event.pointerCount) {
            sx += event.getX(i)
            sy += event.getY(i)
        }
        lastKnownX = sx / event.pointerCount
        lastKnownY = sy / event.pointerCount
        return lastKnownX to lastKnownY
    }

    private fun distance(event: MotionEvent): Float {
        if (event.pointerCount < 2) return 0f
        val dx = event.getX(0) - event.getX(1)
        val dy = event.getY(0) - event.getY(1)
        return Math.sqrt((dx * dx + dy * dy).toDouble()).toFloat()
    }

    /** Fetches a remote image URL (fired by the engine's request upcall)
     *  and pushes the bytes back; empty bytes mark a failed fetch. */
    fun fetchRemoteImage(requestId: Long, url: String) {
        Thread {
            val bytes = runCatching {
                val connection = (java.net.URL(url).openConnection() as java.net.HttpURLConnection).apply {
                    connectTimeout = 10_000
                    readTimeout = 15_000
                    instanceFollowRedirects = true
                }
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
        }.start()
    }

    fun destroy() {
        openDocumentHandler = null
        if (engine != 0L) {
            OpNative.nativeDestroy(engine)
            engine = 0L
        }
    }
}
