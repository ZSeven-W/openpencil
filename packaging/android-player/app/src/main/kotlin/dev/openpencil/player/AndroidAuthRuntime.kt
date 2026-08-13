package dev.openpencil.player

import android.content.Context
import android.os.Build
import android.util.Log
import java.io.File

/** Configures the optional native auth backend once after engine creation. */
internal class AndroidAuthRuntime(context: Context) {
    private val storageDir = File(context.noBackupFilesDir, "auth").apply {
        if (!isDirectory && !mkdirs()) {
            Log.w("OpenPencilPlayer", "could not create the private auth directory")
        }
    }.absolutePath
    private val deviceName = Build.MODEL?.trim().orEmpty().ifEmpty { "Android" }
    private var configured = false

    fun configure(engine: Long, editorMode: Boolean) {
        if (configured || !editorMode || engine == 0L) return
        configured = true
        val status = OpNative.nativeEditorConfigureAuth(
            engine,
            storageDir,
            deviceName,
            BuildConfig.VERSION_NAME,
        )
        if (status != 0 && status != OpNative.STATUS_CLOSING) {
            // A build without mobile auth artifacts intentionally keeps the
            // Rust account UI in its fail-closed unavailable state.
            Log.i("OpenPencilPlayer", "native auth is not ready, status=$status")
        }
    }
}
