package tech.zseven.openpencil

import android.text.Editable
import android.text.Selection
import android.text.SpannableStringBuilder
import android.view.KeyEvent
import android.view.inputmethod.BaseInputConnection

/**
 * Bridges the system IME to the editor-mode engine. The engine decides
 * WHICH input owns the IME (canvas text, property field, chat, rename…)
 * via its focus ladder; this connection just forwards the platform
 * composing/commit events through `OpNative`.
 *
 * The engine owns the real text — this connection never mirrors it. That
 * needs two disciplines so deletion always reaches the engine:
 *
 * 1. The editable the IME inspects is a constant one-sentinel buffer
 *    (below). With a truly empty buffer several Chinese IMEs (讯飞 /
 *    百度-style) decide the field has nothing to delete and swallow the
 *    backspace without emitting `deleteSurroundingText` OR `KEYCODE_DEL`
 *    — the engine never hears the key and typed text becomes
 *    undeletable. The sentinel keeps "text before the cursor" non-empty
 *    so every backspace is emitted; the overrides forward it to the
 *    engine and leave the buffer untouched, so it keeps working forever.
 * 2. Every deletion entry point (`deleteSurroundingText`, `sendKeyEvent`
 *    with `KEYCODE_DEL`, composing-region shrink via `setComposingText`,
 *    the cancel spelling `commitText("")`, and `finishComposingText`)
 *    forwards to the engine instead of mutating local state.
 */
class OpInputConnection(
    private val view: OpSurfaceView,
    private val engine: Long,
) : BaseInputConnection(view, true) {

    /** Constant sentinel buffer; see the class docs. Never mutated. */
    private val sentinelEditable: Editable =
        SpannableStringBuilder(SENTINEL).also { Selection.setSelection(it, SENTINEL.length) }

    /** The composing text last forwarded as a preedit, if any. */
    private var composing: String? = null

    override fun getEditable(): Editable = sentinelEditable

    override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        val value = text?.toString().orEmpty()
        if (value.isEmpty()) {
            // `commitText("")` replaces the composing region with nothing —
            // the IME's cancel/deletion spelling. Dropping it used to leave
            // a dangling preedit in the engine.
            if (composing != null) {
                composing = null
                OpNative.nativeEditorImePreedit(engine, "", 0, 0)
                view.requestFrame()
            }
            return true
        }
        composing = null
        OpNative.nativeEditorImeCommit(engine, value)
        view.requestFrame()
        return true
    }

    override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        if (text == null) return true
        val value = text.toString()
        composing = value.ifEmpty { null }
        // newCursorPosition is relative to the composing text (UTF-16):
        // N > 0 → L + N - 1, else N (see the engine's IME docs).
        val cursor = composingCursorUtf16(value, newCursorPosition)
        OpNative.nativeEditorImePreedit(engine, value, cursor, cursor)
        view.requestFrame()
        return true
    }

    override fun finishComposingText(): Boolean {
        // The in-flight composition becomes committed text as-is. Doing
        // nothing here used to strand the preedit whenever the IME ended a
        // composition without an explicit commit (tap outside the candidate
        // bar, focus hand-off, some IMEs' backspace-after-commit path).
        val pending = composing ?: return true
        composing = null
        OpNative.nativeEditorImeCommit(engine, pending)
        view.requestFrame()
        return true
    }

    override fun deleteSurroundingText(beforeLength: Int, afterLength: Int): Boolean {
        repeat(beforeLength) { OpNative.nativeEditorKey(engine, OpKeys.BACKSPACE) }
        repeat(afterLength) { OpNative.nativeEditorKey(engine, OpKeys.DELETE) }
        view.requestFrame()
        return true
    }

    override fun performEditorAction(actionCode: Int): Boolean {
        OpNative.nativeEditorKey(engine, OpKeys.ENTER)
        view.requestFrame()
        return true
    }

    override fun sendKeyEvent(event: KeyEvent): Boolean {
        val key = when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> OpKeys.BACKSPACE
            KeyEvent.KEYCODE_FORWARD_DEL -> OpKeys.DELETE
            KeyEvent.KEYCODE_ENTER -> OpKeys.ENTER
            KeyEvent.KEYCODE_ESCAPE -> OpKeys.ESCAPE
            KeyEvent.KEYCODE_DPAD_UP -> OpKeys.ARROW_UP
            KeyEvent.KEYCODE_DPAD_DOWN -> OpKeys.ARROW_DOWN
            KeyEvent.KEYCODE_DPAD_LEFT -> OpKeys.ARROW_LEFT
            KeyEvent.KEYCODE_DPAD_RIGHT -> OpKeys.ARROW_RIGHT
            else -> null
        }
        // IMEs deliver ACTION_DOWN and ACTION_UP for every key press;
        // forwarding both doubled each keyed edit (two characters deleted
        // per backspace). Auto-repeat arrives as repeated DOWN events, so
        // gating on DOWN loses nothing.
        if (key != null) {
            if (event.action == KeyEvent.ACTION_DOWN) {
                OpNative.nativeEditorKey(engine, key)
                view.requestFrame()
            }
            return true
        }
        val ch = event.unicodeChar
        if (ch != 0) {
            if (event.action == KeyEvent.ACTION_DOWN) {
                OpNative.nativeEditorText(engine, ch.toChar().toString())
                view.requestFrame()
            }
            return true
        }
        return super.sendKeyEvent(event)
    }

    companion object {
        /**
         * Zero-width space: invisible to suggestion engines but non-empty
         * to "is there anything before the cursor" checks. The initial
         * selection reported through `EditorInfo` must match its length.
         */
        const val SENTINEL: String = "\u200B"
    }
}
