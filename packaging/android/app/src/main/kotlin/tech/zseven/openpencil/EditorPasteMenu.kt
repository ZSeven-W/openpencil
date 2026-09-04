package tech.zseven.openpencil

import android.content.ClipDescription
import android.content.ClipboardManager
import android.content.Context
import android.graphics.drawable.GradientDrawable
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.PopupWindow
import android.widget.TextView

/**
 * The long-press "Paste" bubble over an engine-painted text input.
 *
 * The engine owns the text, so the menu offers exactly the action the shell
 * can deliver through the ABI: reading the platform clipboard and forwarding
 * it via `OpNative.nativeEditorPasteText` into whichever input holds the IME.
 * Mirrors the iOS shell's `UIEditMenuInteraction` Paste action.
 */
object EditorPasteMenu {

    /**
     * Shows the bubble anchored just above the touch point (view-local px).
     * Returns false without showing anything when the clipboard has no text,
     * so the caller can fall back to the engine's long-press path.
     */
    fun show(anchor: View, xPx: Float, yPx: Float, onPaste: (String) -> Unit): Boolean {
        val context = anchor.context
        val clipboard =
            context.getSystemService(Context.CLIPBOARD_SERVICE) as ClipboardManager
        val description = clipboard.primaryClipDescription ?: return false
        val hasText = description.hasMimeType(ClipDescription.MIMETYPE_TEXT_PLAIN) ||
            description.hasMimeType(ClipDescription.MIMETYPE_TEXT_HTML)
        if (!hasText) return false

        val density = context.resources.displayMetrics.density
        val paste = TextView(context).apply {
            setText(android.R.string.paste)
            setTextColor(0xFFFFFFFF.toInt())
            textSize = 14f
            val padH = (16 * density).toInt()
            val padV = (10 * density).toInt()
            setPadding(padH, padV, padH, padV)
            background = GradientDrawable().apply {
                cornerRadius = 8 * density
                setColor(0xFF2B2B2E.toInt())
            }
        }
        val popup = PopupWindow(
            paste,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
            true, // focusable: an outside tap dismisses without reaching the engine
        )
        popup.elevation = 6 * density
        paste.setOnClickListener {
            popup.dismiss()
            // Read the clip only on the user's explicit Paste tap (this is
            // also when Android surfaces its clipboard-access toast).
            val text = clipboard.primaryClip
                ?.takeIf { it.itemCount > 0 }
                ?.getItemAt(0)
                ?.coerceToText(context)
                ?.toString()
            if (!text.isNullOrEmpty()) onPaste(text)
        }
        val location = IntArray(2)
        anchor.getLocationInWindow(location)
        val anchorX = location[0] + xPx.toInt()
        val anchorY = location[1] + yPx.toInt() - (56 * density).toInt()
        popup.showAtLocation(anchor, Gravity.NO_GRAVITY, anchorX, anchorY)
        return true
    }
}
