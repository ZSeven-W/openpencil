import Foundation

/// Result of one native provider-SDK authorization attempt (Douyin /
/// Alipay): an auth code for the SSO native-login exchange, a user cancel
/// that simply returns to the login screen, or a failure surfaced inline.
enum NativeSignInOutcome {
    case authorized(authCode: String)
    case canceled
    case failed
}

/// Routes scheme callbacks from native provider SDK flows (Douyin / Alipay)
/// back into their SDKs. Wired to the SwiftUI scene's `onOpenURL`.
enum NativeProviderCallbacks {
    @discardableResult
    static func handle(_ url: URL) -> Bool {
        if DouyinNativeSignIn.handleOpenURL(url) { return true }
        if AlipayNativeSignIn.handleOpenURL(url) { return true }
        if WechatNativeSignIn.handleOpenURL(url) { return true }
        // SwiftUI delivers universal links here as bare URLs; WeChat's
        // callback must reach its SDK before the generic router can
        // swallow the /app-links/wechat/ path.
        return WechatNativeSignIn.handleUniversalLinkURL(url)
    }
}
