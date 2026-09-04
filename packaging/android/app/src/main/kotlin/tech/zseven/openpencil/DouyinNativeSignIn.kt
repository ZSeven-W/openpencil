package tech.zseven.openpencil

import android.app.Activity
import com.bytedance.sdk.open.aweme.authorize.model.Authorization
import com.bytedance.sdk.open.douyin.DouYinOpenApiFactory
import com.bytedance.sdk.open.douyin.DouYinOpenConfig

/**
 * Runs the Douyin OpenSDK authorization flow and reports the auth code.
 *
 * The SDK opens the Douyin app (or its built-in H5 page when Douyin is not
 * installed) and returns through [tech.zseven.openpencil.douyinapi.DouYinEntryActivity],
 * which relays the response here. The caller exchanges the code at
 * `POST /api/v1/auth/providers/douyin/native-login` and then approves the
 * running device pairing.
 */
internal object DouyinNativeSignIn {
    /** Client key of the Douyin open-platform mobile app. */
    const val CLIENT_KEY = "awbponwo0ls6cjos"

    private var initialized = false
    private var pending: ((NativeSignInOutcome) -> Unit)? = null

    /**
     * Starts the authorization UI; at most one attempt runs at a time.
     * [state] is the SSO-issued single-use value from `native-login-start`;
     * it rides the authorization round trip and the server redeems the code
     * only together with this exact state.
     */
    fun start(activity: Activity, state: String, completion: (NativeSignInOutcome) -> Unit) {
        if (pending != null) {
            completion(NativeSignInOutcome.Failed)
            return
        }
        if (!initialized) {
            DouYinOpenApiFactory.init(DouYinOpenConfig(CLIENT_KEY))
            initialized = true
        }
        val request = Authorization.Request()
        request.scope = "user_info"
        request.state = state
        pending = completion
        val started = try {
            DouYinOpenApiFactory.create(activity).authorize(request)
        } catch (_: Exception) {
            false
        }
        if (!started) {
            pending = null
            completion(NativeSignInOutcome.Failed)
        }
    }

    /** Called by DouYinEntryActivity with the SDK's authorization response. */
    fun deliver(response: Authorization.Response) {
        val completion = pending ?: return
        pending = null
        val code = response.authCode
        val outcome = when {
            response.isCancel -> NativeSignInOutcome.Canceled
            response.isSuccess && !code.isNullOrEmpty() ->
                NativeSignInOutcome.Authorized(code)
            else -> NativeSignInOutcome.Failed
        }
        completion(outcome)
    }

    /** Called by DouYinEntryActivity when the SDK hands over a broken intent. */
    fun deliverError() {
        val completion = pending ?: return
        pending = null
        completion(NativeSignInOutcome.Failed)
    }
}
