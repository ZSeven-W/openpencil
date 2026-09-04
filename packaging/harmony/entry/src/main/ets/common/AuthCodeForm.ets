import { AuthField } from './AuthField';
import { AuthTheme, PasswordRule } from './AuthTheme';
import { LucideGlyph, LucideIcon } from './LucideIcon';
import { SsoAuthClient, SsoAuthError } from './SsoAuthClient';

/** Which email-code flow this form runs. */
export enum AuthFormMode {
  REGISTER = 0,
  RESET = 1,
}

const COUNTDOWN_SECONDS = 60;

/**
 * Shared form for the email-code flows (registration and password recovery):
 * email + verification code with a send-code countdown + password with a live
 * rules checklist mirroring the backend policy (12–50 chars, uppercase, digit,
 * special) + confirmation. On success the form signs in with the new
 * credentials and approves the login page's device pairing, so the whole flow
 * stays native. Ported from `AuthCodeFormOverlay.kt`.
 */
@Component
export struct AuthCodeForm {
  @Link visible: boolean;
  @Prop night: boolean = false;
  @Prop mode: number = AuthFormMode.REGISTER;
  /** Owned by the login page: the client bound to the pairing origin. */
  client: SsoAuthClient | null = null;
  /** Approves the login page's pairing once this form signed in. */
  onApprovePairing: () => Promise<SsoAuthError | null> = async () => null;
  onDismiss: () => void = () => {};

  @State private email: string = '';
  @State private code: string = '';
  @State private password: string = '';
  @State private confirm: string = '';
  @State private errorText: ResourceStr = '';
  @State private statusText: ResourceStr = '';
  @State private busy: boolean = false;
  @State private countdown: number = 0;
  private readonly rules: PasswordRule[] = AuthTheme.passwordRules();
  private countdownTimer: number = -1;

  aboutToDisappear(): void {
    this.stopCountdown();
  }

  private isRegister(): boolean {
    return this.mode === AuthFormMode.REGISTER;
  }

  private dismiss(): void {
    this.stopCountdown();
    this.visible = false;
    this.onDismiss();
  }

  build() {
    Column() {
      Scroll() {
        Column() {
          Row() {
            Button({ type: ButtonType.Normal }) {
              LucideGlyph({
                paths: LucideIcon.ARROW_LEFT,
                color: AuthTheme.secondary(this.night),
                glyphSize: 22,
              })
            }
            .backgroundColor(Color.Transparent)
            .padding(6)
            .onClick(() => this.dismiss())
          }
          .width('100%')
          .justifyContent(FlexAlign.Start)

          Image($r('app.media.zseven_logo'))
            .height(72)
            .objectFit(ImageFit.Contain)

          Text(this.isRegister()
            ? $r('app.string.register_title')
            : $r('app.string.reset_title'))
            .fontSize(26)
            .fontWeight(FontWeight.Bold)
            .fontColor(AuthTheme.text(this.night))
            .margin({ top: 4 })

          Text(this.isRegister()
            ? $r('app.string.register_subtitle')
            : $r('app.string.reset_subtitle'))
            .fontSize(15)
            .fontColor(AuthTheme.secondary(this.night))
            .margin({ top: 2, bottom: 10 })

          AuthField({
            label: $r('app.string.native_login_email_label'),
            placeholder: $r('app.string.native_login_email_placeholder'),
            icon: LucideIcon.MAIL,
            night: this.night,
            active: !this.busy,
            text: $email,
          })

          AuthField({
            label: $r('app.string.register_code_label'),
            placeholder: $r('app.string.register_code_placeholder'),
            icon: LucideIcon.SHIELD,
            night: this.night,
            numeric: true,
            active: !this.busy,
            text: $code,
            trailing: (): void => this.sendCodeLink(),
          })

          AuthField({
            label: $r('app.string.native_login_password_label'),
            placeholder: $r('app.string.native_login_password_placeholder'),
            icon: LucideIcon.LOCK,
            night: this.night,
            password: true,
            active: !this.busy,
            text: $password,
          })

          this.rulesChecklist()

          AuthField({
            label: $r('app.string.register_confirm_label'),
            placeholder: $r('app.string.register_confirm_placeholder'),
            icon: LucideIcon.LOCK,
            night: this.night,
            password: true,
            active: !this.busy,
            text: $confirm,
          })

          if (this.errorText !== '') {
            Text(this.errorText)
              .fontSize(13)
              .fontColor(AuthTheme.DANGER)
              .margin({ top: 8 })
          }

          Button(this.isRegister()
            ? $r('app.string.register_submit')
            : $r('app.string.reset_submit'))
            .type(ButtonType.Normal)
            .width('100%')
            .height(52)
            .borderRadius(12)
            .fontSize(17)
            .fontWeight(FontWeight.Bold)
            .fontColor(Color.White)
            .enabled(!this.busy)
            .backgroundColor(Color.Transparent)
            .linearGradient(AuthTheme.primaryGradient())
            .margin({ top: 12 })
            .onClick(() => this.submit())

          if (this.busy) {
            LoadingProgress().width(28).height(28).margin({ top: 8 })
          }

          if (this.statusText !== '') {
            Text(this.statusText)
              .fontSize(13)
              .fontColor(AuthTheme.secondary(this.night))
              .margin({ top: 4 })
          }
        }
        .width('100%')
        .padding({ left: 28, right: 28, top: 8, bottom: 24 })
        .alignItems(HorizontalAlign.Center)
      }
      .width('100%')
      .layoutWeight(1)
      .align(Alignment.Top)
    }
    .width('100%')
    .height('100%')
    .backgroundColor(AuthTheme.background(this.night))
  }

  @Builder
  private sendCodeLink() {
    Button({ type: ButtonType.Normal }) {
      if (this.countdown > 0) {
        Text(`${this.countdown}s`).fontSize(14).fontColor(AuthTheme.secondary(this.night))
      } else {
        Text($r('app.string.register_send_code')).fontSize(14).fontColor(AuthTheme.ACCENT)
      }
    }
    .backgroundColor(Color.Transparent)
    .padding(8)
    .enabled(this.countdown === 0 && !this.busy)
    .onClick(() => this.sendCode())
  }

  /** Live checklist mirroring the backend's `validNewPassword`. */
  @Builder
  private rulesChecklist() {
    Column() {
      ForEach(this.rules, (rule: PasswordRule) => {
        Row() {
          Text(rule.test(this.password) ? '● ' : '○ ')
            .fontSize(12)
            .fontColor(rule.test(this.password)
              ? AuthTheme.SATISFIED
              : AuthTheme.secondary(this.night))
          Text(rule.label)
            .fontSize(12)
            .fontColor(rule.test(this.password)
              ? AuthTheme.SATISFIED
              : AuthTheme.secondary(this.night))
        }
        .width('100%')
        .justifyContent(FlexAlign.Start)
      }, (rule: PasswordRule, index: number) => `${index}`)
    }
    .width('100%')
    .margin({ top: 6 })
  }

  // ---- Networking --------------------------------------------------------

  private async sendCode(): Promise<void> {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.countdown > 0) {
      return;
    }
    const email: string = this.email.trim();
    if (email.length === 0) {
      this.showError($r('app.string.register_error_missing_email'));
      return;
    }
    this.errorText = '';
    const purpose: string = this.isRegister() ? 'register' : 'password_reset';
    const error: SsoAuthError | null = await client.sendEmailCode(email, purpose);
    if (!this.visible) {
      return;
    }
    if (error !== null) {
      this.showError(AuthTheme.errorText(error));
      return;
    }
    this.statusText = $r('app.string.register_code_sent');
    this.startCountdown();
  }

  private startCountdown(): void {
    this.stopCountdown();
    this.countdown = COUNTDOWN_SECONDS;
    this.countdownTimer = setInterval(() => {
      this.countdown -= 1;
      if (this.countdown <= 0) {
        this.stopCountdown();
      }
    }, 1000);
  }

  private stopCountdown(): void {
    if (this.countdownTimer !== -1) {
      clearInterval(this.countdownTimer);
      this.countdownTimer = -1;
    }
    this.countdown = 0;
  }

  private async submit(): Promise<void> {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.busy) {
      return;
    }
    const email: string = this.email.trim();
    const code: string = this.code.trim();
    if (email.length === 0 || code.length === 0 || this.password.length === 0) {
      this.showError($r('app.string.register_error_missing_fields'));
      return;
    }
    if (this.password !== this.confirm) {
      this.showError($r('app.string.register_error_mismatch'));
      return;
    }
    this.setBusy(true);
    const created: SsoAuthError | null = this.isRegister()
      ? await client.register(email, code, this.password)
      : await client.resetPassword(email, code, this.password);
    if (!this.visible) {
      return;
    }
    if (created !== null) {
      this.setBusy(false);
      this.showError(AuthTheme.errorText(created));
      return;
    }
    // Account ready: sign in and approve the pairing owned by the login page
    // underneath. Rust observes the approval and dismisses the whole stack.
    this.statusText = $r('app.string.native_login_completing');
    const signedIn: SsoAuthError | null = await client.passwordLogin(email, this.password);
    if (!this.visible) {
      return;
    }
    if (signedIn !== null) {
      this.setBusy(false);
      this.showError(AuthTheme.errorText(signedIn));
      return;
    }
    const approved: SsoAuthError | null = await this.onApprovePairing();
    if (!this.visible) {
      return;
    }
    if (approved !== null) {
      this.setBusy(false);
      this.showError(AuthTheme.errorText(approved));
    }
  }

  private setBusy(busy: boolean): void {
    this.busy = busy;
    if (busy) {
      this.errorText = '';
    }
  }

  private showError(text: ResourceStr): void {
    this.errorText = text;
    this.statusText = '';
  }
}
