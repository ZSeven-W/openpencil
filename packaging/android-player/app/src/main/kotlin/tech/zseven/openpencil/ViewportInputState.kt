package tech.zseven.openpencil

/**
 * Keeps touch conversion on the same DPR as the last committed native
 * viewport. Resource density may change before the atomic viewport call runs;
 * input remains blocked until that call succeeds and publishes both together.
 */
internal class ViewportInputState(initialDensity: Float) {
    var pendingDensity: Float = validDensity(initialDensity)
        private set
    var committedDensity: Float = pendingDensity
        private set
    var inputBlocked: Boolean = true
        private set
    private var suppressPhysicalStream = false

    fun stageDensity(density: Float): Boolean {
        val next = validDensity(density)
        if (pendingDensity == next) return false
        pendingDensity = next
        return true
    }

    /** Returns true when a previously live gesture stream must be cancelled. */
    fun beginGeometryUpdate(): Boolean {
        val cancelActiveStream = !inputBlocked
        inputBlocked = true
        suppressPhysicalStream = true
        return cancelActiveStream
    }

    fun commitIfSuccessful(status: Int): Boolean {
        if (status != 0) return false
        committedDensity = pendingDensity
        inputBlocked = false
        return true
    }

    /**
     * Filters the tail of the physical stream that was active when geometry
     * changed. Commit may unblock coordinates, but only a fresh primary Down
     * after all old pointers lifted may start a new native gesture.
     */
    fun acceptsTouch(actionMasked: Int, pointerCount: Int): Boolean {
        if (inputBlocked) return false
        if (!suppressPhysicalStream) return true

        if (actionMasked == ACTION_UP || actionMasked == ACTION_CANCEL) {
            suppressPhysicalStream = false
            return false
        }
        // Some devices deliver a fresh stream as ACTION_DOWN without a prior
        // terminal callback after a window/configuration interruption.
        if (actionMasked == ACTION_DOWN && pointerCount == 1) {
            suppressPhysicalStream = false
            return true
        }
        return false
    }

    companion object {
        internal const val ACTION_DOWN = 0
        internal const val ACTION_UP = 1
        internal const val ACTION_CANCEL = 3

        private fun validDensity(value: Float): Float =
            value.takeIf { it.isFinite() && it > 0f } ?: 1f
    }
}
