import { common, Want } from '@kit.AbilityKit';
import {
  BaseReq,
  BaseResp,
  ErrCode,
  SendAuthReq,
  SendAuthResp,
  WXAPIFactory,
  WXApi,
  WXApiEventHandler,
} from '@tencent/wechat_open_sdk';
import { NativeSignInOutcome } from './NativeSignInOutcome';

/**
 * Runs the WeChat OpenSDK authorization flow (SendAuth) and reports the
 * auth code.
 *
 * The SDK opens the WeChat app and returns by relaunching this ability;
 * EntryAbility drains the returning want through [handleWant]. The caller
 * exchanges the code at `POST /api/v1/auth/providers/wechat/native-login`
 * and then approves the running device pairing.
 */
export class WechatNativeSignIn {
  /** AppID of the WeChat open-platform mobile app. */
  static readonly APP_ID: string = 'wx327d6a759ea9fe62';

  private static api: WXApi | null = null;
  private static pending: ((outcome: NativeSignInOutcome) => void) | null = null;

  private static readonly handler: WXApiEventHandler = {
    onReq: (_req: BaseReq): void => {},
    onResp: (resp: BaseResp): void => {
      if (resp instanceof SendAuthResp) {
        const code: string = resp.code ?? '';
        WechatNativeSignIn.deliver(resp.errCode === ErrCode.ERR_USER_CANCEL
          ? NativeSignInOutcome.canceled()
          : resp.errCode === ErrCode.ERR_OK && code.length > 0
            ? NativeSignInOutcome.authorized(code)
            : NativeSignInOutcome.failed());
      } else {
        WechatNativeSignIn.deliverError();
      }
    },
  };

  private static ensureApi(): WXApi {
    let api = WechatNativeSignIn.api;
    if (api === null) {
      api = WXAPIFactory.createWXAPI(WechatNativeSignIn.APP_ID);
      WechatNativeSignIn.api = api;
    }
    return api;
  }

  /**
   * Starts the authorization UI; at most one attempt runs at a time.
   * `state` is the SSO-issued single-use value from `native-login-start`;
   * it rides the authorization round trip and the server redeems the code
   * only together with this exact state.
   */
  static start(
    context: common.UIAbilityContext,
    state: string,
    completion: (outcome: NativeSignInOutcome) => void
  ): void {
    if (WechatNativeSignIn.pending !== null) {
      completion(NativeSignInOutcome.failed());
      return;
    }
    const request = new SendAuthReq();
    request.scope = 'snsapi_userinfo';
    request.state = state;
    request.callbackAbility = 'EntryAbility';
    WechatNativeSignIn.pending = completion;
    try {
      Promise.resolve(WechatNativeSignIn.ensureApi().sendReq(context, request))
        .then((launched: boolean): void => {
          if (!launched) {
            WechatNativeSignIn.deliverError();
          }
        })
        .catch((_error: Object): void => {
          WechatNativeSignIn.deliverError();
        });
    } catch (_error) {
      WechatNativeSignIn.deliverError();
    }
  }

  /** Forwards the returning want into the SDK; true when it consumed it. */
  static handleWant(want: Want): boolean {
    try {
      return WechatNativeSignIn.ensureApi().handleWant(want, WechatNativeSignIn.handler);
    } catch (_error) {
      return false;
    }
  }

  private static deliver(outcome: NativeSignInOutcome): void {
    const completion = WechatNativeSignIn.pending;
    if (completion === null) {
      return;
    }
    WechatNativeSignIn.pending = null;
    completion(outcome);
  }

  private static deliverError(): void {
    WechatNativeSignIn.deliver(NativeSignInOutcome.failed());
  }
}
