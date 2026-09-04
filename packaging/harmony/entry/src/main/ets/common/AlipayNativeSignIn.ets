import { common, Want } from '@kit.AbilityKit';
import {
  AFAuthResCode,
  AFAuthServiceResponse,
  AFService,
  AFServiceCenter,
  AFServiceParams,
  AFWantParams,
} from '@alipay/afservicesdk';
import { NativeSignInOutcome } from './NativeSignInOutcome';

/**
 * Runs the Alipay in-app authorization ("PURE_OAUTH_SDK", no server-signed
 * payload) via the AFServiceSDK and reports the auth code.
 *
 * The SDK opens the Alipay app on the unsigned authweb URL for the mobile
 * AppID and returns by relaunching this ability; EntryAbility drains the
 * returning want through [handleWant]. The caller exchanges the code at
 * `POST /api/v1/auth/providers/alipay/native-login` and then approves the
 * running device pairing.
 */
export class AlipayNativeSignIn {
  /** AppID of the Alipay open-platform mobile app. */
  static readonly APP_ID: string = '2021006190626680';

  private static expectedState: string = '';

  /**
   * Starts the authorization UI; the SDK rejects duplicate calls itself.
   * `state` is the SSO-issued single-use value from `native-login-start`;
   * it rides the authorization round trip and the server redeems the code
   * only together with this exact state.
   */
  static start(
    context: common.UIAbilityContext,
    state: string,
    completion: (outcome: NativeSignInOutcome) => void
  ): void {
    AlipayNativeSignIn.expectedState = state;
    const authUrl: string = 'https://authweb.alipay.com/auth?auth_type=PURE_OAUTH_SDK'
      + `&app_id=${AlipayNativeSignIn.APP_ID}&scope=auth_user&state=${state}`;
    const bizParams: Map<string, string> = new Map<string, string>();
    bizParams.set('url', encodeURIComponent(authUrl));
    const backWant = new AFWantParams();
    backWant.bundleName = context.abilityInfo.bundleName;
    backWant.moduleName = context.abilityInfo.moduleName;
    backWant.abilityName = context.abilityInfo.name;
    const params = new AFServiceParams(
      bizParams,
      /* isForceH5 */ false,
      /* useLanding */ true,
      'scheme:openpencil',
      backWant,
      (response: AFAuthServiceResponse): void => {
        completion(AlipayNativeSignIn.outcomeOf(response));
      }
    );
    AFServiceCenter.call(AFService.AFServiceAuth, params).catch((_error: Object) => {
      completion(NativeSignInOutcome.failed());
    });
  }

  /** Forwards the returning want into the SDK; true when it consumed it. */
  static handleWant(want: Want): boolean {
    try {
      return AFServiceCenter.handleResponse(want);
    } catch (_error) {
      return false;
    }
  }

  private static outcomeOf(response: AFAuthServiceResponse): NativeSignInOutcome {
    if (response.responseCode === AFAuthResCode.AFAuthResCanceled) {
      return NativeSignInOutcome.canceled();
    }
    if (response.responseCode !== AFAuthResCode.AFAuthResSuccess) {
      return NativeSignInOutcome.failed();
    }
    const result: Record<string, object> | undefined = response.result;
    if (result === undefined) {
      return NativeSignInOutcome.failed();
    }
    const code: string = String(result['auth_code'] ?? '');
    if (code.length === 0) {
      return NativeSignInOutcome.failed();
    }
    const returnedState: string = String(result['state'] ?? '');
    if (returnedState.length > 0 && returnedState !== AlipayNativeSignIn.expectedState) {
      return NativeSignInOutcome.failed();
    }
    return NativeSignInOutcome.authorized(code);
  }

}
