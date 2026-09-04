package tech.zseven.openpencil

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
import java.io.File

private const val TAG = "OpenPencilPlayer"

/** Pointer phases mirroring the C ABI `OpPointerPhase`. */
private const val PHASE_DOWN = 0
private const val PHASE_MOVE = 1
private const val PHASE_UP = 2
private const val PHASE_CANCEL = 3

/** OpStatus::GpuError discriminant. */
private const val GPU_ERROR = 4

/**
 * Hosts the engine's rendering surface and drives the frame pump. The
 * engine is created on the first `surfaceCreated`, or safely re-adopted from
 * the process service after an Activity recreation. The shell owns the
 * Surface→ANativeWindow pairing only indirectly — the native layer acquires
 * and releases the window on the engine thread.
 *
 * Gestures are interpreted by the engine: single-finger tap selects the
 * topmost node under the finger, single-finger drag pans, two-finger pinch
 * zooms around the pinch midpoint. The engine paints the document's active
 * page with the exact painter the desktop editor canvas uses.
 *
 * Cohesive subsystems live beside this file: [OpSurfaceViewEditorTouch]
 * (editor touch ladder), [OpSurfaceViewImeCoordinator] (IME focus latches),
 * and [OpSurfaceViewShellBridge] (Activity-owned shell actions + editor
 * APIs).
 */
class OpSurfaceView(context: Context) : SurfaceView(context), SurfaceHolder.Callback {

    var engine: Long = 0L
        private set

    private val viewportInputState = ViewportInputState(resources.displayMetrics.density)
    private val shellBridge = OpSurfaceViewShellBridge(this)
    private val editorTouch = OpSurfaceViewEditorTouch(this)
    private val ime = OpSurfaceViewImeCoordinator(this)
    private var attachedOnce = false
    private var docBytes: ByteArray = ByteArray(0)
    private var editorMode = false
    private var fontBytes: List<ByteArray> = emptyList()
    internal val authRuntime = AndroidAuthRuntime(context)
    private val privateStorageRoot =
        File(context.applicationContext.noBackupFilesDir, "config").absolutePath

    private val choreographer = Choreographer.getInstance()
    private var frameScheduled = false
    private var scheduledFrameEpoch = 0L
    /**
     * Cross-thread generation token for posted redraw requests. A callback
     * captured for an older Surface must not become valid merely because a
     * later Surface has successfully resumed.
     */
    @Volatile
    private var surfaceFrameEpoch = 0L
    /** nativeFrame is legal only after a successful attach/resume. */
    @Volatile
    private var surfaceReady = false
    private var viewportUpdateScheduled = false
    private var viewportUpdateEpoch = 0L
    private var surfaceWidthPx = 0
    private var surfaceHeightPx = 0
    private val configurationViewportGate = ConfigurationViewportGate()
    private val viewportPreDrawListener = object : ViewTreeObserver.OnPreDrawListener {
        override fun onPreDraw(): Boolean {
            if (viewTreeObserver.isAlive) {
                viewTreeObserver.removeOnPreDrawListener(this)
            }
            val requestedEpoch = viewportUpdateEpoch
            viewportUpdateScheduled = false
            viewportUpdateEpoch = 0L
            if (!surfaceReady || requestedEpoch != surfaceFrameEpoch) return true
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
                    postOnAnimation { scheduleViewportUpdate(requestedEpoch) }
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
    private var backgroundWorkActivationHandler: (() -> Unit)? = null
    private var backgroundPermissionPromptPending = false
    /** While an Activity overlay (login / registration / account center) is
     *  visible its EditTexts own the IME; editor IME sync stands down. */
    private var imeOwnedByOverlay = false

    init {
        holder.addCallback(this)
        isFocusable = true
        isFocusableInTouchMode = true
    }

    // ---- Narrow seams the extracted subsystems read through --------------

    internal fun editorEngine(): Long = engine

    internal val committedInputDensity: Float
        get() = viewportInputState.committedDensity

    internal val isFrameGateOpen: Boolean
        get() = surfaceReady

    /** First paint + chrome drain right after a press lands (DOWN path). */
    internal fun settleEditorPressFlow() {
        shellBridge.pollShellAction()
        requestFrame()
    }

    fun configure(doc: ByteArray, editorMode: Boolean, fonts: List<ByteArray> = emptyList()) {
        docBytes = doc
        this.editorMode = editorMode
        fontBytes = fonts
    }

    fun editorMode(): Boolean = editorMode

    // ---- Activity-owned handler registration (see the shell bridge) ------

    fun setOpenDocumentHandler(handler: () -> Unit) = shellBridge.setOpenDocumentHandler(handler)

    fun setImportImageOrSvgHandler(handler: () -> Unit) =
        shellBridge.setImportImageOrSvgHandler(handler)

    fun setExportDocumentHandler(handler: () -> Unit) =
        shellBridge.setExportDocumentHandler(handler)

    fun setSaveDocumentHandler(handler: () -> Unit) = shellBridge.setSaveDocumentHandler(handler)

    fun setAccountCenterHandler(handler: () -> Unit) =
        shellBridge.setAccountCenterHandler(handler)

    fun setRequestLoginHandler(handler: () -> Unit) = shellBridge.setRequestLoginHandler(handler)

    fun setLanguagePickerHandler(handler: () -> Unit) =
        shellBridge.setLanguagePickerHandler(handler)

    fun setLoginUiHandlers(open: (String) -> Unit, close: () -> Unit) =
        shellBridge.setLoginUiHandlers(open, close)

    fun setSystemChromeAppearanceHandler(handler: (Boolean) -> Unit) =
        shellBridge.setSystemChromeAppearanceHandler(handler)

    /** User close/back: cancel the single auth flow owned by this engine. */
    fun cancelLogin() = shellBridge.cancelLogin()

    /** Starts the device flow; returns the raw engine status. */
    fun beginLogin(): Int = shellBridge.beginLogin()

    /** Applies a UI locale tag; returns the raw engine status. */
    fun setLocale(tag: String): Int = shellBridge.setLocale(tag)

    /** The current UI locale's BCP-47 tag, if readable. */
    fun localeCode(): String? = shellBridge.localeCode()

    /** Copies the engine's JSON account snapshot (never consumed). */
    fun accountSnapshot(): String? = shellBridge.accountSnapshot()

    /** Revokes the device session from the native account center. */
    fun signOutAccount() = shellBridge.signOutAccount()

    /** Copies the frozen export's file name without consuming the artifact. */
    fun exportFileName(): String? = shellBridge.exportFileName()

    /** Writes the frozen export into a new absolute staging file. */
    fun exportToPath(path: String): Int = shellBridge.exportToPath(path)

    /** Discards the frozen export when the save UI cannot run. */
    fun cancelExport() = shellBridge.cancelExport()

    /** Replays the last engine preference after an in-place configuration. */
    fun replaySystemChromeAppearance() = shellBridge.replaySystemChromeAppearance()

    /** Requests notification permission on the inactive -> active edge. */
    fun setBackgroundWorkActivationHandler(handler: () -> Unit) {
        backgroundWorkActivationHandler = handler
    }

    override fun onCheckIsTextEditor(): Boolean = editorMode && engine != 0L

    override fun onCreateInputConnection(outAttrs: EditorInfo): InputConnection? {
        if (!editorMode || engine == 0L) return null
        outAttrs.inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE
        // Keep landscape typing inside the editor. Android's default extract
        // UI replaces nearly the entire app with a full-screen text surface.
        outAttrs.imeOptions =
            EditorInfo.IME_ACTION_DONE or EditorInfo.IME_FLAG_NO_EXTRACT_UI
        // The connection's editable is a constant sentinel (see
        // OpInputConnection); report a matching cursor so IMEs see a
        // consistent "one char before the caret" and always emit deletes.
        outAttrs.initialSelStart = OpInputConnection.SENTINEL.length
        outAttrs.initialSelEnd = OpInputConnection.SENTINEL.length
        return OpInputConnection(this, engine)
    }

    // ---- SurfaceHolder.Callback ------------------------------------------

    override fun surfaceCreated(holder: SurfaceHolder) {
        closeSurfaceFrameGate()
        surfaceGeneration++ // a new surface generation refreshes the recovery budget
        markViewportInputPending()
        refreshDensityFromResources()
        surfaceWidthPx = width
        surfaceHeightPx = height
        val viewportDensity = viewportInputState.pendingDensity
        val wLogical = width / viewportDensity
        val hLogical = height / viewportDensity
        if (engine == 0L) {
            val lease = BackgroundGenerationController.adoptEngine(this, editorMode)
            if (lease != null) {
                engine = lease.engine
                attachedOnce = lease.surfaceWasAttached
                Log.i(TAG, "re-adopted background generation engine")
            } else {
                val callbacks = OpCallbacksImpl(context.applicationContext).also { it.attach(this) }
                engine = OpNative.nativeCreate(
                    docBytes,
                    wLogical,
                    hLogical,
                    viewportDensity,
                    callbacks,
                    privateStorageRoot,
                    if (editorMode) 1 else 0,
                )
                if (engine == 0L) {
                    callbacks.detach(this)
                    Log.e(TAG, "nativeCreate failed: ${OpNative.nativeLastError(0)}")
                    return
                }
                BackgroundGenerationController.registerEngine(engine, callbacks, editorMode)
                Log.i(TAG, "engine created (${wLogical}x$hLogical dpr=$viewportDensity)")
                for (bytes in fontBytes) {
                    OpNative.nativeRegisterFont(engine, bytes)
                }
            }
            authRuntime.configure(engine, editorMode)
            // The Activity may have registered the handler before the engine
            // existed (configure() runs in onCreate); re-declare here so the
            // capability is always in place before the first shell action.
            if (editorMode && shellBridge.hasSavePicker()) {
                OpNative.nativeEditorConfigureSavePicker(engine, true)
            }
            EngineLanguage.storedPreference(context)?.let { tag ->
                OpNative.nativeEditorSetLocale(engine, tag)
            }
        }
        BackgroundGenerationController.markSurfaceResuming(context, engine)
        if (!attachOrResume(holder.surface)) {
            BackgroundGenerationController.markSurfaceSuspended(engine)
            return
        }
        openSurfaceFrameGate()
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
        if (!surfaceReady || extentChanged) {
            // Android may keep the same Surface object across a handled
            // rotation while its EGL window surface remains at the old buffer
            // extent. A failed first attach also retries here even when the
            // dimensions are unchanged.
            closeSurfaceFrameGate()
            // Drain any service tick and close its ownership gate before the
            // suspend/resume pair. This also covers recovery after a failed
            // first attach, whose state was previously marked suspended.
            BackgroundGenerationController.markSurfaceResuming(context, engine)
            // nativeResume is invalid until nativeAttachSurface has succeeded
            // once; attachOrResume owns that distinction.
            if (attachedOnce) {
                cancelStreamsBeforeSuspend()
                OpNative.nativeSuspend(engine)
            }
            val resumed = holder.surface.isValid && attachOrResume(holder.surface)
            if (resumed) {
                openSurfaceFrameGate()
            } else {
                BackgroundGenerationController.markSurfaceSuspended(engine)
            }
        }
        ime.retryAfterConfiguration()
        scheduleViewportUpdate()
    }

    override fun surfaceDestroyed(holder: SurfaceHolder) {
        if (engine == 0L) return
        // Close and drain the main-thread frame gate BEFORE the blocking
        // suspend barrier. A callback already queued by a background redraw
        // can no longer cross the barrier and call nativeFrame afterwards.
        closeSurfaceFrameGate()
        editorTouch.resetTracking()
        // This final foreground-originated observation starts the FGS before
        // handing pump ownership over. onPause performs the same probe before
        // Android's visible-activity start exemption can disappear.
        observeBackgroundGeneration(allowPermissionPrompt = false)
        // Blocking suspend BEFORE returning — the platform reclaims the
        // Surface after this returns. Any live gesture stream is cancelled
        // with the real monotonic clock first so it cannot cross the barrier.
        cancelStreamsBeforeSuspend()
        OpNative.nativeSuspend(engine)
        BackgroundGenerationController.markSurfaceSuspended(engine)
    }

    /**
     * Until the FIRST successful attach, GPU mode was never selected and
     * `nativeResume` is invalid: retry `nativeAttachSurface` on each
     * surfaceCreated until it succeeds, then use `nativeResume` thereafter.
     */
    private fun attachOrResume(surface: Surface): Boolean {
        if (!attachedOnce) {
            val status = OpNative.nativeAttachSurface(engine, surface)
            if (status == 0) {
                attachedOnce = true
                BackgroundGenerationController.markSurfaceAttached(engine)
            } else {
                Log.w(TAG, "attach failed status=$status: ${OpNative.nativeLastError(engine)}")
            }
            return status == 0
        } else {
            return OpNative.nativeResume(engine, surface) == 0
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
        val visibilityChanged = ime.observeKeyboardVisibility(visible)
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
        ime.retryAfterConfiguration()
        requestFrame()
    }

    private fun refreshDensityFromResources(): Boolean {
        return viewportInputState.stageDensity(resources.displayMetrics.density)
    }

    private fun markViewportInputPending() {
        if (!viewportInputState.beginGeometryUpdate() || engine == 0L) return
        if (editorMode) {
            OpNative.nativeEditorCancelGestureAt(engine, uptimeClockMs())
            editorTouch.resetTracking()
        } else {
            // Generic pointer Cancel is global in the engine; coordinates and
            // pointer id are intentionally ignored for cancellation. This is
            // a shell-fabricated geometry transition with no MotionEvent of
            // its own, so it carries the monotonic uptime clock instead of a
            // zero timestamp.
            OpNative.nativePointer(engine, 0, PHASE_CANCEL, 0f, 0f, uptimeClockMs())
        }
    }

    /**
     * Cancels any live Editor/Viewer gesture stream with the platform cancel
     * clock and clears local touch tracking. Every pre-suspend barrier
     * (surfaceChanged / surfaceDestroyed / GPU recovery / destroy) MUST run
     * this before [OpNative.nativeSuspend] — 先 Cancel、后 suspend — so an
     * armed press/move ladder can never cross a suspended surface. Idempotent
     * while no gesture is live.
     */
    private fun cancelStreamsBeforeSuspend() {
        if (engine == 0L) return
        if (editorMode) {
            OpNative.nativeEditorCancelGestureAt(engine, uptimeClockMs())
        } else {
            // Generic pointer Cancel is global in the engine; coordinates and
            // pointer id are intentionally ignored for cancellation.
            OpNative.nativePointer(engine, 0, PHASE_CANCEL, 0f, 0f, uptimeClockMs())
        }
        editorTouch.resetTracking()
    }

    private fun scheduleViewportUpdate() {
        val requestedEpoch = surfaceFrameEpoch
        if (!surfaceReady) return
        scheduleViewportUpdate(requestedEpoch)
    }

    private fun scheduleViewportUpdate(requestedEpoch: Long) {
        if (
            !surfaceReady || requestedEpoch != surfaceFrameEpoch ||
            viewportUpdateScheduled
        ) {
            return
        }
        viewportUpdateScheduled = true
        viewportUpdateEpoch = requestedEpoch
        if (!isAttachedToWindow) {
            post {
                if (viewportUpdateEpoch != requestedEpoch) return@post
                viewportUpdateScheduled = false
                viewportUpdateEpoch = 0L
                scheduleViewportUpdate(requestedEpoch)
            }
            return
        }
        viewTreeObserver.addOnPreDrawListener(viewportPreDrawListener)
        invalidate()
    }

    private fun applyViewportTuple() {
        if (!surfaceReady || engine == 0L || width <= 0 || height <= 0) return
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
        val requestedEpoch = surfaceFrameEpoch
        if (!surfaceReady) return
        requestFrame(requestedEpoch)
    }

    /** Posts a frame only for the Surface generation that requested it. */
    private fun requestFrame(requestedEpoch: Long) {
        post {
            if (
                !surfaceReady || requestedEpoch != surfaceFrameEpoch ||
                frameScheduled || engine == 0L
            ) {
                return@post
            }
            frameScheduled = true
            scheduledFrameEpoch = requestedEpoch
            choreographer.postFrameCallback(frameCallback)
        }
    }

    private val frameCallback = Choreographer.FrameCallback { frameTimeNanos ->
        val requestedEpoch = scheduledFrameEpoch
        frameScheduled = false
        scheduledFrameEpoch = 0L
        if (
            !surfaceReady || requestedEpoch == 0L ||
            requestedEpoch != surfaceFrameEpoch || engine == 0L
        ) {
            return@FrameCallback
        }
        val status = OpNative.nativeFrame(engine, frameTimeNanos / 1_000_000)
        when (status) {
            GPU_ERROR -> recoverGpu()
            0 -> {
                shellBridge.syncSystemChromeAppearance()
                ime.sync()
                shellBridge.drainCopyText()
                shellBridge.pollShellAction()
                observeBackgroundGeneration(allowPermissionPrompt = true)
            }
        }
    }

    /** Final active-work probe before MainActivity leaves the foreground. */
    fun prepareForBackground() {
        observeBackgroundGeneration(allowPermissionPrompt = false)
    }

    private fun observeBackgroundGeneration(allowPermissionPrompt: Boolean) {
        val current = engine
        if (current == 0L) return
        if (BackgroundGenerationController.observeForeground(context, current)) {
            backgroundPermissionPromptPending = true
        }
        if (allowPermissionPrompt && backgroundPermissionPromptPending) {
            backgroundPermissionPromptPending = false
            backgroundWorkActivationHandler?.invoke()
        }
    }

    /**
     * One suspend → resume recovery per surface generation on a GpuError.
     * A surfaceDestroyed that raced in wins (the surface is invalid → drop
     * the recovery); a spent budget stops the pump and lets the engine's
     * onRuntimeError report it.
     */
    private fun recoverGpu() {
        // Invalidate every queued redraw before inspecting the recovery
        // target. An invalid/replaced Surface must leave the gate closed.
        closeSurfaceFrameGate()
        BackgroundGenerationController.markSurfaceResuming(context, engine)
        val gen = surfaceGeneration
        val surface = holder.surface
        if (gen == lastRecoveredGeneration) {
            Log.w(TAG, "GpuError with recovery budget spent (generation $gen) — pump stopped")
            cancelStreamsBeforeSuspend()
            OpNative.nativeSuspend(engine)
            BackgroundGenerationController.markSurfaceSuspended(engine)
            return
        }
        if (surface == null || !surface.isValid) {
            cancelStreamsBeforeSuspend()
            OpNative.nativeSuspend(engine)
            BackgroundGenerationController.markSurfaceSuspended(engine)
            return // raced surfaceDestroyed wins
        }
        lastRecoveredGeneration = gen
        Log.i(TAG, "GpuError → suspend/resume recovery (generation $gen)")
        cancelStreamsBeforeSuspend()
        OpNative.nativeSuspend(engine)
        val resumed = surface.isValid && OpNative.nativeResume(engine, surface) == 0
        if (resumed) {
            openSurfaceFrameGate()
            requestFrame()
        } else {
            BackgroundGenerationController.markSurfaceSuspended(engine)
        }
    }

    /** Main-thread close for every path that may release the native surface. */
    private fun closeSurfaceFrameGate() {
        surfaceReady = false
        surfaceFrameEpoch = nextSurfaceFrameEpoch()
        if (frameScheduled) {
            choreographer.removeFrameCallback(frameCallback)
            frameScheduled = false
        }
        scheduledFrameEpoch = 0L
        if (viewTreeObserver.isAlive) {
            viewTreeObserver.removeOnPreDrawListener(viewportPreDrawListener)
        }
        viewportUpdateScheduled = false
        viewportUpdateEpoch = 0L
    }

    /** Opens a fresh generation only after attach/resume returned success. */
    private fun openSurfaceFrameGate() {
        surfaceFrameEpoch = nextSurfaceFrameEpoch()
        surfaceReady = true
    }

    private fun nextSurfaceFrameEpoch(): Long =
        if (surfaceFrameEpoch == Long.MAX_VALUE) 1L else surfaceFrameEpoch + 1L

    /** Captures the epoch so a delayed wake cannot target a replacement Surface. */
    private fun scheduleFrameForEpoch(delayMs: Long, requestedEpoch: Long) {
        if (delayMs <= 0) {
            requestFrame(requestedEpoch)
        } else {
            postDelayed({ requestFrame(requestedEpoch) }, delayMs)
        }
    }

    /** Schedules a frame `delayMs` from now (the engine's next animation wake). */
    fun scheduleFrame(delayMs: Long) {
        val requestedEpoch = surfaceFrameEpoch
        if (!surfaceReady) return
        scheduleFrameForEpoch(delayMs, requestedEpoch)
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
            return editorTouch.editorTouch(event)
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

    /** Floating "Paste" menu at view coordinates (px) over the focused
     *  engine text input. Returns false to fall back to the engine's
     *  long-press (right-click) path. */
    internal fun showPasteMenuIfEditingText(xPx: Float, yPx: Float): Boolean {
        if (!editorMode || engine == 0L) return false
        if (!OpNative.nativeEditorImeFocused(engine)) return false
        return EditorPasteMenu.show(this, xPx, yPx) { text ->
            val current = engine
            if (current != 0L && text.isNotEmpty()) {
                OpNative.nativeEditorPasteText(current, text)
                requestFrame()
            }
        }
    }

    /** Replaces the document atomically in the engine and schedules its first frame. */
    fun openDocument(bytes: ByteArray, displayName: String): Int =
        shellBridge.openDocument(bytes, displayName)

    /** Returns one platform-picked image/SVG to Rust, then repaints success or rejection. */
    fun importImageOrSvg(bytes: ByteArray, displayName: String): Int =
        shellBridge.importImageOrSvg(bytes, displayName)

    /** Editor-mode IME sync kept in sync with the engine's focus each frame. */
    fun syncIme() {
        ime.sync()
    }

    internal fun imeOwnedByOverlay(): Boolean = imeOwnedByOverlay

    /**
     * Overlay visibility gate for [OpSurfaceViewImeCoordinator]: while an
     * Activity overlay with its own EditTexts is up, per-frame IME sync must
     * neither hide the overlay's keyboard nor pull focus back to this view.
     * On release, re-latch and reconcile with the engine's focus on the next
     * frame (show canvas IME again, or hide a keyboard the overlay left up).
     */
    fun setImeOwnedByOverlay(owned: Boolean) {
        if (imeOwnedByOverlay == owned) return
        imeOwnedByOverlay = owned
        if (!owned) {
            ime.retryAfterConfiguration()
            requestFrame()
        }
    }

    fun destroy() {
        shellBridge.releaseHandlers()
        backgroundWorkActivationHandler = null
        backgroundPermissionPromptPending = false
        editorTouch.detach()
        ime.teardown()
        closeSurfaceFrameGate()
        if (viewTreeObserver.isAlive) {
            viewTreeObserver.removeOnPreDrawListener(viewportPreDrawListener)
        }
        if (engine != 0L) {
            // onDestroy can race ahead of SurfaceHolder.surfaceDestroyed on
            // aggressive "don't keep activities" / OEM paths. Suspend here
            // as an idempotent barrier before a service-retained lease loses
            // its last View, otherwise the background pump gate never opens.
            cancelStreamsBeforeSuspend()
            OpNative.nativeSuspend(engine)
            BackgroundGenerationController.markSurfaceSuspended(engine)
            BackgroundGenerationController.releaseView(context, engine, this)
            engine = 0L
        }
    }
}
