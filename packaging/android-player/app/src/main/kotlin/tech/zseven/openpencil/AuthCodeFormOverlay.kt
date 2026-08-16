package tech.zseven.openpencil

import android.graphics.Typeface
import android.os.Handler
import android.os.Looper
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

/**
 * Shared form for the email-code flows (registration and password
 * recovery): email + verification code with a send-code countdown +
 * password with a live rules checklist mirroring the backend policy
 * (12–50 chars, uppercase, digit, special) + confirmation. On success the
 * form signs in with the new credentials and approves the login overlay's
 * device pairing, so the whole flow stays native.
 */
internal class AuthCodeFormOverlay(
    private val activity: ComponentActivity,
    private val host: FrameLayout,
    private val login: NativeLoginOverlay,
    private val mode: Mode,
    private val onDismissed: () -> Unit,
) {
    enum class Mode { REGISTER, RESET }

    private var container: FrameLayout? = null
    private lateinit var emailField: EditText
    private lateinit var codeField: EditText
    private lateinit var passwordField: EditText
    private lateinit var confirmField: EditText
    private lateinit var sendCodeButton: TextView
    private lateinit var errorLabel: TextView
    private lateinit var statusLabel: TextView
    private lateinit var progress: ProgressBar
    private val ruleLabels = mutableListOf<Pair<TextView, (String) -> Boolean>>()
    private val mainThread = Handler(Looper.getMainLooper())
    private var countdownRemaining = 0

    fun show() {
        val view = buildContent()
        container = view
        host.addView(
            view,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
    }

    fun dismiss() {
        val view = container ?: return
        container = null
        host.removeView(view)
        onDismissed()
    }

    private val isVisible: Boolean
        get() = container != null

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

        val backRow = TextView(context)
        backRow.text = "←"
        backRow.setTextColor(secondary)
        backRow.textSize = 22f
        backRow.setPadding(0, AuthUi.dp(context, 6), 0, AuthUi.dp(context, 6))
        backRow.setOnClickListener { dismiss() }

        val logo = ImageView(context)
        logo.setImageResource(R.drawable.zseven_logo)
        logo.adjustViewBounds = true
        val logoParams = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.WRAP_CONTENT,
            AuthUi.dp(context, 72),
        )
        logoParams.gravity = Gravity.CENTER_HORIZONTAL
        logo.layoutParams = logoParams

        val title = TextView(context)
        title.text = context.getString(
            if (mode == Mode.REGISTER) R.string.register_title else R.string.reset_title,
        )
        title.setTextColor(AuthUi.textColor(context))
        title.setTypeface(null, Typeface.BOLD)
        title.setTextSize(TypedValue.COMPLEX_UNIT_SP, 26f)
        title.gravity = Gravity.CENTER_HORIZONTAL

        val subtitle = TextView(context)
        subtitle.text = context.getString(
            if (mode == Mode.REGISTER) R.string.register_subtitle else R.string.reset_subtitle,
        )
        subtitle.setTextColor(secondary)
        subtitle.textSize = 15f
        subtitle.gravity = Gravity.CENTER_HORIZONTAL
        subtitle.setPadding(0, 0, 0, AuthUi.dp(context, 10))

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

        codeField = EditText(context)
        codeField.inputType = android.text.InputType.TYPE_CLASS_NUMBER
        val codeBox = AuthUi.boxedField(
            context,
            codeField,
            context.getString(R.string.register_code_placeholder),
            iconRes = R.drawable.ic_shield,
        )
        sendCodeButton = AuthUi.link(
            context,
            context.getString(R.string.register_send_code),
        ) { sendCode() }
        codeBox.addView(sendCodeButton)

        passwordField = EditText(context)
        val passwordBox = AuthUi.boxedField(
            context,
            passwordField,
            context.getString(R.string.native_login_password_placeholder),
            password = true,
            iconRes = R.drawable.ic_lock,
        )
        passwordField.addTextChangedListener(object : android.text.TextWatcher {
            override fun beforeTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun onTextChanged(s: CharSequence?, a: Int, b: Int, c: Int) {}
            override fun afterTextChanged(s: android.text.Editable?) {
                refreshRules(s?.toString().orEmpty())
            }
        })

        confirmField = EditText(context)
        val confirmBox = AuthUi.boxedField(
            context,
            confirmField,
            context.getString(R.string.register_confirm_placeholder),
            password = true,
            iconRes = R.drawable.ic_lock,
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

        val submit = AuthUi.primaryButton(
            context,
            context.getString(
                if (mode == Mode.REGISTER) R.string.register_submit else R.string.reset_submit,
            ),
        ) { submit() }

        fun params(top: Int = 8) = LinearLayout.LayoutParams(
            ViewGroup.LayoutParams.MATCH_PARENT,
            ViewGroup.LayoutParams.WRAP_CONTENT,
        ).apply { topMargin = AuthUi.dp(context, top) }

        column.addView(backRow)
        column.addView(logo)
        column.addView(title, params(4))
        column.addView(subtitle, params(2))
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.native_login_email_label)),
        )
        column.addView(emailBox)
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.register_code_label)),
        )
        column.addView(codeBox)
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.native_login_password_label)),
        )
        column.addView(passwordBox)
        column.addView(buildRules(), params(6))
        column.addView(
            AuthUi.fieldLabel(context, context.getString(R.string.register_confirm_label)),
        )
        column.addView(confirmBox)
        column.addView(errorLabel, params(8))
        column.addView(submit, params(12))
        column.addView(progress, params(6))
        column.addView(statusLabel, params(4))

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
        return overlay
    }

    /** Live checklist mirroring the backend's `validNewPassword`. */
    private fun buildRules(): LinearLayout {
        val context = activity
        val columnRules = LinearLayout(context)
        columnRules.orientation = LinearLayout.VERTICAL
        val rules: List<Pair<Int, (String) -> Boolean>> = listOf(
            R.string.register_rule_length to { value: String -> value.length in 12..50 },
            R.string.register_rule_uppercase to { value: String ->
                value.any { it.isUpperCase() }
            },
            R.string.register_rule_digit to { value: String -> value.any { it.isDigit() } },
            R.string.register_rule_special to { value: String ->
                value.any { !it.isLetterOrDigit() && !it.isWhitespace() }
            },
        )
        for ((textRes, check) in rules) {
            val label = TextView(context)
            label.textSize = 12f
            label.setTextColor(AuthUi.secondaryColor(context))
            label.text = "○ " + context.getString(textRes)
            ruleLabels.add(label to check)
            columnRules.addView(label)
        }
        return columnRules
    }

    private fun refreshRules(password: String) {
        for ((label, check) in ruleLabels) {
            val satisfied = check(password)
            val body = label.text.toString().drop(2)
            label.text = (if (satisfied) "● " else "○ ") + body
            label.setTextColor(
                if (satisfied) 0xFF2E7D32.toInt() else AuthUi.secondaryColor(activity),
            )
        }
    }

    // MARK: networking

    private fun sendCode() {
        if (countdownRemaining > 0) return
        val client = login.client ?: return
        val email = emailField.text?.toString()?.trim().orEmpty()
        if (email.isEmpty()) {
            showError(activity.getString(R.string.register_error_missing_email))
            return
        }
        errorLabel.visibility = View.GONE
        val purpose = if (mode == Mode.REGISTER) "register" else "password_reset"
        client.sendEmailCode(email, purpose) { error ->
            if (!isVisible) return@sendEmailCode
            if (error != null) {
                showError(login.errorText(error))
                return@sendEmailCode
            }
            statusLabel.text = activity.getString(R.string.register_code_sent)
            statusLabel.visibility = View.VISIBLE
            startCountdown()
        }
    }

    private fun startCountdown() {
        countdownRemaining = 60
        fun tick() {
            if (!isVisible) return
            sendCodeButton.text = if (countdownRemaining > 0) {
                "${countdownRemaining}s"
            } else {
                activity.getString(R.string.register_send_code)
            }
            if (countdownRemaining > 0) {
                countdownRemaining -= 1
                mainThread.postDelayed({ tick() }, 1_000)
            }
        }
        tick()
    }

    private fun submit() {
        val client = login.client ?: return
        val email = emailField.text?.toString()?.trim().orEmpty()
        val code = codeField.text?.toString()?.trim().orEmpty()
        val password = passwordField.text?.toString().orEmpty()
        val confirm = confirmField.text?.toString().orEmpty()
        if (email.isEmpty() || code.isEmpty() || password.isEmpty()) {
            showError(activity.getString(R.string.register_error_missing_fields))
            return
        }
        if (password != confirm) {
            showError(activity.getString(R.string.register_error_mismatch))
            return
        }
        setBusy(true)
        val finish: (SsoAuthError?) -> Unit = { error ->
            if (isVisible) {
                if (error != null) {
                    setBusy(false)
                    showError(login.errorText(error))
                } else {
                    // Account ready: sign in and approve the pairing owned
                    // by the login overlay underneath. Rust observes the
                    // approval and dismisses the whole stack.
                    statusLabel.text = activity.getString(R.string.native_login_completing)
                    statusLabel.visibility = View.VISIBLE
                    client.passwordLogin(email, password) { loginError ->
                        if (!isVisible) return@passwordLogin
                        if (loginError != null) {
                            setBusy(false)
                            showError(login.errorText(loginError))
                        } else {
                            login.approvePairingFromForm { approveError ->
                                if (!isVisible) return@approvePairingFromForm
                                if (approveError != null) {
                                    setBusy(false)
                                    showError(login.errorText(approveError))
                                }
                            }
                        }
                    }
                }
            }
        }
        if (mode == Mode.REGISTER) {
            client.register(email, code, password, finish)
        } else {
            client.resetPassword(email, code, password, finish)
        }
    }

    private fun setBusy(busy: Boolean) {
        progress.visibility = if (busy) View.VISIBLE else View.GONE
        for (field in listOf(emailField, codeField, passwordField, confirmField)) {
            field.isEnabled = !busy
        }
        if (busy) errorLabel.visibility = View.GONE
    }

    private fun showError(text: String) {
        errorLabel.text = text
        errorLabel.visibility = View.VISIBLE
        statusLabel.visibility = View.GONE
    }
}
