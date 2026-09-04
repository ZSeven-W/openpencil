package tech.zseven.openpencil

import android.content.ContentResolver
import android.content.Context
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import android.util.Log
import java.io.File
import java.io.IOException
import java.util.UUID

private const val TAG = "OpenPencilSave"

/**
 * Bridges the engine's Save / Save As shell action to the Storage Access
 * Framework.
 *
 * Android 11+ hides `Android/data` from the file manager, so a document the
 * engine writes to its own private directory is unreachable — and `.op` is
 * not an export format, so there is no other way out either. Save therefore
 * goes through `ACTION_CREATE_DOCUMENT` and the user picks the destination.
 *
 * The engine stays the only writer of canonical `.op` bytes: it streams them
 * into an app-private staging file (so a multi-megabyte document never
 * crosses JNI as a `ByteArray`), and this class copies that file into the
 * picked `content://` URI. Only after the copy succeeds does
 * [OpSurfaceView.commitSave] mark the document saved.
 *
 * A committed save takes a persistable URI permission, so the *next* plain
 * Save rewrites the same file with no picker at all. If that permission has
 * since been revoked — or the provider refuses the write — the round trip
 * falls back to the picker rather than failing the save.
 *
 * Every terminal path either commits or cancels the engine's pending save
 * and removes the staging directory.
 */
class DocumentSaveCoordinator(
    private val context: Context,
    private val surfaceView: OpSurfaceView,
    private val launchPicker: (Intent) -> Unit,
    private val onError: () -> Unit,
) {
    private val mainHandler = Handler(Looper.getMainLooper())
    private var stagingDirectory: File? = null
    private var stagedFile: File? = null
    private var stagedFilename: String? = null

    /** Whether a picker round trip currently owns the engine's pending save. */
    val isActive: Boolean
        get() = stagingDirectory != null

    /** Shell action 11: stage the document, then rewrite or prompt. */
    fun begin() {
        // A second shell action must not replace a staged file the save UI is
        // still copying. Discard only the newly frozen engine request.
        if (isActive) {
            surfaceView.cancelSave(false)
            return
        }
        val filename = DocumentExportSupport.validatedFilename(surfaceView.saveFileName())
        if (filename == null) {
            Log.w(TAG, "engine offered no usable save file name")
            fail()
            return
        }
        val directory = File(context.cacheDir, "saves/${UUID.randomUUID()}")
        if (!directory.mkdirs()) {
            Log.w(TAG, "could not create save staging directory")
            fail()
            return
        }
        stagingDirectory = directory
        val staged = File(directory, filename)
        val status = surfaceView.stageSaveToPath(staged.absolutePath)
        if (status != 0) {
            Log.w(
                TAG,
                "could not stage save '$filename', status=$status: " +
                    OpNative.nativeLastError(surfaceView.engine),
            )
            fail()
            return
        }
        stagedFile = staged
        stagedFilename = filename

        val bound = surfaceView.saveTarget()
        if (bound.isNullOrEmpty()) {
            presentPicker(filename)
            return
        }
        // Silent rewrite of the destination the user already picked.
        copyStagedInto(Uri.parse(bound), bound, filename, allowPickerFallback = true)
    }

    /** Result of the `ACTION_CREATE_DOCUMENT` launch. */
    fun onPickerResult(uri: Uri?) {
        val staged = stagedFile
        val filename = stagedFilename
        if (uri == null || staged == null || filename == null) {
            // The user abandoned the save UI: the document keeps its changes
            // and its previous binding, and the next Save starts over.
            surfaceView.cancelSave(false)
            cleanup()
            return
        }
        // Without this the URI is only usable until the process dies, and the
        // next plain Save would have to prompt again.
        val persisted = runCatching {
            context.contentResolver.takePersistableUriPermission(
                uri,
                Intent.FLAG_GRANT_READ_URI_PERMISSION or Intent.FLAG_GRANT_WRITE_URI_PERMISSION,
            )
        }.isSuccess
        if (!persisted) {
            Log.w(TAG, "could not persist write permission for the picked save destination")
        }
        copyStagedInto(uri, uri.toString(), filename, allowPickerFallback = false)
    }

    /** Activity teardown: never leave the engine believing a save is live. */
    fun cancelForTeardown() {
        if (!isActive) return
        surfaceView.cancelSave(false)
        cleanup()
    }

    private fun presentPicker(filename: String) {
        val intent = Intent(Intent.ACTION_CREATE_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            // `.op` has no system-wide MIME registration; the generic type
            // keeps every provider willing to create the file, and
            // EXTRA_TITLE carries the real name.
            type = DocumentExportSupport.mimeTypeFor(filename)
            putExtra(Intent.EXTRA_TITLE, filename)
            addFlags(
                Intent.FLAG_GRANT_READ_URI_PERMISSION or
                    Intent.FLAG_GRANT_WRITE_URI_PERMISSION or
                    Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION,
            )
        }
        try {
            launchPicker(intent)
        } catch (e: Exception) {
            Log.w(TAG, "could not launch the save UI", e)
            fail()
        }
    }

    /**
     * Copies the staged bytes into `uri` off the main thread, then reports
     * the outcome to the engine.
     *
     * `allowPickerFallback` is set only for a silent rewrite: a bound
     * destination that can no longer be written (permission revoked, file
     * deleted, provider gone) must ask the user for a new one rather than
     * report a failed save for a document they never saw a dialog for.
     */
    private fun copyStagedInto(
        uri: Uri,
        handle: String,
        filename: String,
        allowPickerFallback: Boolean,
    ) {
        val staged = stagedFile ?: return
        Thread({
            val result = runCatching { writeInto(context.contentResolver, staged, uri) }
            val displayName = if (result.isSuccess) {
                queryDisplayName(context.contentResolver, uri) ?: filename
            } else {
                filename
            }
            runOnMain {
                result.fold(
                    onSuccess = {
                        val status = surfaceView.commitSave(handle, displayName)
                        if (status != 0 && status != OpNative.STATUS_CLOSING) {
                            Log.w(TAG, "commitSave returned status=$status")
                            onError()
                        }
                        cleanup()
                    },
                    onFailure = { error ->
                        Log.w(TAG, "could not write the save destination", error)
                        if (allowPickerFallback) {
                            // The engine's pending save is still alive and the
                            // staged bytes are still on disk: re-ask for a
                            // destination instead of losing the save.
                            presentPicker(filename)
                        } else {
                            surfaceView.cancelSave(true)
                            cleanup()
                            onError()
                        }
                    },
                )
            }
        }, "OpenPencilSaveWriter").start()
    }

    private fun writeInto(resolver: ContentResolver, staged: File, uri: Uri) {
        val output = try {
            // "wt" truncates when the destination already holds an older
            // revision; some providers reject the mode, so fall back to "w".
            resolver.openOutputStream(uri, "wt")
        } catch (e: Exception) {
            resolver.openOutputStream(uri)
        } ?: throw IOException("content resolver returned no output stream")
        output.use { destination ->
            staged.inputStream().use { input -> input.copyTo(destination) }
        }
    }

    private fun queryDisplayName(resolver: ContentResolver, uri: Uri): String? = try {
        resolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)?.use { cursor ->
            val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
            if (index >= 0 && cursor.moveToFirst()) {
                cursor.getString(index)?.trim()?.takeIf { it.isNotEmpty() }
            } else {
                null
            }
        }
    } catch (e: Exception) {
        Log.w(TAG, "could not read the saved document's display name", e)
        null
    }

    /** Terminal failure: discard the engine's pending save and surface it. */
    private fun fail() {
        surfaceView.cancelSave(true)
        cleanup()
        onError()
    }

    private fun cleanup() {
        stagedFile = null
        stagedFilename = null
        val directory = stagingDirectory ?: return
        stagingDirectory = null
        if (!directory.deleteRecursively()) {
            Log.w(TAG, "could not remove save staging directory")
        }
    }

    /** The SurfaceView may already be detached when a copy finishes, so the
     *  main looper — not `View.post` — is what guarantees delivery. */
    private fun runOnMain(block: () -> Unit) {
        mainHandler.post(block)
    }
}
