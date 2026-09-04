import UIKit

#if canImport(DouyinOpenSDK) && !targetEnvironment(simulator)
import DouyinOpenSDK
#endif

/// Runs the Douyin OpenSDK authorization flow and reports the auth code.
///
/// The SDK opens the Douyin app (or its built-in H5 page when Douyin is not
/// installed) and returns through the app's `awbponwo…` URL scheme, which
/// `NativeProviderCallbacks` forwards back into the SDK. The caller exchanges
/// the code at `POST /api/v1/auth/providers/douyin/native-login` and then
/// approves the running device pairing.
///
/// The vendored `DouyinOpenSDK.framework` ships device (arm64) and x86_64
/// slices only, so simulator arm64 builds compile a stub that always fails —
/// the login screen shows its inline error there.
enum DouyinNativeSignIn {
    /// Client key of the Douyin open-platform mobile app. Also the app's
    /// callback URL scheme and the Info.plist `DouyinAppID`.
    static let clientKey = "awbponwo0ls6cjos"

    #if targetEnvironment(simulator) || !canImport(DouyinOpenSDK)

    static func start(
        from viewController: UIViewController,
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        completion(.failed)
    }

    static func handleOpenURL(_ url: URL) -> Bool { false }

    #else

    private static var registered = false

    private static func registerIfNeeded() {
        guard !registered else { return }
        registered = true
        let delegate = DouyinOpenSDKApplicationDelegate.sharedInstance()
        delegate.application(UIApplication.shared, didFinishLaunchingWithOptions: nil)
        _ = delegate.registerAppId(clientKey)
    }

    static func start(
        from viewController: UIViewController,
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        registerIfNeeded()
        let request = DouyinOpenSDKAuthRequest()
        request.permissions = NSOrderedSet(object: "user_info")
        // The SSO-issued single-use state rides the whole round trip; the
        // server redeems the code only together with this exact state.
        request.state = state
        _ = request.send(viewController) { response in
            DispatchQueue.main.async {
                guard let response else {
                    completion(.failed)
                    return
                }
                if response.errCode.rawValue == -2 {
                    completion(.canceled)
                    return
                }
                guard response.isSucceed, let code = response.code, !code.isEmpty else {
                    completion(.failed)
                    return
                }
                completion(.authorized(authCode: code))
            }
        }
    }

    /// Forwards a scheme callback into the SDK; returns whether it was one.
    static func handleOpenURL(_ url: URL) -> Bool {
        registerIfNeeded()
        return DouyinOpenSDKApplicationDelegate.sharedInstance().application(
            UIApplication.shared,
            open: url,
            sourceApplication: nil,
            annotation: ""
        )
    }

    #endif
}
