package dev.openpencil.player

import android.content.Context
import android.text.InputType
import android.util.Log
import android.view.Choreographer
import android.view.MotionEvent
import android.view.Surface
import android.view.SurfaceHolder
import android.view.SurfaceView
import android.view.ViewTreeObserver
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodManager

private const val TAG = "OpenPencilPlayer"

/** Long-press delay (ms) before a right-click context menu. */
private const val LONG_PRESS_MS = 500L
/** Movement (logical dp) that cancels a long-press candidate. */
private const val LONG_PRESS_SLOP = 8f

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

    private val viewportInputState = ViewportInputState(resources.displayMetrics.density)
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
    private var editorReleaseSuppressed = false
    /** Actual platform visibility, updated only from WindowInsets. */
    private var imeVisible = false
    /** Avoid duplicate requests while Android is accepting a show request. */
    private var imeShowRequestPending = false
    private var imeShowNeeded = false
    private var imeWasFocused = false
    private var imeShowAttempts = 0
    private val clearImeShowRequest = Runnable {
        imeShowRequestPending = false
        if (imeShowNeeded && imeShowAttempts < 2) requestFrame()
    }

    private val choreographer = Choreographer.getInstance()
    private var frameScheduled = false
    private var viewportUpdateScheduled = false
    private var surfaceWidthPx = 0
    private var surfaceHeightPx = 0
    private val configurationViewportGate = ConfigurationViewportGate()
    private val viewportPreDrawListener = object : ViewTreeObserver.OnPreDrawListener {
        override fun onPreDraw(): Boolean {
            if (viewTreeObserver.isAlive) {
                viewTreeObserver.removeOnPreDrawListener(this)
            }
            viewportUpdateScheduled = false
            when (
                configurationViewportGate.evaluatePreDraw(
                    width,
                    height,
                    surfaceWidthPx,
                    surfaceHeightPx,
                )
            ) {
                ViewportGateDecision.APPLY -> applyViewportTuple()
                ViewportGateDecision.WAIT_FOR_INSETS -> Unit
                ViewportGateDecision.RETRY_NEXT_PRE_DRAW -> {
                    // Only scheduling happens in the next animation phase;
                    // the fallback decision itself is made after traversal.
                    postOnAnimation { scheduleViewportUpdate() }
                }
            }
            return true
        }
    }

    /** Increments ONLY on platform surfaceCreated; the GpuError recovery
     *  budget is one attempt per generation (never refilled by a successful
     *  frame or a recovery-driven resume). */
    private var surfaceGeneration = 0
    private var lastRecoveredGeneration = -1

    /** Latest physical inset pixels, converted with the current DPR only when
     *  an atomic viewport tuple is committed. */
    private var safeAreaPx = intArrayOf(0, 0, 0, 0) // t, r, b, l
    private var keyboardHeight = 0f
    private var openDocumentHandler: (() -> Unit)? = null
    private var systemChromeAppearanceHandler: ((Boolean) -> Unit)? = null
    private var prefersLightSystemIcons: Boolean? = null

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

    /** Registers a main-thread window-chrome updater owned by the Activity. */
    fun setSystemChromeAppearanceHandler(handler: (Boolean) -> Unit) {
        systemChromeAppearanceHandler = handler
    }

    /** Replays the last engine preference after an in-place configuration. */
    fun replaySystemChromeAppearance() {
        val preference = prefersLightSystemIcons ?: return
        post { systemChromeAppearanceHandler?.invoke(preference) }
    }

    private val imm: InputMethodManager
        get() = context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

    /** Editor mode: the engine's host owns the IME focus; keep the system
     *  keyboard in sync after every interaction + frame. */
    fun syncIme() {
        if (!editorMode || engine == 0L) return
        val focused = OpNative.nativeEditorImeFocused(engine)
        if (focused && !imeWasFocused) {
            imeShowNeeded = true
            imeShowAttempts = 0
        }
        imeWasFocused = focused
        if (focused && !imeVisible && imeShowNeeded &&
            !imeShowRequestPending && imeShowAttempts < 2
        ) {
            requestFocus()
            // Insets, not this return value, are the visibility source of
            // truth. If Android accepts but never shows (or rotation hides)
            // the IME, a later frame may retry after the short request gate.
            imeShowAttempts++
            imeShowRequestPending = imm.showSoftInput(this, 0)
            removeCallbacks(clearImeShowRequest)
            if (imeShowAttempts < 2) {
                // A rejected request also needs a later frame; otherwise no
                // native animation may remain to drive the bounded retry.
                postDelayed(clearImeShowRequest, 400L)
            }
        } else if (!focused && imeVisible) {
            imm.hideSoftInputFromWindow(windowToken, 0)
        }
        if (!focused) {
            imeShowNeeded = false
            imeShowAttempts = 0
            imeShowRequestPending = false
        }
    }

    override fun onCheckIsTextEditor(): Boolean = editorMode && engine != 0L

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        if (!editorMode || engine == 0L) return null
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        // Keep landscape typing inside the editor. Android's default extract
        // UI replaces nearly the entire app with a full-screen text surface.
        outAttrs.imeOptions =
            EditorInfo.IME_ACTION_DONE or EditorInfo.IME_FLAG_NO_EXTRACT_UI
        return OpInputConnection(this, engine)
    }

    // ---- SurfaceHolder.Callback ------------------------------------------

    override fun surfaceCreated(holder: SurfaceHolder) {
        surfaceGeneration++ // a new surface generation refreshes the recovery budget
        markViewportInputPending()
        refreshDensityFromResources()
        surfaceWidthPx = width
        surfaceHeightPx = height
        val viewportDensity = viewportInputState.pendingDensity
        val wLogical = width / viewportDensity
        val hLogical = height / viewportDensity
        if (engine == 0L) {
            engine = OpNative.nativeCreate(
                docBytes,
                wLogical,
                hLogical,
                viewportDensity,
                callbacks,
                if (editorMode) 1 else 0,
            )
            if (engine == 0L) {
                Log.e(TAG, "nativeCreate failed: ${OpNative.nativeLastError(0)}")
                return
            }
            Log.i(TAG, "engine created (${wLogical}x$hLogical dpr=$viewportDensity)")
            for (bytes in fontBytes) {
                OpNative.nativeRegisterFont(engine, bytes)
            }
        }
        attachOrResume(holder.surface)
        scheduleViewportUpdate()
        requestFrame()
    }

    override fun surfaceChanged(holder: SurfaceHolder, format: Int, wPx: Int, hPx: Int) {
        markViewportInputPending()
        refreshDensityFromResources()
        val extentChanged = surfaceWidthPx > 0 && surfaceHeightPx > 0 &&
            (surfaceWidthPx != wPx || surfaceHeightPx != hPx)
        surfaceWidthPx = wPx
        surfaceHeightPx = hPx
        if (engine == 0L) return
        if (extentChanged && holder.surface.isValid) {
            // Android may keep the same Surface object across a handled
            // rotation while its EGL window surface remains at the old buffer
            // extent. Rebind so edge-to-edge rendering covers the new window.
            OpNative.nativeSuspend(engine)
            OpNative.nativeResume(engine, holder.surface)
        }
        markImeForConfigurationRetry()
        scheduleViewportUpdate()
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

    fun updateSafeAreaPx(t: Int, r: Int, b: Int, l: Int) {
        val next = intArrayOf(t, r, b, l)
        val changed = !safeAreaPx.contentEquals(next)
        if (changed) markViewportInputPending()
        safeAreaPx = next
        configurationViewportGate.onInsetsDispatched()
        // Insets and Surface size can arrive in either order during rotation.
        // Commit only during pre-draw, after layout + Surface + inset dispatch
        // have converged on the same configuration.
        if (changed || engine != 0L) scheduleViewportUpdate()
    }

    fun updateKeyboard(h: Float, visible: Boolean) {
        val next = if (visible) h.coerceAtLeast(0f) else 0f
        val visibilityChanged = imeVisible != visible
        imeVisible = visible
        imeShowRequestPending = false
        removeCallbacks(clearImeShowRequest)
        if (visible) {
            imeShowNeeded = false
            imeShowAttempts = 0
        }
        if (keyboardHeight == next && !visibilityChanged) return
        keyboardHeight = next
        if (engine != 0L) OpNative.nativeSetKeyboard(engine, next)
        requestFrame()
    }

    /** Refreshes dp conversion after an in-place density/config change. */
    fun refreshDisplayMetrics() {
        markViewportInputPending()
        refreshDensityFromResources()
        // Wait for the explicitly requested inset redispatch before applying
        // a rotated tuple. The pre-draw gate has a bounded fallback for a
        // density-only change that emits neither insets nor surfaceChanged.
        configurationViewportGate.begin(width, height)
        scheduleViewportUpdate()
        // A rotation can dismiss the IME while native input remains focused.
        // Insets clear the visibility latch; the next frame retries the show.
        markImeForConfigurationRetry()
        requestFrame()
    }

    private fun markImeForConfigurationRetry() {
        imeShowNeeded = true
        imeShowAttempts = 0
        imeShowRequestPending = false
        removeCallbacks(clearImeShowRequest)
    }

    private fun refreshDensityFromResources(): Boolean {
        return viewportInputState.stageDensity(resources.displayMetrics.density)
    }

    private fun markViewportInputPending() {
        if (!viewportInputState.beginGeometryUpdate() || engine == 0L) return
        if (editorMode) {
            OpNative.nativeEditorCancelGesture(engine)
            resetEditorTouchTracking()
        } else {
            // Generic pointer Cancel is global in the engine; coordinates and
            // pointer id are intentionally ignored for cancellation.
            OpNative.nativePointer(engine, 0, PHASE_CANCEL, 0f, 0f, 0L)
        }
    }

    private fun scheduleViewportUpdate() {
        if (viewportUpdateScheduled) return
        viewportUpdateScheduled = true
        if (!isAttachedToWindow) {
            post {
                viewportUpdateScheduled = false
                scheduleViewportUpdate()
            }
            return
        }
        viewTreeObserver.addOnPreDrawListener(viewportPreDrawListener)
        invalidate()
    }

    private fun applyViewportTuple() {
        if (engine == 0L || width <= 0 || height <= 0) return
        // SurfaceView can receive its backing-surface update during pre-draw.
        // If it has not caught up with the laid-out view yet, surfaceChanged
        // schedules another pre-draw with the authoritative size.
        if (surfaceWidthPx != width || surfaceHeightPx != height) return
        refreshDensityFromResources()
        val viewportDensity = viewportInputState.pendingDensity
        val status = OpNative.nativeResizeWithSafeArea(
            engine,
            surfaceWidthPx / viewportDensity,
            surfaceHeightPx / viewportDensity,
            viewportDensity,
            safeAreaPx[0] / viewportDensity,
            safeAreaPx[1] / viewportDensity,
            safeAreaPx[2] / viewportDensity,
            safeAreaPx[3] / viewportDensity,
        )
        viewportInputState.commitIfSuccessful(status)
        OpNative.nativeSetKeyboard(engine, keyboardHeight)
        requestFrame()
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
            syncSystemChromeAppearance()
            syncIme()
        }
        pollShellAction()
    }

    /** Poll after a successful frame so a theme toggle and its icon contrast
     *  are presented together. JNI errors/closing use a light-surface-safe
     *  false fallback; the main-thread window update is value-deduplicated. */
    private fun syncSystemChromeAppearance() {
        if (engine == 0L) return
        val next = OpNative.nativePrefersLightSystemIcons(engine)
        if (prefersLightSystemIcons == next) return
        prefersLightSystemIcons = next
        post { systemChromeAppearanceHandler?.invoke(next) }
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
        // A handled rotation/density change may have updated Java resources
        // before the atomic native viewport tuple. Do not mutate the document
        // in that split state; the old stream was cancelled when it began.
        if (!viewportInputState.acceptsTouch(event.actionMasked, event.pointerCount)) return true
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
        val inputDensity = viewportInputState.committedDensity
        OpNative.nativePointer(
            engine,
            id,
            phase,
            event.getX(index) / inputDensity,
            event.getY(index) / inputDensity,
            tMs,
        )
    }

    // ---- Editor-mode touch: press/move/release + long-press + pan/pinch --

    private fun editorTouch(event: MotionEvent): Boolean {
        val inputDensity = viewportInputState.committedDensity
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                primaryPointerId = event.getPointerId(0)
                longPressArmed = true
                longPressFired = false
                lastKnownX = event.x
                lastKnownY = event.y
                downX = event.x / inputDensity
                downY = event.y / inputDensity
                postDelayed(longPressRunnable, LONG_PRESS_MS)
                OpNative.nativeEditorPress(engine, event.x / inputDensity, event.y / inputDensity)
                pollShellAction()
                requestFrame()
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (event.pointerCount == 2) {
                    // Two fingers: pan + pinch take over.
                    longPressArmed = false
                    removeCallbacks(longPressRunnable)
                    // The first pointer already entered the editor press
                    // ladder. Cancel that capture before multi-touch starts
                    // so no marquee/node drag survives the takeover.
                    OpNative.nativeEditorCancelGesture(engine)
                    editorReleaseSuppressed = true
                    twoFingerActive = true
                    val (midX, midY) = midpoint(event)
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = distance(event)
                    OpNative.nativeEditorBeginTransform(
                        engine,
                        midX / inputDensity,
                        midY / inputDensity,
                    )
                }
            }
            MotionEvent.ACTION_MOVE -> {
                if (twoFingerActive && event.pointerCount >= 2) {
                    val (midX, midY) = midpoint(event)
                    val dx = (midX - lastMidX) / inputDensity
                    val dy = (midY - lastMidY) / inputDensity
                    val dist = distance(event)
                    val pinchDelta = (dist - lastPinchDist) / inputDensity
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = dist
                    if (dx != 0f || dy != 0f) {
                        OpNative.nativeEditorPan(
                            engine,
                            midX / inputDensity,
                            midY / inputDensity,
                            dx,
                            dy,
                        )
                    }
                    if (pinchDelta != 0f) {
                        OpNative.nativeEditorPinch(
                            engine,
                            midX / inputDensity,
                            midY / inputDensity,
                            pinchDelta,
                        )
                    }
                    requestFrame()
                } else if (primaryPointerId >= 0) {
                    val index = event.findPointerIndex(primaryPointerId)
                    if (index >= 0) {
                        val x = event.getX(index) / inputDensity
                        val y = event.getY(index) / inputDensity
                        lastKnownX = event.getX(index)
                        lastKnownY = event.getY(index)
                        OpNative.nativeEditorMove(engine, x, y)
                        // Movement cancels the long-press candidate.
                        if (longPressArmed) {
                            val deltaX = x - downX
                            val deltaY = y - downY
                            if (deltaX * deltaX + deltaY * deltaY >
                                LONG_PRESS_SLOP * LONG_PRESS_SLOP
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
                    // End transform ownership before the remaining pointer is
                    // re-armed; its eventual Up must never release the press
                    // ladder cancelled at second-finger Down.
                    OpNative.nativeEditorCancelGesture(engine)
                    twoFingerActive = false
                    // Track the remaining physical pointer only so its final
                    // Up can terminate this suppressed stream. A fresh Down
                    // is required before press/move/release may resume.
                    val index = if (event.actionIndex == 0) 1 else 0
                    if (index < event.pointerCount) {
                        primaryPointerId = event.getPointerId(index)
                        longPressArmed = false
                    }
                }
            }
            MotionEvent.ACTION_UP -> {
                removeCallbacks(longPressRunnable)
                if (twoFingerActive) {
                    OpNative.nativeEditorCancelGesture(engine)
                    twoFingerActive = false
                } else if (!longPressFired && !editorReleaseSuppressed) {
                    val x = event.x / inputDensity
                    val y = event.y / inputDensity
                    OpNative.nativeEditorRelease(engine, x, y)
                }
                resetEditorTouchTracking()
                requestFrame()
            }
            MotionEvent.ACTION_CANCEL -> {
                // A platform cancellation must never run the release ladder:
                // release may commit a deferred tap, drag/drop, or history.
                OpNative.nativeEditorCancelGesture(engine)
                resetEditorTouchTracking()
                requestFrame()
            }
            else -> return false
        }
        return true
    }

    private fun resetEditorTouchTracking() {
        removeCallbacks(longPressRunnable)
        primaryPointerId = -1
        longPressArmed = false
        longPressFired = false
        twoFingerActive = false
        editorReleaseSuppressed = false
        lastMidX = 0f
        lastMidY = 0f
        lastPinchDist = 0f
        lastKnownX = 0f
        lastKnownY = 0f
        downX = 0f
        downY = 0f
    }

    private fun fireLongPress() {
        longPressArmed = false
        longPressFired = true
        val inputDensity = viewportInputState.committedDensity
        OpNative.nativeEditorRightPress(
            engine,
            lastKnownX / inputDensity,
            lastKnownY / inputDensity,
        )
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
        systemChromeAppearanceHandler = null
        if (engine != 0L) {
            OpNative.nativeDestroy(engine)
            engine = 0L
        }
    }
}
