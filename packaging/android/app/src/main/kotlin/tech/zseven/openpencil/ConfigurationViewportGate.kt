package tech.zseven.openpencil

/** Result of evaluating a handled configuration change at pre-draw time. */
internal enum class ViewportGateDecision {
    APPLY,
    WAIT_FOR_INSETS,
    RETRY_NEXT_PRE_DRAW,
}

/**
 * Keeps a handled configuration change from publishing mixed viewport state.
 *
 * Rotation and window resizing must wait for the explicitly requested inset
 * redispatch. Density-only changes are allowed to reuse cached physical inset
 * pixels when two traversal-complete samples prove that both View and Surface
 * bounds stayed unchanged. Requiring the second pre-draw prevents the animation
 * phase before a rotation layout from being mistaken for a density-only change.
 */
internal class ConfigurationViewportGate {
    var awaitingInsets: Boolean = false
        private set

    private var startWidthPx = 0
    private var startHeightPx = 0
    private var stablePreDrawSamples = 0

    fun begin(widthPx: Int, heightPx: Int) {
        awaitingInsets = true
        startWidthPx = widthPx
        startHeightPx = heightPx
        stablePreDrawSamples = 0
    }

    fun onInsetsDispatched() {
        awaitingInsets = false
        stablePreDrawSamples = 0
    }

    fun evaluatePreDraw(
        viewWidthPx: Int,
        viewHeightPx: Int,
        surfaceWidthPx: Int,
        surfaceHeightPx: Int,
    ): ViewportGateDecision {
        if (!awaitingInsets) return ViewportGateDecision.APPLY

        val unchangedAfterTraversal =
            viewWidthPx > 0 && viewHeightPx > 0 &&
                viewWidthPx == startWidthPx && viewHeightPx == startHeightPx &&
                surfaceWidthPx == viewWidthPx && surfaceHeightPx == viewHeightPx
        if (!unchangedAfterTraversal) {
            stablePreDrawSamples = 0
            return ViewportGateDecision.WAIT_FOR_INSETS
        }

        stablePreDrawSamples++
        if (stablePreDrawSamples < 2) {
            return ViewportGateDecision.RETRY_NEXT_PRE_DRAW
        }

        // A second traversal-complete sample with identical pixel bounds is
        // the bounded density-only/no-surfaceChanged fallback.
        awaitingInsets = false
        stablePreDrawSamples = 0
        return ViewportGateDecision.APPLY
    }
}
