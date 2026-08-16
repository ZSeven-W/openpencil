package tech.zseven.openpencil

import org.json.JSONObject

/**
 * One third-party sign-in method the regional SSO deployment advertises
 * (`GET /api/v1/auth/providers?channel=web_mobile`). Each deployment lists
 * only its own enabled providers, so the native login screen shows WeChat /
 * Alipay / Douyin on the mainland site and Apple / GitHub / Google on the
 * global site without any hardcoded region table.
 */
internal data class SsoProviderEntry(val id: String, val displayName: String)

internal object SsoProviderList {
    /**
     * Decodes the providers response, dropping malformed rows. Returns an
     * empty list for structurally invalid payloads so the login screen
     * falls back to its generic browser button.
     */
    fun parse(json: String): List<SsoProviderEntry> = try {
        val rows = JSONObject(json).getJSONArray("providers")
        (0 until rows.length()).mapNotNull { index ->
            val row = rows.optJSONObject(index) ?: return@mapNotNull null
            val id = row.optString("id")
            val name = row.optString("display_name")
            if (id.isEmpty() || name.isEmpty() || name.length > 64) {
                null
            } else {
                SsoProviderEntry(id, name)
            }
        }
    } catch (_: Exception) {
        emptyList()
    }
}
