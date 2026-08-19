import { url } from '@kit.ArkTS';

/**
 * Parsed engine verification URL (`https://<origin>/login?device_pairing=<id>`).
 *
 * The native login screen signs in against `origin`'s JSON API and approves
 * `pairingId`; a URL that does not carry a pairing on an HTTPS origin is
 * rejected so the shell cancels the flow instead of presenting a screen that
 * could never approve anything. Ported from `DeviceLoginRequest.kt`.
 *
 * NOTE: the origin is taken from the ENGINE's URL, never from a constant —
 * that is what keeps regional deployments working with no hardcoded host.
 */
export class DeviceLoginRequest {
  readonly origin: string;
  readonly pairingId: string;
  readonly verificationUrl: string;

  private constructor(origin: string, pairingId: string, verificationUrl: string) {
    this.origin = origin;
    this.pairingId = pairingId;
    this.verificationUrl = verificationUrl;
  }

  static parse(rawUrl: string): DeviceLoginRequest | null {
    let parsed: url.URL;
    try {
      parsed = url.URL.parseURL(rawUrl);
    } catch (_error) {
      return null;
    }
    if (parsed.protocol.toLowerCase() !== 'https:') {
      return null;
    }
    // Userinfo in a verification URL is always a spoofing attempt.
    if (parsed.username.length > 0 || parsed.password.length > 0) {
      return null;
    }
    const host: string = parsed.host;
    if (host.length === 0) {
      return null;
    }
    const pairing: string | null = parsed.params.get('device_pairing');
    if (pairing === null || pairing.length === 0) {
      return null;
    }
    // `URL.host` already carries any non-default port.
    return new DeviceLoginRequest(`https://${host}`, pairing, rawUrl);
  }
}
