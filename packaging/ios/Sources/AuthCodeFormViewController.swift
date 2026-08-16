import UIKit

/// Shared skeleton for the email-code flows (registration and password
/// recovery): logo header, email + verification-code + password + confirm
/// fields, a send-code countdown, an optional live password-rules checklist
/// mirroring the backend policy (12–50 chars, uppercase, digit, special),
/// and a gradient submit button. On success the form signs in with the new
/// credentials and approves the login screen's device pairing.
class AuthCodeFormViewController: UIViewController {
    let login: NativeLoginViewController

    private let scrollView = UIScrollView()
    private let contentStack = UIStackView()
    private let emailField = UITextField()
    private let codeField = UITextField()
    private let passwordField = UITextField()
    private let confirmField = UITextField()
    private var sendCodeButton: UIButton!
    private var submitButton: UIButton!
    private let spinner = UIActivityIndicatorView(style: .medium)
    private let errorLabel = UILabel()
    private let statusLabel = UILabel()
    private var ruleLabels: [(UILabel, (String) -> Bool)] = []
    private var countdownTimer: Timer?
    private var countdownRemaining = 0
    private var isFinishing = false

    // Subclass surface -------------------------------------------------

    var formTitle: String { "" }
    var formSubtitle: String { "" }
    var submitTitle: String { "" }
    var codePurpose: String { "register" }
    var showsPasswordRules: Bool { false }
    var footerPrompt: (prompt: String, link: String)? { nil }

    func submit(
        email: String,
        code: String,
        password: String,
        completion: @escaping (SsoAuthError?) -> Void
    ) {
        completion(SsoAuthError(code: "protocol", message: ""))
    }

    // ------------------------------------------------------------------

    init(login: NativeLoginViewController) {
        self.login = login
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    deinit {
        countdownTimer?.invalidate()
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground
        buildLayout()
    }

    private func buildLayout() {
        scrollView.translatesAutoresizingMaskIntoConstraints = false
        scrollView.alwaysBounceVertical = true
        scrollView.keyboardDismissMode = .interactive
        view.addSubview(scrollView)

        contentStack.axis = .vertical
        contentStack.spacing = 10
        contentStack.translatesAutoresizingMaskIntoConstraints = false
        scrollView.addSubview(contentStack)

        NSLayoutConstraint.activate([
            scrollView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            scrollView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            scrollView.topAnchor.constraint(equalTo: view.topAnchor),
            scrollView.bottomAnchor.constraint(equalTo: view.keyboardLayoutGuide.topAnchor),
            contentStack.topAnchor.constraint(
                equalTo: scrollView.contentLayoutGuide.topAnchor,
                constant: 12
            ),
            contentStack.bottomAnchor.constraint(
                equalTo: scrollView.contentLayoutGuide.bottomAnchor,
                constant: -28
            ),
            contentStack.leadingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.leadingAnchor,
                constant: 28
            ),
            contentStack.trailingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.trailingAnchor,
                constant: -28
            ),
            contentStack.widthAnchor.constraint(
                equalTo: scrollView.frameLayoutGuide.widthAnchor,
                constant: -56
            ),
        ])

        let logo = UIImageView(image: UIImage(named: "ZSevenLogo"))
        logo.contentMode = .scaleAspectFit
        logo.heightAnchor.constraint(equalToConstant: 72).isActive = true

        let title = UILabel()
        title.text = formTitle
        title.font = .systemFont(ofSize: 26, weight: .bold)
        title.textAlignment = .center

        let subtitle = UILabel()
        subtitle.text = formSubtitle
        subtitle.font = .systemFont(ofSize: 15)
        subtitle.textColor = .secondaryLabel
        subtitle.textAlignment = .center

        emailField.keyboardType = .emailAddress
        emailField.autocapitalizationType = .none
        emailField.autocorrectionType = .no
        let emailBox = AuthTheme.makeBoxedField(
            emailField,
            placeholder: NSLocalizedString(
                "nativeLogin.emailPlaceholder",
                value: "Enter your email",
                comment: "Email placeholder"
            ),
            iconAsset: "Field-mail"
        )

        sendCodeButton = AuthTheme.makeLink(
            title: NSLocalizedString(
                "register.sendCode",
                value: "Send code",
                comment: "Send verification code"
            ),
            target: self,
            action: #selector(sendCodePressed)
        )
        codeField.keyboardType = .numberPad
        codeField.textContentType = .oneTimeCode
        let codeBox = AuthTheme.makeBoxedField(
            codeField,
            placeholder: NSLocalizedString(
                "register.codePlaceholder",
                value: "Enter the code",
                comment: "Code placeholder"
            ),
            iconAsset: "Field-shield",
            trailing: sendCodeButton
        )

        passwordField.isSecureTextEntry = true
        passwordField.textContentType = .newPassword
        passwordField.addTarget(
            self,
            action: #selector(passwordEdited),
            for: .editingChanged
        )
        let passwordEye = UIButton(type: .system)
        passwordEye.setImage(AuthTheme.lucideIcon("Field-eyeoff", pointSize: 20), for: .normal)
        passwordEye.tintColor = .tertiaryLabel
        passwordEye.addTarget(
            self,
            action: #selector(togglePasswordVisibility(_:)),
            for: .touchUpInside
        )
        let passwordBox = AuthTheme.makeBoxedField(
            passwordField,
            placeholder: NSLocalizedString(
                "nativeLogin.passwordPlaceholder",
                value: "Enter your password",
                comment: "Password placeholder"
            ),
            iconAsset: "Field-lock",
            trailing: passwordEye
        )

        confirmField.isSecureTextEntry = true
        confirmField.textContentType = .newPassword
        let confirmEye = UIButton(type: .system)
        confirmEye.setImage(AuthTheme.lucideIcon("Field-eyeoff", pointSize: 20), for: .normal)
        confirmEye.tintColor = .tertiaryLabel
        confirmEye.addTarget(
            self,
            action: #selector(toggleConfirmVisibility(_:)),
            for: .touchUpInside
        )
        let confirmBox = AuthTheme.makeBoxedField(
            confirmField,
            placeholder: NSLocalizedString(
                "register.confirmPlaceholder",
                value: "Enter the password again",
                comment: "Confirm placeholder"
            ),
            iconAsset: "Field-lock",
            trailing: confirmEye
        )

        errorLabel.font = .preferredFont(forTextStyle: .footnote)
        errorLabel.textColor = .systemRed
        errorLabel.numberOfLines = 0
        errorLabel.textAlignment = .center
        errorLabel.isHidden = true

        statusLabel.font = .preferredFont(forTextStyle: .footnote)
        statusLabel.textColor = .secondaryLabel
        statusLabel.numberOfLines = 0
        statusLabel.textAlignment = .center
        statusLabel.isHidden = true

        spinner.hidesWhenStopped = true

        submitButton = AuthTheme.makePrimaryButton(
            title: submitTitle,
            target: self,
            action: #selector(submitPressed)
        )

        contentStack.addArrangedSubview(logo)
        contentStack.addArrangedSubview(title)
        contentStack.addArrangedSubview(subtitle)
        contentStack.setCustomSpacing(4, after: title)
        contentStack.setCustomSpacing(20, after: subtitle)
        addLabeledField(NSLocalizedString(
            "nativeLogin.emailLabel",
            value: "Email address",
            comment: "Email field label"
        ), emailBox)
        addLabeledField(NSLocalizedString(
            "register.codeLabel",
            value: "Email verification code",
            comment: "Code field label"
        ), codeBox)
        addLabeledField(NSLocalizedString(
            "nativeLogin.passwordLabel",
            value: "Password",
            comment: "Password field label"
        ), passwordBox)
        if showsPasswordRules {
            installPasswordRules(after: passwordBox)
        }
        addLabeledField(NSLocalizedString(
            "register.confirmLabel",
            value: "Confirm password",
            comment: "Confirm field label"
        ), confirmBox)
        contentStack.setCustomSpacing(20, after: confirmBox)
        contentStack.addArrangedSubview(errorLabel)
        contentStack.addArrangedSubview(submitButton)
        contentStack.addArrangedSubview(spinner)
        contentStack.addArrangedSubview(statusLabel)

        if let footer = footerPrompt {
            let row = AuthTheme.makeFooterPrompt(
                prompt: footer.prompt,
                linkTitle: footer.link,
                target: self,
                action: #selector(backToLogin)
            )
            contentStack.setCustomSpacing(20, after: statusLabel)
            contentStack.addArrangedSubview(row)
        }
    }

    private func addLabeledField(_ label: String, _ box: UIView) {
        contentStack.addArrangedSubview(AuthTheme.makeFieldLabel(label))
        contentStack.addArrangedSubview(box)
        contentStack.setCustomSpacing(14, after: box)
    }

    /// Live checklist mirroring the backend's `validNewPassword`.
    private func installPasswordRules(after anchor: UIView) {
        let rules: [(String, (String) -> Bool)] = [
            (
                NSLocalizedString(
                    "register.rule.length",
                    value: "Use 12–50 characters",
                    comment: "Password rule"
                ),
                { (12...50).contains($0.count) }
            ),
            (
                NSLocalizedString(
                    "register.rule.uppercase",
                    value: "At least one uppercase letter",
                    comment: "Password rule"
                ),
                { $0.contains(where: \.isUppercase) }
            ),
            (
                NSLocalizedString(
                    "register.rule.digit",
                    value: "At least one number",
                    comment: "Password rule"
                ),
                { $0.contains(where: \.isNumber) }
            ),
            (
                NSLocalizedString(
                    "register.rule.special",
                    value: "At least one special character",
                    comment: "Password rule"
                ),
                { $0.contains { $0.isPunctuation || $0.isSymbol } }
            ),
        ]
        let column = UIStackView()
        column.axis = .vertical
        column.spacing = 4
        for (text, check) in rules {
            let label = UILabel()
            label.font = .systemFont(ofSize: 12)
            label.textColor = .secondaryLabel
            label.text = "○ \(text)"
            ruleLabels.append((label, check))
            column.addArrangedSubview(label)
        }
        contentStack.addArrangedSubview(column)
        contentStack.setCustomSpacing(14, after: column)
    }

    @objc private func passwordEdited() {
        let password = passwordField.text ?? ""
        for (label, check) in ruleLabels {
            let satisfied = check(password)
            let text = label.text ?? ""
            let body = text.dropFirst(2)
            label.text = (satisfied ? "● " : "○ ") + body
            label.textColor = satisfied ? .systemGreen : .secondaryLabel
        }
    }

    @objc private func togglePasswordVisibility(_ sender: UIButton) {
        passwordField.isSecureTextEntry.toggle()
        sender.setImage(
            AuthTheme.lucideIcon(
                passwordField.isSecureTextEntry ? "Field-eyeoff" : "Field-eye",
                pointSize: 20
            ),
            for: .normal
        )
    }

    @objc private func toggleConfirmVisibility(_ sender: UIButton) {
        confirmField.isSecureTextEntry.toggle()
        sender.setImage(
            AuthTheme.lucideIcon(
                confirmField.isSecureTextEntry ? "Field-eyeoff" : "Field-eye",
                pointSize: 20
            ),
            for: .normal
        )
    }

    @objc private func backToLogin() {
        navigationController?.popViewController(animated: true)
    }

    // MARK: - Networking

    @objc private func sendCodePressed() {
        guard countdownRemaining == 0 else { return }
        let email = emailField.text?.trimmingCharacters(in: .whitespaces) ?? ""
        guard !email.isEmpty else {
            showError(NSLocalizedString(
                "register.error.missingEmail",
                value: "Enter your email first.",
                comment: "Validation failure"
            ))
            return
        }
        errorLabel.isHidden = true
        login.client.sendEmailCode(email: email, purpose: codePurpose) { [weak self] result in
            guard let self else { return }
            switch result {
            case .failure(let error):
                self.showError(error.localizedText)
            case .success:
                self.statusLabel.text = NSLocalizedString(
                    "register.codeSent",
                    value: "Verification code sent. Check your inbox.",
                    comment: "Code sent"
                )
                self.statusLabel.isHidden = false
                self.startCountdown()
            }
        }
    }

    private func startCountdown() {
        countdownRemaining = 60
        updateCountdownTitle()
        countdownTimer?.invalidate()
        countdownTimer = Timer.scheduledTimer(
            withTimeInterval: 1,
            repeats: true
        ) { [weak self] timer in
            guard let self else {
                timer.invalidate()
                return
            }
            self.countdownRemaining -= 1
            if self.countdownRemaining <= 0 {
                timer.invalidate()
                self.countdownRemaining = 0
            }
            self.updateCountdownTitle()
        }
    }

    private func updateCountdownTitle() {
        let title = countdownRemaining > 0
            ? "\(countdownRemaining)s"
            : NSLocalizedString(
                "register.sendCode",
                value: "Send code",
                comment: "Send verification code"
            )
        sendCodeButton.configuration?.title = title
    }

    @objc private func submitPressed() {
        let email = emailField.text?.trimmingCharacters(in: .whitespaces) ?? ""
        let code = codeField.text?.trimmingCharacters(in: .whitespaces) ?? ""
        let password = passwordField.text ?? ""
        let confirm = confirmField.text ?? ""
        guard !email.isEmpty, !code.isEmpty, !password.isEmpty else {
            showError(NSLocalizedString(
                "register.error.missingFields",
                value: "Fill in every field.",
                comment: "Validation failure"
            ))
            return
        }
        guard password == confirm else {
            showError(NSLocalizedString(
                "register.error.mismatch",
                value: "The passwords do not match.",
                comment: "Validation failure"
            ))
            return
        }
        view.endEditing(true)
        setBusy(true)
        submit(email: email, code: code, password: password) { [weak self] error in
            guard let self, !self.isFinishing else { return }
            if let error {
                self.setBusy(false)
                self.showError(error.localizedText)
                return
            }
            // Account ready: sign in with the same credentials and approve
            // the device pairing owned by the login screen underneath.
            self.statusLabel.text = NSLocalizedString(
                "nativeLogin.completing",
                value: "Finishing sign-in…",
                comment: "Post-approval wait"
            )
            self.statusLabel.isHidden = false
            self.login.client.passwordLogin(email: email, password: password) { [weak self] result in
                guard let self, !self.isFinishing else { return }
                switch result {
                case .failure(let error):
                    self.setBusy(false)
                    self.showError(error.localizedText)
                case .success:
                    self.login.approvePairingFromSibling { [weak self] error in
                        guard let self, !self.isFinishing else { return }
                        if let error {
                            self.setBusy(false)
                            self.showError(error.localizedText)
                        }
                        // Success: Rust observes the approval and dismisses
                        // the whole navigation stack.
                    }
                }
            }
        }
    }

    private func setBusy(_ busy: Bool) {
        submitButton.isEnabled = !busy
        for field in [emailField, codeField, passwordField, confirmField] {
            field.isEnabled = !busy
        }
        if busy {
            errorLabel.isHidden = true
            spinner.startAnimating()
        } else {
            spinner.stopAnimating()
        }
    }

    private func showError(_ text: String) {
        errorLabel.text = text
        errorLabel.isHidden = false
        statusLabel.isHidden = true
    }
}
