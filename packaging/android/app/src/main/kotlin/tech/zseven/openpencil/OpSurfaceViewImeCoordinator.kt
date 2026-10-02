package tech.zseven.openpencil

import android.content.Context
import android.os.Build
import android.view.KeyEvent
import android.view.inputmethod.InputMethodManager

/**
 * Editor-mode IME coordination, split out of [OpSurfaceView] verbatim: the
 * engine's host owns the IME focus, so after every interaction + frame this
 * keeps the system keyboard in sync — bounded show retries behind an
 * insets-driven visibility latch, hide on unfocus.
 */
internal class OpSurfaceViewImeCoordinator(private val view: OpSurfaceView) {

    private val imm: InputMethodManager =
        view.context.getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

    /** Actual platform visibility, updated only from WindowInsets. */
    private var imeVisible = false
    /** Avoid duplicate requests while Android is accepting a show request. */
    private var imeShowRequestPending = false
    private var imeShowNeeded = false
    private var imeWasFocused = false
    private var imeShowAttempts = 0
    private var imeBackKeyHandled = false
    private val imeBack = if (Build.VERSION.SDK_INT >= 33) {
        OpSurfaceViewImeBack(view) { dismissPlatformKeyboard() }
    } else null

    private val clearImeShowRequest = Runnable {
        imeShowRequestPending = false
        if (imeShowNeeded && imeShowAttempts < 2) view.requestFrame()
    }

    fun sync() {
        if (!view.editorMode() || view.editorEngine() == 0L) return
        // An Android-view overlay (login / registration / account center)
        // owns the IME while visible: its EditTexts show the keyboard, and
        // the engine reports no focused text. Without this gate the very
        // insets dispatch that reports the overlay's keyboard as visible
        // schedules the frame whose sync() hides it again (and requestFocus
        // below would steal view focus from the overlay's fields).
        if (view.imeOwnedByOverlay()) {
            releaseBackDispatcher()
            return
        }
        val focused = OpNative.nativeEditorImeFocused(view.editorEngine())
        imeBack?.setActive(focused && imeVisible)
        if (focused && !imeWasFocused) {
            imeShowNeeded = true
            imeShowAttempts = 0
        }
        imeWasFocused = focused
        if (focused && !imeVisible && imeShowNeeded &&
            !imeShowRequestPending && imeShowAttempts < 2
        ) {
            view.requestFocus()
            // Insets, not this return value, are the visibility source of
            // truth. If Android accepts but never shows (or rotation hides)
            // the IME, a later frame may retry after the short request gate.
            imeShowAttempts++
            imeShowRequestPending = imm.showSoftInput(view, 0)
            view.removeCallbacks(clearImeShowRequest)
            if (imeShowAttempts < 2) {
                // A rejected request also needs a later frame; otherwise no
                // native animation may remain to drive the bounded retry.
                view.postDelayed(clearImeShowRequest, 400L)
            }
        } else if (!focused && imeVisible) {
            imm.hideSoftInputFromWindow(view.windowToken, 0)
        }
        if (!focused) {
            imeShowNeeded = false
            imeShowAttempts = 0
            imeShowRequestPending = false
        }
    }

    /**
     * The WindowInsets-driven half of [OpSurfaceView.updateKeyboard]:
     * records platform visibility and clears the show-request latches,
     * returning whether visibility changed so the caller can skip no-op
     * height commits.
     */
    fun observeKeyboardVisibility(visible: Boolean): Boolean {
        val visibilityChanged = imeVisible != visible
        imeVisible = visible
        imeShowRequestPending = false
        view.removeCallbacks(clearImeShowRequest)
        if (visible) {
            imeShowNeeded = false
            imeShowAttempts = 0
        }
        return visibilityChanged
    }

    /** A rotation can dismiss the IME while native input remains focused;
     *  the next frame retries the show. */
    fun retryAfterConfiguration() {
        imeShowNeeded = true
        imeShowAttempts = 0
        imeShowRequestPending = false
        view.removeCallbacks(clearImeShowRequest)
    }

    /** Legacy Back reaches a View before the IME consumes the navigation key. */
    fun handleBackKey(keyCode: Int, event: KeyEvent): Boolean {
        if (keyCode != KeyEvent.KEYCODE_BACK) return false
        if (event.action == KeyEvent.ACTION_UP && imeBackKeyHandled) {
            imeBackKeyHandled = false
            return true
        }
        if (event.action != KeyEvent.ACTION_DOWN || !imeVisible ||
            !view.editorMode() || view.editorEngine() == 0L || view.imeOwnedByOverlay()
        ) return false
        if (!OpNative.nativeEditorImeDismiss(view.editorEngine())) return false
        imeBackKeyHandled = true
        dismissPlatformKeyboard(focusAlreadyDismissed = true)
        return true
    }

    private fun dismissPlatformKeyboard(focusAlreadyDismissed: Boolean = false) {
        if (view.imeOwnedByOverlay() || view.editorEngine() == 0L) return
        if (!focusAlreadyDismissed) OpNative.nativeEditorImeDismiss(view.editorEngine())
        imeShowNeeded = false
        imeShowAttempts = 0
        imeShowRequestPending = false
        view.removeCallbacks(clearImeShowRequest)
        imm.hideSoftInputFromWindow(view.windowToken, 0)
        view.settleEditorPressFlow()
    }

    fun releaseBackDispatcher() {
        imeBack?.setActive(false)
    }

    fun teardown() {
        view.removeCallbacks(clearImeShowRequest)
        releaseBackDispatcher()
    }
}
