package tech.zseven.openpencil

import android.content.Context
import android.util.Log

private const val TAG = "OpenPencilPlayer"

/**
 * Editor shell bridge, split out of [OpSurfaceView] verbatim: the
 * Activity-owned handlers for engine-emitted shell actions, the login /
 * locale / export / account editor APIs the Activity calls through the
 * View, outbound clipboard draining, and the light-system-icons chrome
 * preference pump. Bodies are unchanged — they run in the View's caller
 * thread exactly as before.
 */
internal class OpSurfaceViewShellBridge(private val view: OpSurfaceView) {

    private var openDocumentHandler: (() -> Unit)? = null
    private var importImageOrSvgHandler: (() -> Unit)? = null
    private var exportDocumentHandler: (() -> Unit)? = null
    private var saveDocumentHandler: (() -> Unit)? = null
    private var accountCenterHandler: (() -> Unit)? = null
    private var requestLoginHandler: (() -> Unit)? = null
    private var languagePickerHandler: (() -> Unit)? = null
    private var openLoginUiHandler: ((String) -> Unit)? = null
    private var closeLoginUiHandler: (() -> Unit)? = null
    private var systemChromeAppearanceHandler: ((Boolean) -> Unit)? = null
    private var prefersLightSystemIcons: Boolean? = null

    /** Registers the main-thread shell action handler owned by the Activity. */
    fun setOpenDocumentHandler(handler: () -> Unit) {
        openDocumentHandler = handler
    }

    /** Registers the Activity-owned image / SVG picker handler. */
    fun setImportImageOrSvgHandler(handler: () -> Unit) {
        importImageOrSvgHandler = handler
    }

    /** Registers the Activity-owned save-UI handler for frozen exports. */
    fun setExportDocumentHandler(handler: () -> Unit) {
        exportDocumentHandler = handler
    }

    /**
     * Registers the Activity-owned Save / Save As picker handler AND tells
     * the engine this shell can present one. Without the declaration the
     * engine keeps painting its own name dialog and writes into the private
     * `documents/` fallback, which Android 11+ hides from the file manager.
     */
    fun setSaveDocumentHandler(handler: () -> Unit) {
        saveDocumentHandler = handler
        val current = view.editorEngine()
        if (view.editorMode() && current != 0L) {
            OpNative.nativeEditorConfigureSavePicker(current, true)
        }
    }

    /** Registers the Activity-owned native account-center handler. */
    fun setAccountCenterHandler(handler: () -> Unit) {
        accountCenterHandler = handler
    }

    /** Registers the Activity-owned sign-in starter (lazy auth configure). */
    fun setRequestLoginHandler(handler: () -> Unit) {
        requestLoginHandler = handler
    }

    /** Registers the Activity-owned native language picker. */
    fun setLanguagePickerHandler(handler: () -> Unit) {
        languagePickerHandler = handler
    }

    /** Registers lifecycle callbacks for the Activity-owned native login UI. */
    fun setLoginUiHandlers(open: (String) -> Unit, close: () -> Unit) {
        openLoginUiHandler = open
        closeLoginUiHandler = close
    }

    /** Registers a main-thread window-chrome updater owned by the Activity. */
    fun setSystemChromeAppearanceHandler(handler: (Boolean) -> Unit) {
        systemChromeAppearanceHandler = handler
    }

    /** User close/back: cancel the single auth flow owned by this engine. */
    fun cancelLogin() {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return
        val status = OpNative.nativeEditorCancelLogin(current)
        if (status != 0 && status != OpNative.STATUS_CLOSING) {
            Log.i(TAG, "login cancel returned status=$status")
        }
        view.requestFrame()
    }

    /** Starts the device flow; returns the raw engine status. */
    fun beginLogin(): Int {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return OpNative.STATUS_CLOSING
        val status = OpNative.nativeEditorBeginLogin(current)
        view.requestFrame()
        return status
    }

    /** Applies a UI locale tag; returns the raw engine status. */
    fun setLocale(tag: String): Int {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return OpNative.STATUS_CLOSING
        val status = OpNative.nativeEditorSetLocale(current, tag)
        view.requestFrame()
        return status
    }

    /** The current UI locale's BCP-47 tag, if readable. */
    fun localeCode(): String? {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return null
        return OpNative.nativeEditorLocaleCode(current)
    }

    /** Copies the engine's JSON account snapshot (never consumed). */
    fun accountSnapshot(): String? {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return null
        return OpNative.nativeEditorAccountSnapshot(current)
    }

    /** Revokes the device session from the native account center. */
    fun signOutAccount() {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return
        val status = OpNative.nativeEditorSignOut(current)
        if (status != 0 && status != OpNative.STATUS_CLOSING) {
            Log.i(TAG, "sign out returned status=$status")
        }
        view.requestFrame()
    }

    /** Copies the frozen export's file name without consuming the artifact. */
    fun exportFileName(): String? {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return null
        return OpNative.nativeEditorExportFileName(current)
    }

    /** Writes the frozen export into a new absolute staging file. */
    fun exportToPath(path: String): Int {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return OpNative.STATUS_CLOSING
        return OpNative.nativeEditorExportToPath(current, path)
    }

    /** Discards the frozen export when the save UI cannot run. */
    fun cancelExport() {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return
        val status = OpNative.nativeEditorCancelExport(current)
        if (status != 0 && status != OpNative.STATUS_CLOSING) {
            Log.i(TAG, "export cancel returned status=$status")
        }
        view.requestFrame()
    }

    /** True while the Activity has registered its Save / Save As picker. */
    fun hasSavePicker(): Boolean = saveDocumentHandler != null

    /** Replays the last engine preference after an in-place configuration. */
    fun replaySystemChromeAppearance() {
        val preference = prefersLightSystemIcons ?: return
        view.post { systemChromeAppearanceHandler?.invoke(preference) }
    }

    /** Outbound clipboard bridge: engine copy buttons (collab invite /
     *  share address, MCP config, chat copy) queue text that the desktop
     *  drains into the OS clipboard; here the same queue lands on the
     *  Android clipboard. NotReady (null) is the common per-frame case. */
    fun drainCopyText() {
        if (!view.editorMode() || view.editorEngine() == 0L) return
        val text = OpNative.nativeEditorTakeCopyText(view.editorEngine()) ?: return
        val clipboard =
            view.context.getSystemService(Context.CLIPBOARD_SERVICE) as android.content.ClipboardManager
        clipboard.setPrimaryClip(android.content.ClipData.newPlainText("OpenPencil", text))
    }

    /** Poll after a successful frame so a theme toggle and its icon contrast
     *  are presented together. JNI errors/closing use a light-surface-safe
     *  false fallback; the main-thread window update is value-deduplicated. */
    fun syncSystemChromeAppearance() {
        if (view.editorEngine() == 0L) return
        val next = OpNative.nativePrefersLightSystemIcons(view.editorEngine())
        if (prefersLightSystemIcons == next) return
        prefersLightSystemIcons = next
        view.post { systemChromeAppearanceHandler?.invoke(next) }
    }

    /**
     * Shell actions are consumed only after a blocking JNI call has returned.
     * Posting the handler prevents the Activity Result launcher from running
     * inside an engine callback or touch dispatch stack.
     */
    fun pollShellAction() {
        if (!view.editorMode() || view.editorEngine() == 0L) return
        val action = OpNative.nativeEditorTakeShellAction(view.editorEngine())
        when {
            action < 0 || action == OpNative.SHELL_ACTION_NONE -> Unit
            action == OpNative.SHELL_ACTION_OPEN_DOCUMENT -> view.post {
                if (view.editorEngine() != 0L) openDocumentHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_IMPORT_IMAGE_OR_SVG -> view.post {
                if (view.editorEngine() != 0L) importImageOrSvgHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_EXPORT_DOCUMENT -> view.post {
                if (view.editorEngine() != 0L) exportDocumentHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_SAVE_DOCUMENT -> view.post {
                if (view.editorEngine() != 0L) saveDocumentHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_OPEN_ACCOUNT_CENTER -> view.post {
                if (view.editorEngine() != 0L) accountCenterHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_REQUEST_LOGIN -> view.post {
                if (view.editorEngine() != 0L) requestLoginHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_OPEN_LANGUAGE_PICKER -> view.post {
                if (view.editorEngine() != 0L) languagePickerHandler?.invoke()
            }
            action == OpNative.SHELL_ACTION_OPEN_LOGIN_WEBVIEW -> {
                val url = OpNative.nativeEditorTakeLoginUrl(view.editorEngine())
                if (url.isNullOrBlank()) {
                    Log.w(TAG, "login action had no URL; canceling the flow")
                    OpNative.nativeEditorCancelLogin(view.editorEngine())
                } else {
                    view.post {
                        if (view.editorEngine() != 0L) openLoginUiHandler?.invoke(url)
                    }
                }
            }
            action == OpNative.SHELL_ACTION_CLOSE_LOGIN_WEBVIEW -> view.post {
                closeLoginUiHandler?.invoke()
            }
            else -> Log.w(TAG, "unknown editor shell action=$action")
        }
    }

    /** Replaces the document atomically in the engine and schedules its first frame. */
    fun openDocument(bytes: ByteArray, displayName: String): Int {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return OpNative.STATUS_CLOSING
        val status = OpNative.nativeEditorOpenDocument(current, bytes, displayName)
        if (status == 0) {
            view.syncIme()
            view.requestFrame()
        }
        return status
    }

    /** Returns one platform-picked image/SVG to Rust, then repaints success or rejection. */
    fun importImageOrSvg(bytes: ByteArray, displayName: String): Int {
        val current = view.editorEngine()
        if (!view.editorMode() || current == 0L) return OpNative.STATUS_CLOSING
        val status = OpNative.nativeEditorImportImageOrSvg(current, bytes, displayName)
        view.requestFrame()
        return status
    }

    /** Nulls every Activity-owned handler at teardown. */
    fun releaseHandlers() {
        openDocumentHandler = null
        importImageOrSvgHandler = null
        exportDocumentHandler = null
        saveDocumentHandler = null
        accountCenterHandler = null
        requestLoginHandler = null
        languagePickerHandler = null
        openLoginUiHandler = null
        closeLoginUiHandler = null
        systemChromeAppearanceHandler = null
    }
}
