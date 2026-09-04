package tech.zseven.openpencil

import android.content.Context
import android.os.Build
import android.util.Log
import java.io.File

/**
 * Configures the optional native auth backend. The proprietary runtime
 * initializes once per process and locks its regional origin, so a fresh
 * install (no persisted credential) defers configuration until the first
 * sign-in — the login region then reflects the latest IP verdict and any
 * user switch made before that first tap. Returning users configure at
 * engine creation so their session restores immediately.
 */
internal class AndroidAuthRuntime(context: Context) {
    private val storageDirFile = File(context.noBackupFilesDir, "auth").apply {
        if (!isDirectory && !mkdirs()) {
            Log.w(TAG, "could not create the private auth directory")
        }
    }
    private val storageDir = storageDirFile.absolutePath
    private val deviceName = Build.MODEL?.trim().orEmpty().ifEmpty { "Android" }
    private val regionStore = SsoRegionStore(context)
    private var configured = false

    fun regionStore(): SsoRegionStore = regionStore

    /** Engine-creation hook: restore returning users, defer fresh installs. */
    fun configure(engine: Long, editorMode: Boolean) {
        if (configured || !editorMode || engine == 0L) return
        if (!hasPersistedCredential()) {
            regionStore.refreshDetectedRegionAsync()
            return
        }
        configured = true
        regionStore.resolveForStartup { region ->
            configureNow(engine, region)
        }
    }

    /**
     * `RequestLogin` follow-up: configure for the resolved region if this
     * process has not yet, then start the device flow. `onUnavailable` runs
     * when the build has no sign-in backend.
     */
    fun startLogin(surface: OpSurfaceView, onUnavailable: () -> Unit) {
        val engine = surface.engine
        if (engine == 0L) return
        if (configured) {
            beginNow(surface, onUnavailable)
            return
        }
        configured = true
        regionStore.resolveForStartup { region ->
            configureNow(engine, region)
            beginNow(surface, onUnavailable)
        }
    }

    private fun beginNow(surface: OpSurfaceView, onUnavailable: () -> Unit) {
        val status = surface.beginLogin()
        if (status == STATUS_NOT_READY) {
            onUnavailable()
        } else if (status != 0 && status != OpNative.STATUS_CLOSING) {
            Log.i(TAG, "begin login returned status=$status")
        }
    }

    private fun configureNow(engine: Long, region: SsoRegion) {
        val status = OpNative.nativeEditorConfigureAuth(
            engine,
            storageDir,
            deviceName,
            BuildConfig.VERSION_NAME,
            region.authRegionCode,
        )
        if (status != 0 && status != OpNative.STATUS_CLOSING && status != STATUS_NOT_READY) {
            Log.i(TAG, "native auth configure returned status=$status")
        }
    }

    private fun hasPersistedCredential(): Boolean =
        storageDirFile.listFiles()?.isNotEmpty() == true

    private companion object {
        const val TAG = "OpenPencilPlayer"
        const val STATUS_NOT_READY = 10
    }
}
