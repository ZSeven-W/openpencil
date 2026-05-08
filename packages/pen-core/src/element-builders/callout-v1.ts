import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type CalloutV1Tone = 'info' | 'success' | 'warning' | 'danger' | 'note';

const VALID_CALLOUT_TONES = new Set<string>(['info', 'success', 'warning', 'danger', 'note']);

export interface CalloutV1Params {
  /** Body text. Required. */
  body: string;
  /** Optional bold heading line above the body. */
  title?: string;
  /** Color tone. Default 'note' (slate). */
  tone?: CalloutV1Tone;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_callout_v0.
   * - `'dark'`: dark-mode fills using semantic alert palette tokens.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   *
   * Note: 'note' tone uses surface colors (not alert tokens) since it
   * has no dedicated semantic alert token.
   */
  theme?: V1Theme;
}

interface ToneSpec {
  bg: string;
  fg: string;
  icon: string;
}

// Light-mode tone specs — must match v0 exactly for byte-parity
const TONES_LIGHT: Record<CalloutV1Tone, ToneSpec> = {
  info: { bg: '#DBEAFE', fg: '#1E40AF', icon: 'info' },
  success: { bg: '#DCFCE7', fg: '#166534', icon: 'check-circle' },
  warning: { bg: '#FEF3C7', fg: '#92400E', icon: 'alert-triangle' },
  danger: { bg: '#FEE2E2', fg: '#991B1B', icon: 'alert-octagon' },
  note: { bg: '#F1F5F9', fg: '#0F172A', icon: 'sticky-note' },
};

/**
 * Inline doc callout — theme-aware version of buildCallout.
 * Light mode is byte-equal to add_callout_v0.
 *
 * Dark/system modes map tones to semantic alert palette tokens:
 * - info/success/warning/danger → alertColors tokens
 * - note → surface2 bg + textPrimary fg (no dedicated alert token for note)
 */
export function buildCalloutV1(params: CalloutV1Params): ElementTree {
  const requestedTone = (params.tone ?? 'note') as string;
  if (!VALID_CALLOUT_TONES.has(requestedTone)) {
    throw new Error(
      `add_callout_v1: invalid tone "${requestedTone}"; expected one of: info, success, warning, danger, note`,
    );
  }
  const tone = requestedTone as CalloutV1Tone;
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';

  function getToneBg(tn: CalloutV1Tone): string {
    if (isLight) return TONES_LIGHT[tn].bg;
    if (tn === 'note') return t.colors.surface2;
    if (tn === 'info') return t.alertColors.infoBg;
    if (tn === 'success') return t.alertColors.successBg;
    if (tn === 'warning') return t.alertColors.warningBg;
    return t.alertColors.dangerBg;
  }

  function getToneFg(tn: CalloutV1Tone): string {
    if (isLight) return TONES_LIGHT[tn].fg;
    if (tn === 'note') return t.colors.textPrimary;
    if (tn === 'info') return t.alertColors.infoText;
    if (tn === 'success') return t.alertColors.successText;
    if (tn === 'warning') return t.alertColors.warningText;
    return t.alertColors.dangerText;
  }

  const bg = getToneBg(tone);
  const fg = getToneFg(tone);
  const iconName = TONES_LIGHT[tone].icon;

  const stackChildren: ElementTree[] = [];
  if (params.title) {
    stackChildren.push({
      type: 'text',
      name: 'Title',
      role: 'callout-title',
      content: params.title,
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: fg }],
    });
  }
  stackChildren.push({
    type: 'text',
    name: 'Body',
    role: 'callout-body',
    content: params.body,
    fontSize: 13,
    fontWeight: 400,
    lineHeight: 1.5,
    fill: [{ type: 'solid', color: fg }],
    width: 'fill_container',
    textGrowth: 'fixed-width',
  });
  return {
    type: 'frame',
    name: 'Callout',
    role: 'callout',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'start',
    gap: 12,
    padding: [12, 16],
    cornerRadius: 8,
    fill: [{ type: 'solid', color: bg }],
    children: [
      {
        type: 'icon_font',
        name: 'Tone Icon',
        role: 'callout-icon',
        iconFontName: iconName,
        iconFontFamily: 'lucide',
        width: 18,
        height: 18,
        fill: [{ type: 'solid', color: fg }],
      },
      {
        type: 'frame',
        name: 'Text Stack',
        role: 'callout-text',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 4,
        children: stackChildren,
      },
    ],
  };
}
