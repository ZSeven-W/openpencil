package dev.openpencil.player

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class LoginWebViewPolicyTest {
    @Test
    fun initialHttpsOriginIsAllowedWithoutAStaticPlatformUrl() {
        val result = LoginWebViewPolicy.validate(
            LoginWebViewRequest(
                initialUrl = "https://sso.example.test/device?code=ABCD",
            ),
        )

        assertTrue(result.accepted)
        assertNull(result.error)
        assertEquals("https://sso.example.test", result.policy?.initialOrigin)
        assertEquals(
            LoginNavigationDecision.ALLOW_IN_WEBVIEW,
            result.policy!!.decideTopLevelNavigation(
                "https://sso.example.test/approve?code=ABCD",
                userInitiated = false,
            ),
        )
    }

    @Test
    fun exactInitialOriginIsMatchedWithoutSubdomainOrPortConfusion() {
        val policy = LoginWebViewPolicy.validate(
            LoginWebViewRequest(
                initialUrl = "https://sso.example.test/login",
            ),
        ).policy!!

        assertEquals(
            LoginNavigationDecision.ALLOW_IN_WEBVIEW,
            policy.decideTopLevelNavigation(
                "https://sso.example.test/continue",
                userInitiated = false,
            ),
        )
        for (candidate in listOf(
            "https://sso.example.test.evil.test/login",
            "https://id.example.test/login",
            "https://sso.example.test:8443/login",
        )) {
            assertEquals(
                LoginNavigationDecision.BLOCK,
                policy.decideTopLevelNavigation(candidate, userInitiated = false),
            )
        }
    }

    @Test
    fun insecureCredentialedAndRelativeInitialUrlsFailClosed() {
        val candidates = listOf(
            "http://sso.example.test/login",
            "https://user:password@sso.example.test/login",
            "/device/login",
            "https://sso.example.test\\@evil.test/login",
            " https://sso.example.test/login",
        )

        for (candidate in candidates) {
            val result = LoginWebViewPolicy.validate(
                LoginWebViewRequest(initialUrl = candidate),
            )
            assertFalse("unexpectedly accepted $candidate", result.accepted)
            assertEquals(LoginWebViewRequestError.UNSAFE_INITIAL_URL, result.error)
        }
    }

    @Test
    fun userTapMayOpenExternalHttpsButScriptRedirectCannot() {
        val policy = LoginWebViewPolicy.validate(
            LoginWebViewRequest(
                initialUrl = "https://sso.example.test/login",
            ),
        ).policy!!

        assertEquals(
            LoginNavigationDecision.OPEN_INITIAL_IN_SYSTEM_BROWSER,
            policy.decideTopLevelNavigation(
                "https://accounts.example.test/oauth",
                userInitiated = true,
            ),
        )
        assertEquals(
            policy.initialUrl,
            policy.systemBrowserFallbackUrl(
                "https://accounts.example.test/oauth",
                userInitiated = true,
            ),
        )
        assertNull(
            policy.systemBrowserFallbackUrl(
                "https://accounts.example.test/oauth",
                userInitiated = false,
            ),
        )
        assertEquals(
            LoginNavigationDecision.BLOCK,
            policy.decideTopLevelNavigation(
                "https://accounts.example.test/oauth",
                userInitiated = false,
            ),
        )
    }

    @Test
    fun unsafeTargetsStayBlockedEvenAfterAUserGesture() {
        val policy = LoginWebViewPolicy.validate(
            LoginWebViewRequest(initialUrl = "https://sso.example.test/login"),
        ).policy!!

        for (candidate in listOf(
            "javascript:alert(1)",
            "intent://login",
            "http://accounts.example.test/login",
            "https://user@accounts.example.test/login",
        )) {
            assertEquals(
                LoginNavigationDecision.BLOCK,
                policy.decideTopLevelNavigation(candidate, userInitiated = true),
            )
        }
    }

    @Test
    fun invalidPortsFailClosed() {
        for (candidate in listOf(
            "https://sso.example.test:0/login",
            "https://sso.example.test:65536/login",
            "https://sso.example.test:99999/login",
        )) {
            assertFalse(
                LoginWebViewPolicy.validate(LoginWebViewRequest(candidate)).accepted,
            )
        }
    }

    @Test
    fun backUsesHistoryBeforeCancelingTheNativeFlow() {
        assertEquals(LoginBackDecision.NOT_HANDLED, decideLoginBack(false, false))
        assertEquals(LoginBackDecision.NAVIGATE_WEB_HISTORY, decideLoginBack(true, true))
        assertEquals(LoginBackDecision.CANCEL_FLOW, decideLoginBack(true, false))
    }

    @Test
    fun sameOriginUserSubmissionMayAuthorizeOnlyItsImmediateProviderRedirect() {
        val policy = LoginWebViewPolicy.validate(
            LoginWebViewRequest("https://sso.example.test/device"),
        ).policy!!
        val chain = LoginNavigationChain()

        assertEquals(
            LoginNavigationDecision.ALLOW_IN_WEBVIEW,
            chain.decide(
                policy,
                "https://sso.example.test/approve",
                hasGesture = true,
                isRedirect = false,
            ),
        )
        assertEquals(
            LoginNavigationDecision.OPEN_INITIAL_IN_SYSTEM_BROWSER,
            chain.decide(
                policy,
                "https://accounts.example.test/oauth",
                hasGesture = false,
                isRedirect = true,
            ),
        )
        assertEquals(
            LoginNavigationDecision.BLOCK,
            chain.decide(
                policy,
                "https://second.example.test/oauth",
                hasGesture = false,
                isRedirect = true,
            ),
        )
    }

    @Test
    fun completedPageClearsUserInitiatedRedirectAuthority() {
        val policy = LoginWebViewPolicy.validate(
            LoginWebViewRequest("https://sso.example.test/device"),
        ).policy!!
        val chain = LoginNavigationChain()
        chain.decide(
            policy,
            "https://sso.example.test/approve",
            hasGesture = true,
            isRedirect = false,
        )
        chain.pageFinished()

        assertEquals(
            LoginNavigationDecision.BLOCK,
            chain.decide(
                policy,
                "https://accounts.example.test/oauth",
                hasGesture = false,
                isRedirect = true,
            ),
        )
    }
}
