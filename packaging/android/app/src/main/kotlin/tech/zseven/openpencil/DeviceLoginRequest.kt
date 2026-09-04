package tech.zseven.openpencil

import java.net.URI
import java.net.URLDecoder

/**
 * Parsed engine verification URL (`https://<origin>/login?device_pairing=<id>`).
 *
 * The native login screen signs in against [origin]'s JSON API and approves
 * [pairingId]; a URL that does not carry a pairing on an HTTPS origin is
 * rejected so the shell cancels the flow instead of presenting a screen that
 * could never approve anything.
 */
internal data class DeviceLoginRequest(
    val origin: String,
    val pairingId: String,
    val verificationUrl: String,
) {
    companion object {
        fun parse(rawUrl: String): DeviceLoginRequest? {
            val uri = try {
                URI(rawUrl)
            } catch (_: Exception) {
                return null
            }
            if (!uri.scheme.equals("https", ignoreCase = true)) return null
            if (uri.rawUserInfo != null) return null
            val host = uri.host?.takeIf { it.isNotEmpty() } ?: return null
            val pairing = queryValue(uri.rawQuery, "device_pairing")
                ?.takeIf { it.isNotEmpty() }
                ?: return null
            val port = if (uri.port == -1) "" else ":${uri.port}"
            return DeviceLoginRequest(
                origin = "https://$host$port",
                pairingId = pairing,
                verificationUrl = rawUrl,
            )
        }

        private fun queryValue(rawQuery: String?, name: String): String? {
            if (rawQuery.isNullOrEmpty()) return null
            for (pair in rawQuery.split('&')) {
                val separator = pair.indexOf('=')
                if (separator <= 0) continue
                if (pair.substring(0, separator) != name) continue
                return try {
                    URLDecoder.decode(pair.substring(separator + 1), "UTF-8")
                } catch (_: Exception) {
                    null
                }
            }
            return null
        }
    }
}
