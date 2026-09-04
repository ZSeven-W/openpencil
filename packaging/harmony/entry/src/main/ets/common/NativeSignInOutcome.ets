/**
 * Result of one native provider-SDK authorization attempt (Douyin / Alipay):
 * an auth code for the SSO native-login exchange, a user cancel that simply
 * returns to the login page, or a failure surfaced as an inline error.
 */
export enum NativeSignInKind {
  AUTHORIZED = 0,
  CANCELED = 1,
  FAILED = 2,
}

export class NativeSignInOutcome {
  readonly kind: NativeSignInKind;
  readonly authCode: string;

  private constructor(kind: NativeSignInKind, authCode: string) {
    this.kind = kind;
    this.authCode = authCode;
  }

  static authorized(authCode: string): NativeSignInOutcome {
    return new NativeSignInOutcome(NativeSignInKind.AUTHORIZED, authCode);
  }

  static canceled(): NativeSignInOutcome {
    return new NativeSignInOutcome(NativeSignInKind.CANCELED, '');
  }

  static failed(): NativeSignInOutcome {
    return new NativeSignInOutcome(NativeSignInKind.FAILED, '');
  }
}
