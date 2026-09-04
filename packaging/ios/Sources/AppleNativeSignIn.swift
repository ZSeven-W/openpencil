import AuthenticationServices
import UIKit

/// Runs the system Sign in with Apple sheet and reports the identity token
/// plus the request-bound nonce.
///
/// The nonce is generated per attempt in the exact shape the SSO backend
/// requires (32 random bytes, base64url) and attached to the authorization
/// request, so the returned identity token can only satisfy this attempt.
/// The caller exchanges the token at
/// `POST /api/v1/auth/providers/apple/native-login` and then approves the
/// running device pairing with the resulting session.
final class AppleNativeSignIn: NSObject {
    enum Outcome {
        case authorized(identityToken: String, nonce: String, displayName: String)
        case canceled
        case failed
    }

    private let anchorProvider: () -> UIWindow?
    private var completion: ((Outcome) -> Void)?
    private var nonce = ""
    /// Keeps the coordinator alive for the duration of the system sheet.
    private var retainedSelf: AppleNativeSignIn?

    init(anchorProvider: @escaping () -> UIWindow?) {
        self.anchorProvider = anchorProvider
    }

    func start(completion: @escaping (Outcome) -> Void) {
        var randomBytes = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, randomBytes.count, &randomBytes)
            == errSecSuccess else {
            completion(.failed)
            return
        }
        nonce = Data(randomBytes).base64URLEncodedString()
        self.completion = completion
        retainedSelf = self

        let request = ASAuthorizationAppleIDProvider().createRequest()
        // Apple reveals the person's name only here, and only on the FIRST
        // authorization for this app; the identity token never carries it.
        request.requestedScopes = [.fullName]
        request.nonce = nonce
        let controller = ASAuthorizationController(authorizationRequests: [request])
        controller.delegate = self
        controller.presentationContextProvider = self
        controller.performRequests()
    }

    private func finish(_ outcome: Outcome) {
        let completion = self.completion
        self.completion = nil
        retainedSelf = nil
        completion?(outcome)
    }
}

extension AppleNativeSignIn: ASAuthorizationControllerDelegate {
    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithAuthorization authorization: ASAuthorization
    ) {
        guard
            let credential = authorization.credential as? ASAuthorizationAppleIDCredential,
            let tokenData = credential.identityToken,
            let token = String(data: tokenData, encoding: .utf8),
            !token.isEmpty
        else {
            finish(.failed)
            return
        }
        let displayName = credential.fullName.map {
            PersonNameComponentsFormatter.localizedString(from: $0, style: .default)
        } ?? ""
        finish(.authorized(identityToken: token, nonce: nonce, displayName: displayName))
    }

    func authorizationController(
        controller: ASAuthorizationController,
        didCompleteWithError error: Error
    ) {
        let canceled = (error as? ASAuthorizationError)?.code == .canceled
        finish(canceled ? .canceled : .failed)
    }
}

extension AppleNativeSignIn: ASAuthorizationControllerPresentationContextProviding {
    func presentationAnchor(for controller: ASAuthorizationController) -> ASPresentationAnchor {
        anchorProvider() ?? ASPresentationAnchor()
    }
}

extension Data {
    /// Base64url without padding — the 43-character nonce shape the SSO
    /// backend validates.
    func base64URLEncodedString() -> String {
        base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }
}
