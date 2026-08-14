package dev.openpencil.player

import android.text.InputType
import android.view.KeyEvent
import android.view.inputmethod.BaseInputConnection
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection

/**
 * Bridges the system IME to the editor-mode engine. The engine decides
 * WHICH input owns the IME (canvas text, property field, chat, rename…)
 * via its focus ladder; this connection just forwards the platform
 * composing/commit events through `OpNative`.
 */
class OpInputConnection(
    private val view: OpSurfaceView,
    private val engine: Long,
) : BaseInputConnection(view, true) {

    override fun commitText(text: CharSequence?, newCursorPosition: Int): Boolean {
        if (text.isNullOrEmpty()) return true
        OpNative.nativeEditorImeCommit(engine, text.toString())
        view.requestFrame()
        return true
    }

    override fun setComposingText(text: CharSequence?, newCursorPosition: Int): Boolean {
        if (text == null) return true
        // newCursorPosition is relative to the composing text (UTF-16):
        // N > 0 → L + N - 1, else N (see the engine's IME docs).
        val len = text.length
        val cursor = if (newCursorPosition > 0) len + newCursorPosition - 1 else newCursorPosition
        OpNative.nativeEditorImePreedit(engine, text.toString(), cursor, cursor)
        view.requestFrame()
        return true
    }

    override fun finishComposingText(): Boolean {
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
        when (event.keyCode) {
            KeyEvent.KEYCODE_DEL -> OpNative.nativeEditorKey(engine, OpKeys.BACKSPACE)
            KeyEvent.KEYCODE_FORWARD_DEL -> OpNative.nativeEditorKey(engine, OpKeys.DELETE)
            KeyEvent.KEYCODE_ENTER -> OpNative.nativeEditorKey(engine, OpKeys.ENTER)
            KeyEvent.KEYCODE_ESCAPE -> OpNative.nativeEditorKey(engine, OpKeys.ESCAPE)
            KeyEvent.KEYCODE_DPAD_UP -> OpNative.nativeEditorKey(engine, OpKeys.ARROW_UP)
            KeyEvent.KEYCODE_DPAD_DOWN -> OpNative.nativeEditorKey(engine, OpKeys.ARROW_DOWN)
            KeyEvent.KEYCODE_DPAD_LEFT -> OpNative.nativeEditorKey(engine, OpKeys.ARROW_LEFT)
            KeyEvent.KEYCODE_DPAD_RIGHT -> OpNative.nativeEditorKey(engine, OpKeys.ARROW_RIGHT)
            else -> {
                val ch = event.unicodeChar
                if (ch != 0) {
                    OpNative.nativeEditorText(engine, ch.toChar().toString())
                } else {
                    return super.sendKeyEvent(event)
                }
            }
        }
        view.requestFrame()
        return true
    }
}
