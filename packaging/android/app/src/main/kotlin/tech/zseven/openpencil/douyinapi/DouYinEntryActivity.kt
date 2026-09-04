package tech.zseven.openpencil.douyinapi

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import com.bytedance.sdk.open.aweme.authorize.model.Authorization
import com.bytedance.sdk.open.aweme.common.handler.IApiEventHandler
import com.bytedance.sdk.open.aweme.common.model.BaseReq
import com.bytedance.sdk.open.aweme.common.model.BaseResp
import com.bytedance.sdk.open.douyin.DouYinOpenApiFactory
import tech.zseven.openpencil.DouyinNativeSignIn

/**
 * Douyin OpenSDK callback landing activity. The SDK requires the fixed
 * `<applicationId>.douyinapi.DouYinEntryActivity` name; it only parses the
 * returning intent, relays the auth response to [DouyinNativeSignIn], and
 * finishes immediately — it never shows UI of its own.
 */
class DouYinEntryActivity : Activity(), IApiEventHandler {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val handled = try {
            DouYinOpenApiFactory.create(this).handleIntent(intent, this)
        } catch (_: Exception) {
            false
        }
        if (!handled) {
            DouyinNativeSignIn.deliverError()
        }
        finish()
    }

    override fun onReq(req: BaseReq?) {}

    override fun onResp(resp: BaseResp?) {
        if (resp is Authorization.Response) {
            DouyinNativeSignIn.deliver(resp)
        } else {
            DouyinNativeSignIn.deliverError()
        }
    }

    override fun onErrorIntent(intent: Intent?) {
        DouyinNativeSignIn.deliverError()
    }
}
