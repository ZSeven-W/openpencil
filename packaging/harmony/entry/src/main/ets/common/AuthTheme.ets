import { SsoAuthError } from './SsoAuthClient';

/** One live password-policy row (mirrors the backend's `validNewPassword`). */
export class PasswordRule {
  readonly label: Resource;
  readonly test: (value: string) => boolean;

  constructor(label: Resource, test: (value: string) => boolean) {
    this.label = label;
    this.test = test;
  }
}

/**
 * Shared visual language for the native SSO screens, matching the ZSeven web
 * sign-in design and `AuthUi.kt`: labeled boxed inputs, a blue→purple gradient
 * primary button, hairline-bordered provider icon cards, and blue text links.
 *
 * `night` is passed in rather than read here: the page owns the ability
 * configuration and re-reads it on a colour-mode change.
 */
export class AuthTheme {
  static readonly ACCENT: string = '#FF2E6BFF';
  static readonly GRADIENT_END: string = '#FF7A5CFF';
  static readonly DANGER: string = '#FFD32F2F';
  static readonly SATISFIED: string = '#FF2E7D32';

  static text(night: boolean): string {
    return night ? '#FFFFFFFF' : '#FF000000';
  }

  static secondary(night: boolean): string {
    return night ? '#FF9E9E9E' : '#FF616161';
  }

  static background(night: boolean): string {
    return night ? '#FF101010' : '#FFFFFFFF';
  }

  static border(night: boolean): string {
    return night ? '#2EFFFFFF' : '#1F000000';
  }

  static fieldFill(night: boolean): string {
    return night ? '#14FFFFFF' : '#08000000';
  }

  static cardFill(night: boolean): string {
    return night ? '#14FFFFFF' : '#FFFFFFFF';
  }

  /** Brand gradient used by every primary button. */
  static primaryGradient(): LinearGradient {
    return {
      angle: 90,
      colors: [[AuthTheme.ACCENT, 0.0], [AuthTheme.GRADIENT_END, 1.0]],
    } as LinearGradient;
  }

  /**
   * Backend error code -> user-facing string. Unknown codes fall back to the
   * server's own message, then to a generic failure; NOTHING here echoes a
   * credential.
   */
  static errorText(error: SsoAuthError): ResourceStr {
    switch (error.code) {
      case 'invalid_credentials':
        return $r('app.string.native_login_error_invalid_credentials');
      case 'rate_limited':
        return $r('app.string.native_login_error_rate_limited');
      case 'network':
        return $r('app.string.native_login_error_network');
      default:
        return error.message.length > 0
          ? error.message
          : $r('app.string.native_login_error_generic');
    }
  }

  /** 12–50 characters, one uppercase, one digit, one special character. */
  static passwordRules(): PasswordRule[] {
    return [
      new PasswordRule(
        $r('app.string.register_rule_length'),
        (value: string): boolean => value.length >= 12 && value.length <= 50
      ),
      new PasswordRule(
        $r('app.string.register_rule_uppercase'),
        (value: string): boolean => /[A-Z]/.test(value)
      ),
      new PasswordRule(
        $r('app.string.register_rule_digit'),
        (value: string): boolean => /[0-9]/.test(value)
      ),
      new PasswordRule(
        $r('app.string.register_rule_special'),
        (value: string): boolean => /[^A-Za-z0-9\s]/.test(value)
      ),
    ];
  }
}
