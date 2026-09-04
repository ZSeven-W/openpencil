package tech.zseven.openpencil

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.content.res.Configuration
import android.net.Uri
import android.os.Build
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import androidx.core.view.ViewCompat
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.util.Locale
import java.util.UUID

private const val TAG = "OpenPencilPlayer"
// JNI, the UTF-8 ABI copy, and JSON decoding temporarily coexist. Keep the
// picker limit below the engine's generic 256 MiB cap to protect mobile memory.
private const val MAX_DOCUMENT_BYTES = 32L * 1024L * 1024L

private data class DocumentMetadata(
    val displayName: String?,
    val size: Long?,
)

/**
 * Hosts the [OpSurfaceView]. Edge-to-edge so the surface spans the full
 * window; `configChanges` on the activity (manifest) keeps the engine alive
 * across rotation. `onDestroy` detaches the View; active service-owned work
 * retains an adoptable engine lease, while inactive engines tear down.
 */
class MainActivity : ComponentActivity() {

    private lateinit var surfaceView: OpSurfaceView
    private lateinit var rootView: FrameLayout
    private lateinit var nativeLogin: NativeLoginOverlay
    private lateinit var accountCenter: AccountCenterOverlay
    private lateinit var loginBackCallback: OnBackPressedCallback
    private var documentOpenInProgress = false
    private var imageImportInProgress = false
    private var exportStagedFile: File? = null
    private var exportStagingDir: File? = null
    private lateinit var documentSave: DocumentSaveCoordinator
    private var backgroundNotificationPermissionRequested = false

    private val backgroundNotificationPermissionLauncher = registerForActivityResult(
        ActivityResultContracts.RequestPermission(),
    ) { granted ->
        if (!granted) {
            Log.i(TAG, "notification permission denied; FGS remains visible in Task Manager")
        }
    }

    private val openDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) {
            documentOpenInProgress = false
        } else {
            readAndOpenDocument(uri)
        }
    }

    private val imageImportLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) {
            imageImportInProgress = false
        } else {
            readAndImportImageOrSvg(uri)
        }
    }

    private val exportDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        val staged = exportStagedFile
        val uri = result.data?.data
        if (staged == null || result.resultCode != RESULT_OK || uri == null) {
            // The user abandoned the save UI (or it returned nothing usable);
            // the engine artifact was already consumed by the staged write, so
            // only the app-private copy needs to go.
            cleanupExportStaging()
        } else {
            copyStagedExport(staged, uri)
        }
    }

    private val saveDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.StartActivityForResult(),
    ) { result ->
        // A result restored after process death can arrive before onCreate
        // finished wiring the coordinator; there is no save to report then.
        if (!::documentSave.isInitialized) return@registerForActivityResult
        val uri = if (result.resultCode == RESULT_OK) result.data?.data else null
        documentSave.onPickerResult(uri)
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Keep the drawable edge-to-edge, then deliver four-sided safe-area
        // insets to the engine so only interactive chrome avoids system UI.
        configureEdgeToEdge(window)

        val editorMode = intent.getBooleanExtra("editor", true)
        val explicitDocName = intent.getStringExtra("doc")
        val doc = if (editorMode && explicitDocName == null) {
            // An empty editor payload selects the engine's canonical starter document.
            ByteArray(0)
        } else {
            // Preserve bundled-document overrides, and retain the demo fallback
            // for the viewer-only shell.
            val docName = explicitDocName ?: "ppt-demo"
            readAsset("$docName.op") ?: run {
                Toast.makeText(this, R.string.document_open_failed, Toast.LENGTH_SHORT).show()
                finish()
                return
            }
        }
        val fonts = readFontAssets()

        surfaceView = OpSurfaceView(this).apply {
            configure(doc, editorMode, fonts)
            setOpenDocumentHandler(::launchDocumentPicker)
            setImportImageOrSvgHandler(::launchImageOrSvgPicker)
            setExportDocumentHandler(::beginDocumentExport)
            setSystemChromeAppearanceHandler { prefersLightIcons ->
                updateSystemChromeAppearance(window, prefersLightIcons)
            }
            setBackgroundWorkActivationHandler(::requestBackgroundNotificationPermission)
        }
        // Do not pad or resize the SurfaceView: its background should remain
        // visually continuous below transparent system bars. The Rust host
        // offsets only its interactive chrome using the insets below.
        rootView = FrameLayout(this)
        rootView.addView(
            surfaceView,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        // Save / Save As go through the system file picker: nothing the app
        // writes to its own directory is browsable on Android 11+, and `.op`
        // is a document format, not an export format.
        documentSave = DocumentSaveCoordinator(
            context = this,
            surfaceView = surfaceView,
            launchPicker = { intent -> saveDocumentLauncher.launch(intent) },
            onError = { if (!isFinishing && !isDestroyed) showSaveError() },
        )
        surfaceView.setSaveDocumentHandler { documentSave.begin() }

        val regionStore = surfaceView.authRuntime.regionStore()
        loginBackCallback = object : OnBackPressedCallback(false) {
            override fun handleOnBackPressed() {
                if (accountCenter.isVisible) {
                    accountCenter.handleBack()
                } else {
                    nativeLogin.handleBack()
                }
            }
        }
        onBackPressedDispatcher.addCallback(this, loginBackCallback)
        val syncBackCallback = {
            val overlayVisible = nativeLogin.isVisible || accountCenter.isVisible
            loginBackCallback.isEnabled = overlayVisible
            // While either overlay is up, its EditTexts own the IME; stop the
            // editor's per-frame IME sync from hiding their keyboard or
            // stealing focus back to the SurfaceView.
            surfaceView.setImeOwnedByOverlay(overlayVisible)
        }
        nativeLogin = NativeLoginOverlay(
            activity = this,
            root = rootView,
            regionStore = regionStore,
            onCanceled = { surfaceView.cancelLogin() },
            onRequestRejected = {
                Log.w(TAG, "rejected native login request (bad verification URL)")
                surfaceView.cancelLogin()
                Toast.makeText(this, R.string.native_login_load_failed, Toast.LENGTH_SHORT).show()
            },
            onVisibilityChanged = { _ -> syncBackCallback() },
        )
        accountCenter = AccountCenterOverlay(
            activity = this,
            root = rootView,
            regionStore = regionStore,
            onSignOut = { surfaceView.signOutAccount() },
            onVisibilityChanged = { _ -> syncBackCallback() },
        )
        surfaceView.setLoginUiHandlers(
            open = { url -> nativeLogin.show(url) },
            close = { nativeLogin.dismissFromNative() },
        )
        surfaceView.setAccountCenterHandler {
            val snapshot = surfaceView.accountSnapshot()?.let(AccountSnapshot::parse)
            if (snapshot != null) accountCenter.show(snapshot)
        }
        surfaceView.setRequestLoginHandler {
            surfaceView.authRuntime.startLogin(surfaceView) {
                Toast.makeText(this, R.string.native_login_unavailable, Toast.LENGTH_SHORT)
                    .show()
            }
        }
        surfaceView.setLanguagePickerHandler { presentLanguagePicker() }
        setContentView(rootView)

        installEditorInsets(rootView, surfaceView)
    }

    override fun onStart() {
        super.onStart()
        BackgroundGenerationController.setActivityVisible(this, true)
    }

    override fun onPause() {
        // Probe while Android still considers this a foreground-originated
        // transition, so Android 12+ permits starting the FGS before Home or
        // the lock screen removes the visible-activity exemption.
        if (::surfaceView.isInitialized) surfaceView.prepareForBackground()
        super.onPause()
    }

    override fun onStop() {
        BackgroundGenerationController.setActivityVisible(this, false)
        super.onStop()
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        if (!::surfaceView.isInitialized) return
        // Density is a handled config change, so neither the Activity nor the
        // SurfaceView is recreated. Refresh the logical-pixel conversion used
        // by resize and touch, then request fresh cutout/bar/IME insets.
        surfaceView.refreshDisplayMetrics()
        surfaceView.replaySystemChromeAppearance()
        if (::rootView.isInitialized) ViewCompat.requestApplyInsets(rootView)
    }

    override fun onDestroy() {
        if (::nativeLogin.isInitialized) nativeLogin.destroy()
        if (::accountCenter.isInitialized) accountCenter.destroy()
        cleanupExportStaging()
        if (::documentSave.isInitialized) documentSave.cancelForTeardown()
        // Rotation never reaches here thanks to configChanges. Other teardown
        // detaches the View; active background generation retains the engine.
        if (::surfaceView.isInitialized) surfaceView.destroy()
        super.onDestroy()
    }

    private fun requestBackgroundNotificationPermission() {
        if (
            Build.VERSION.SDK_INT < Build.VERSION_CODES.TIRAMISU ||
            backgroundNotificationPermissionRequested ||
            ContextCompat.checkSelfPermission(this, Manifest.permission.POST_NOTIFICATIONS) ==
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        backgroundNotificationPermissionRequested = true
        backgroundNotificationPermissionLauncher.launch(Manifest.permission.POST_NOTIFICATIONS)
    }

    private fun launchDocumentPicker() {
        if (documentOpenInProgress || isFinishing || isDestroyed) return
        documentOpenInProgress = true
        try {
            // .op and .pen have no system-wide MIME registration. Let the
            // provider show documents, then validate the display name and
            // the document bytes in the engine.
            openDocumentLauncher.launch(arrayOf("*/*"))
        } catch (e: Exception) {
            documentOpenInProgress = false
            Log.w(TAG, "could not launch document picker", e)
            showOpenDocumentError()
        }
    }

    private fun launchImageOrSvgPicker() {
        if (imageImportInProgress || isFinishing || isDestroyed) return
        imageImportInProgress = true
        try {
            // OpenDocument uses EXTRA_MIME_TYPES when more than one type is
            // supplied. Keep SVG explicit because it is not included by all
            // providers in their `image/*` filter.
            imageImportLauncher.launch(
                arrayOf("image/png", "image/jpeg", "image/gif", "image/webp", "image/svg+xml"),
            )
        } catch (e: Exception) {
            imageImportInProgress = false
            Log.w(TAG, "could not launch image picker", e)
            showImageImportError()
        }
    }

    /**
     * Bridges one frozen Rust export to the system save UI. Rust writes the
     * payload into an app-private, single-use staging directory so large
     * PNG/PDF exports never cross JNI as a byte array; the SAF create-document
     * result then receives a plain file copy. Every terminal path removes the
     * staging directory.
     */
    private fun beginDocumentExport() {
        if (isFinishing || isDestroyed) return
        // A second shell action must not replace a staged file that the save
        // UI is still copying. Discard only the newly frozen engine request.
        if (exportStagingDir != null) {
            surfaceView.cancelExport()
            return
        }
        val filename = DocumentExportSupport.validatedFilename(surfaceView.exportFileName())
        if (filename == null) {
            surfaceView.cancelExport()
            showExportError()
            return
        }
        val directory = File(cacheDir, "exports/${UUID.randomUUID()}")
        if (!directory.mkdirs()) {
            Log.w(TAG, "could not create export staging directory")
            surfaceView.cancelExport()
            showExportError()
            return
        }
        exportStagingDir = directory
        val staged = File(directory, filename)
        val status = surfaceView.exportToPath(staged.absolutePath)
        if (status != 0) {
            Log.w(
                TAG,
                "could not stage export '$filename', status=$status: " +
                    OpNative.nativeLastError(surfaceView.engine),
            )
            // The failed write leaves the artifact frozen; discard it so the
            // next export request is not refused as busy.
            surfaceView.cancelExport()
            cleanupExportStaging()
            showExportError()
            return
        }
        // A successful atomic write consumed the frozen Rust request; from
        // here only the staged copy needs lifecycle care.
        exportStagedFile = staged
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            type = DocumentExportSupport.mimeTypeFor(filename)
            putExtra(Intent.EXTRA_TITLE, filename)
        }
        try {
            exportDocumentLauncher.launch(intent)
        } catch (e: Exception) {
            Log.w(TAG, "could not launch export save UI", e)
            cleanupExportStaging()
            showExportError()
        }
    }

    private fun copyStagedExport(staged: File, uri: Uri) {
        Thread({
            val result = runCatching {
                val output = try {
                    // "wt" truncates when the user picked an existing file;
                    // some providers reject the mode, so fall back to "w".
                    contentResolver.openOutputStream(uri, "wt")
                } catch (e: Exception) {
                    contentResolver.openOutputStream(uri)
                } ?: throw IOException("content resolver returned no output stream")
                output.use { destination ->
                    staged.inputStream().use { input -> input.copyTo(destination) }
                }
            }
            runOnUiThread {
                cleanupExportStaging()
                result.exceptionOrNull()?.let { error ->
                    Log.w(TAG, "could not save export '${staged.name}'", error)
                    if (!isFinishing && !isDestroyed) showExportError()
                }
            }
        }, "OpenPencilExportWriter").start()
    }

    private fun cleanupExportStaging() {
        exportStagedFile = null
        val directory = exportStagingDir ?: return
        exportStagingDir = null
        if (!directory.deleteRecursively()) {
            Log.w(TAG, "could not remove export staging directory")
        }
    }

    /** Native 15-language sheet; the engine applies the choice immediately
     *  and the preference re-applies on launch. */
    private fun presentLanguagePicker() {
        val current = surfaceView.localeCode()
        val labels = EngineLanguage.all.map { (code, name) ->
            if (code == current) "✓ $name" else name
        }
        androidx.appcompat.app.AlertDialog.Builder(this)
            .setTitle(R.string.language_picker_title)
            .setItems(labels.toTypedArray()) { _, index ->
                val (code, _) = EngineLanguage.all[index]
                if (surfaceView.setLocale(code) == 0) {
                    EngineLanguage.savePreference(this, code)
                }
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun showExportError() {
        Toast.makeText(this, R.string.document_export_failed, Toast.LENGTH_SHORT).show()
    }

    private fun showSaveError() {
        Toast.makeText(this, R.string.document_save_failed, Toast.LENGTH_SHORT).show()
    }

    private fun readAndOpenDocument(uri: Uri) {
        val metadata = queryDocumentMetadata(uri)
        val displayName = metadata.displayName ?: fallbackDisplayName(uri)
        if (!hasSupportedDocumentExtension(displayName)) {
            documentOpenInProgress = false
            Toast.makeText(this, R.string.document_type_unsupported, Toast.LENGTH_SHORT).show()
            return
        }
        if (metadata.size != null && metadata.size > MAX_DOCUMENT_BYTES) {
            documentOpenInProgress = false
            Log.w(TAG, "document '$displayName' exceeds the 32 MiB mobile input limit")
            showOpenDocumentError()
            return
        }

        Thread({
            val result = try {
                val bytes = readDocumentBytes(uri, metadata.size)
                Result.success(bytes)
            } catch (e: Exception) {
                Result.failure(e)
            } catch (e: OutOfMemoryError) {
                Result.failure(IOException("not enough memory to load the document", e))
            }

            runOnUiThread {
                documentOpenInProgress = false
                if (isFinishing || isDestroyed || !::surfaceView.isInitialized) return@runOnUiThread
                result.fold(
                    onSuccess = { bytes -> openDocument(bytes, displayName) },
                    onFailure = { error ->
                        Log.w(TAG, "could not read document '$displayName'", error)
                        showOpenDocumentError()
                    },
                )
            }
        }, "OpenPencilDocumentReader").start()
    }

    private fun openDocument(bytes: ByteArray, displayName: String) {
        val status = surfaceView.openDocument(bytes, displayName)
        if (status == 0) {
            Log.i(TAG, "opened document '$displayName'")
            return
        }
        if (status == OpNative.STATUS_CLOSING) {
            Log.w(TAG, "document open ignored because the engine is closing")
            return
        }
        Log.w(
            TAG,
            "could not open document '$displayName', status=$status: " +
                OpNative.nativeLastError(surfaceView.engine),
        )
        showOpenDocumentError()
    }

    private fun readAndImportImageOrSvg(uri: Uri) {
        val metadata = queryDocumentMetadata(uri)
        val displayName = imageImportDisplayName(uri, metadata.displayName)
        if (metadata.size != null && metadata.size > MAX_DOCUMENT_BYTES) {
            imageImportInProgress = false
            Log.w(TAG, "image '$displayName' exceeds the 32 MiB mobile input limit")
            showImageImportError()
            return
        }

        Thread({
            val result = try {
                Result.success(readDocumentBytes(uri, metadata.size))
            } catch (e: Exception) {
                Result.failure(e)
            } catch (e: OutOfMemoryError) {
                Result.failure(IOException("not enough memory to import the image", e))
            }

            runOnUiThread {
                imageImportInProgress = false
                if (isFinishing || isDestroyed || !::surfaceView.isInitialized) return@runOnUiThread
                result.fold(
                    onSuccess = { bytes -> importImageOrSvg(bytes, displayName) },
                    onFailure = { error ->
                        Log.w(TAG, "could not read image '$displayName'", error)
                        showImageImportError()
                    },
                )
            }
        }, "OpenPencilImageImporter").start()
    }

    private fun importImageOrSvg(bytes: ByteArray, displayName: String) {
        val status = surfaceView.importImageOrSvg(bytes, displayName)
        if (status == 0) {
            Log.i(TAG, "imported image '$displayName'")
            return
        }
        if (status == OpNative.STATUS_CLOSING) {
            Log.w(TAG, "image import ignored because the engine is closing")
            return
        }
        Log.w(
            TAG,
            "could not import image '$displayName', status=$status: " +
                OpNative.nativeLastError(surfaceView.engine),
        )
        // Busy is the collaboration race gate. Rust already painted the
        // precise rejection notice, so a generic platform toast would hide it.
        if (status != OpNative.STATUS_BUSY) showImageImportError()
    }

    private fun imageImportDisplayName(uri: Uri, providerName: String?): String {
        val mime = contentResolver.getType(uri)
            ?.substringBefore(';')
            ?.trim()
            ?.lowercase(Locale.ROOT)
        val fallbackExtension = when (mime) {
            "image/png" -> "png"
            "image/jpeg" -> "jpg"
            "image/gif" -> "gif"
            "image/webp" -> "webp"
            "image/svg+xml" -> "svg"
            else -> "img"
        }
        val candidate = providerName
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: uri.lastPathSegment
                ?.substringAfterLast('/')
                ?.trim()
                ?.takeIf { it.isNotEmpty() }
            ?: "image.$fallbackExtension"
        // Rust deliberately uses the extension to select the editable SVG
        // importer; preserve that path even when a provider omits the suffix.
        return if (mime == "image/svg+xml" && !candidate.endsWith(".svg", ignoreCase = true)) {
            "$candidate.svg"
        } else {
            candidate
        }
    }

    private fun queryDocumentMetadata(uri: Uri): DocumentMetadata = try {
        contentResolver.query(
            uri,
            arrayOf(OpenableColumns.DISPLAY_NAME, OpenableColumns.SIZE),
            null,
            null,
            null,
        )?.use { cursor ->
            if (!cursor.moveToFirst()) return@use DocumentMetadata(null, null)
            val nameIndex = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            val sizeIndex = cursor.getColumnIndex(OpenableColumns.SIZE)
            val name = if (nameIndex >= 0) cursor.getString(nameIndex)?.trim() else null
            val size = if (sizeIndex >= 0 && !cursor.isNull(sizeIndex)) {
                cursor.getLong(sizeIndex).takeIf { it > 0L }
            } else {
                null
            }
            DocumentMetadata(name?.takeIf { it.isNotEmpty() }, size)
        } ?: DocumentMetadata(null, null)
    } catch (e: Exception) {
        Log.w(TAG, "could not query document metadata", e)
        DocumentMetadata(null, null)
    }

    private fun readDocumentBytes(uri: Uri, knownSize: Long?): ByteArray {
        val input = contentResolver.openInputStream(uri)
            ?: throw IOException("content resolver returned no input stream")
        return input.use {
            if (knownSize != null) {
                readKnownSizeDocument(it, knownSize)
            } else {
                readUnknownSizeDocument(it)
            }
        }
    }

    private fun readKnownSizeDocument(input: InputStream, knownSize: Long): ByteArray {
        if (knownSize > MAX_DOCUMENT_BYTES) throw IOException("document is too large")
        val bytes = ByteArray(knownSize.toInt())
        var offset = 0
        while (offset < bytes.size) {
            val count = input.read(bytes, offset, bytes.size - offset)
            if (count < 0) return bytes.copyOf(offset)
            offset += count
        }
        if (input.read() >= 0) throw IOException("document grew while it was being read")
        return bytes
    }

    private fun readUnknownSizeDocument(input: InputStream): ByteArray {
        val temporary = File.createTempFile("open-document-", ".tmp", cacheDir)
        try {
            FileOutputStream(temporary).use { output ->
                val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
                var total = 0L
                while (true) {
                    val count = input.read(buffer)
                    if (count < 0) break
                    total += count
                    if (total > MAX_DOCUMENT_BYTES) throw IOException("document is too large")
                    output.write(buffer, 0, count)
                }
            }
            return temporary.readBytes()
        } finally {
            if (!temporary.delete()) Log.w(TAG, "could not delete temporary document copy")
        }
    }

    private fun fallbackDisplayName(uri: Uri): String =
        uri.lastPathSegment
            ?.substringAfterLast('/')
            ?.trim()
            ?.takeIf { it.isNotEmpty() }
            ?: "document.op"

    private fun hasSupportedDocumentExtension(name: String): Boolean {
        val dot = name.lastIndexOf('.')
        if (dot < 0 || dot == name.lastIndex) return true
        return when (name.substring(dot + 1).lowercase(Locale.ROOT)) {
            "op", "pen" -> true
            else -> false
        }
    }

    private fun showOpenDocumentError() {
        Toast.makeText(this, R.string.document_open_failed, Toast.LENGTH_SHORT).show()
    }

    private fun showImageImportError() {
        Toast.makeText(this, R.string.image_import_failed, Toast.LENGTH_LONG).show()
    }

    // Reads every fonts/*.ttf asset (from the APK assets dir) for the engine's font registry.
    private fun readFontAssets(): List<ByteArray> {
        val names = try {
            assets.list("fonts") ?: emptyArray()
        } catch (e: Exception) {
            emptyArray()
        }
        return names.filter { it.endsWith(".ttf") || it.endsWith(".otf") }
            .mapNotNull { readAsset("fonts/$it") }
    }

    private fun readAsset(path: String): ByteArray? = try {
        assets.open(path).use { it.readBytes() }
    } catch (e: Exception) {
        Log.w(TAG, "could not read asset $path", e)
        null
    }

    // Reserved: APK assets are not plain files, so a future media-backed
    // document would extract its referenced assets here and pass the root
    // as the engine's asset base.
    @Suppress("unused")
    private fun extractAssetsRoot(): String {
        val root = File(filesDir, "assets")
        root.mkdirs()
        return root.absolutePath
    }
}
