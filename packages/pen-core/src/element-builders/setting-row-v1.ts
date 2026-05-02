import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type SettingRowV1Trailing =
  | { kind: 'chevron' }
  | { kind: 'value'; value: string }
  | { kind: 'switch'; on: boolean }
  | { kind: 'badge'; value: string };

export interface SettingRowV1Params {
  /** Title text on the left (15/500). */
  title: string;
  /** Optional second-line text below the title (13/400, muted). */
  subtitle?: string;
  /** Optional leading lucide icon slug (24×24). */
  leading_icon?: string;
  /**
   * Trailing control. Default: chevron.
   * - chevron: a `chevron-right` icon (link-style row)
   * - value: muted text (e.g. "English") with no chevron
   * - switch: 36×22 pill, on/off
   * - badge: small colored capsule with text (e.g. "New")
   */
  trailing?: SettingRowV1Trailing;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_setting_row_v0.
   * - `'dark'`: dark-mode hex fills.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   */
  theme?: V1Theme;
}

// Builder-private constants (all modes)
const KNOB = '#FFFFFF'; // knob is always white regardless of theme

// v0 badge colors: these don't exactly match alertColors tokens (infoText light = #1E40AF,
// but v0 uses #1D4ED8). Keep as builder-private for light; use alertColors in dark/system.
const BADGE_BG_LIGHT = '#DBEAFE';
const BADGE_FG_LIGHT = '#1D4ED8';

function buildTrailing(trailing: SettingRowV1Trailing, theme: V1Theme): ElementTree {
  const t = resolveTheme(theme);

  if (trailing.kind === 'value') {
    return {
      type: 'text',
      name: 'Trailing Value',
      role: 'setting-row-value',
      content: trailing.value,
      fontSize: 14,
      fontWeight: 400,
      fill: [{ type: 'solid', color: t.colors.textMuted }],
    };
  }
  if (trailing.kind === 'switch') {
    return {
      type: 'frame',
      name: 'Switch',
      role: 'setting-row-switch',
      width: 36,
      height: 22,
      cornerRadius: 11,
      fill: [{ type: 'solid', color: trailing.on ? t.colors.accent : t.colors.borderStrong }],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: trailing.on ? 'flex-end' : 'flex-start',
      padding: [0, 3],
      children: [
        {
          type: 'frame',
          name: 'Knob',
          role: 'setting-row-switch-knob',
          width: 16,
          height: 16,
          cornerRadius: 8,
          fill: [{ type: 'solid', color: KNOB }],
          children: [],
        },
      ],
    };
  }
  if (trailing.kind === 'badge') {
    // Light: use v0's exact hex literals for byte-parity.
    // Dark/system: use alertColors.infoBg / alertColors.infoText tokens.
    const isLight = theme === 'light';
    const badgeBg = isLight ? BADGE_BG_LIGHT : t.alertColors.infoBg;
    const badgeFg = isLight ? BADGE_FG_LIGHT : t.alertColors.infoText;
    return {
      type: 'frame',
      name: 'Badge',
      role: 'setting-row-badge',
      width: 'fit_content',
      height: 'fit_content',
      cornerRadius: 4,
      fill: [{ type: 'solid', color: badgeBg }],
      padding: [2, 8],
      children: [
        {
          type: 'text',
          name: 'Badge Text',
          role: 'setting-row-badge-text',
          content: trailing.value,
          fontSize: 12,
          fontWeight: 600,
          fill: [{ type: 'solid', color: badgeFg }],
        },
      ],
    };
  }
  // chevron
  return {
    type: 'icon_font',
    name: 'Chevron',
    role: 'setting-row-chevron',
    iconFontName: 'chevron-right',
    iconFontFamily: 'lucide',
    width: 20,
    height: 20,
    fill: [{ type: 'solid', color: t.colors.textMuted }],
  };
}

/**
 * Settings menu row — theme-aware version of buildSettingRow.
 * Light mode is byte-equal to add_setting_row_v0.
 * Dark/system modes use resolveTheme() for all color fills.
 */
export function buildSettingRowV1(params: SettingRowV1Params): ElementTree {
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const trailing = params.trailing ?? { kind: 'chevron' };
  const rowChildren: ElementTree[] = [];

  if (params.leading_icon) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Leading Icon',
      role: 'setting-row-icon',
      iconFontName: params.leading_icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: t.colors.textPrimary }],
    });
  }

  const textStackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Title',
      role: 'setting-row-title',
      content: params.title,
      fontSize: 15,
      fontWeight: 500,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: t.colors.textPrimary }],
    },
  ];
  if (params.subtitle) {
    textStackChildren.push({
      type: 'text',
      name: 'Subtitle',
      role: 'setting-row-subtitle',
      content: params.subtitle,
      fontSize: 13,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: t.colors.textMuted }],
    });
  }

  rowChildren.push({
    type: 'frame',
    name: 'Text Stack',
    role: 'setting-row-text',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 2,
    children: textStackChildren,
  });

  rowChildren.push(buildTrailing(trailing, theme));

  return {
    type: 'frame',
    name: 'Setting Row',
    role: 'setting-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [14, 16],
    children: rowChildren,
  };
}
