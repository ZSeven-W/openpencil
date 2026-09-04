package tech.zseven.openpencil

import android.os.Handler
import android.os.Looper
import org.json.JSONObject
import java.io.IOException
import java.net.CookieManager
import java.net.CookiePolicy
import java.net.HttpURLConnection
import java.net.URL

/**
 * Typed failure from the SSO JSON API (or the transport underneath it):
 * a backend error code (`invalid_credentials`, …) or a transport sentinel
 * (`network`, `protocol`).
 */
internal data class SsoAuthError(val code: String, val message: String)

/**
 * Minimal JSON client for the native login flow against one regional SSO
 * origin. Cookies live in a per-client in-memory jar, so the short-lived web
 * session obtained for the device approval never persists on disk; the
 * durable credential remains the Rust runtime's device token.
 */
internal class SsoAuthClient(private val origin: String) {
    private val cookies = CookieManager(null, CookiePolicy.ACCEPT_ALL)
    private val mainThread = Handler(Looper.getMainLooper())

    /** `POST /api/v1/auth/password-login`; stores the session cookie. */
    fun passwordLogin(
        email: String,
        password: String,
        completion: (SsoAuthError?) -> Unit,
    ) {
        post(
            "/api/v1/auth/password-login",
            JSONObject().put("email", email).put("password", password),
            completion,
        )
    }

    /**
     * `POST /api/v1/auth/providers/<id>/native-login-start` — issues the
     * single-use state for a native authorization-code sign-in (Douyin /
     * Alipay) and drops the binder cookie into this client's jar; the same
     * client instance must complete the login so the cookie travels back.
     * Delivers the state on success, or null with the error.
     */
    fun nativeLoginStart(
        providerId: String,
        completion: (String?, SsoAuthError?) -> Unit,
    ) {
        Thread({
            var state: String? = null
            val error = try {
                val payload = executeForBody(
                    "/api/v1/auth/providers/$providerId/native-login-start",
                    JSONObject(),
                )
                if (payload.second == null) {
                    state = payload.first?.let { JSONObject(it).optString("state") }
                        ?.takeIf { it.isNotEmpty() }
                    if (state == null) SsoAuthError("protocol", "") else null
                } else {
                    payload.second
                }
            } catch (e: IOException) {
                SsoAuthError("network", e.message.orEmpty())
            } catch (e: Exception) {
                SsoAuthError("protocol", e.message.orEmpty())
            }
            mainThread.post { completion(state, error) }
        }, "OpenPencilSsoRequest").start()
    }

    /**
     * `POST /api/v1/auth/providers/<id>/native-login` with the
     * authorization-code shape — redeems a native SDK auth code (Douyin /
     * Alipay) against the state issued by [nativeLoginStart]; the binder
     * cookie in this client's jar proves both calls came from one attempt.
     */
    fun nativeLogin(
        providerId: String,
        state: String,
        code: String,
        completion: (SsoAuthError?) -> Unit,
    ) {
        post(
            "/api/v1/auth/providers/$providerId/native-login",
            JSONObject().put("state", state).put("code", code),
            completion,
        )
    }

    /**
     * `POST /api/v1/auth/email-codes` — sends a verification code for
     * registration (`purpose: "register"`) or password recovery
     * (`purpose: "password_reset"`), localized to the device language.
     */
    fun sendEmailCode(email: String, purpose: String, completion: (SsoAuthError?) -> Unit) {
        val locale = if (java.util.Locale.getDefault().language == "zh") "zh-CN" else "en"
        post(
            "/api/v1/auth/email-codes",
            JSONObject().put("email", email).put("purpose", purpose).put("locale", locale),
            completion,
        )
    }

    /** `POST /api/v1/auth/register` — creates the account. */
    fun register(
        email: String,
        code: String,
        password: String,
        completion: (SsoAuthError?) -> Unit,
    ) {
        post(
            "/api/v1/auth/register",
            JSONObject()
                .put("email", email)
                .put("verification_code", code)
                .put("password", password),
            completion,
        )
    }

    /** `POST /api/v1/auth/password-reset` — sets a new password. */
    fun resetPassword(
        email: String,
        code: String,
        newPassword: String,
        completion: (SsoAuthError?) -> Unit,
    ) {
        post(
            "/api/v1/auth/password-reset",
            JSONObject()
                .put("email", email)
                .put("verification_code", code)
                .put("new_password", newPassword),
            completion,
        )
    }

    /**
     * `GET /api/v1/auth/providers?channel=web_mobile` — the third-party
     * sign-in methods this regional deployment advertises (WeChat / Alipay /
     * Douyin on the mainland site, Apple / GitHub / Google on the global
     * site). Failures resolve to an empty list so the login screen keeps
     * its generic browser button.
     */
    fun fetchProviders(completion: (List<SsoProviderEntry>) -> Unit) {
        Thread({
            val providers = try {
                val connection = URL("$origin/api/v1/auth/providers?channel=web_mobile")
                    .openConnection() as HttpURLConnection
                try {
                    connection.connectTimeout = 10_000
                    connection.readTimeout = 10_000
                    connection.useCaches = false
                    connection.instanceFollowRedirects = false
                    if (connection.responseCode in 200..299) {
                        val body = connection.inputStream.bufferedReader().use { reader ->
                            reader.readText().take(MAX_ERROR_BODY_CHARS)
                        }
                        SsoProviderList.parse(body)
                    } else {
                        emptyList()
                    }
                } finally {
                    connection.disconnect()
                }
            } catch (_: Exception) {
                emptyList()
            }
            mainThread.post { completion(providers) }
        }, "OpenPencilSsoProviders").start()
    }

    /**
     * `POST /api/v1/device/login/approve` with the logged-in session cookie —
     * approves the pairing the engine's device flow is polling.
     */
    fun approvePairing(pairingId: String, completion: (SsoAuthError?) -> Unit) {
        post(
            "/api/v1/device/login/approve",
            JSONObject().put("pairing_id", pairingId),
            completion,
        )
    }

    private fun post(path: String, body: JSONObject, completion: (SsoAuthError?) -> Unit) {
        Thread({
            val error = try {
                execute(path, body)
            } catch (e: IOException) {
                SsoAuthError("network", e.message.orEmpty())
            } catch (e: Exception) {
                SsoAuthError("protocol", e.message.orEmpty())
            }
            mainThread.post { completion(error) }
        }, "OpenPencilSsoRequest").start()
    }

    private fun execute(path: String, body: JSONObject): SsoAuthError? =
        executeForBody(path, body).second

    /** Runs one POST; on success delivers the response body, else the error. */
    private fun executeForBody(path: String, body: JSONObject): Pair<String?, SsoAuthError?> {
        val url = URL("$origin$path")
        val connection = url.openConnection() as HttpURLConnection
        try {
            connection.requestMethod = "POST"
            connection.connectTimeout = 15_000
            connection.readTimeout = 15_000
            connection.doOutput = true
            connection.useCaches = false
            connection.instanceFollowRedirects = false
            connection.setRequestProperty("Content-Type", "application/json")
            attachCookies(connection, url)
            connection.outputStream.use { it.write(body.toString().toByteArray(Charsets.UTF_8)) }
            val status = connection.responseCode
            storeCookies(connection, url)
            if (status in 200..299) {
                val payload = connection.inputStream?.bufferedReader()?.use { reader ->
                    reader.readText().take(MAX_ERROR_BODY_CHARS)
                }
                return Pair(payload, null)
            }
            val payload = connection.errorStream?.bufferedReader()?.use { reader ->
                reader.readText().take(MAX_ERROR_BODY_CHARS)
            }
            return Pair(null, decodeError(payload, status))
        } finally {
            connection.disconnect()
        }
    }

    private fun attachCookies(connection: HttpURLConnection, url: URL) {
        val header = cookies.get(url.toURI(), emptyMap())["Cookie"]
        if (!header.isNullOrEmpty()) {
            connection.setRequestProperty("Cookie", header.joinToString("; "))
        }
    }

    private fun storeCookies(connection: HttpURLConnection, url: URL) {
        cookies.put(url.toURI(), connection.headerFields)
    }

    private companion object {
        const val MAX_ERROR_BODY_CHARS = 16 * 1024

        fun decodeError(payload: String?, status: Int): SsoAuthError {
            if (payload != null) {
                try {
                    val error = JSONObject(payload).getJSONObject("error")
                    val code = error.optString("code")
                    if (code.isNotEmpty()) {
                        return SsoAuthError(code, error.optString("message"))
                    }
                } catch (_: Exception) {
                    // Fall through to the status-based sentinel below.
                }
            }
            return if (status == 429) {
                SsoAuthError("rate_limited", "")
            } else {
                SsoAuthError("protocol", "")
            }
        }
    }
}
