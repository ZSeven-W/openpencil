package tech.zseven.openpencil.wxapi

import android.app.Activity
import android.os.Bundle
import com.tencent.mm.opensdk.modelbase.BaseReq
import com.tencent.mm.opensdk.modelbase.BaseResp
import com.tencent.mm.opensdk.modelmsg.SendAuth
import com.tencent.mm.opensdk.openapi.IWXAPIEventHandler
import tech.zseven.openpencil.WechatNativeSignIn

/**
 * WeChat OpenSDK callback landing activity. The SDK requires the fixed
 * `<applicationId>.wxapi.WXEntryActivity` name; it only parses the
 * returning intent, relays the auth response to [WechatNativeSignIn], and
 * finishes immediately — it never shows UI of its own.
 */
class WXEntryActivity : Activity(), IWXAPIEventHandler {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val handled = try {
            WechatNativeSignIn.api(this).handleIntent(intent, this)
        } catch (_: Exception) {
            false
        }
        if (!handled) {
            WechatNativeSignIn.deliverError()
        }
        finish()
    }

    override fun onReq(req: BaseReq?) {}

    override fun onResp(resp: BaseResp?) {
        if (resp is SendAuth.Resp) {
            WechatNativeSignIn.deliver(resp)
        } else {
            WechatNativeSignIn.deliverError()
        }
    }
}
