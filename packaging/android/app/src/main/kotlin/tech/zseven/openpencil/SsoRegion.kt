package tech.zseven.openpencil

import android.content.Context
import android.content.SharedPreferences
import android.util.Log
import java.net.HttpURLConnection
import java.net.URI
import java.net.URL
import java.util.Locale

/**
 * Regional SSO deployment this install signs in against. Mirrors the
 * engine's `OpAuthRegion` codes; both origins are pinned first-party hosts.
 */
internal enum class SsoRegion(val origin: String, val authRegionCode: Int) {
    CHINA("https://sso.zseven.cn", 0),
    GLOBAL("https://sso.zseven.tech", 1),
}

/**
 * Region resolution: a user override always wins, then the last IP-informed
 * detection, then a device-locale default. The auth runtime initializes once
 * per process, so a change (override or fresh detection) applies on the next
 * launch — callers surface that explicitly instead of pretending otherwise.
 */
internal class SsoRegionStore(context: Context) {
    private val preferences: SharedPreferences =
        context.getSharedPreferences("sso-region", Context.MODE_PRIVATE)

    fun resolved(): SsoRegion =
        stored(OVERRIDE_KEY) ?: stored(DETECTED_KEY) ?: localeDefault()

    fun hasUserOverride(): Boolean = stored(OVERRIDE_KEY) != null

    fun saveUserOverride(region: SsoRegion) {
        preferences.edit().putString(OVERRIDE_KEY, region.name).apply()
    }

    /**
     * Startup resolution for the auth runtime. With a stored answer
     * (override or earlier detection) it completes immediately on the main
     * thread and refreshes the detection in the background for the next
     * launch. On a first launch it runs the IP probe first — bounded by the
     * probe's own timeouts — and only falls back to the device locale when
     * the probe is inconclusive, so the region really comes from the IP.
     */
    fun resolveForStartup(completion: (SsoRegion) -> Unit) {
        val mainThread = android.os.Handler(android.os.Looper.getMainLooper())
        if (stored(OVERRIDE_KEY) != null || stored(DETECTED_KEY) != null) {
            completion(resolved())
            refreshDetectedRegionAsync()
            return
        }
        Thread({
            val detected = probeRegion()
            if (detected != null) {
                preferences.edit().putString(DETECTED_KEY, detected.name).apply()
            }
            val region = detected ?: localeDefault()
            mainThread.post { completion(region) }
        }, "OpenPencilRegionProbe").start()
    }

    /**
     * One IP-informed detection per launch, skipped once the user chose a
     * region; the result applies on the next launch.
     */
    fun refreshDetectedRegionAsync() {
        if (hasUserOverride()) return
        Thread({
            val detected = probeRegion() ?: return@Thread
            preferences.edit().putString(DETECTED_KEY, detected.name).apply()
        }, "OpenPencilRegionProbe").start()
    }

    /**
     * The IP probe asks the ZSeven global gateway itself: nginx
     * 302-redirects cookie-less mainland-China browser requests on
     * `op.zseven.tech` to `op.zseven.cn` (APNIC-derived list, no
     * third-party geolocation), so a plain no-follow GET reads the
     * gateway's own IP verdict. An unreachable global host also reads as
     * mainland (typical when only the domestic site is reachable), while
     * other outcomes are inconclusive.
     */
    private fun probeRegion(): SsoRegion? = try {
        val connection = URL(REGION_PROBE_URL)
            .openConnection() as HttpURLConnection
        try {
            connection.instanceFollowRedirects = false
            connection.useCaches = false
            connection.connectTimeout = 3_000
            connection.readTimeout = 3_000
            val status = connection.responseCode
            val location = connection.getHeaderField("Location")
            when {
                status in 300..399 && redirectsToChina(location) -> SsoRegion.CHINA
                status < 400 -> SsoRegion.GLOBAL
                else -> null
            }
        } finally {
            connection.disconnect()
        }
    } catch (e: Exception) {
        Log.i(TAG, "global SSO host unreachable; detecting mainland region: $e")
        SsoRegion.CHINA
    }

    private fun redirectsToChina(location: String?): Boolean {
        if (location.isNullOrEmpty()) return false
        val host = try {
            URI(location).host
        } catch (_: Exception) {
            null
        }
        return host?.lowercase(Locale.ROOT) == MAINLAND_REDIRECT_HOST
    }

    private fun stored(key: String): SsoRegion? =
        preferences.getString(key, null)?.let { value ->
            SsoRegion.entries.firstOrNull { it.name == value }
        }

    private fun localeDefault(): SsoRegion =
        if (Locale.getDefault().country.equals("CN", ignoreCase = true)) {
            SsoRegion.CHINA
        } else {
            SsoRegion.GLOBAL
        }

    private companion object {
        const val TAG = "OpenPencilPlayer"
        const val OVERRIDE_KEY = "region-override"
        const val DETECTED_KEY = "region-detected"
        const val REGION_PROBE_URL = "https://op.zseven.tech/"
        const val MAINLAND_REDIRECT_HOST = "op.zseven.cn"
    }
}
