package tech.zseven.openpencil

import android.util.Log

private const val TAG = "OpenPencilSaveBridge"

// The engine half of the picker-backed Save / Save As round trip, called by
// DocumentSaveCoordinator.
//
// These are plain JNI passthroughs and would sit on OpSurfaceView next to the
// export ones, except that file is already well past the repo's 800-line cap;
// the HarmonyOS shell splits the same five calls out of EngineHost for the
// same reason. They stay extension functions so the coordinator calls them
// exactly as if they were members.

/** Suggested `<stem>.op` the picker opens pre-filled with. */
fun OpSurfaceView.saveFileName(): String? {
    val current = engine
    if (!editorMode() || current == 0L) return null
    return OpNative.nativeEditorSaveFileName(current)
}

/** Bound destination URI a plain Save rewrites; null = show the picker. */
fun OpSurfaceView.saveTarget(): String? {
    val current = engine
    if (!editorMode() || current == 0L) return null
    return OpNative.nativeEditorSaveTarget(current)
}

/** Writes the document's canonical `.op` bytes into a new staging file. */
fun OpSurfaceView.stageSaveToPath(path: String): Int {
    val current = engine
    if (!editorMode() || current == 0L) return OpNative.STATUS_CLOSING
    return OpNative.nativeEditorStageSaveToPath(current, path)
}

/** The staged bytes reached the picked URI: bind it and mark it saved. */
fun OpSurfaceView.commitSave(handle: String, displayName: String): Int {
    val current = engine
    if (!editorMode() || current == 0L) return OpNative.STATUS_CLOSING
    val status = OpNative.nativeEditorCommitSave(current, handle, displayName)
    if (status == 0) requestFrame()
    return status
}

/** Picker dismissed (`failed = false`) or the copy blew up (`true`). */
fun OpSurfaceView.cancelSave(failed: Boolean) {
    val current = engine
    if (!editorMode() || current == 0L) return
    val status = OpNative.nativeEditorCancelSave(current, failed)
    if (status != 0 && status != OpNative.STATUS_CLOSING) {
        Log.i(TAG, "save cancel returned status=$status")
    }
    requestFrame()
}
