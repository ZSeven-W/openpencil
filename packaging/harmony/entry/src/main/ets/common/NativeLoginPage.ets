import { common } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { AlipayNativeSignIn } from './AlipayNativeSignIn';
import { AuthCodeForm, AuthFormMode } from './AuthCodeForm';
import { AuthField } from './AuthField';
import { AuthTheme } from './AuthTheme';
import { DouyinNativeSignIn } from './DouyinNativeSignIn';
import { LucideGlyph, LucideIcon } from './LucideIcon';
import { NativeSignInKind, NativeSignInOutcome } from './NativeSignInOutcome';
import { WechatNativeSignIn } from './WechatNativeSignIn';
import { SsoAuthClient, SsoAuthError } from './SsoAuthClient';
import { SsoProviderEntry, SsoProviderList } from './SsoProviderList';
import { SsoRegion, SsoRegionStore } from './SsoRegion';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * Full-window, platform-native sign-in for one engine device-login flow,
 * styled after the ZSeven web sign-in page (logo, 欢迎回来, labeled boxed
 * inputs, gradient primary button, provider icon cards). Ported from
 * `NativeLoginOverlay.kt`.
 *
 * PHONE / TABLET ONLY. Desktop-class (2in1) shells never build this page: they
 * hand the engine's verification URL to the system browser instead, because a
 * PC always has one and the browser already holds the user's web session.
 *
 * Email/password sign-in runs against the PAIRING ORIGIN's JSON API (the
 * origin comes from the engine's URL, never from a constant here) and approves
 * the pairing directly; registration and password recovery are native forms
 * stacked on this page. Third-party providers open the regional web page in
 * the system browser — the running pairing is approved there instead. Rust
 * stays authoritative: the parent dismisses this page on shell action 3, while
 * a user close cancels the engine flow exactly once.
 */
@Component
export struct NativeLoginPage {
  @Link visible: boolean;
  @Prop night: boolean = false;
  /** Parsed from the engine's verification URL by the page's owner. */
  @Prop origin: string = '';
  @Prop pairingId: string = '';
  @Prop verificationUrl: string = '';
  /** Shared with the account center; the page never constructs one. */
  regions: SsoRegionStore | null = null;
  onCancel: () => void = () => {};
  onOpenBrowser: (url: string) => void = () => {};
  /** The user switched region: the owner offers the restart. */
  onRegionSwitched: (region: SsoRegion) => void = () => {};

  @State private email: string = '';
  @State private password: string = '';
  @State private errorText: ResourceStr = '';
  @State private statusText: ResourceStr = '';
  @State private busy: boolean = false;
  @State private providers: SsoProviderEntry[] = [];
  @State private regionName: string = '';
  @State private showForm: boolean = false;
  @State private formMode: number = AuthFormMode.REGISTER;
  private client: SsoAuthClient | null = null;

  aboutToAppear(): void {
    this.client = new SsoAuthClient(this.origin);
    this.refreshRegionName();
    this.loadProviders();
  }

  private refreshRegionName(): void {
    const store: SsoRegionStore | null = this.regions;
    const region: SsoRegion = store === null ? SsoRegion.GLOBAL : store.resolved();
    this.regionName = SsoRegionStore.displayName(
      getContext(this) as common.UIAbilityContext,
      region
    );
  }

  private async loadProviders(): Promise<void> {
    const client: SsoAuthClient | null = this.client;
    if (client === null) {
      return;
    }
    const entries: SsoProviderEntry[] = await client.fetchProviders();
    if (this.visible) {
      this.providers = entries;
    }
  }

  build() {
    Stack() {
      Scroll() {
        Column() {
          Row() {
            Button({ type: ButtonType.Normal }) {
              LucideGlyph({
                paths: LucideIcon.X,
                color: AuthTheme.secondary(this.night),
                glyphSize: 20,
              })
            }
            .backgroundColor(Color.Transparent)
            .padding(6)
            .onClick(() => this.close())
          }
          .width('100%')
          .justifyContent(FlexAlign.Start)

          Image($r('app.media.zseven_logo'))
            .height(84)
            .objectFit(ImageFit.Contain)

          Text($r('app.string.native_login_welcome'))
            .fontSize(26)
            .fontWeight(FontWeight.Bold)
            .fontColor(AuthTheme.text(this.night))
            .margin({ top: 6 })

          Text($r('app.string.native_login_subtitle'))
            .fontSize(15)
            .fontColor(AuthTheme.secondary(this.night))
            .margin({ top: 2, bottom: 14 })

          AuthField({
            label: $r('app.string.native_login_email_label'),
            placeholder: $r('app.string.native_login_email_placeholder'),
            icon: LucideIcon.MAIL,
            night: this.night,
            active: !this.busy,
            text: $email,
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

          Row() {
            Text($r('app.string.native_login_forgot_password'))
              .fontSize(14)
              .fontColor(AuthTheme.ACCENT)
              .padding(8)
              .onClick(() => this.presentForm(AuthFormMode.RESET))
          }
          .width('100%')
          .justifyContent(FlexAlign.End)

          if (this.errorText !== '') {
            Text(this.errorText)
              .fontSize(13)
              .fontColor(AuthTheme.DANGER)
              .margin({ top: 6 })
          }

          Button($r('app.string.native_login_sign_in'))
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
            .margin({ top: 10 })
            .onClick(() => this.signIn())

          if (this.busy) {
            LoadingProgress().width(28).height(28).margin({ top: 6 })
          }

          if (this.statusText !== '') {
            Text(this.statusText)
              .fontSize(13)
              .fontColor(AuthTheme.secondary(this.night))
              .margin({ top: 4 })
          }

          if (this.providers.length > 0) {
            this.providerSection()
          }

          Row() {
            Text($r('app.string.native_login_no_account'))
              .fontSize(14)
              .fontColor(AuthTheme.secondary(this.night))
            Text($r('app.string.native_login_register_now'))
              .fontSize(14)
              .fontColor(AuthTheme.ACCENT)
              .padding(8)
              .onClick(() => this.presentForm(AuthFormMode.REGISTER))
          }
          .margin({ top: 18 })
          .justifyContent(FlexAlign.Center)

          Text($r('app.string.native_login_region', this.regionName))
            .fontSize(13)
            .fontColor(AuthTheme.secondary(this.night))
            .margin({ top: 12 })
            .onClick(() => this.toggleRegion())
        }
        .width('100%')
        .padding({ left: 28, right: 28, top: 8, bottom: 24 })
        .alignItems(HorizontalAlign.Center)
      }
      .width('100%')
      .height('100%')
      .align(Alignment.Top)

      if (this.showForm) {
        AuthCodeForm({
          visible: $showForm,
          night: this.night,
          mode: this.formMode,
          client: this.client,
          onApprovePairing: (): Promise<SsoAuthError | null> => this.approvePairing(),
          onDismiss: (): void => {},
        })
      }
    }
    .width('100%')
    .height('100%')
    .backgroundColor(AuthTheme.background(this.night))
  }

  /**
   * One brand-icon card per advertised provider. Every card opens the regional
   * OAuth start endpoint in the system browser carrying this pairing — the
   * approval must happen in that page's context, so provider cards are
   * region-accurate entry points, not separate OAuth launches.
   */
  @Builder
  private providerSection() {
    Row() {
      Divider().layoutWeight(1).color(AuthTheme.border(this.night))
      Text($r('app.string.native_login_continue_with'))
        .fontSize(13)
        .fontColor(AuthTheme.secondary(this.night))
        .padding({ left: 12, right: 12 })
      Divider().layoutWeight(1).color(AuthTheme.border(this.night))
    }
    .width('100%')
    .margin({ top: 18 })

    Row() {
      ForEach(this.providers, (provider: SsoProviderEntry, index: number) => {
        Column() {
          // Monochrome brand marks follow the theme text colour; colored
          // marks keep their brand colors as-is.
          if (SsoProviderList.isMonochrome(provider.id)) {
            Image(SsoProviderList.icon(provider.id))
              .width(30)
              .height(30)
              .objectFit(ImageFit.Contain)
              .fillColor(AuthTheme.text(this.night))
          } else if (SsoProviderList.icon(provider.id) !== null) {
            Image(SsoProviderList.icon(provider.id))
              .width(30)
              .height(30)
              .objectFit(ImageFit.Contain)
          } else {
            Text(provider.displayName.substring(0, 1))
              .fontSize(18)
              .fontColor(AuthTheme.text(this.night))
          }
        }
        .width(56)
        .height(56)
        .borderRadius(14)
        .borderWidth(1)
        .borderColor(AuthTheme.border(this.night))
        .backgroundColor(AuthTheme.cardFill(this.night))
        .justifyContent(FlexAlign.Center)
        .margin({ left: index > 0 ? 16 : 0 })
        .onClick(() => this.providerTapped(provider.id))
      }, (provider: SsoProviderEntry) => provider.id)
    }
    .margin({ top: 14 })
    .justifyContent(FlexAlign.Center)
  }

  // ---- Actions -----------------------------------------------------------

  /** User close: stack first, then the whole flow (cancelling it once). */
  private close(): void {
    if (this.showForm) {
      this.showForm = false;
      return;
    }
    this.visible = false;
    this.onCancel();
  }

  private presentForm(mode: number): void {
    this.formMode = mode;
    this.showForm = true;
  }

  private async approvePairing(): Promise<SsoAuthError | null> {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.pairingId.length === 0) {
      return new SsoAuthError('protocol', '');
    }
    return client.approvePairing(this.pairingId);
  }

  private async signIn(): Promise<void> {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.busy) {
      return;
    }
    const email: string = this.email.trim();
    if (email.length === 0 || this.password.length === 0) {
      this.showError($r('app.string.native_login_error_missing_fields'));
      return;
    }
    this.setBusy(true);
    const loginError: SsoAuthError | null = await client.passwordLogin(email, this.password);
    if (!this.visible) {
      return;
    }
    if (loginError !== null) {
      this.setBusy(false);
      this.showError(AuthTheme.errorText(loginError));
      return;
    }
    const approveError: SsoAuthError | null = await this.approvePairing();
    if (!this.visible) {
      return;
    }
    if (approveError !== null) {
      this.setBusy(false);
      this.showError(approveError.code === 'not_found'
        ? $r('app.string.native_login_error_pairing_expired')
        : AuthTheme.errorText(approveError));
      return;
    }
    // Rust's poll observes the approval, exchanges the pairing, and drives the
    // close action that dismisses this page.
    this.statusText = $r('app.string.native_login_completing');
  }

  /** Douyin and Alipay run their native SDK flows; the rest open the web page. */
  private providerTapped(providerId: string): void {
    if (providerId === 'douyin') {
      this.startNativeProviderSignIn(providerId, (state, done) => {
        DouyinNativeSignIn.start(getContext(this) as common.UIAbilityContext, state, done);
      });
      return;
    }
    if (providerId === 'alipay') {
      this.startNativeProviderSignIn(providerId, (state, done) => {
        AlipayNativeSignIn.start(getContext(this) as common.UIAbilityContext, state, done);
      });
      return;
    }
    if (providerId === 'wechat') {
      this.startNativeProviderSignIn(providerId, (state, done) => {
        WechatNativeSignIn.start(getContext(this) as common.UIAbilityContext, state, done);
      });
      return;
    }
    this.openProviderLogin(providerId);
  }

  /**
   * Obtains a single-use state (with its binder cookie) from the SSO, runs
   * one native-SDK authorization carrying that state, exchanges
   * `{state, code}` at the provider's native-login endpoint, and approves
   * the pairing. A user cancel just returns to this page; a failure surfaces
   * an inline error instead of bouncing to the browser flow.
   */
  private async startNativeProviderSignIn(
    providerId: string,
    run: (state: string, done: (outcome: NativeSignInOutcome) => void) => void
  ): Promise<void> {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.pairingId.length === 0 || this.busy) {
      return;
    }
    this.setBusy(true);
    const start = await client.nativeLoginStart(providerId);
    if (!this.visible) {
      return;
    }
    if (start.error !== null || start.state.length === 0) {
      this.setBusy(false);
      this.showError(start.error !== null
        ? AuthTheme.errorText(start.error)
        : $r('app.string.native_login_error_native_provider'));
      return;
    }
    run(start.state, (outcome: NativeSignInOutcome): void => {
      this.finishNativeProviderSignIn(client, providerId, start.state, outcome);
    });
  }

  private async finishNativeProviderSignIn(
    client: SsoAuthClient,
    providerId: string,
    state: string,
    outcome: NativeSignInOutcome
  ): Promise<void> {
    if (!this.visible) {
      return;
    }
    if (outcome.kind === NativeSignInKind.CANCELED) {
      this.setBusy(false);
      return;
    }
    if (outcome.kind === NativeSignInKind.FAILED) {
      this.setBusy(false);
      this.showError($r('app.string.native_login_error_native_provider'));
      return;
    }
    const loginError: SsoAuthError | null =
      await client.nativeLogin(providerId, state, outcome.authCode);
    if (!this.visible) {
      return;
    }
    if (loginError !== null) {
      this.setBusy(false);
      this.showError(AuthTheme.errorText(loginError));
      return;
    }
    const approveError: SsoAuthError | null = await this.approvePairing();
    if (!this.visible) {
      return;
    }
    if (approveError !== null) {
      this.setBusy(false);
      this.showError(approveError.code === 'not_found'
        ? $r('app.string.native_login_error_pairing_expired')
        : AuthTheme.errorText(approveError));
      return;
    }
    // Rust's poll observes the approval, exchanges the pairing, and drives the
    // close action that dismisses this page.
    this.statusText = $r('app.string.native_login_completing');
  }

  /**
   * Providers without a native SDK finish on their own OAuth page: the SSO
   * start endpoint 302s straight to the provider's authorize screen carrying
   * the pairing, and the callback lands on the dedicated approval page.
   */
  private openProviderLogin(providerId: string): void {
    const client: SsoAuthClient | null = this.client;
    if (client === null || this.pairingId.length === 0) {
      return;
    }
    this.statusText = $r('app.string.native_login_browser_hint');
    this.onOpenBrowser(client.providerStartUrl(providerId, this.pairingId));
  }

  /**
   * The region row toggles directly between the two deployments; the auth
   * runtime initializes once per process, so applying it means restarting the
   * app — the owner offers that.
   */
  private toggleRegion(): void {
    const store: SsoRegionStore | null = this.regions;
    if (store === null) {
      return;
    }
    const next: SsoRegion = store.resolved() === SsoRegion.CHINA
      ? SsoRegion.GLOBAL
      : SsoRegion.CHINA;
    store.saveUserOverride(next)
      .then(() => {
        this.refreshRegionName();
        this.onRegionSwitched(next);
      })
      .catch((error: Object) => {
        hilog.warn(DOMAIN, TAG, 'region override failed: %{public}s', JSON.stringify(error));
      });
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
