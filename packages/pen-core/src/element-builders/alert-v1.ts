import type { ElementTree } from './helpers.js';
import type { V1Theme } from './resolve-theme.js';

export interface AlertV1Params {
  message: string;
  icon?: string;
  dismissible?: boolean;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_alert_v0.
   * - `'dark'`: byte-parity with light (alert has no hardcoded color fills).
   * - `'system'`: byte-parity with light (no $color-* refs needed).
   *
   * `buildAlert` emits no fill literals — semantic color is applied via
   * a follow-up batch_design U-op. Theme param is accepted for API
   * consistency with other v1 tools.
   */
  theme?: V1Theme;
}

/**
 * Inline alert / callout banner — theme-aware version of buildAlert.
 * Light mode is byte-equal to add_alert_v0.
 *
 * Since buildAlert emits no hardcoded color fills, all three theme modes
 * produce identical output. The theme parameter exists for API consistency
 * so callers can pass theme uniformly across all v1 element tools.
 */
export function buildAlertV1(params: AlertV1Params): ElementTree {
  const children: ElementTree[] = [];
  if (params.icon) {
    children.push({
      type: 'icon_font',
      name: 'Leading Icon',
      iconFontName: params.icon,
      iconFontFamily: 'lucide',
      width: 20,
      height: 20,
    });
  }
  children.push({
    type: 'text',
    name: 'Message',
    role: 'alert-message',
    content: params.message,
    fontSize: 14,
    fontWeight: 400,
  });
  if (params.dismissible) {
    children.push({
      type: 'icon_font',
      name: 'Close',
      role: 'alert-close',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
    });
  }
  return {
    type: 'frame',
    name: 'Alert',
    role: 'alert',
    width: 'fill_container',
    cornerRadius: 8,
    padding: [12, 16],
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    children,
  };
}
