import { common, Want } from '@kit.AbilityKit';
import {
  douYinOpenSDK,
  DouYinOpenConfigBuilder,
  DouYinOpenListener,
  OpenAuthRequest,
  OpenAuthResponse,
} from '@douyin/opensdk-common-external';
import { NativeSignInOutcome } from './NativeSignInOutcome';
import { DOUYIN_AUTH_APP_LINK } from './SsoRegion';

/**
 * Runs the Douyin OpenSDK authorization flow and reports the auth code.
 *
 * The SDK opens the Douyin app and returns through the configured App Link
 * ([REDIRECT_URI]), delivered to EntryAbility as a want and drained via
 * [handleWant]. The caller exchanges the code at
 * `POST /api/v1/auth/providers/douyin/native-login` and then approves the
 * running device pairing.
 */
export class DouyinNativeSignIn {
  /** Client key of the Douyin open-platform mobile app. */
  static readonly CLIENT_KEY: string = 'awbponwo0ls6cjos';

  /** HTTPS App Link the SDK returns on; owned by SsoRegion.ets. */
  static readonly REDIRECT_URI: string = DOUYIN_AUTH_APP_LINK;

  private static initialized: boolean = false;
  private static pending: ((outcome: NativeSignInOutcome) => void) | null = null;

  private static ensureInit(): void {
    if (DouyinNativeSignIn.initialized) {
      return;
    }
    DouyinNativeSignIn.initialized = true;
    douYinOpenSDK.init(new DouYinOpenConfigBuilder(DouyinNativeSignIn.CLIENT_KEY)
      .setDebug(false)
      .build());
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
    if (DouyinNativeSignIn.pending !== null) {
      completion(NativeSignInOutcome.failed());
      return;
    }
    DouyinNativeSignIn.ensureInit();
    const api = douYinOpenSDK.createDouYinOpenApi();
    if (api === null) {
      completion(NativeSignInOutcome.failed());
      return;
    }
    const request = new OpenAuthRequest();
    request.scope = 'user_info';
    request.state = state;
    request.callerLocalEntry = 'EntryAbility';
    request.redirectUri = DouyinNativeSignIn.REDIRECT_URI;
    DouyinNativeSignIn.pending = completion;
    const listener: DouYinOpenListener = {
      onSuccess: (response: OpenAuthResponse, authCode: string): void => {
        const code: string = authCode.length > 0 ? authCode : (response.authCode ?? '');
        DouyinNativeSignIn.deliver(code.length > 0
          ? NativeSignInOutcome.authorized(code)
          : NativeSignInOutcome.failed());
      },
      onError: (_response: Object, errCode: number, _errMsg: string): void => {
        DouyinNativeSignIn.deliver(errCode === -2
          ? NativeSignInOutcome.canceled()
          : NativeSignInOutcome.failed());
      },
    };
    try {
      api.authorize(context, request, listener);
    } catch (_error) {
      DouyinNativeSignIn.deliver(NativeSignInOutcome.failed());
    }
  }

  /** Forwards the returning App Link want into the SDK. */
  static handleWant(context: common.UIAbilityContext, want: Want): void {
    if (DouyinNativeSignIn.pending === null) {
      return;
    }
    try {
      douYinOpenSDK.doResult(context, want);
    } catch (_error) {
      DouyinNativeSignIn.deliver(NativeSignInOutcome.failed());
    }
  }

  private static deliver(outcome: NativeSignInOutcome): void {
    const completion = DouyinNativeSignIn.pending;
    if (completion === null) {
      return;
    }
    DouyinNativeSignIn.pending = null;
    completion(outcome);
  }
}
