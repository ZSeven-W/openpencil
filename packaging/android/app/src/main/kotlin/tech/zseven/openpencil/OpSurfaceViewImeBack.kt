package tech.zseven.openpencil

import android.window.OnBackInvokedCallback
import android.window.OnBackInvokedDispatcher
import androidx.annotation.RequiresApi

/** Intercept gesture Back only while the engine owns the visible keyboard. */
@RequiresApi(33)
internal class OpSurfaceViewImeBack(
    private val view: OpSurfaceView,
    dismiss: () -> Unit,
) {
    private val callback = OnBackInvokedCallback { dismiss() }
    private var registered: OnBackInvokedDispatcher? = null

    fun setActive(active: Boolean) {
        val next = if (active) view.findOnBackInvokedDispatcher() else null
        if (next === registered) return
        registered?.unregisterOnBackInvokedCallback(callback)
        registered = next
        next?.registerOnBackInvokedCallback(OnBackInvokedDispatcher.PRIORITY_OVERLAY, callback)
    }
}
