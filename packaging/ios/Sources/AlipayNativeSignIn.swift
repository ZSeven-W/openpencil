import Foundation

#if canImport(AFServiceSDK) && !targetEnvironment(simulator)
import AFServiceSDK
#endif

/// Runs the Alipay in-app authorization ("PURE_OAUTH_SDK", no server-signed
/// payload) via the AFServiceSDK and reports the auth code.
///
/// The SDK opens the Alipay app on the unsigned authweb URL for the mobile
/// AppID and jumps back through `openpencilalipay://apmqpdispatch/…`, which
/// `NativeProviderCallbacks` forwards back into the SDK. The caller
/// exchanges the code at `POST /api/v1/auth/providers/alipay/native-login`
/// and then approves the running device pairing.
///
/// The vendored `AFServiceSDK.framework` ships device (arm64) and x86_64
/// slices only, so simulator arm64 builds compile a stub that always fails.
enum AlipayNativeSignIn {
    /// AppID of the Alipay open-platform mobile app.
    static let appID = "2021006190626680"

    /// App-unique return scheme, registered in the Info.plist URL types.
    static let callbackScheme = "openpencilalipay"

    #if targetEnvironment(simulator) || !canImport(AFServiceSDK)

    static func start(
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        completion(.failed)
    }

    static func handleOpenURL(_ url: URL) -> Bool { false }

    #else

    /// `state` is the SSO-issued single-use value from `native-login-start`;
    /// it rides the authorization round trip and the server redeems the code
    /// only together with this exact state.
    static func start(
        state: String,
        completion: @escaping (NativeSignInOutcome) -> Void
    ) {
        let authURL = "https://authweb.alipay.com/auth?auth_type=PURE_OAUTH_SDK"
            + "&app_id=\(appID)&scope=auth_user&state=\(state)"
        let params: [AnyHashable: Any] = [
            kAFServiceOptionBizParams: [kAFServiceBizParamsKeyUrl: authURL],
            kAFServiceOptionCallbackScheme: callbackScheme,
        ]
        AFServiceCenter.call(AFService.auth, withParams: params) { response in
            DispatchQueue.main.async {
                completion(outcome(of: response, expectedState: state))
            }
        }
    }

    /// The SDK dispatches its return URLs on the `apmqpdispatch` host; the
    /// normal path resolves the pending `callService` block, and this entry
    /// covers cold starts after the app was reclaimed mid-authorization —
    /// where no pairing flow is running any more, so the result is dropped.
    static func handleOpenURL(_ url: URL) -> Bool {
        guard url.scheme == callbackScheme, url.host == "apmqpdispatch" else {
            return false
        }
        AFServiceCenter.handleResponseURL(url) { _ in }
        return true
    }

    private static func outcome(
        of response: AFAuthServiceResponse?,
        expectedState: String
    ) -> NativeSignInOutcome {
        guard let response, response.responseCode == .success else {
            return .failed
        }
        let result = response.result as? [String: Any] ?? [:]
        let resultCode = result["result_code"] as? String
        guard let code = result["auth_code"] as? String, !code.isEmpty else {
            // Alipay signals a user abort with the standard 6001 status.
            return resultCode == "6001" ? .canceled : .failed
        }
        if let returnedState = result["state"] as? String, returnedState != expectedState {
            return .failed
        }
        return .authorized(authCode: code)
    }

    #endif
}
