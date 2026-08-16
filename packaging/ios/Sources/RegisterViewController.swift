import UIKit

/// Native account registration, styled after the ZSeven web sign-up page:
/// email + emailed verification code + password (with a live rules
/// checklist) + confirmation, then a gradient primary button. A successful
/// registration signs in with the same credentials and approves the login
/// screen's device pairing, so the whole flow stays native.
final class RegisterViewController: AuthCodeFormViewController {
    override var formTitle: String {
        NSLocalizedString("register.title", value: "Create account", comment: "Register title")
    }

    override var formSubtitle: String {
        NSLocalizedString(
            "register.subtitle",
            value: "Sign up for ZSeven",
            comment: "Register subtitle"
        )
    }

    override var submitTitle: String {
        NSLocalizedString("register.submit", value: "Sign Up", comment: "Register button")
    }

    override var codePurpose: String { "register" }
    override var showsPasswordRules: Bool { true }

    override var footerPrompt: (prompt: String, link: String)? {
        (
            NSLocalizedString(
                "register.haveAccount",
                value: "Already have an account?",
                comment: "Login prompt"
            ),
            NSLocalizedString("register.signInNow", value: "Sign in", comment: "Login link")
        )
    }

    override func submit(
        email: String,
        code: String,
        password: String,
        completion: @escaping (SsoAuthError?) -> Void
    ) {
        login.client.register(email: email, code: code, password: password) { result in
            switch result {
            case .success: completion(nil)
            case .failure(let error): completion(error)
            }
        }
    }
}

/// Native password recovery: identical form shape minus the sign-up footer.
final class ForgotPasswordViewController: AuthCodeFormViewController {
    override var formTitle: String {
        NSLocalizedString("reset.title", value: "Reset password", comment: "Reset title")
    }

    override var formSubtitle: String {
        NSLocalizedString(
            "reset.subtitle",
            value: "We'll email you a verification code",
            comment: "Reset subtitle"
        )
    }

    override var submitTitle: String {
        NSLocalizedString("reset.submit", value: "Reset and sign in", comment: "Reset button")
    }

    override var codePurpose: String { "password_reset" }
    override var showsPasswordRules: Bool { true }
    override var footerPrompt: (prompt: String, link: String)? { nil }

    override func submit(
        email: String,
        code: String,
        password: String,
        completion: @escaping (SsoAuthError?) -> Void
    ) {
        login.client.resetPassword(email: email, code: code, newPassword: password) { result in
            switch result {
            case .success: completion(nil)
            case .failure(let error): completion(error)
            }
        }
    }
}
