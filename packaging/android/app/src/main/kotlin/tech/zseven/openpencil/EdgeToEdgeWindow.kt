package tech.zseven.openpencil

import android.graphics.Color
import android.os.Build
import android.view.View
import android.view.Window
import android.view.WindowManager
import androidx.core.view.ViewCompat
import androidx.core.view.WindowCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Extends the editor backdrop below system UI without moving any chrome into
 * the status bar, navigation controls, or a display cutout.
 */
@Suppress("DEPRECATION") // Required for edge-to-edge behavior below Android 15.
internal fun configureEdgeToEdge(window: Window) {
    WindowCompat.setDecorFitsSystemWindows(window, false)
    window.clearFlags(
        WindowManager.LayoutParams.FLAG_TRANSLUCENT_STATUS or
            WindowManager.LayoutParams.FLAG_TRANSLUCENT_NAVIGATION,
    )
    window.addFlags(WindowManager.LayoutParams.FLAG_DRAWS_SYSTEM_BAR_BACKGROUNDS)
    window.statusBarColor = Color.TRANSPARENT
    window.navigationBarColor = Color.TRANSPARENT
    // The first frame has not resolved the document/editor theme yet. Start
    // from the viewer's neutral light surface; the frame pump updates this
    // only when the engine-reported preference changes.
    updateSystemChromeAppearance(window, prefersLightIcons = false)

    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
        window.attributes = window.attributes.apply {
            layoutInDisplayCutoutMode =
                WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES
        }
        window.navigationBarDividerColor = Color.TRANSPARENT
    }
    if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.Q) {
        // The editor paints a dark, high-contrast background itself. Android's
        // default three-button scrim would otherwise look like a separate bar.
        window.isStatusBarContrastEnforced = false
        window.isNavigationBarContrastEnforced = false
    }
}

/** Keeps system icons legible over the theme-colored edge-to-edge bands. */
internal fun updateSystemChromeAppearance(window: Window, prefersLightIcons: Boolean) {
    // Android's "light bars" naming means dark foreground icons. Invert the
    // content preference so a dark editor gets light icons and vice versa.
    val useDarkIcons = !prefersLightIcons
    WindowCompat.getInsetsController(window, window.decorView).apply {
        isAppearanceLightStatusBars = useDarkIcons
        isAppearanceLightNavigationBars = useDarkIcons
    }
    // Covers the short startup interval before the first GPU frame presents.
    window.decorView.setBackgroundColor(
        if (prefersLightIcons) Color.BLACK else Color.rgb(245, 245, 247),
    )
}

/**
 * Sends stable system/cutout insets and transient IME occlusion to the engine.
 * The root remains full-window; only the engine's safe-area-local chrome moves.
 */
internal fun installEditorInsets(root: View, surface: OpSurfaceView) {
    ViewCompat.setOnApplyWindowInsetsListener(root) { _, insets ->
        // Read current metrics on every dispatch. Density is handled in-place
        // by the Activity and may change while this View hierarchy survives.
        val density = root.resources.displayMetrics.density
            .takeIf { it.isFinite() && it > 0f }
            ?: 1f
        val safe = insets.getInsets(
            WindowInsetsCompat.Type.systemBars() or
                WindowInsetsCompat.Type.displayCutout(),
        )
        surface.updateSafeAreaPx(safe.top, safe.right, safe.bottom, safe.left)

        val imeType = WindowInsetsCompat.Type.ime()
        val imeVisible = insets.isVisible(imeType)
        val imeBottom = if (imeVisible) {
            insets.getInsets(imeType).bottom
        } else {
            // Some OEMs retain the previous IME inset during dismissal.
            0
        }
        surface.updateKeyboard(imeBottom / density, imeVisible)

        // Do not consume the insets: future overlays and accessibility views
        // in this root must still be able to observe the platform safe area.
        insets
    }
    ViewCompat.requestApplyInsets(root)
}
