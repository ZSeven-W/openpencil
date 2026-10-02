package tech.zseven.openpencil

import android.view.inputmethod.InputMethodManager
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/** Dismiss the keyboard or one Rust-owned surface; false keeps Android's root Back. */
internal fun OpSurfaceView.handleSystemBack(): Boolean {
    val current = editorEngine()
    if (!editorMode() || current == 0L || imeOwnedByOverlay()) return false
    val imeVisible = ViewCompat.getRootWindowInsets(this)
        ?.isVisible(WindowInsetsCompat.Type.ime()) == true
    // If Back reaches the Activity with an IME still up, first release the
    // engine's focus so the next frame cannot request that keyboard again.
    if (imeVisible && !OpNative.nativeEditorImeFocused(current)) {
        hideSystemKeyboard()
        return true
    }
    val consumed = OpNative.nativeEditorBack(current)
    if (consumed) {
        syncIme()
        settleEditorPressFlow()
    }
    if (imeVisible) {
        hideSystemKeyboard()
    }
    return consumed || imeVisible
}

private fun OpSurfaceView.hideSystemKeyboard() {
    context.getSystemService(InputMethodManager::class.java)
        ?.hideSoftInputFromWindow(windowToken, 0)
}
