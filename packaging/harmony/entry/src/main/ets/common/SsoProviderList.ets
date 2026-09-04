/**
 * One third-party sign-in method the regional SSO deployment advertises
 * (`GET /api/v1/auth/providers?channel=web_mobile`). Each deployment lists
 * only its own enabled providers, so the native login screen shows WeChat /
 * Alipay / Douyin on the mainland site and Apple / GitHub / Google on the
 * global site without any hardcoded region table. Ported from
 * `SsoProviderList.kt`.
 */
export class SsoProviderEntry {
  readonly id: string;
  readonly displayName: string;

  constructor(id: string, displayName: string) {
    this.id = id;
    this.displayName = displayName;
  }
}

/** Untyped row shape of the providers payload. */
interface ProviderRow {
  id?: string;
  display_name?: string;
}

interface ProviderPayload {
  providers?: ProviderRow[];
}

/** Brand marks drawn in the theme's text colour rather than their own. */
const MONOCHROME_PROVIDERS: string[] = ['apple', 'github', 'douyin'];

export class SsoProviderList {
  /**
   * Decodes the providers response, dropping malformed rows. Returns an empty
   * list for structurally invalid payloads so the login screen falls back to
   * its generic browser button.
   */
  static parse(json: string): SsoProviderEntry[] {
    const entries: SsoProviderEntry[] = [];
    let payload: ProviderPayload;
    try {
      payload = JSON.parse(json) as ProviderPayload;
    } catch (_error) {
      return entries;
    }
    const rows: ProviderRow[] | undefined = payload.providers;
    if (rows === undefined || !Array.isArray(rows)) {
      return entries;
    }
    for (const row of rows) {
      if (row === null || typeof row !== 'object') {
        continue;
      }
      const id: string = typeof row.id === 'string' ? row.id : '';
      const name: string = typeof row.display_name === 'string' ? row.display_name : '';
      if (id.length === 0 || name.length === 0 || name.length > 64) {
        continue;
      }
      entries.push(new SsoProviderEntry(id, name));
    }
    return entries;
  }

  static isMonochrome(id: string): boolean {
    return MONOCHROME_PROVIDERS.includes(id);
  }

  /**
   * Brand icon for a provider id. The set mirrors
   * `packaging/android/.../res/drawable-nodpi/provider_*.png`; an unknown id
   * renders as a generic card rather than breaking the row.
   */
  static icon(id: string): Resource | null {
    switch (id) {
      case 'wechat':
        return $r('app.media.provider_wechat');
      case 'alipay':
        return $r('app.media.provider_alipay');
      case 'douyin':
        return $r('app.media.provider_douyin');
      case 'apple':
        return $r('app.media.provider_apple');
      case 'github':
        return $r('app.media.provider_github');
      case 'google':
        return $r('app.media.provider_google');
      default:
        return null;
    }
  }
}
