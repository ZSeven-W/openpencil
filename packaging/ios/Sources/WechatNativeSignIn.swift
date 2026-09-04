import Foundation
import UIKit

#if canImport(WechatOpenSDK) && !targetEnvironment(simulator)
import WechatOpenSDK
#endif

/// Runs the WeChat OpenSDK authorization flow (SendAuth) and reports the
/// auth code.
///
/// The SDK opens the WeChat app and returns through the app's registered
/// universal link (`https://op.zseven.cn/app-links/wechat/`), forwarded by
/// the scene's continue-user-activity handler; the legacy `wx…` scheme
/// callback stays wired as a fallback. The caller exchanges the code at
/// `POST /api/v1/auth/providers/wechat/native-login` and then approves the
/// running device pairing.
///
/// The vendored `WechatOpenSDK.framework` is the xcframework's device
/// slice, so simulator builds compile a stub that always fails.
enum WechatNativeSignIn {
    /// AppID of the WeChat open-platform mobile app; also the app's legacy
    /// callback URL scheme.
    static let appID = "wx327d6a759ea9fe62"

    /// Must byte-match the universal link registered for this app in the
    /// WeChat console, trailing slash included.
    static let universalLink = "https://op.zseven.cn/app-links/wechat/"

    #if targetEnvironment(simulator) || !canImport(WechatOpenSDK)

    static func start(
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        completion(.failed)
    }

    static func handleOpenURL(_ url: URL) -> Bool { false }

    static func handleUniversalLink(_ userActivity: NSUserActivity) -> Bool { false }

    static func handleUniversalLinkURL(_ url: URL) -> Bool { false }

    #else

    /// Retained WXApiDelegate bridge; the SDK holds it weakly.
    private final class Delegate: NSObject, WXApiDelegate {
        func onReq(_ req: BaseReq) {}

        func onResp(_ resp: BaseResp) {
            guard let auth = resp as? SendAuthResp else { return }
            let outcome: NativeSignInOutcome
            if auth.errCode == WXErrCodeUserCancel.rawValue {
                outcome = .canceled
            } else if auth.errCode == WXSuccess.rawValue,
                let code = auth.code, !code.isEmpty
            {
                outcome = .authorized(authCode: code)
            } else {
                outcome = .failed
            }
            WechatNativeSignIn.deliver(outcome)
        }
    }

    private static let delegate = Delegate()
    private static var registered = false
    private static var pending: ((NativeSignInOutcome) -> Void)?

    private static func registerIfNeeded() {
        guard !registered else { return }
        registered = true
        _ = WXApi.registerApp(appID, universalLink: universalLink)
    }

    /// Starts the authorization UI; at most one attempt runs at a time.
    /// `state` is the SSO-issued single-use value from `native-login-start`;
    /// it rides the authorization round trip and the server redeems the
    /// code only together with this exact state.
    static func start(
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        guard pending == nil else {
            completion(.failed)
            return
        }
        registerIfNeeded()
        let request = SendAuthReq()
        request.scope = "snsapi_userinfo"
        request.state = state
        pending = completion
        WXApi.send(request) { success in
            if !success {
                DispatchQueue.main.async { deliver(.failed) }
            }
        }
    }

    /// Forwards a legacy scheme callback into the SDK.
    static func handleOpenURL(_ url: URL) -> Bool {
        guard url.scheme == appID else { return false }
        registerIfNeeded()
        return WXApi.handleOpen(url, delegate: delegate)
    }

    /// Forwards the returning universal link into the SDK; returns whether
    /// the SDK consumed it.
    static func handleUniversalLink(_ userActivity: NSUserActivity) -> Bool {
        registerIfNeeded()
        return WXApi.handleOpenUniversalLink(userActivity, delegate: delegate)
    }

    /// The SwiftUI lifecycle can deliver universal links through `onOpenURL`
    /// (a bare URL) instead of the user-activity handler; wrap such a URL
    /// back into an activity so the SDK sees its callback either way.
    static func handleUniversalLinkURL(_ url: URL) -> Bool {
        guard
            let linkURL = URL(string: universalLink),
            url.scheme == "https",
            url.host == linkURL.host,
            url.path.hasPrefix(linkURL.path)
        else { return false }
        let activity = NSUserActivity(activityType: NSUserActivityTypeBrowsingWeb)
        activity.webpageURL = url
        return handleUniversalLink(activity)
    }

    private static func deliver(_ outcome: NativeSignInOutcome) {
        let completion = pending
        pending = nil
        completion?(outcome)
    }

    #endif
}
