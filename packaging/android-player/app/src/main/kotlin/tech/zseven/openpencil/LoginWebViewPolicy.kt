package tech.zseven.openpencil

import java.net.URI
import java.util.Locale

/**
 * A login request supplied by the native authentication runtime.
 *
 * The Android shell deliberately owns no production login URL. The initial
 * URL must arrive with the request so a build without a working
 * authentication backend fails closed.
 */
internal data class LoginWebViewRequest(
    val initialUrl: String,
)

internal enum class LoginWebViewRequestError {
    UNSAFE_INITIAL_URL,
}

internal enum class LoginNavigationDecision {
    ALLOW_IN_WEBVIEW,
    OPEN_INITIAL_IN_SYSTEM_BROWSER,
    BLOCK,
}

internal enum class LoginBackDecision {
    NOT_HANDLED,
    NAVIGATE_WEB_HISTORY,
    CANCEL_FLOW,
}

internal fun decideLoginBack(visible: Boolean, canGoBack: Boolean): LoginBackDecision = when {
    !visible -> LoginBackDecision.NOT_HANDLED
    canGoBack -> LoginBackDecision.NAVIGATE_WEB_HISTORY
    else -> LoginBackDecision.CANCEL_FLOW
}

/** One-page navigation-chain gesture latch for SSO form -> 302 provider hops. */
internal class LoginNavigationChain {
    private var userInitiatedChain = false

    fun decide(
        policy: LoginWebViewPolicy,
        url: String,
        hasGesture: Boolean,
        isRedirect: Boolean,
    ): LoginNavigationDecision {
        if (hasGesture && !isRedirect) userInitiatedChain = true
        val userInitiated = hasGesture && !isRedirect || isRedirect && userInitiatedChain
        val decision = policy.decideTopLevelNavigation(url, userInitiated)
        if (decision != LoginNavigationDecision.ALLOW_IN_WEBVIEW) userInitiatedChain = false
        return decision
    }

    fun pageFinished() {
        userInitiatedChain = false
    }
}

internal data class LoginWebViewPolicyResult(
    val policy: LoginWebViewPolicy?,
    val error: LoginWebViewRequestError?,
) {
    val accepted: Boolean
        get() = policy != null
}

/** Exact-origin policy for top-level WebView navigation. */
internal class LoginWebViewPolicy private constructor(
    val initialUrl: String,
    val initialOrigin: String,
) {
    /**
     * Keeps the device-login page inside its exact initial origin. A user tap
     * may leave to another HTTPS origin in the system browser (for providers
     * that reject embedded user agents); script redirects and every non-HTTPS
     * target fail closed.
     */
    fun decideTopLevelNavigation(url: String, userInitiated: Boolean): LoginNavigationDecision {
        val origin = canonicalHttpsOrigin(url) ?: return LoginNavigationDecision.BLOCK
        if (origin == initialOrigin) return LoginNavigationDecision.ALLOW_IN_WEBVIEW
        return if (userInitiated) {
            LoginNavigationDecision.OPEN_INITIAL_IN_SYSTEM_BROWSER
        } else {
            LoginNavigationDecision.BLOCK
        }
    }

    /** The fallback always restarts the verified device flow from its entry URI. */
    fun systemBrowserFallbackUrl(url: String, userInitiated: Boolean): String? =
        if (decideTopLevelNavigation(url, userInitiated) ==
            LoginNavigationDecision.OPEN_INITIAL_IN_SYSTEM_BROWSER
        ) {
            initialUrl
        } else {
            null
        }

    companion object {
        fun validate(request: LoginWebViewRequest): LoginWebViewPolicyResult {
            val initialOrigin = canonicalHttpsOrigin(request.initialUrl)
                ?: return LoginWebViewPolicyResult(
                policy = null,
                error = LoginWebViewRequestError.UNSAFE_INITIAL_URL,
            )

            return LoginWebViewPolicyResult(
                policy = LoginWebViewPolicy(
                    initialUrl = request.initialUrl,
                    initialOrigin = initialOrigin,
                ),
                error = null,
            )
        }

        private fun canonicalHttpsOrigin(raw: String): String? {
            val value = raw.trim()
            if (value.isEmpty() || value != raw) return null
            val uri = try {
                URI(value)
            } catch (_: Exception) {
                return null
            }
            if (!uri.scheme.equals("https", ignoreCase = true)) return null
            if (uri.rawUserInfo != null || uri.rawAuthority?.contains('\\') == true) return null
            val host = uri.host?.lowercase(Locale.ROOT)?.trimEnd('.') ?: return null
            if (host.isEmpty() || host != uri.host.lowercase(Locale.ROOT)) return null
            if (uri.port < -1 || uri.port == 0 || uri.port > 65_535) return null
            val port = when (uri.port) {
                -1, 443 -> ""
                else -> ":${uri.port}"
            }
            return "https://$host$port"
        }
    }
}
