import UIKit

/// Native SSO presentation glue (login screen + account center), split from
/// `OpPlayerView.swift` purely for the 800-line file cap.
extension OpPlayerView {
    // MARK: - Native login + account center

    /// Presents the native sign-in screen for the engine's active device
    /// flow. The verification URL supplies the SSO origin and pairing id;
    /// a URL that is not a valid pairing entry cancels the flow instead.
    func showNativeLogin(url: URL) {
        precondition(Thread.isMainThread)
        // Duplicate actions must not cancel the already-visible screen.
        guard nativeLoginController == nil else { return }
        guard
            !didTearDown,
            let presenter = nearestViewController(),
            presenter.presentedViewController == nil,
            let (navigation, controller) = makeNativeLoginPresentation(
                verificationURL: url,
                onCancel: { [weak self] in self?.host.cancelLoginFlow() }
            )
        else {
            host.cancelLoginFlow()
            return
        }

        if imeTextView.isFirstResponder {
            imeTextView.resignFirstResponder()
        }
        nativeLoginController = controller
        presenter.present(navigation, animated: true)
    }

    /// Rust is authoritative for success/error/cancellation and emits the
    /// close shell action only after its auth state reaches a terminal phase.
    func closeNativeLoginFromHost() {
        precondition(Thread.isMainThread)
        nativeLoginController?.finishFromHost(animated: true)
        nativeLoginController = nil
    }

    /// Presents the native account center over the editor.
    func showAccountCenter(snapshot: AccountSnapshot) {
        precondition(Thread.isMainThread)
        guard
            !didTearDown,
            let presenter = nearestViewController(),
            presenter.presentedViewController == nil
        else { return }
        if imeTextView.isFirstResponder {
            imeTextView.resignFirstResponder()
        }
        let navigation = makeAccountCenterPresentation(snapshot: snapshot) { [weak self] in
            self?.host.signOutAccount()
        }
        presenter.present(navigation, animated: true)
    }
}
