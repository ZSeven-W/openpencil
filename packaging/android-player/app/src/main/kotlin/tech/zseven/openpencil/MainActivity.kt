package tech.zseven.openpencil

import android.content.res.Configuration
import android.net.Uri
import android.os.Bundle
import android.provider.OpenableColumns
import android.util.Log
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.view.ViewCompat
import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.io.InputStream
import java.util.Locale

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
 * across rotation. `onDestroy` always tears the engine down.
 */
class MainActivity : ComponentActivity() {

    private lateinit var surfaceView: OpSurfaceView
    private lateinit var rootView: FrameLayout
    private lateinit var loginWebView: LoginWebViewOverlay
    private lateinit var loginBackCallback: OnBackPressedCallback
    private var documentOpenInProgress = false

    private val openDocumentLauncher = registerForActivityResult(
        ActivityResultContracts.OpenDocument(),
    ) { uri ->
        if (uri == null) {
            documentOpenInProgress = false
        } else {
            readAndOpenDocument(uri)
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)

        // Keep the drawable edge-to-edge, then deliver four-sided safe-area
        // insets to the engine so only interactive chrome avoids system UI.
        configureEdgeToEdge(window)

        val docName = intent.getStringExtra("doc") ?: "ppt-demo"
        val doc = readAsset("$docName.op") ?: ByteArray(0)
        val editorMode = intent.getBooleanExtra("editor", true)
        val fonts = readFontAssets()

        surfaceView = OpSurfaceView(this).apply {
            configure(doc, editorMode, fonts)
            setOpenDocumentHandler(::launchDocumentPicker)
            setSystemChromeAppearanceHandler { prefersLightIcons ->
                updateSystemChromeAppearance(
                    window,
                    if (::loginWebView.isInitialized && loginWebView.isVisible) {
                        false
                    } else {
                        prefersLightIcons
                    },
                )
            }
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
        loginBackCallback = object : OnBackPressedCallback(false) {
            override fun handleOnBackPressed() {
                loginWebView.handleBack()
            }
        }
        onBackPressedDispatcher.addCallback(this, loginBackCallback)
        loginWebView = LoginWebViewOverlay(
            activity = this,
            root = rootView,
            onCanceled = { surfaceView.cancelLogin() },
            onRequestRejected = { error ->
                Log.w(TAG, "rejected login WebView request: $error")
                surfaceView.cancelLogin()
                Toast.makeText(this, R.string.login_webview_load_failed, Toast.LENGTH_SHORT).show()
            },
            onVisibilityChanged = { visible ->
                loginBackCallback.isEnabled = visible
                if (visible) {
                    // The WebView safe-area band is light even when the
                    // document behind it uses a dark editor theme.
                    updateSystemChromeAppearance(window, prefersLightIcons = false)
                } else {
                    surfaceView.replaySystemChromeAppearance()
                }
            },
        )
        surfaceView.setLoginWebViewHandlers(
            open = { url -> loginWebView.show(LoginWebViewRequest(initialUrl = url)) },
            close = { loginWebView.dismissFromNative() },
        )
        setContentView(rootView)

        installEditorInsets(rootView, surfaceView)
    }

    override fun onConfigurationChanged(newConfig: Configuration) {
        super.onConfigurationChanged(newConfig)
        if (!::surfaceView.isInitialized) return
        // Density is a handled config change, so neither the Activity nor the
        // SurfaceView is recreated. Refresh the logical-pixel conversion used
        // by resize and touch, then request fresh cutout/bar/IME insets.
        surfaceView.refreshDisplayMetrics()
        if (::loginWebView.isInitialized && loginWebView.isVisible) {
            updateSystemChromeAppearance(window, prefersLightIcons = false)
        } else {
            surfaceView.replaySystemChromeAppearance()
        }
        if (::rootView.isInitialized) ViewCompat.requestApplyInsets(rootView)
    }

    override fun onPause() {
        if (::loginWebView.isInitialized) loginWebView.onPause()
        super.onPause()
    }

    override fun onResume() {
        super.onResume()
        if (::loginWebView.isInitialized) {
            loginWebView.onResume()
            if (loginWebView.isVisible) {
                updateSystemChromeAppearance(window, prefersLightIcons = false)
            }
        }
    }

    override fun onDestroy() {
        if (::loginWebView.isInitialized) loginWebView.destroy()
        // Teardown unconditionally (rotation never reaches here thanks to
        // configChanges).
        if (::surfaceView.isInitialized) surfaceView.destroy()
        super.onDestroy()
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
