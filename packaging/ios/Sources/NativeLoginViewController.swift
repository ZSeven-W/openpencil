import SafariServices
import UIKit

/// Platform-native sign-in for one engine device-login flow, styled after
/// the ZSeven web sign-in page (logo, labeled boxed inputs, gradient
/// primary button, provider icon cards).
///
/// The engine starts a device pairing and hands this screen its verification
/// URL (`<origin>/login?device_pairing=<id>`). Email/password sign-in runs
/// against that origin's JSON API and then approves the pairing directly;
/// registration and password recovery are native sibling screens on the same
/// navigation stack. Third-party providers open the regional web page in the
/// system browser — the running pairing is approved there instead. Rust
/// stays authoritative: the host dismisses this screen only after its auth
/// flow reaches a terminal state.
final class NativeLoginViewController: UIViewController {
    private let request: DeviceLoginRequestInfo
    private var origin: URL { request.origin }
    private var pairingID: String { request.pairingID }
    private var verificationURL: URL { request.verificationURL }
    private let onCancel: () -> Void
    let client: SsoAuthClient

    private let scrollView = UIScrollView()
    private let contentStack = UIStackView()
    private let emailField = UITextField()
    private let passwordField = UITextField()
    private var signInButton: UIButton!
    private let spinner = UIActivityIndicatorView(style: .medium)
    private let errorLabel = UILabel()
    private let statusLabel = UILabel()
    private let providerRow = UIStackView()
    private let regionLabel = UILabel()
    private let regionNoteLabel = UILabel()
    private var isFinishing = false
    private var cancellationReported = false

    /// Fails when the engine-provided URL is not a device-pairing entry on an
    /// HTTPS origin — the caller then cancels the flow instead of presenting
    /// a screen that could never approve anything.
    init?(verificationURL: URL, onCancel: @escaping () -> Void) {
        guard let request = DeviceLoginRequestInfo(verificationURL: verificationURL) else {
            return nil
        }
        self.request = request
        self.onCancel = onCancel
        self.client = SsoAuthClient(origin: request.origin)
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        view.backgroundColor = .systemBackground
        navigationItem.leftBarButtonItem = UIBarButtonItem(
            image: AuthTheme.lucideIcon("Icon-x", pointSize: 20),
            style: .plain,
            target: self,
            action: #selector(cancelPressed)
        )
        buildLayout()
        client.fetchProviders { [weak self] providers in
            self?.installProviderCards(providers)
        }
    }

    /// Called by the host after Rust reports a terminal auth status.
    func finishFromHost(animated: Bool) {
        guard !isFinishing else { return }
        isFinishing = true
        dismiss(animated: animated)
    }

    func finishForTeardown(animated: Bool) {
        finishFromHost(animated: animated)
    }

    /// The engine's device flow completed elsewhere on this stack (register /
    /// reset screens sign in with the same client + pairing).
    func approvePairingFromSibling(completion: @escaping (SsoAuthError?) -> Void) {
        client.approvePairing(pairingID: pairingID) { result in
            switch result {
            case .success: completion(nil)
            case .failure(let error): completion(error)
            }
        }
    }

    // MARK: - Layout

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

        // Brand header: logo + welcome title + subtitle.
        let logo = UIImageView(image: UIImage(named: "ZSevenLogo"))
        logo.contentMode = .scaleAspectFit
        logo.heightAnchor.constraint(equalToConstant: 84).isActive = true

        let title = UILabel()
        title.text = NSLocalizedString(
            "nativeLogin.welcome",
            value: "Welcome back",
            comment: "Login title"
        )
        title.font = .systemFont(ofSize: 26, weight: .bold)
        title.textAlignment = .center

        let subtitle = UILabel()
        subtitle.text = NSLocalizedString(
            "nativeLogin.subtitle",
            value: "Sign in to ZSeven",
            comment: "Login subtitle"
        )
        subtitle.font = .systemFont(ofSize: 15)
        subtitle.textColor = .secondaryLabel
        subtitle.textAlignment = .center

        emailField.keyboardType = .emailAddress
        emailField.textContentType = .username
        emailField.autocapitalizationType = .none
        emailField.autocorrectionType = .no
        emailField.delegate = self
        let emailBox = AuthTheme.makeBoxedField(
            emailField,
            placeholder: NSLocalizedString(
                "nativeLogin.emailPlaceholder",
                value: "Enter your email",
                comment: "Email placeholder"
            ),
            iconAsset: "Field-mail"
        )

        passwordField.isSecureTextEntry = true
        passwordField.textContentType = .password
        passwordField.delegate = self
        let eye = UIButton(type: .system)
        eye.setImage(AuthTheme.lucideIcon("Field-eyeoff", pointSize: 20), for: .normal)
        eye.tintColor = .tertiaryLabel
        eye.addTarget(self, action: #selector(togglePasswordVisibility(_:)), for: .touchUpInside)
        let passwordBox = AuthTheme.makeBoxedField(
            passwordField,
            placeholder: NSLocalizedString(
                "nativeLogin.passwordPlaceholder",
                value: "Enter your password",
                comment: "Password placeholder"
            ),
            iconAsset: "Field-lock",
            trailing: eye
        )

        // 忘记密码? — right aligned under the password box.
        let forgotRow = UIView()
        let forgot = AuthTheme.makeLink(
            title: NSLocalizedString(
                "nativeLogin.forgotPassword",
                value: "Forgot password?",
                comment: "Password recovery link"
            ),
            target: self,
            action: #selector(openForgotPassword)
        )
        forgot.translatesAutoresizingMaskIntoConstraints = false
        forgotRow.addSubview(forgot)
        NSLayoutConstraint.activate([
            forgot.trailingAnchor.constraint(equalTo: forgotRow.trailingAnchor),
            forgot.topAnchor.constraint(equalTo: forgotRow.topAnchor),
            forgot.bottomAnchor.constraint(equalTo: forgotRow.bottomAnchor),
        ])

        signInButton = AuthTheme.makePrimaryButton(
            title: NSLocalizedString("nativeLogin.signIn", value: "Sign In", comment: "Sign in"),
            target: self,
            action: #selector(signInPressed)
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

        // Third-party providers: brand icon cards per what this regional
        // deployment advertises (mainland: WeChat / Alipay / Douyin; global:
        // Apple / GitHub / Google), fetched from the pairing origin. Hidden
        // until the list arrives; every card opens the system browser at the
        // engine's verification URL because the pairing approval must happen
        // in that page's context.
        let dividerRow = AuthTheme.makeDivider(text: NSLocalizedString(
            "nativeLogin.continueWith",
            value: "Or continue with",
            comment: "Provider divider"
        ))
        providerRow.axis = .horizontal
        providerRow.spacing = 16
        providerRow.alignment = .center
        let providerHost = UIView()
        providerRow.translatesAutoresizingMaskIntoConstraints = false
        providerHost.addSubview(providerRow)
        NSLayoutConstraint.activate([
            providerRow.centerXAnchor.constraint(equalTo: providerHost.centerXAnchor),
            providerRow.topAnchor.constraint(equalTo: providerHost.topAnchor),
            providerRow.bottomAnchor.constraint(equalTo: providerHost.bottomAnchor),
        ])
        dividerRow.tag = Self.dividerTag
        providerHost.tag = Self.providerHostTag
        dividerRow.isHidden = true
        providerHost.isHidden = true

        let registerRow = AuthTheme.makeFooterPrompt(
            prompt: NSLocalizedString(
                "nativeLogin.noAccount",
                value: "No account yet?",
                comment: "Registration prompt"
            ),
            linkTitle: NSLocalizedString(
                "nativeLogin.registerNow",
                value: "Sign up",
                comment: "Registration link"
            ),
            target: self,
            action: #selector(openRegister)
        )

        regionLabel.font = .systemFont(ofSize: 13)
        regionLabel.textColor = .secondaryLabel
        regionLabel.textAlignment = .center
        regionLabel.text = regionRowTitle()
        regionLabel.isUserInteractionEnabled = true
        regionLabel.addGestureRecognizer(
            UITapGestureRecognizer(target: self, action: #selector(toggleRegion))
        )

        regionNoteLabel.font = .systemFont(ofSize: 12)
        regionNoteLabel.textColor = .tertiaryLabel
        regionNoteLabel.textAlignment = .center
        regionNoteLabel.numberOfLines = 0
        regionNoteLabel.isHidden = true

        contentStack.addArrangedSubview(logo)
        contentStack.addArrangedSubview(title)
        contentStack.addArrangedSubview(subtitle)
        contentStack.setCustomSpacing(4, after: title)
        contentStack.setCustomSpacing(24, after: subtitle)
        contentStack.addArrangedSubview(AuthTheme.makeFieldLabel(NSLocalizedString(
            "nativeLogin.emailLabel",
            value: "Email address",
            comment: "Email field label"
        )))
        contentStack.addArrangedSubview(emailBox)
        contentStack.setCustomSpacing(16, after: emailBox)
        contentStack.addArrangedSubview(AuthTheme.makeFieldLabel(NSLocalizedString(
            "nativeLogin.passwordLabel",
            value: "Password",
            comment: "Password field label"
        )))
        contentStack.addArrangedSubview(passwordBox)
        contentStack.setCustomSpacing(4, after: passwordBox)
        contentStack.addArrangedSubview(forgotRow)
        contentStack.setCustomSpacing(18, after: forgotRow)
        contentStack.addArrangedSubview(errorLabel)
        contentStack.addArrangedSubview(signInButton)
        contentStack.addArrangedSubview(spinner)
        contentStack.addArrangedSubview(statusLabel)
        contentStack.setCustomSpacing(22, after: signInButton)
        contentStack.addArrangedSubview(dividerRow)
        contentStack.setCustomSpacing(18, after: dividerRow)
        contentStack.addArrangedSubview(providerHost)
        contentStack.setCustomSpacing(24, after: providerHost)
        contentStack.addArrangedSubview(registerRow)
        contentStack.setCustomSpacing(20, after: registerRow)
        contentStack.addArrangedSubview(regionLabel)
        contentStack.addArrangedSubview(regionNoteLabel)
    }

    private static let dividerTag = 61
    private static let providerHostTag = 62

    /// Installs one brand-icon card per advertised provider. Cards are
    /// region-accurate entry points, not separate OAuth launches.
    private func installProviderCards(_ providers: [SsoProviderEntry]) {
        guard !isFinishing, !providers.isEmpty else { return }
        providerRow.arrangedSubviews.forEach { view in
            providerRow.removeArrangedSubview(view)
            view.removeFromSuperview()
        }
        for provider in providers {
            let card = AuthTheme.makeProviderCard(
                assetName: "Provider-\(provider.id)",
                target: self,
                action: #selector(providerCardTapped(_:))
            )
            card.accessibilityLabel = provider.displayName
            card.accessibilityIdentifier = provider.id
            providerRow.addArrangedSubview(card)
        }
        contentStack.viewWithTag(Self.dividerTag)?.isHidden = false
        contentStack.viewWithTag(Self.providerHostTag)?.isHidden = false
    }

    // MARK: - Actions

    @objc private func cancelPressed() {
        guard !isFinishing else { return }
        isFinishing = true
        if !cancellationReported {
            cancellationReported = true
            onCancel()
        }
        dismiss(animated: true)
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

    @objc private func signInPressed() {
        let email = emailField.text?.trimmingCharacters(in: .whitespaces) ?? ""
        let password = passwordField.text ?? ""
        guard !email.isEmpty, !password.isEmpty else {
            showError(NSLocalizedString(
                "nativeLogin.error.missingFields",
                value: "Enter your email and password.",
                comment: "Validation failure"
            ))
            return
        }
        view.endEditing(true)
        setBusy(true)
        client.passwordLogin(email: email, password: password) { [weak self] result in
            guard let self, !self.isFinishing else { return }
            switch result {
            case .failure(let error):
                self.setBusy(false)
                self.showError(error.localizedText)
            case .success:
                self.approvePairing()
            }
        }
    }

    private func approvePairing() {
        client.approvePairing(pairingID: pairingID) { [weak self] result in
            guard let self, !self.isFinishing else { return }
            switch result {
            case .failure(let error):
                self.setBusy(false)
                self.showError(
                    error.code == "not_found"
                        ? NSLocalizedString(
                            "nativeLogin.error.pairingExpired",
                            value: "This sign-in request expired. Close and try again.",
                            comment: "Pairing expiry"
                        )
                        : error.localizedText
                )
            case .success:
                // Rust's poll observes the approval, exchanges the pairing,
                // and emits the close action that dismisses this screen.
                self.statusLabel.text = NSLocalizedString(
                    "nativeLogin.completing",
                    value: "Finishing sign-in…",
                    comment: "Post-approval wait"
                )
                self.statusLabel.isHidden = false
            }
        }
    }

    @objc private func providerCardTapped(_ sender: UIButton) {
        let providerID = sender.accessibilityIdentifier
        if providerID == "apple" {
            startAppleNativeSignIn()
            return
        }
        openProviderLogin(providerID: providerID)
    }

    /// Apple sign-in runs fully natively: the system sheet mints an identity
    /// token bound to a fresh nonce, the SSO exchanges it for a session in
    /// this screen's cookie jar, and the running pairing is approved without
    /// any web page. A user cancel just returns to this screen; a failure
    /// surfaces an error instead of bouncing to the browser flow.
    private func startAppleNativeSignIn() {
        setBusy(true)
        let signIn = AppleNativeSignIn { [weak self] in
            self?.view.window
        }
        signIn.start { [weak self] outcome in
            guard let self else { return }
            switch outcome {
            case .canceled:
                self.setBusy(false)
            case .failed:
                self.setBusy(false)
                self.showError(NSLocalizedString(
                    "nativeLogin.error.appleUnavailable",
                    value: "Apple sign-in is unavailable. Sign in to your Apple Account in Settings and try again.",
                    comment: "Native Apple sheet failure"
                ))
            case .authorized(let identityToken, let nonce):
                self.client.nativeLogin(
                    providerID: "apple",
                    identityToken: identityToken,
                    nonce: nonce
                ) { [weak self] result in
                    guard let self else { return }
                    switch result {
                    case .failure(let error):
                        self.setBusy(false)
                        self.showError(error.localizedText)
                    case .success:
                        self.approvePairing()
                    }
                }
            }
        }
    }

    /// Providers without a native SDK stay inside the app on their own OAuth
    /// page: the SSO start endpoint 302s straight to the provider's authorize
    /// screen carrying the pairing, and the callback lands directly on the
    /// dedicated pairing-approval page — the ZSeven login page never appears.
    /// The deliberate approve tap is kept so a shared start link cannot
    /// silently sign a foreign device in.
    private func openProviderLogin(providerID: String?) {
        guard let providerID, !providerID.isEmpty else { return }
        var components = URLComponents(
            url: origin.appendingPathComponent(
                "/api/v1/auth/providers/\(providerID)/start"
            ),
            resolvingAgainstBaseURL: false
        )
        components?.queryItems = [
            URLQueryItem(name: "channel", value: "web_mobile"),
            URLQueryItem(name: "device_pairing", value: pairingID),
        ]
        guard let url = components?.url else { return }
        statusLabel.text = NSLocalizedString(
            "nativeLogin.browserHint",
            value: "Finish signing in, and you will return automatically.",
            comment: "In-app provider sign-in hint"
        )
        statusLabel.isHidden = false
        present(SFSafariViewController(url: url), animated: true)
    }

    @objc private func openRegister() {
        navigationController?.pushViewController(
            RegisterViewController(login: self),
            animated: true
        )
    }

    @objc private func openForgotPassword() {
        navigationController?.pushViewController(
            ForgotPasswordViewController(login: self),
            animated: true
        )
    }

    /// The region row toggles directly between Mainland China and Global —
    /// there are exactly two deployments — and applies on the next launch
    /// because the auth runtime initializes once per process.
    @objc private func toggleRegion() {
        let next: SsoRegion = SsoRegionStore.resolved() == .china ? .global : .china
        SsoRegionStore.saveUserOverride(next)
        regionLabel.text = regionRowTitle()
        regionNoteLabel.text = NSLocalizedString(
            "sso.region.restartNote",
            value: "A region change takes effect after you restart OpenPencil.",
            comment: "Region restart note"
        )
        regionNoteLabel.isHidden = false
        // The auth runtime initializes once per process, and reaching this
        // screen means it already initialized for the previous region — so
        // offer an immediate relaunch alongside the passive note.
        let alert = UIAlertController(
            title: String(
                format: NSLocalizedString(
                    "sso.region.switchedTitle",
                    value: "Region set to %@",
                    comment: "Region switched (%@ = region name)"
                ),
                next.displayName
            ),
            message: NSLocalizedString(
                "sso.region.restartNote",
                value: "A region change takes effect after you restart OpenPencil.",
                comment: "Region restart note"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("sso.region.later", value: "Later", comment: "Defer"),
            style: .cancel
        ))
        alert.addAction(UIAlertAction(
            title: NSLocalizedString(
                "sso.region.quitNow",
                value: "Quit now",
                comment: "Exit to apply region"
            ),
            style: .destructive
        ) { _ in
            // A clean exit; the user relaunches into the new region.
            exit(0)
        })
        present(alert, animated: true)
    }

    private func regionRowTitle() -> String {
        String(
            format: NSLocalizedString(
                "nativeLogin.region",
                value: "Region: %@ · Switch",
                comment: "Region row (%@ = region name)"
            ),
            SsoRegionStore.resolved().displayName
        )
    }

    // MARK: - State helpers

    private func setBusy(_ busy: Bool) {
        signInButton.isEnabled = !busy
        emailField.isEnabled = !busy
        passwordField.isEnabled = !busy
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

extension NativeLoginViewController: UITextFieldDelegate {
    func textFieldShouldReturn(_ textField: UITextField) -> Bool {
        if textField === emailField {
            passwordField.becomeFirstResponder()
        } else {
            signInPressed()
        }
        return true
    }
}

/// Wraps the login screen in an app-owned navigation bar; register and
/// password-recovery screens push onto this same stack.
func makeNativeLoginPresentation(
    verificationURL: URL,
    onCancel: @escaping () -> Void
) -> (UINavigationController, NativeLoginViewController)? {
    guard
        let controller = NativeLoginViewController(
            verificationURL: verificationURL,
            onCancel: onCancel
        )
    else { return nil }
    let navigation = UINavigationController(rootViewController: controller)
    navigation.modalPresentationStyle = .formSheet
    navigation.isModalInPresentation = true
    return (navigation, controller)
}
