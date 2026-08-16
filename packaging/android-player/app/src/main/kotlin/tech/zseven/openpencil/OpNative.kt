package tech.zseven.openpencil

import android.view.Surface

/**
 * The C-ABI engine surface, marshalled by `crates/op-engine-jni`. Every
 * `external` method dispatches onto the engine's dedicated thread; a
 * closed/unknown handle returns [STATUS_CLOSING]. Loaded from
 * `libop_engine_jni.so` (packaged into jniLibs by `cargo ndk`).
 */
object OpNative {
    const val STATUS_CLOSING = -1
    const val SHELL_ACTION_NONE = 0
    const val SHELL_ACTION_OPEN_DOCUMENT = 1
    const val SHELL_ACTION_OPEN_LOGIN_WEBVIEW = 2
    const val SHELL_ACTION_CLOSE_LOGIN_WEBVIEW = 3
    const val SHELL_ACTION_EXPORT_DOCUMENT = 4

    init {
        System.loadLibrary("op_engine_jni")
    }

    external fun nativeCreate(
        doc: ByteArray,
        w: Float,
        h: Float,
        dpr: Float,
        receiver: OpCallbacks,
        storageRoot: String,
        mode: Int, // 0 = viewer, 1 = full editor
    ): Long // 0 = failure

    external fun nativeLastError(engine: Long): String
    external fun nativeAttachSurface(engine: Long, surface: Surface): Int
    external fun nativeSuspend(engine: Long): Int // blocking barrier
    external fun nativeResume(engine: Long, surface: Surface?): Int
    external fun nativeResize(engine: Long, w: Float, h: Float, dpr: Float): Int
    external fun nativeResizeWithSafeArea(
        engine: Long,
        w: Float,
        h: Float,
        dpr: Float,
        t: Float,
        r: Float,
        b: Float,
        l: Float,
    ): Int
    external fun nativeSetSafeArea(engine: Long, t: Float, r: Float, b: Float, l: Float): Int
    external fun nativeSetKeyboard(engine: Long, h: Float): Int
    external fun nativePrefersLightSystemIcons(engine: Long): Boolean
    external fun nativeFrame(engine: Long, tMs: Long): Int // blocking barrier; the TRUE frame status
    external fun nativePointer(engine: Long, id: Int, phase: Int, x: Float, y: Float, tMs: Long): Int
    external fun nativeDestroy(engine: Long)

    // ---- Text editing + IME ----------------------------------------------

    external fun nativeTextBegin(engine: Long, nodeId: String): Int
    external fun nativeTextEnd(engine: Long): Int
    external fun nativeTextInsert(engine: Long, text: String): Int
    external fun nativeTextBackspace(engine: Long): Int
    external fun nativeTextDeleteForward(engine: Long): Int
    external fun nativeTextSetCaret(engine: Long, offset: Int, extend: Boolean): Int
    external fun nativeTextSelectRange(engine: Long, anchor: Int, focus: Int): Int
    external fun nativeImeSetComposingText(engine: Long, text: String, selStart: Int, selEnd: Int): Int
    external fun nativeImeCommitComposition(engine: Long): Int
    external fun nativeImeCancelComposition(engine: Long): Int
    external fun nativeTextGetState(engine: Long, out: OpTextState): Int
    external fun nativeTextCaretRect(engine: Long, out: FloatArray): Int

    // ---- Remote images / fonts / pages -----------------------------------

    external fun nativeRemoteImageResult(engine: Long, requestId: Long, bytes: ByteArray?): Int
    external fun nativeRegisterFont(engine: Long, bytes: ByteArray): Int
    external fun nativeGetPageCount(engine: Long): Int // or STATUS_CLOSING
    external fun nativeSetActivePage(engine: Long, index: Int): Int

    // ---- Full-editor mode (desktop chrome) -------------------------------

    external fun nativeEditorPress(engine: Long, x: Float, y: Float): Int
    external fun nativeEditorMove(engine: Long, x: Float, y: Float): Int
    external fun nativeEditorRelease(engine: Long, x: Float, y: Float): Int
    external fun nativeEditorCancelGesture(engine: Long): Int
    external fun nativeEditorBeginTransform(engine: Long, x: Float, y: Float): Int
    external fun nativeEditorRightPress(engine: Long, x: Float, y: Float): Int
    external fun nativeEditorPan(engine: Long, x: Float, y: Float, dx: Float, dy: Float): Int
    external fun nativeEditorPinch(engine: Long, x: Float, y: Float, deltaY: Float): Int
    external fun nativeEditorText(engine: Long, text: String): Int
    external fun nativeEditorKey(engine: Long, key: Int): Int
    external fun nativeEditorImePreedit(engine: Long, text: String, selStart: Int, selEnd: Int): Int
    external fun nativeEditorImeCommit(engine: Long, text: String): Int
    external fun nativeEditorImeFocused(engine: Long): Boolean
    external fun nativeEditorConfigureAuth(
        engine: Long,
        storageDir: String,
        deviceName: String,
        appVersion: String,
    ): Int
    external fun nativeEditorTakeLoginUrl(engine: Long): String?
    external fun nativeEditorCancelLogin(engine: Long): Int
    external fun nativeEditorTakeShellAction(engine: Long): Int
    external fun nativeEditorOpenDocument(engine: Long, bytes: ByteArray, name: String): Int
    external fun nativeEditorExportFileName(engine: Long): String?
    external fun nativeEditorExportToPath(engine: Long, path: String): Int
    external fun nativeEditorCancelExport(engine: Long): Int
}

/** Editor key codes (mirror `KEY_*` in op-engine-ffi/src/editor.rs). */
object OpKeys {
    const val BACKSPACE = 1
    const val DELETE = 2
    const val ENTER = 3
    const val ESCAPE = 4
    const val DUPLICATE = 5
    const val UNDO = 6
    const val REDO = 7
    const val ARROW_UP = 9
    const val ARROW_DOWN = 10
    const val ARROW_LEFT = 11
    const val ARROW_RIGHT = 12
}

/** Owned surrounding-text snapshot filled by [OpNative.nativeTextGetState]. */
class OpTextState {
    @JvmField var text = ""
    @JvmField var selectionStart = 0
    @JvmField var selectionEnd = 0
    @JvmField var hasComposing = false
    @JvmField var composingStart = 0
    @JvmField var composingEnd = 0
}
