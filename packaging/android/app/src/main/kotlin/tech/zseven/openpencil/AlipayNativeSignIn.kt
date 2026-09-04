package tech.zseven.openpencil

import android.app.Activity
import com.alipay.sdk.app.OpenAuthTask

/**
 * Runs the Alipay in-app authorization ("极简版授权", PURE_OAUTH_SDK) and
 * reports the auth code.
 *
 * The SDK opens the Alipay app (or an install landing page when Alipay is
 * missing) on the unsigned authweb URL for the mobile AppID and jumps back
 * through [SCHEME] into the SDK's own result activity. The caller exchanges
 * the code at `POST /api/v1/auth/providers/alipay/native-login` and then
 * approves the running device pairing.
 */
internal object AlipayNativeSignIn {
    /** AppID of the Alipay open-platform mobile app. */
    const val APP_ID = "2021006190626680"

    /**
     * App-unique return scheme; must match the intent-filter re-declared on
     * `com.alipay.sdk.app.AlipayResultActivity` in the manifest.
     */
    const val SCHEME = "openpencilalipay"

    private var expectedState: String? = null

    /**
     * Starts the authorization UI; the SDK rejects duplicate calls itself.
     * [state] is the SSO-issued single-use value from `native-login-start`;
     * it rides the authorization round trip and the server redeems the code
     * only together with this exact state.
     */
    fun start(activity: Activity, state: String, completion: (NativeSignInOutcome) -> Unit) {
        expectedState = state
        val bizParams = mapOf(
            "url" to "https://authweb.alipay.com/auth?auth_type=PURE_OAUTH_SDK" +
                "&app_id=$APP_ID&scope=auth_user&state=$state",
        )
        val started = try {
            OpenAuthTask(activity).execute(
                SCHEME,
                OpenAuthTask.BizType.AccountAuth,
                bizParams,
                { resultCode, _, bundle ->
                    val code = bundle?.getString("auth_code")
                    val returnedState = bundle?.getString("state")
                    completion(
                        when {
                            resultCode == USER_CANCELED -> NativeSignInOutcome.Canceled
                            resultCode != OpenAuthTask.OK -> NativeSignInOutcome.Failed
                            code.isNullOrEmpty() -> NativeSignInOutcome.Failed
                            returnedState != null && returnedState != expectedState ->
                                NativeSignInOutcome.Failed
                            else -> NativeSignInOutcome.Authorized(code)
                        },
                    )
                },
                true,
            )
            true
        } catch (_: Exception) {
            false
        }
        if (!started) completion(NativeSignInOutcome.Failed)
    }

    /** Alipay reports a user cancel with the standard 6001 status. */
    private const val USER_CANCELED = 6001
}
