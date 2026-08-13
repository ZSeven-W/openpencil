package dev.openpencil.player

import android.annotation.SuppressLint
import android.annotation.TargetApi
import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Color
import android.graphics.drawable.ColorDrawable
import android.net.Uri
import android.net.http.SslError
import android.os.Build
import android.os.Message
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.webkit.CookieManager
import android.webkit.GeolocationPermissions
import android.webkit.HttpAuthHandler
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.SafeBrowsingResponse
import android.webkit.SslErrorHandler
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.TextView
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Full-window, lifecycle-owned authentication WebView.
 *
 * This class has no authentication URL or JNI dependency. The engine injects
 * a validated [LoginWebViewRequest], then independently polls the device-login
 * flow. Native success closes the single active request with [dismissFromNative];
 * user close/back invokes [onCanceled] so native can abort that flow.
 */
internal class LoginWebViewOverlay(
    private val activity: ComponentActivity,
    private val root: FrameLayout,
    private val onCanceled: () -> Unit,
    private val onRequestRejected: (error: LoginWebViewRequestError) -> Unit,
    private val onVisibilityChanged: (visible: Boolean) -> Unit = {},
) {
    private data class ActiveSession(
        val policy: LoginWebViewPolicy,
        val container: FrameLayout,
        val webView: WebView,
        val progress: ProgressBar,
    )

    private var active: ActiveSession? = null
    private var generation = 0L

    val isVisible: Boolean
        get() = active != null

    /** Installs a new native-provided request, canceling any older flow. */
    fun show(request: LoginWebViewRequest): Boolean {
        val result = LoginWebViewPolicy.validate(request)
        val policy = result.policy
        if (policy == null) {
            onRequestRejected(result.error ?: LoginWebViewRequestError.UNSAFE_INITIAL_URL)
            return false
        }
        if (active?.policy?.initialUrl == policy.initialUrl) return true
        dismiss(notifyCancellation = true)

        val nextGeneration = ++generation
        val session = createSession(policy)
        active = session
        root.addView(
            session.container,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        session.container.bringToFront()
        ViewCompat.requestApplyInsets(session.container)
        onVisibilityChanged(true)
        beginLoading(session, nextGeneration)
        return true
    }

    /** Back first walks WebView history; at the root it is an explicit cancel. */
    fun handleBack(): Boolean {
        val session = active
        return when (decideLoginBack(session != null, session?.webView?.canGoBack() == true)) {
            LoginBackDecision.NOT_HANDLED -> false
            LoginBackDecision.NAVIGATE_WEB_HISTORY -> {
                session?.webView?.goBack()
                true
            }
            LoginBackDecision.CANCEL_FLOW -> {
                dismiss(notifyCancellation = true)
                true
            }
        }
    }

    /** The engine owns one authentication flow per session. */
    fun dismissFromNative() {
        dismiss(notifyCancellation = false)
    }

    fun onPause() {
        active?.webView?.onPause()
    }

    fun onResume() {
        active?.webView?.onResume()
    }

    /** Activity teardown must not call back into an engine being destroyed. */
    fun destroy() {
        dismiss(notifyCancellation = false)
    }

    @SuppressLint("SetJavaScriptEnabled")
    private fun createSession(policy: LoginWebViewPolicy): ActiveSession {
        val container = FrameLayout(activity).apply {
            setBackgroundColor(Color.rgb(24, 24, 27))
            isClickable = true
            isFocusable = true
            contentDescription = activity.getString(R.string.login_webview_title)
        }
        val safeContent = LinearLayout(activity).apply {
            orientation = LinearLayout.VERTICAL
            setBackgroundColor(Color.WHITE)
        }
        container.addView(
            safeContent,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        ViewCompat.setOnApplyWindowInsetsListener(container) { _, insets ->
            val safe = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or
                    WindowInsetsCompat.Type.displayCutout(),
            )
            val ime = insets.getInsets(WindowInsetsCompat.Type.ime())
            safeContent.setPadding(safe.left, safe.top, safe.right, maxOf(safe.bottom, ime.bottom))
            insets
        }

        val toolbar = FrameLayout(activity).apply {
            setBackgroundColor(Color.rgb(32, 32, 36))
        }
        safeContent.addView(
            toolbar,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                dp(56),
            ),
        )

        val title = TextView(activity).apply {
            text = activity.getString(R.string.login_webview_title)
            setTextColor(Color.WHITE)
            textSize = 17f
            gravity = Gravity.CENTER_VERTICAL
            maxLines = 1
        }
        toolbar.addView(
            title,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ).apply {
                marginStart = dp(20)
                marginEnd = dp(64)
            },
        )

        val close = TextView(activity).apply {
            text = "\u00d7"
            setTextColor(Color.WHITE)
            textSize = 30f
            gravity = Gravity.CENTER
            contentDescription = activity.getString(R.string.login_webview_close)
            background = ColorDrawable(Color.TRANSPARENT)
            isClickable = true
            isFocusable = true
            setOnClickListener { dismiss(notifyCancellation = true) }
        }
        toolbar.addView(
            close,
            FrameLayout.LayoutParams(dp(48), dp(48), Gravity.END or Gravity.CENTER_VERTICAL).apply {
                marginEnd = dp(4)
            },
        )

        val progress = ProgressBar(
            activity,
            null,
            android.R.attr.progressBarStyleHorizontal,
        ).apply {
            max = 100
            visibility = View.INVISIBLE
        }
        toolbar.addView(
            progress,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                dp(2),
                Gravity.BOTTOM,
            ),
        )

        val webView = WebView(activity).apply {
            setBackgroundColor(Color.WHITE)
            isFocusable = true
            isFocusableInTouchMode = true
            settings.apply {
                javaScriptEnabled = true
                domStorageEnabled = true
                allowFileAccess = false
                allowContentAccess = false
                javaScriptCanOpenWindowsAutomatically = false
                setSupportMultipleWindows(false)
                mixedContentMode = android.webkit.WebSettings.MIXED_CONTENT_NEVER_ALLOW
                safeBrowsingEnabled = true
            }
        }
        CookieManager.getInstance().apply {
            setAcceptCookie(true)
            setAcceptThirdPartyCookies(webView, false)
        }
        webView.webViewClient = navigationClient(policy, progress)
        webView.webChromeClient = chromeClient(policy, progress)
        webView.setDownloadListener { _, _, _, _, _ -> reportBlockedNavigation() }
        safeContent.addView(
            webView,
            LinearLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                0,
                1f,
            ),
        )

        return ActiveSession(policy, container, webView, progress)
    }

    private fun navigationClient(
        policy: LoginWebViewPolicy,
        progress: ProgressBar,
    ): WebViewClient = object : WebViewClient() {
        private val navigationChain = LoginNavigationChain()

        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
            val url = request.url.toString()
            if (!request.isForMainFrame) return !isHttps(url)
            return handleNavigationDecision(
                policy = policy,
                url = url,
                decision = navigationChain.decide(
                    policy = policy,
                    url = url,
                    hasGesture = request.hasGesture(),
                    isRedirect = request.isRedirect,
                ),
            )
        }

        @Deprecated("Used on Android WebView implementations that call the legacy overload")
        override fun shouldOverrideUrlLoading(view: WebView, url: String): Boolean =
            handleTopLevelNavigation(policy, url, userInitiated = false)

        override fun onPageStarted(view: WebView, url: String, favicon: android.graphics.Bitmap?) {
            if (policy.decideTopLevelNavigation(url, userInitiated = false) !=
                LoginNavigationDecision.ALLOW_IN_WEBVIEW
            ) {
                view.stopLoading()
                reportBlockedNavigation()
                return
            }
            progress.visibility = View.VISIBLE
        }

        override fun onPageFinished(view: WebView, url: String) {
            navigationChain.pageFinished()
            progress.visibility = View.INVISIBLE
            CookieManager.getInstance().flush()
        }

        override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
            handler.cancel()
            reportFatalLoadFailure(policy)
        }

        override fun onReceivedHttpAuthRequest(
            view: WebView,
            handler: HttpAuthHandler,
            host: String,
            realm: String,
        ) {
            handler.cancel()
            reportFatalLoadFailure(policy)
        }

        override fun onReceivedError(
            view: WebView,
            request: WebResourceRequest,
            error: WebResourceError,
        ) {
            if (request.isForMainFrame) {
                progress.visibility = View.INVISIBLE
                Toast.makeText(
                    activity,
                    R.string.login_webview_load_failed,
                    Toast.LENGTH_SHORT,
                ).show()
            }
        }

        @TargetApi(Build.VERSION_CODES.O_MR1)
        override fun onSafeBrowsingHit(
            view: WebView,
            request: WebResourceRequest,
            threatType: Int,
            callback: SafeBrowsingResponse,
        ) {
            callback.backToSafety(true)
            reportFatalLoadFailure(policy)
        }

        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
            view.post { reportFatalLoadFailure(policy) }
            return true
        }
    }

    private fun chromeClient(
        policy: LoginWebViewPolicy,
        progress: ProgressBar,
    ): WebChromeClient = object : WebChromeClient() {
        override fun onProgressChanged(view: WebView, newProgress: Int) {
            progress.progress = newProgress
            progress.visibility = if (newProgress in 0..99) View.VISIBLE else View.INVISIBLE
        }

        override fun onCreateWindow(
            view: WebView,
            isDialog: Boolean,
            isUserGesture: Boolean,
            resultMsg: Message,
        ): Boolean {
            if (isUserGesture) {
                openExternalHttps(policy.initialUrl)
            } else {
                reportBlockedNavigation()
            }
            return false
        }

        override fun onPermissionRequest(request: PermissionRequest) {
            request.deny()
        }

        override fun onGeolocationPermissionsShowPrompt(
            origin: String,
            callback: GeolocationPermissions.Callback,
        ) {
            callback.invoke(origin, false, false)
        }

        override fun onShowFileChooser(
            webView: WebView,
            filePathCallback: ValueCallback<Array<Uri>>,
            fileChooserParams: FileChooserParams,
        ): Boolean {
            filePathCallback.onReceiveValue(null)
            return true
        }
    }

    private fun beginLoading(session: ActiveSession, expectedGeneration: Long) {
        if (generation == expectedGeneration && active === session) {
            session.webView.loadUrl(session.policy.initialUrl)
        }
    }

    private fun handleTopLevelNavigation(
        policy: LoginWebViewPolicy,
        url: String,
        userInitiated: Boolean,
    ): Boolean = handleNavigationDecision(
        policy,
        url,
        policy.decideTopLevelNavigation(url, userInitiated),
    )

    private fun handleNavigationDecision(
        policy: LoginWebViewPolicy,
        url: String,
        decision: LoginNavigationDecision,
    ): Boolean = when (decision) {
        LoginNavigationDecision.ALLOW_IN_WEBVIEW -> false
        LoginNavigationDecision.OPEN_INITIAL_IN_SYSTEM_BROWSER -> {
            // Restart from the verification URI. Opening the intercepted
            // provider URL directly can lose SSO pairing state/cookies.
            openExternalHttps(policy.initialUrl)
            true
        }
        LoginNavigationDecision.BLOCK -> {
            reportBlockedNavigation()
            true
        }
    }

    private fun openExternalHttps(url: String) {
        val intent = Intent(Intent.ACTION_VIEW, Uri.parse(url)).apply {
            addCategory(Intent.CATEGORY_BROWSABLE)
        }
        try {
            activity.startActivity(intent)
        } catch (_: ActivityNotFoundException) {
            reportBlockedNavigation()
        } catch (_: SecurityException) {
            reportBlockedNavigation()
        }
    }

    private fun isHttps(url: String): Boolean =
        runCatching { Uri.parse(url).scheme.equals("https", ignoreCase = true) }.getOrDefault(false)

    private fun reportBlockedNavigation() {
        Toast.makeText(
            activity,
            R.string.login_webview_navigation_blocked,
            Toast.LENGTH_SHORT,
        ).show()
    }

    private fun reportFatalLoadFailure(policy: LoginWebViewPolicy) {
        if (active?.policy !== policy) return
        Toast.makeText(activity, R.string.login_webview_load_failed, Toast.LENGTH_SHORT).show()
        dismiss(notifyCancellation = true)
    }

    private fun dismiss(notifyCancellation: Boolean) {
        val session = active ?: return
        active = null
        generation++
        root.removeView(session.container)
        session.webView.stopLoading()
        session.webView.webChromeClient = null
        session.webView.webViewClient = WebViewClient()
        session.webView.removeAllViews()
        session.webView.destroy()
        onVisibilityChanged(false)
        if (notifyCancellation) onCanceled()
    }

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt().coerceAtLeast(value)
}
