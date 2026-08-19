import { preferences } from '@kit.ArkData';
import { common } from '@kit.AbilityKit';
import { http } from '@kit.NetworkKit';
import { i18n } from '@kit.LocalizationKit';
import { hilog } from '@kit.PerformanceAnalysisKit';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';

const PREFS = 'sso-region';
const OVERRIDE_KEY = 'region-override';
const DETECTED_KEY = 'region-detected';

/**
 * THE ONLY place in this shell that may name an SSO / gateway origin. Every
 * other file receives an origin at runtime (from this store, or from the
 * engine's own verification URL); `Tests/HarmonyShellContractTests.rb` pins
 * that discipline, exactly as the Android shell does with `SsoRegion.kt`.
 */
const REGION_PROBE_URL = 'https://op.zseven.tech/';
const MAINLAND_REDIRECT_HOST = 'op.zseven.cn';
const PROBE_TIMEOUT_MS = 3000;

/**
 * Regional SSO deployment this install signs in against. Mirrors the engine's
 * `OpAuthRegion` codes (`OpAuthRegion_China = 0`, `OpAuthRegion_Global = 1`);
 * both origins are pinned first-party hosts. Ported from `SsoRegion.kt`.
 */
export class SsoRegion {
  readonly key: string;
  readonly origin: string;
  readonly authRegionCode: number;

  private constructor(key: string, origin: string, authRegionCode: number) {
    this.key = key;
    this.origin = origin;
    this.authRegionCode = authRegionCode;
  }

  static readonly CHINA: SsoRegion = new SsoRegion('CHINA', 'https://sso.zseven.cn', 0);
  static readonly GLOBAL: SsoRegion = new SsoRegion('GLOBAL', 'https://sso.zseven.tech', 1);

  /** Same order as the Kotlin enum, so the picker rows match Android. */
  static readonly all: SsoRegion[] = [SsoRegion.CHINA, SsoRegion.GLOBAL];

  static fromKey(key: string | null): SsoRegion | null {
    if (key === null) {
      return null;
    }
    for (const region of SsoRegion.all) {
      if (region.key === key) {
        return region;
      }
    }
    return null;
  }
}

/**
 * Region resolution: a user override always wins, then the last IP-informed
 * detection, then a device-locale default. The auth runtime initializes once
 * per process and locks its origin, so a change (override or fresh detection)
 * applies on the NEXT launch — callers surface that explicitly instead of
 * pretending otherwise.
 *
 * Preferences on OHOS are async, so `load()` warms an in-memory mirror once
 * and `resolved()` stays synchronous for the UI, matching `SsoRegionStore.kt`.
 */
export class SsoRegionStore {
  /**
   * Localized display name as a PLAIN string. `$r()` format arguments accept
   * only strings and numbers, so a nested Resource would render as garbage in
   * the "Region: %s · Switch" rows.
   */
  static displayName(context: common.Context, region: SsoRegion): string {
    const key: Resource = region === SsoRegion.CHINA
      ? $r('app.string.sso_region_china')
      : $r('app.string.sso_region_global');
    try {
      return context.resourceManager.getStringSync(key.id);
    } catch (_error) {
      return region.key;
    }
  }

  private readonly context: common.Context;
  private store: preferences.Preferences | null = null;
  private overrideKey: string | null = null;
  private detectedKey: string | null = null;
  private probeInFlight: boolean = false;

  constructor(context: common.Context) {
    this.context = context;
  }

  /** Warms the in-memory mirror; safe to call more than once. */
  async load(): Promise<void> {
    if (this.store !== null) {
      return;
    }
    try {
      const store: preferences.Preferences = await preferences.getPreferences(this.context, PREFS);
      this.store = store;
      this.overrideKey = await SsoRegionStore.readKey(store, OVERRIDE_KEY);
      this.detectedKey = await SsoRegionStore.readKey(store, DETECTED_KEY);
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'region preferences unavailable: %{public}s', JSON.stringify(error));
    }
  }

  private static async readKey(
    store: preferences.Preferences,
    key: string
  ): Promise<string | null> {
    const value: preferences.ValueType = await store.get(key, '');
    const text: string = typeof value === 'string' ? value : '';
    return text.length > 0 ? text : null;
  }

  resolved(): SsoRegion {
    const override: SsoRegion | null = SsoRegion.fromKey(this.overrideKey);
    if (override !== null) {
      return override;
    }
    const detected: SsoRegion | null = SsoRegion.fromKey(this.detectedKey);
    if (detected !== null) {
      return detected;
    }
    return SsoRegionStore.localeDefault();
  }

  hasUserOverride(): boolean {
    return SsoRegion.fromKey(this.overrideKey) !== null;
  }

  async saveUserOverride(region: SsoRegion): Promise<void> {
    this.overrideKey = region.key;
    await this.put(OVERRIDE_KEY, region.key);
  }

  /**
   * Startup resolution for the auth runtime. With a stored answer (override or
   * an earlier detection) it completes immediately and refreshes the detection
   * in the background for the next launch. On a first launch it runs the IP
   * probe FIRST — bounded by the probe's own timeouts — and only falls back to
   * the device locale when the probe is inconclusive, so the region really
   * comes from the IP.
   */
  async resolveForStartup(): Promise<SsoRegion> {
    await this.load();
    if (this.overrideKey !== null || this.detectedKey !== null) {
      this.refreshDetectedRegionAsync();
      return this.resolved();
    }
    const detected: SsoRegion | null = await this.probeRegion();
    if (detected !== null) {
      this.detectedKey = detected.key;
      await this.put(DETECTED_KEY, detected.key);
      return detected;
    }
    return SsoRegionStore.localeDefault();
  }

  /**
   * One IP-informed detection per launch, skipped once the user chose a
   * region; the result applies on the next launch.
   */
  refreshDetectedRegionAsync(): void {
    if (this.hasUserOverride() || this.probeInFlight) {
      return;
    }
    this.probeInFlight = true;
    this.probeRegion()
      .then(async (detected: SsoRegion | null) => {
        this.probeInFlight = false;
        if (detected === null) {
          return;
        }
        this.detectedKey = detected.key;
        await this.put(DETECTED_KEY, detected.key);
      })
      .catch(() => {
        this.probeInFlight = false;
      });
  }

  private async put(key: string, value: string): Promise<void> {
    await this.load();
    const store: preferences.Preferences | null = this.store;
    if (store === null) {
      return;
    }
    try {
      await store.put(key, value);
      await store.flush();
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not persist the region: %{public}s', JSON.stringify(error));
    }
  }

  /**
   * The IP probe asks the ZSeven global gateway itself: nginx 302-redirects
   * cookie-less mainland-China browser requests on the global host to the
   * mainland host (APNIC-derived list, no third-party geolocation), so the
   * gateway's own IP verdict is readable from the redirect.
   *
   * OHOS's `@ohos.net.http` exposes no redirect switch on API 12
   * (`maxRedirects` is API 23), but its header map is accumulated across the
   * WHOLE redirect chain — a `Location` naming the mainland host is therefore
   * still visible after the redirect was followed. An unreachable global host
   * also reads as mainland (typical when only the domestic site is
   * reachable), while other outcomes are inconclusive.
   */
  private async probeRegion(): Promise<SsoRegion | null> {
    const request: http.HttpRequest = http.createHttp();
    try {
      const response: http.HttpResponse = await request.request(REGION_PROBE_URL, {
        method: http.RequestMethod.GET,
        expectDataType: http.HttpDataType.STRING,
        usingCache: false,
        connectTimeout: PROBE_TIMEOUT_MS,
        readTimeout: PROBE_TIMEOUT_MS,
      });
      const status: number = response.responseCode;
      if (SsoRegionStore.redirectsToChina(response.header)) {
        return SsoRegion.CHINA;
      }
      if (status > 0 && status < 400) {
        return SsoRegion.GLOBAL;
      }
      return null;
    } catch (error) {
      hilog.info(
        DOMAIN,
        TAG,
        'global SSO host unreachable; detecting mainland region: %{public}s',
        JSON.stringify(error)
      );
      return SsoRegion.CHINA;
    } finally {
      request.destroy();
    }
  }

  /** True when any `Location` in the chain points at the mainland host. */
  private static redirectsToChina(header: Object): boolean {
    let raw: string = '';
    try {
      raw = JSON.stringify(header) ?? '';
    } catch (_error) {
      return false;
    }
    const lowered: string = raw.toLowerCase();
    const marker: number = lowered.indexOf('"location"');
    if (marker < 0) {
      return false;
    }
    // Only a Location value is trusted, never an arbitrary body echo.
    const tail: string = lowered.substring(marker);
    return tail.indexOf(`//${MAINLAND_REDIRECT_HOST}`) >= 0 ||
      tail.indexOf(`.${MAINLAND_REDIRECT_HOST}`) >= 0;
  }

  private static localeDefault(): SsoRegion {
    try {
      const region: string = i18n.System.getSystemRegion();
      return region.toUpperCase() === 'CN' ? SsoRegion.CHINA : SsoRegion.GLOBAL;
    } catch (_error) {
      return SsoRegion.GLOBAL;
    }
  }
}
