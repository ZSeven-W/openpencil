package tech.zseven.openpencil

import android.os.SystemClock
import android.view.MotionEvent

/** Long-press delay (ms) before a right-click context menu. */
internal const val LONG_PRESS_MS = 500L
/** Movement (logical dp) that cancels a long-press candidate. */
internal const val LONG_PRESS_SLOP = 8f

/**
 * The platform cancel clock for the editor ABI: `MotionEvent.eventTime`
 * is NOT trustworthy on ACTION_CANCEL / synthetic cancels the shell
 * fabricates itself (two-finger takeover, long-press paste, geometry
 * transitions), so every synthetic Cancel uses the same monotonic
 * boot-uptime clock [SystemClock.uptimeMillis] — the same domain as
 * MotionEvent.eventTime, so the engine's global clock never sees a
 * bridge between two time domains.
 */
internal fun uptimeClockMs(): Long = SystemClock.uptimeMillis()

/**
 * Editor-mode touch state machine, split out of [OpSurfaceView] verbatim:
 * press/move/release streaming for the primary pointer, long-press
 * arming/firing (right-click or paste menu), and the two-finger pan +
 * pinch takeover. All engine calls mirror `OpSurfaceView.editorTouch`
 * exactly; gesture interpretation itself lives in the engine.
 */
internal class OpSurfaceViewEditorTouch(private val view: OpSurfaceView) {

    private var primaryPointerId = -1
    private var longPressArmed = false
    private var longPressFired = false
    private val longPressRunnable = Runnable { fireLongPress() }
    private var lastMidX = 0f
    private var lastMidY = 0f
    private var lastPinchDist = 0f
    private var twoFingerActive = false
    private var editorReleaseSuppressed = false
    private var lastKnownX = 0f
    private var lastKnownY = 0f
    private var downX = 0f
    private var downY = 0f

    fun editorTouch(event: MotionEvent): Boolean {
        val inputDensity = view.committedInputDensity
        when (event.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                primaryPointerId = event.getPointerId(0)
                longPressArmed = true
                longPressFired = false
                lastKnownX = event.x
                lastKnownY = event.y
                downX = event.x / inputDensity
                downY = event.y / inputDensity
                view.postDelayed(longPressRunnable, LONG_PRESS_MS)
                OpNative.nativeEditorPressAt(
                    view.editorEngine(),
                    event.x / inputDensity,
                    event.y / inputDensity,
                    event.eventTime,
                )
                view.settleEditorPressFlow()
            }
            MotionEvent.ACTION_POINTER_DOWN -> {
                if (event.pointerCount == 2) {
                    // Two fingers: pan + pinch take over.
                    longPressArmed = false
                    view.removeCallbacks(longPressRunnable)
                    // The first pointer already entered the editor press
                    // ladder. Cancel that capture before multi-touch starts
                    // so no marquee/node drag survives the takeover.
                    OpNative.nativeEditorCancelGestureAt(
                        view.editorEngine(),
                        uptimeClockMs(),
                    )
                    editorReleaseSuppressed = true
                    twoFingerActive = true
                    val (midX, midY) = midpoint(event)
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = distance(event)
                    OpNative.nativeEditorBeginTransform(
                        view.editorEngine(),
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
                    val pinchDelta = PinchZoomDelta.wheelDelta(
                        previousDistance = lastPinchDist,
                        currentDistance = dist,
                    )
                    lastMidX = midX
                    lastMidY = midY
                    lastPinchDist = dist
                    if (dx != 0f || dy != 0f) {
                        OpNative.nativeEditorPan(
                            view.editorEngine(),
                            midX / inputDensity,
                            midY / inputDensity,
                            dx,
                            dy,
                        )
                    }
                    if (pinchDelta != 0f) {
                        OpNative.nativeEditorPinch(
                            view.editorEngine(),
                            midX / inputDensity,
                            midY / inputDensity,
                            pinchDelta,
                        )
                    }
                    view.requestFrame()
                } else if (primaryPointerId >= 0) {
                    val index = event.findPointerIndex(primaryPointerId)
                    if (index >= 0) {
                        val x = event.getX(index) / inputDensity
                        val y = event.getY(index) / inputDensity
                        lastKnownX = event.getX(index)
                        lastKnownY = event.getY(index)
                        OpNative.nativeEditorMoveAt(view.editorEngine(), x, y, event.eventTime)
                        // Movement cancels the long-press candidate.
                        if (longPressArmed) {
                            val deltaX = x - downX
                            val deltaY = y - downY
                            if (deltaX * deltaX + deltaY * deltaY >
                                LONG_PRESS_SLOP * LONG_PRESS_SLOP
                            ) {
                                longPressArmed = false
                                view.removeCallbacks(longPressRunnable)
                            }
                        }
                        view.requestFrame()
                    }
                }
            }
            MotionEvent.ACTION_POINTER_UP -> {
                if (twoFingerActive) {
                    // End transform ownership before the remaining pointer is
                    // re-armed; its eventual Up must never release the press
                    // ladder cancelled at second-finger Down.
                    OpNative.nativeEditorCancelGestureAt(
                        view.editorEngine(),
                        uptimeClockMs(),
                    )
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
                view.removeCallbacks(longPressRunnable)
                if (twoFingerActive) {
                    OpNative.nativeEditorCancelGestureAt(
                        view.editorEngine(),
                        uptimeClockMs(),
                    )
                    twoFingerActive = false
                } else if (!longPressFired && !editorReleaseSuppressed) {
                    val x = event.x / inputDensity
                    val y = event.y / inputDensity
                    OpNative.nativeEditorReleaseAt(view.editorEngine(), x, y, event.eventTime)
                }
                resetTracking()
                view.requestFrame()
            }
            MotionEvent.ACTION_CANCEL -> {
                // A platform cancellation must never run the release ladder:
                // release may commit a deferred tap, drag/drop, or history.
                // ACTION_CANCEL is a real MotionEvent, so it carries its own
                // eventTime; only the shell-fabricated cancels below fall back
                // to SystemClock.uptimeMillis().
                OpNative.nativeEditorCancelGestureAt(view.editorEngine(), event.eventTime)
                resetTracking()
                view.requestFrame()
            }
            else -> return false
        }
        return true
    }

    fun resetTracking() {
        view.removeCallbacks(longPressRunnable)
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

    /** Drops only the pending long-press timer (teardown path). */
    fun detach() {
        view.removeCallbacks(longPressRunnable)
    }

    private fun fireLongPress() {
        longPressArmed = false
        if (!view.isFrameGateOpen || view.editorEngine() == 0L) return
        longPressFired = true
        val inputDensity = view.committedInputDensity
        // The Down at press time already ran the engine's press ladder, so
        // the engine's IME focus reflects THIS touch: focused means the
        // finger is holding an editable text field — offer Paste instead of
        // the right-click context menu.
        if (view.showPasteMenuIfEditingText(lastKnownX, lastKnownY)) {
            // The press capture opened at Down must not leak while the
            // release is suppressed by longPressFired.
            OpNative.nativeEditorCancelGestureAt(view.editorEngine(), uptimeClockMs())
        } else {
            OpNative.nativeEditorRightPress(
                view.editorEngine(),
                lastKnownX / inputDensity,
                lastKnownY / inputDensity,
            )
        }
        view.requestFrame()
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
}
