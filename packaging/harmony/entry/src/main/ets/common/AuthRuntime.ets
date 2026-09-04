import { bundleManager, common } from '@kit.AbilityKit';
import { deviceInfo } from '@kit.BasicServicesKit';
import { fileIo as fs } from '@kit.CoreFileKit';
import { hilog } from '@kit.PerformanceAnalysisKit';
import { EngineHost } from './EngineHost';
import { OpStatus } from './OpNative';
import { SsoRegion, SsoRegionStore } from './SsoRegion';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

/** `OpStatus::NotReady` — this build has no `op-auth` backend linked in. */
export const STATUS_NOT_READY = 10;

/**
 * Configures the optional native auth backend, mirroring
 * `AndroidAuthRuntime.kt`.
 *
 * The proprietary runtime initializes once per process and LOCKS its regional
 * origin, so a fresh install (no persisted credential) defers configuration
 * until the first sign-in — the login region then reflects the latest IP
 * verdict and any user switch made before that first tap. Returning users
 * configure at engine creation so their session restores immediately.
 *
 * The credential directory is app-private (`filesDir/auth`) and separate from
 * the engine's `filesDir/config` storage root. HarmonyOS backs an app up only
 * through a declared `BackupExtensionAbility`; this module declares none, so
 * nothing here is ever copied off the device.
 */
export class AuthRuntime {
  private readonly storageDir: string;
  private readonly deviceName: string;
  private readonly appVersion: string;
  private readonly regions: SsoRegionStore;
  private configured: boolean = false;

  constructor(context: common.Context) {
    this.storageDir = `${context.filesDir}/auth`;
    try {
      // `mkdirSync` throws when the directory already exists, which is the
      // normal case for every launch after the first.
      if (!fs.accessSync(this.storageDir)) {
        fs.mkdirSync(this.storageDir, true);
      }
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not create the private auth directory');
    }
    this.deviceName = AuthRuntime.resolveDeviceName();
    this.appVersion = AuthRuntime.resolveAppVersion();
    this.regions = new SsoRegionStore(context);
  }

  regionStore(): SsoRegionStore {
    return this.regions;
  }

  /** Engine-creation hook: restore returning users, defer fresh installs. */
  async configureAtStartup(host: EngineHost): Promise<void> {
    if (this.configured || !host.isEditorMode() || host.engineHandle() === 0) {
      return;
    }
    // DESKTOP-CLASS EXCEPTION to the lazy rule. Android is touch-only, so its
    // sign-in ALWAYS arrives as `RequestLogin` and the runtime can be
    // configured on that tap. The engine's DESKTOP chrome instead starts the
    // browser pairing itself and only reports the finished verification URL
    // (action 2), so there is no shell-visible moment left to configure in —
    // a 2in1 therefore configures at startup. Nothing is lost: the PC login
    // page has no region row to change the answer before that first flow.
    if (host.isDesktopClass()) {
      this.configured = true;
      const region: SsoRegion = await this.regions.resolveForStartup();
      this.configureNow(host, region);
      return;
    }
    if (!this.hasPersistedCredential()) {
      // Fresh install: leave the region unlocked and only refresh the IP
      // verdict for the first sign-in tap.
      await this.regions.load();
      this.regions.refreshDetectedRegionAsync();
      return;
    }
    this.configured = true;
    const region: SsoRegion = await this.regions.resolveForStartup();
    this.configureNow(host, region);
  }

  /**
   * `RequestLogin` follow-up: configure for the resolved region if this
   * process has not yet, then start the device flow. Resolves `false` when
   * the build has no sign-in backend, so the caller can say so.
   */
  async startLogin(host: EngineHost): Promise<boolean> {
    if (host.engineHandle() === 0) {
      return true;
    }
    if (!this.configured) {
      this.configured = true;
      const region: SsoRegion = await this.regions.resolveForStartup();
      this.configureNow(host, region);
    }
    const status: number = host.beginLogin();
    if (status === STATUS_NOT_READY) {
      return false;
    }
    if (status !== OpStatus.OK && status !== OpStatus.CLOSING) {
      hilog.info(DOMAIN, TAG, 'begin login returned status=%{public}d', status);
    }
    return true;
  }

  private configureNow(host: EngineHost, region: SsoRegion): void {
    const status: number = host.configureAuth(
      this.storageDir,
      this.deviceName,
      this.appVersion,
      region.authRegionCode
    );
    // Region CODE only — never the origin, which stays inside `SsoRegion`.
    hilog.info(
      DOMAIN,
      TAG,
      'auth configured region=%{public}d status=%{public}d',
      region.authRegionCode,
      status
    );
    if (status !== OpStatus.OK && status !== OpStatus.CLOSING && status !== STATUS_NOT_READY) {
      hilog.info(DOMAIN, TAG, 'native auth configure returned status=%{public}d', status);
    }
  }

  /** A non-empty credential directory means this install signed in before. */
  private hasPersistedCredential(): boolean {
    try {
      return fs.listFileSync(this.storageDir).length > 0;
    } catch (_error) {
      return false;
    }
  }

  private static resolveDeviceName(): string {
    try {
      const marketName: string = deviceInfo.marketName;
      if (marketName !== undefined && marketName.trim().length > 0) {
        return marketName.trim();
      }
      const productModel: string = deviceInfo.productModel;
      if (productModel !== undefined && productModel.trim().length > 0) {
        return productModel.trim();
      }
    } catch (_error) {
      // Fall through to the platform name.
    }
    return 'HarmonyOS';
  }

  private static resolveAppVersion(): string {
    try {
      const info: bundleManager.BundleInfo = bundleManager.getBundleInfoForSelfSync(
        bundleManager.BundleFlag.GET_BUNDLE_INFO_DEFAULT
      );
      if (info.versionName !== undefined && info.versionName.length > 0) {
        return info.versionName;
      }
    } catch (_error) {
      // Fall through to the placeholder below.
    }
    return '0.0.0';
  }
}
