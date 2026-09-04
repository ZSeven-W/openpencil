package tech.zseven.openpencil

import android.content.Context
import com.tencent.mm.opensdk.modelmsg.SendAuth
import com.tencent.mm.opensdk.openapi.IWXAPI
import com.tencent.mm.opensdk.openapi.WXAPIFactory

/**
 * Runs the WeChat OpenSDK authorization flow (SendAuth) and reports the
 * auth code.
 *
 * The SDK opens the WeChat app and returns through
 * [tech.zseven.openpencil.wxapi.WXEntryActivity], which relays the response
 * here. The caller exchanges the code at
 * `POST /api/v1/auth/providers/wechat/native-login` and then approves the
 * running device pairing.
 */
internal object WechatNativeSignIn {
    /** AppID of the WeChat open-platform mobile app. */
    const val APP_ID = "wx327d6a759ea9fe62"

    private var api: IWXAPI? = null
    private var pending: ((NativeSignInOutcome) -> Unit)? = null

    /** One process-wide SDK handle; also serves WXEntryActivity's intents. */
    fun api(context: Context): IWXAPI {
        api?.let { return it }
        val created = WXAPIFactory.createWXAPI(context.applicationContext, APP_ID, true)
        created.registerApp(APP_ID)
        api = created
        return created
    }

    /**
     * Starts the authorization UI; at most one attempt runs at a time.
     * [state] is the SSO-issued single-use value from `native-login-start`;
     * it rides the authorization round trip and the server redeems the code
     * only together with this exact state.
     */
    fun start(context: Context, state: String, completion: (NativeSignInOutcome) -> Unit) {
        if (pending != null) {
            completion(NativeSignInOutcome.Failed)
            return
        }
        val request = SendAuth.Req()
        request.scope = "snsapi_userinfo"
        request.state = state
        pending = completion
        val started = try {
            api(context).sendReq(request)
        } catch (_: Exception) {
            false
        }
        if (!started) {
            pending = null
            completion(NativeSignInOutcome.Failed)
        }
    }

    /** Called by WXEntryActivity with the SDK's authorization response. */
    fun deliver(response: SendAuth.Resp) {
        val completion = pending ?: return
        pending = null
        val code = response.code
        val outcome = when {
            response.errCode == USER_CANCELED -> NativeSignInOutcome.Canceled
            response.errCode == OK && !code.isNullOrEmpty() ->
                NativeSignInOutcome.Authorized(code)
            else -> NativeSignInOutcome.Failed
        }
        completion(outcome)
    }

    /** Called by WXEntryActivity when the SDK hands over a broken intent. */
    fun deliverError() {
        val completion = pending ?: return
        pending = null
        completion(NativeSignInOutcome.Failed)
    }

    private const val OK = 0
    private const val USER_CANCELED = -2
}
