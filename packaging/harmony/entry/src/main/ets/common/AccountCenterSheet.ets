import { common } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { AccountSnapshot } from './AccountSnapshot';
import { AuthTheme } from './AuthTheme';
import { LucideGlyph, LucideIcon } from './LucideIcon';
import { SsoRegion, SsoRegionStore } from './SsoRegion';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/**
 * Platform-native account center. Profile data comes from the engine snapshot
 * (`editorAccountSnapshot` — the private auth runtime never exposes its device
 * token to the shell), region selection is a next-launch preference, and
 * deeper management opens the regional web account page in the system browser.
 * Ported from `AccountCenterOverlay.kt`.
 *
 * `compact` picks the presentation: desktop-class (2in1) shells get a small
 * centered card, phones and tablets the full-window sheet Android uses.
 */
@Component
export struct AccountCenterSheet {
  @Link visible: boolean;
  @Prop night: boolean = false;
  @Prop compact: boolean = false;
  /** Decoded engine snapshot; null renders the signed-out label. */
  snapshot: AccountSnapshot | null = null;
  regions: SsoRegionStore | null = null;
  onManageInBrowser: (url: string) => void = () => {};
  onRequestSignOut: () => void = () => {};
  onRegionSwitched: (region: SsoRegion) => void = () => {};

  @State private regionName: string = '';

  aboutToAppear(): void {
    this.refreshRegionName();
  }

  private refreshRegionName(): void {
    const store: SsoRegionStore | null = this.regions;
    const region: SsoRegion = store === null ? SsoRegion.GLOBAL : store.resolved();
    this.regionName = SsoRegionStore.displayName(
      getContext(this) as common.UIAbilityContext,
      region
    );
  }

  build() {
    if (this.compact) {
      Column() {
        Column() {
          this.body()
        }
        .width(340)
        .backgroundColor(AuthTheme.background(this.night))
        .borderRadius(16)
        .borderWidth(1)
        .borderColor(AuthTheme.border(this.night))
        .clip(true)
      }
      .width('100%')
      .height('100%')
      .backgroundColor('#66000000')
      .justifyContent(FlexAlign.Center)
      .alignItems(HorizontalAlign.Center)
      .onClick(() => {
        this.visible = false;
      })
    } else {
      Column() {
        this.body()
      }
      .width('100%')
      .height('100%')
      .backgroundColor(AuthTheme.background(this.night))
    }
  }

  @Builder
  private body() {
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
        .onClick(() => {
          this.visible = false;
        })
      }
      .width('100%')
      .justifyContent(FlexAlign.Start)

      Text(this.displayLabel())
        .fontSize(24)
        .fontWeight(FontWeight.Bold)
        .fontColor(AuthTheme.text(this.night))
        .margin({ top: 12, bottom: 2 })

      if (this.detailLabel().length > 0) {
        Text(this.detailLabel())
          .fontSize(14)
          .fontColor(AuthTheme.secondary(this.night))
          .margin({ bottom: 18 })
      }

      this.row($r('app.string.account_center_region_value', this.regionName),
        AuthTheme.text(this.night), (): void => this.toggleRegion())
      this.row($r('app.string.account_center_manage'),
        AuthTheme.text(this.night), (): void => this.openAccountPage())
      this.row($r('app.string.account_center_sign_out'),
        AuthTheme.DANGER, (): void => this.onRequestSignOut())
    }
    .width('100%')
    .padding({ left: 24, right: 24, top: 12, bottom: 20 })
    .alignItems(HorizontalAlign.Center)
  }

  @Builder
  private row(label: ResourceStr, color: string, action: () => void) {
    Text(label)
      .fontSize(16)
      .fontColor(color)
      .width('100%')
      .textAlign(TextAlign.Start)
      .padding({ top: 14, bottom: 14 })
      .onClick(action)
  }

  private displayLabel(): ResourceStr {
    const snapshot: AccountSnapshot | null = this.snapshot;
    const label: string | null = snapshot === null ? null : snapshot.label();
    return label ?? $r('app.string.account_center_signed_out');
  }

  private detailLabel(): string {
    const snapshot: AccountSnapshot | null = this.snapshot;
    return snapshot === null ? '' : snapshot.detail();
  }

  /** Region-scoped account page; the origin comes from the region store. */
  private openAccountPage(): void {
    const store: SsoRegionStore | null = this.regions;
    if (store === null) {
      return;
    }
    this.onManageInBrowser(`${store.resolved().origin}/account`);
  }

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
}
