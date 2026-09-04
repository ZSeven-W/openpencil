import { http } from '@kit.NetworkKit';
import { i18n } from '@kit.LocalizationKit';
import { SsoProviderEntry, SsoProviderList } from './SsoProviderList';

const REQUEST_TIMEOUT_MS = 15000;
const PROVIDERS_TIMEOUT_MS = 10000;
const MAX_ERROR_BODY_CHARS = 16 * 1024;

/**
 * Typed failure from the SSO JSON API (or the transport underneath it): a
 * backend error code (`invalid_credentials`, …) or a transport sentinel
 * (`network`, `protocol`). Ported from `SsoAuthClient.kt`.
 */
export class SsoAuthError {
  readonly code: string;
  readonly message: string;

  constructor(code: string, message: string) {
    this.code = code;
    this.message = message;
  }
}

interface ErrorBody {
  code?: string;
  message?: string;
}

/** One POST's outcome: the success payload, or the typed failure. */
class PostBodyResult {
  readonly payload: string;
  readonly error: SsoAuthError | null;

  constructor(payload: string, error: SsoAuthError | null) {
    this.payload = payload;
    this.error = error;
  }
}

interface ErrorPayload {
  error?: ErrorBody;
}

/**
 * ArkTS forbids untyped object literals, so every request body is a declared
 * shape. The field names are the SSO JSON API's own, verbatim.
 */
interface PasswordLoginBody {
  email: string;
  password: string;
}

interface NativeLoginStartBody {
}

interface NativeLoginStartPayload {
  state?: string;
}

interface NativeLoginBody {
  state: string;
  code: string;
}

/**
 * Result of `nativeLoginStart`: the single-use state on success, or the
 * failure. Exactly one of the two fields is set.
 */
export class NativeLoginStart {
  readonly state: string;
  readonly error: SsoAuthError | null;

  constructor(state: string, error: SsoAuthError | null) {
    this.state = state;
    this.error = error;
  }
}

interface EmailCodeBody {
  email: string;
  purpose: string;
  locale: string;
}

interface RegisterBody {
  email: string;
  verification_code: string;
  password: string;
}

interface PasswordResetBody {
  email: string;
  verification_code: string;
  new_password: string;
}

interface ApprovePairingBody {
  pairing_id: string;
}

/**
 * Minimal JSON client for the native login flow against ONE regional SSO
 * origin — always the origin the engine put in its verification URL, never a
 * constant in this file.
 *
 * COOKIES ARE MEMORY-ONLY: the short-lived web session obtained for the
 * device approval lives in this instance's `Map` and dies with it, so it is
 * never written to disk, never shared with another origin, and never survives
 * the flow. The durable credential remains the Rust runtime's device token,
 * which never leaves the engine.
 */
export class SsoAuthClient {
  private readonly origin: string;
  /** In-memory cookie jar: name -> value. Deliberately not persisted. */
  private readonly cookies: Map<string, string> = new Map<string, string>();

  constructor(origin: string) {
    this.origin = origin;
  }

  /** `POST /api/v1/auth/password-login`; stores the session cookie. */
  async passwordLogin(email: string, password: string): Promise<SsoAuthError | null> {
    const body: PasswordLoginBody = { email: email, password: password };
    return this.post('/api/v1/auth/password-login', body);
  }

  /**
   * `POST /api/v1/auth/providers/<id>/native-login-start` — issues the
   * single-use state for a native authorization-code sign-in (Douyin /
   * Alipay) and drops the binder cookie into this client's jar; the same
   * client instance must complete the login so the cookie travels back.
   */
  async nativeLoginStart(providerId: string): Promise<NativeLoginStart> {
    const body: NativeLoginStartBody = {};
    const result = await this.postForBody(
      `/api/v1/auth/providers/${providerId}/native-login-start`,
      body
    );
    if (result.error !== null) {
      return new NativeLoginStart('', result.error);
    }
    let state = '';
    try {
      const payload: NativeLoginStartPayload = JSON.parse(result.payload) as NativeLoginStartPayload;
      state = payload.state ?? '';
    } catch (_error) {
      state = '';
    }
    if (state.length === 0) {
      return new NativeLoginStart('', new SsoAuthError('protocol', ''));
    }
    return new NativeLoginStart(state, null);
  }

  /**
   * `POST /api/v1/auth/providers/<id>/native-login` with the
   * authorization-code shape — redeems a native SDK auth code (Douyin /
   * Alipay) against the state issued by `nativeLoginStart`; the binder
   * cookie in this client's jar proves both calls came from one attempt.
   */
  async nativeLogin(providerId: string, state: string, code: string): Promise<SsoAuthError | null> {
    const body: NativeLoginBody = { state: state, code: code };
    return this.post(`/api/v1/auth/providers/${providerId}/native-login`, body);
  }

  /**
   * `POST /api/v1/auth/email-codes` — sends a verification code for
   * registration (`purpose: "register"`) or password recovery
   * (`purpose: "password_reset"`), localized to the device language.
   */
  async sendEmailCode(email: string, purpose: string): Promise<SsoAuthError | null> {
    const body: EmailCodeBody = {
      email: email,
      purpose: purpose,
      locale: SsoAuthClient.codeLocale(),
    };
    return this.post('/api/v1/auth/email-codes', body);
  }

  /** `POST /api/v1/auth/register` — creates the account. */
  async register(
    email: string,
    code: string,
    password: string
  ): Promise<SsoAuthError | null> {
    const body: RegisterBody = {
      email: email,
      verification_code: code,
      password: password,
    };
    return this.post('/api/v1/auth/register', body);
  }

  /** `POST /api/v1/auth/password-reset` — sets a new password. */
  async resetPassword(
    email: string,
    code: string,
    newPassword: string
  ): Promise<SsoAuthError | null> {
    const body: PasswordResetBody = {
      email: email,
      verification_code: code,
      new_password: newPassword,
    };
    return this.post('/api/v1/auth/password-reset', body);
  }

  /**
   * `POST /api/v1/device/login/approve` with the logged-in session cookie —
   * approves the pairing the engine's device flow is polling.
   */
  async approvePairing(pairingId: string): Promise<SsoAuthError | null> {
    const body: ApprovePairingBody = { pairing_id: pairingId };
    return this.post('/api/v1/device/login/approve', body);
  }

  /**
   * `GET /api/v1/auth/providers?channel=web_mobile` — the third-party sign-in
   * methods this regional deployment advertises. Failures resolve to an empty
   * list so the login screen keeps its generic browser button.
   */
  async fetchProviders(): Promise<SsoProviderEntry[]> {
    const request: http.HttpRequest = http.createHttp();
    try {
      const response: http.HttpResponse = await request.request(
        `${this.origin}/api/v1/auth/providers?channel=web_mobile`,
        {
          method: http.RequestMethod.GET,
          expectDataType: http.HttpDataType.STRING,
          usingCache: false,
          connectTimeout: PROVIDERS_TIMEOUT_MS,
          readTimeout: PROVIDERS_TIMEOUT_MS,
        }
      );
      if (response.responseCode < 200 || response.responseCode > 299) {
        return [];
      }
      return SsoProviderList.parse(SsoAuthClient.bodyText(response.result));
    } catch (_error) {
      return [];
    } finally {
      request.destroy();
    }
  }

  /** The regional web page that finishes a third-party sign-in for `pairingId`. */
  providerStartUrl(providerId: string, pairingId: string): string {
    const provider: string = encodeURIComponent(providerId);
    const pairing: string = encodeURIComponent(pairingId);
    return `${this.origin}/api/v1/auth/providers/${provider}/start` +
      `?channel=web_mobile&device_pairing=${pairing}`;
  }

  // ---- Transport ---------------------------------------------------------

  private async post(path: string, body: object): Promise<SsoAuthError | null> {
    return (await this.postForBody(path, body)).error;
  }

  /** Runs one POST; on success delivers the response body, else the error. */
  private async postForBody(path: string, body: object): Promise<PostBodyResult> {
    const request: http.HttpRequest = http.createHttp();
    try {
      const headers: Record<string, string> = { 'Content-Type': 'application/json' };
      const jar: string = this.cookieHeader();
      if (jar.length > 0) {
        headers['Cookie'] = jar;
      }
      const response: http.HttpResponse = await request.request(`${this.origin}${path}`, {
        method: http.RequestMethod.POST,
        extraData: JSON.stringify(body),
        expectDataType: http.HttpDataType.STRING,
        usingCache: false,
        header: headers,
        connectTimeout: REQUEST_TIMEOUT_MS,
        readTimeout: REQUEST_TIMEOUT_MS,
      });
      this.storeCookies(response.cookies);
      const status: number = response.responseCode;
      if (status >= 200 && status <= 299) {
        return new PostBodyResult(SsoAuthClient.bodyText(response.result), null);
      }
      return new PostBodyResult(
        '',
        SsoAuthClient.decodeError(SsoAuthClient.bodyText(response.result), status)
      );
    } catch (error) {
      // The transport never surfaces the request body, so nothing sensitive
      // can leak through this sentinel.
      return new PostBodyResult('', new SsoAuthError('network', SsoAuthClient.errorMessage(error)));
    } finally {
      request.destroy();
    }
  }

  private cookieHeader(): string {
    const parts: string[] = [];
    this.cookies.forEach((value: string, name: string) => {
      parts.push(`${name}=${value}`);
    });
    return parts.join('; ');
  }

  /** Keeps only the `name=value` pair of every `Set-Cookie`, in memory. */
  private storeCookies(raw: string): void {
    if (raw === undefined || raw === null || raw.length === 0) {
      return;
    }
    for (const line of raw.split('\n')) {
      const cookie: string = line.split(';')[0].trim();
      const separator: number = cookie.indexOf('=');
      if (separator <= 0) {
        continue;
      }
      const name: string = cookie.substring(0, separator).trim();
      const value: string = cookie.substring(separator + 1).trim();
      if (name.length === 0) {
        continue;
      }
      this.cookies.set(name, value);
    }
  }

  private static bodyText(result: string | Object | ArrayBuffer): string {
    if (typeof result === 'string') {
      return result.substring(0, MAX_ERROR_BODY_CHARS);
    }
    try {
      return (JSON.stringify(result) ?? '').substring(0, MAX_ERROR_BODY_CHARS);
    } catch (_error) {
      return '';
    }
  }

  private static decodeError(payload: string, status: number): SsoAuthError {
    if (payload.length > 0) {
      try {
        const parsed: ErrorPayload = JSON.parse(payload) as ErrorPayload;
        const error: ErrorBody | undefined = parsed.error;
        if (error !== undefined && error !== null) {
          const code: string = typeof error.code === 'string' ? error.code : '';
          if (code.length > 0) {
            return new SsoAuthError(
              code,
              typeof error.message === 'string' ? error.message : ''
            );
          }
        }
      } catch (_error) {
        // Fall through to the status-based sentinel below.
      }
    }
    return status === 429 ? new SsoAuthError('rate_limited', '') : new SsoAuthError('protocol', '');
  }

  private static errorMessage(error: Object): string {
    const message: string | undefined = (error as Error)?.message;
    return typeof message === 'string' ? message : '';
  }

  private static codeLocale(): string {
    try {
      return i18n.System.getSystemLanguage().toLowerCase().startsWith('zh') ? 'zh-CN' : 'en';
    } catch (_error) {
      return 'en';
    }
  }
}
