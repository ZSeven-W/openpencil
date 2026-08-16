package tech.zseven.openpencil

import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Typeface
import android.net.Uri
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.EditText
import android.widget.FrameLayout
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.ProgressBar
import android.widget.ScrollView
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.appcompat.app.AlertDialog
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Full-window, platform-native sign-in for one engine device-login flow,
 * styled after the ZSeven web sign-in page (logo, labeled boxed inputs,
 * gradient primary button, provider icon cards).
 *
 * Email/password sign-in runs against the pairing origin's JSON API and
 * approves the pairing directly; registration and password recovery are
 * native forms stacked on this overlay. Third-party providers open the
 * regional web page in the system browser — the running pairing is approved
 * there instead. Rust stays authoritative: [dismissFromNative] runs only
 * after its auth flow reaches a terminal state, while user close/back
 * invokes [onCanceled] exactly once per shown flow.
 */
internal class NativeLoginOverlay(
    private val activity: ComponentActivity,
    private val root: FrameLayout,
    private val regionStore: SsoRegionStore,
    private val onCanceled: () -> Unit,
    private val onRequestRejected: () -> Unit,
    private val onVisibilityChanged: (visible: Boolean) -> Unit = {},
) {
    private var container: FrameLayout? = null
    private var formOverlay: AuthCodeFormOverlay? = null
    private var request: DeviceLoginRequest? = null
    var client: SsoAuthClient? = null
        private set
    private var cancellationReported = false

    private lateinit var emailField: EditText
    private lateinit var passwordField: EditText
    private lateinit var signInButton: TextView
    private lateinit var errorLabel: TextView
    private lateinit var statusLabel: TextView
    private lateinit var progress: ProgressBar
    private lateinit var providerRow: LinearLayout
    private lateinit var dividerRow: LinearLayout
    private lateinit var regionLabel: TextView
    private lateinit var regionNote: TextView

    val isVisible: Boolean
        get() = container != null

    fun pairingId(): String? = request?.pairingId

    /** Installs a new engine-provided request, canceling any older flow. */
    fun show(verificationUrl: String): Boolean {
        val parsed = DeviceLoginRequest.parse(verificationUrl)
        if (parsed == null) {
            onRequestRejected()
            return false
        }
        if (request?.verificationUrl == parsed.verificationUrl) return true
        dismiss(notifyCancellation = true)

        request = parsed
        client = SsoAuthClient(parsed.origin)
        cancellationReported = false
        val view = buildContent()
        container = view
        root.addView(
            view,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        onVisibilityChanged(true)
        val shownUrl = parsed.verificationUrl
        client?.fetchProviders { providers ->
            if (!isVisible || request?.verificationUrl != shownUrl) return@fetchProviders
            installProviderCards(providers)
        }
        return true
    }

    /** Engine-terminal close; never reports a cancellation. */
    fun dismissFromNative() {
        dismiss(notifyCancellation = false)
    }

    /** User close/back — pops a stacked form first. */
    fun handleBack() {
        val form = formOverlay
        if (form != null) {
            form.dismiss()
            formOverlay = null
            return
        }
        dismiss(notifyCancellation = true)
    }

    fun destroy() {
        dismiss(notifyCancellation = false)
    }

    /** Register / password recovery completed a sign-in with [client]. */
    fun approvePairingFromForm(completion: (SsoAuthError?) -> Unit) {
        val pairing = request?.pairingId
        val client = client
        if (pairing == null || client == null) {
            completion(SsoAuthError("protocol", ""))
            return
        }
        client.approvePairing(pairing, completion)
    }

    private fun dismiss(notifyCancellation: Boolean) {
        formOverlay?.dismiss()
        formOverlay = null
        val view = container ?: return
        container = null
        request = null
        client = null
        root.removeView(view)
        onVisibilityChanged(false)
        if (notifyCancellation && !cancellationReported) {
            cancellationReported = true
            onCanceled()
        }
    }

    // MARK: content

    private fun buildContent(): FrameLayout {
        val context = activity
        val overlay = FrameLayout(context)
        overlay.isClickable = true
        overlay.setBackgroundColor(AuthUi.backgroundColor(context))
        val secondary = AuthUi.secondaryColor(context)

        val scroll = ScrollView(context)
        scroll.isFillViewport = true
        val column = LinearLayout(context)
        column.orientation = LinearLayout.VERTICAL
        column.setPadding(
            AuthUi.dp(context, 28),
            AuthUi.dp(context, 8),
            AuthUi.dp(context, 28),
            AuthUi.dp(context, 24),
        )

        val closeRow = android.widget.ImageView(context)
        closeRow.setImageResource(R.drawable.ic_x)
        closeRow.setColorFilter(secondary)
        closeRow.setPadding(0, AuthUi.dp(context, 6), 0, AuthUi.dp(context, 6))
        closeRow.setOnClickListener { handleBack() }

        val logo = ImageView(context)
        logo.setImageResource(R.drawable.zseven_logo)
        logo.adjustViewBounds = true
        val logoParams = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            AuthUi.dp(context, 84),
        )
        logoParams.gravity = Gravity.CENTER_HORIZONTAL
        logo.layoutParams = logoParams

        val title = TextView(context)
        title.text = context.getString(R.string.native_login_welcome)
        title.setTextColor(AuthUi.textColor(context))
        title.setTypeface(null, Typeface.BOLD)
        title.setTextSize(TypedValue.COMPLEX_UNIT_SP, 26f)
        title.gravity = Gravity.CENTER_HORIZONTAL

        val subtitle = TextView(context)
        subtitle.text = context.getString(R.string.native_login_subtitle)
        subtitle.setTextColor(secondary)
        subtitle.textSize = 15f
        subtitle.gravity = Gravity.CENTER_HORIZONTAL
        subtitle.setPadding(0, 0, 0, AuthUi.dp(context, 14))

        emailField = EditText(context)
        emailField.inputType =
            android.text.InputType.TYPE_CLASS_TEXT or
            android.text.InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS
        val emailBox = AuthUi.boxedField(
            context,
            emailField,
            context.getString(R.string.native_login_email_placeholder),
            iconRes = R.drawable.ic_mail,
        )

        passwordField = EditText(context)
        val passwordBox = AuthUi.boxedField(
            context,
            passwordField,
            context.getString(R.string.native_login_password_placeholder),
            password = true,
            iconRes = R.drawable.ic_lock,
        )

        val forgotRow = LinearLayout(context)
        forgotRow.orientation = LinearLayout.HORIZONTAL
        forgotRow.gravity = Gravity.END
        forgotRow.addView(
            AuthUi.link(context, context.getString(R.string.native_login_forgot_password)) {
                presentForm(AuthCodeFormOverlay.Mode.RESET)
            },
        )

        errorLabel = TextView(context)
        errorLabel.setTextColor(AuthUi.DANGER)
        errorLabel.textSize = 13f
        errorLabel.gravity = Gravity.CENTER_HORIZONTAL
        errorLabel.visibility = View.GONE

        statusLabel = TextView(context)
        statusLabel.setTextColor(secondary)
        statusLabel.textSize = 13f
        statusLabel.gravity = Gravity.CENTER_HORIZONTAL
        statusLabel.visibility = View.GONE

        progress = ProgressBar(context)
        progress.visibility = View.GONE

        signInButton = AuthUi.primaryButton(
            context,
            context.getString(R.string.native_login_sign_in),
        ) { signIn() }

        dividerRow = AuthUi.divider(
            context,
            context.getString(R.string.native_login_continue_with),
        )
        dividerRow.visibility = View.GONE
        providerRow = LinearLayout(context)
        providerRow.orientation = LinearLayout.HORIZONTAL
        providerRow.gravity = Gravity.CENTER_HORIZONTAL
        providerRow.visibility = View.GONE

        val registerRow = LinearLayout(context)
        registerRow.orientation = LinearLayout.HORIZONTAL
        registerRow.gravity = Gravity.CENTER_HORIZONTAL
        val registerPrompt = TextView(context)
        registerPrompt.text = context.getString(R.string.native_login_no_account)
        registerPrompt.setTextColor(secondary)
        registerPrompt.textSize = 14f
        registerRow.addView(registerPrompt)
        registerRow.addView(
            AuthUi.link(context, context.getString(R.string.native_login_register_now)) {
                presentForm(AuthCodeFormOverlay.Mode.REGISTER)
            },
        )

        regionLabel = TextView(context)
        regionLabel.setTextColor(secondary)
        regionLabel.textSize = 13f
        regionLabel.gravity = Gravity.CENTER_HORIZONTAL
        regionLabel.text = regionRowText()
        regionLabel.setPadding(0, AuthUi.dp(context, 12), 0, 0)
        regionLabel.setOnClickListener { toggleRegion() }

        regionNote = TextView(context)
        regionNote.setTextColor(secondary)
        regionNote.alpha = 0.7f
        regionNote.textSize = 12f
        regionNote.gravity = Gravity.CENTER_HORIZONTAL
        regionNote.visibility = View.GONE

        fun params(top: Int = 8) = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = AuthUi.dp(context, top) }

        column.addView(closeRow)
        column.addView(logo)
        column.addView(title, params(6))
        column.addView(subtitle, params(2))
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.native_login_email_label)),
        )
        column.addView(emailBox)
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.native_login_password_label)),
        )
        column.addView(passwordBox)
        column.addView(forgotRow, params(2))
        column.addView(errorLabel, params(6))
        column.addView(signInButton, params(10))
        column.addView(progress, params(6))
        column.addView(statusLabel, params(4))
        column.addView(dividerRow, params(18))
        column.addView(providerRow, params(14))
        column.addView(registerRow, params(18))
        column.addView(regionLabel)
        column.addView(regionNote)

        scroll.addView(
            column,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )
        overlay.addView(
            scroll,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        ViewCompat.setOnApplyWindowInsetsListener(overlay) { view, insets ->
            val bars = insets.getInsets(
                WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.ime(),
            )
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }
        ViewCompat.requestApplyInsets(overlay)
        return overlay
    }

    /**
     * One brand-icon card per advertised provider. Every card opens the
     * system browser at the engine's verification URL — the pairing approval
     * must happen in that page's context, so provider cards are
     * region-accurate entry points, not separate OAuth launches.
     */
    private fun installProviderCards(providers: List<SsoProviderEntry>) {
        if (providers.isEmpty()) return
        providerRow.removeAllViews()
        for ((position, provider) in providers.withIndex()) {
            val card = AuthUi.providerCard(activity, provider.id) { openProvidersInBrowser() }
            card.contentDescription = provider.displayName
            val params = LinearLayout.LayoutParams(
                AuthUi.dp(activity, 56),
                AuthUi.dp(activity, 56),
            )
            if (position > 0) params.marginStart = AuthUi.dp(activity, 16)
            providerRow.addView(card, params)
        }
        dividerRow.visibility = View.VISIBLE
        providerRow.visibility = View.VISIBLE
    }

    private fun presentForm(mode: AuthCodeFormOverlay.Mode) {
        val host = container ?: return
        formOverlay?.dismiss()
        val overlay = AuthCodeFormOverlay(activity, host, this, mode) {
            formOverlay = null
        }
        formOverlay = overlay
        overlay.show()
    }

    // MARK: actions

    private fun signIn() {
        val client = client ?: return
        val pairing = request?.pairingId ?: return
        val email = emailField.text?.toString()?.trim().orEmpty()
        val password = passwordField.text?.toString().orEmpty()
        if (email.isEmpty() || password.isEmpty()) {
            showError(activity.getString(R.string.native_login_error_missing_fields))
            return
        }
        setBusy(true)
        client.passwordLogin(email, password) { loginError ->
            if (!isVisible) return@passwordLogin
            if (loginError != null) {
                setBusy(false)
                showError(errorText(loginError))
                return@passwordLogin
            }
            client.approvePairing(pairing) { approveError ->
                if (!isVisible) return@approvePairing
                if (approveError != null) {
                    setBusy(false)
                    showError(
                        if (approveError.code == "not_found") {
                            activity.getString(R.string.native_login_error_pairing_expired)
                        } else {
                            errorText(approveError)
                        },
                    )
                    return@approvePairing
                }
                // Rust's poll observes the approval, exchanges the pairing,
                // and drives the close action that dismisses this overlay.
                statusLabel.text = activity.getString(R.string.native_login_completing)
                statusLabel.visibility = View.VISIBLE
            }
        }
    }

    private fun openProvidersInBrowser() {
        statusLabel.text = activity.getString(R.string.native_login_browser_hint)
        statusLabel.visibility = View.VISIBLE
        openExternal(request?.verificationUrl)
    }

    private fun openExternal(url: String?) {
        if (url.isNullOrEmpty()) return
        try {
            activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        } catch (_: ActivityNotFoundException) {
            showError(activity.getString(R.string.native_login_error_generic))
        }
    }

    /**
     * The region row toggles directly between the two deployments; the auth
     * runtime initializes once per process, so applying it here means
     * restarting the app — offered immediately, Android can relaunch itself.
     */
    private fun toggleRegion() {
        val next = if (regionStore.resolved() == SsoRegion.CHINA) {
            SsoRegion.GLOBAL
        } else {
            SsoRegion.CHINA
        }
        regionStore.saveUserOverride(next)
        regionLabel.text = regionRowText()
        regionNote.text = activity.getString(R.string.sso_region_restart_note)
        regionNote.visibility = View.VISIBLE
        AlertDialog.Builder(activity)
            .setTitle(
                activity.getString(R.string.sso_region_switched_title, regionName(next)),
            )
            .setMessage(R.string.sso_region_restart_note)
            .setNegativeButton(R.string.sso_region_later, null)
            .setPositiveButton(R.string.sso_region_restart_now) { _, _ ->
                restartApp()
            }
            .show()
    }

    private fun restartApp() {
        val intent = activity.packageManager
            .getLaunchIntentForPackage(activity.packageName) ?: return
        intent.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK or Intent.FLAG_ACTIVITY_CLEAR_TASK)
        activity.startActivity(intent)
        Runtime.getRuntime().exit(0)
    }

    private fun regionName(region: SsoRegion): String = activity.getString(
        if (region == SsoRegion.CHINA) R.string.sso_region_china else R.string.sso_region_global,
    )

    private fun regionRowText(): String = activity.getString(
        R.string.native_login_region,
        regionName(regionStore.resolved()),
    )

    // MARK: state

    internal fun errorText(error: SsoAuthError): String = when (error.code) {
        "invalid_credentials" ->
            activity.getString(R.string.native_login_error_invalid_credentials)
        "rate_limited" -> activity.getString(R.string.native_login_error_rate_limited)
        "network" -> activity.getString(R.string.native_login_error_network)
        else -> error.message.ifEmpty {
            activity.getString(R.string.native_login_error_generic)
        }
    }

    private fun setBusy(busy: Boolean) {
        signInButton.isEnabled = !busy
        emailField.isEnabled = !busy
        passwordField.isEnabled = !busy
        progress.visibility = if (busy) View.VISIBLE else View.GONE
        if (busy) errorLabel.visibility = View.GONE
    }

    private fun showError(text: String) {
        errorLabel.text = text
        errorLabel.visibility = View.VISIBLE
        statusLabel.visibility = View.GONE
    }
}
