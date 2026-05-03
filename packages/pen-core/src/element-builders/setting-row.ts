import type { ElementTree } from './helpers.js';

export type SettingRowTrailing =
  | { kind: 'chevron' }
  | { kind: 'value'; value: string }
  | { kind: 'switch'; on: boolean }
  | { kind: 'badge'; value: string };

export interface SettingRowParams {
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
  trailing?: SettingRowTrailing;
}

const MUTED = '#64748B';
const FG = '#0F172A';
const PRIMARY = '#2563EB';
const TRACK_OFF = '#CBD5E1';
const KNOB = '#FFFFFF';
const BADGE_BG = '#DBEAFE';
const BADGE_FG = '#1D4ED8';

function buildTrailing(t: SettingRowTrailing): ElementTree {
  if (t.kind === 'value') {
    return {
      type: 'text',
      name: 'Trailing Value',
      role: 'setting-row-value',
      content: t.value,
      fontSize: 14,
      fontWeight: 400,
      fill: [{ type: 'solid', color: MUTED }],
    };
  }
  if (t.kind === 'switch') {
    return {
      type: 'frame',
      name: 'Switch',
      role: 'setting-row-switch',
      width: 36,
      height: 22,
      cornerRadius: 11,
      fill: [{ type: 'solid', color: t.on ? PRIMARY : TRACK_OFF }],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: t.on ? 'flex-end' : 'flex-start',
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
  if (t.kind === 'badge') {
    return {
      type: 'frame',
      name: 'Badge',
      role: 'setting-row-badge',
      width: 'fit_content',
      height: 'fit_content',
      cornerRadius: 4,
      fill: [{ type: 'solid', color: BADGE_BG }],
      padding: [2, 8],
      children: [
        {
          type: 'text',
          name: 'Badge Text',
          role: 'setting-row-badge-text',
          content: t.value,
          fontSize: 12,
          fontWeight: 600,
          fill: [{ type: 'solid', color: BADGE_FG }],
        },
      ],
    };
  }
  return {
    type: 'icon_font',
    name: 'Chevron',
    role: 'setting-row-chevron',
    iconFontName: 'chevron-right',
    iconFontFamily: 'lucide',
    width: 20,
    height: 20,
    fill: [{ type: 'solid', color: MUTED }],
  };
}

/**
 * Settings menu row — leading icon + (title over optional subtitle) +
 * trailing control. Distinct from `add_list_row_v0` (generic; trailing
 * is always an icon, no switch/value/badge variants) and
 * `add_form_field_v0` (label-above-input, used inside forms not menus).
 *
 * Structure:
 *   frame(horizontal, fill_container, fit_content, padding=[14,16],
 *         gap=12, alignItems=center, role='setting-row')
 *     ├ [icon_font 24×24] (if leading_icon)
 *     ├ frame(vertical, fill_container, gap=2, role='setting-row-text')
 *     │   ├ text(title, 15/500)
 *     │   └ text(subtitle, 13/400, muted) (if subtitle)
 *     └ <trailing>  (chevron | value text | switch | badge)
 *
 * The text-stack uses fill_container so the title can wrap and push
 * row height naturally — same no-overlap invariant as `add_list_row_v0`.
 */
export function buildSettingRow(params: SettingRowParams): ElementTree {
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
      fill: [{ type: 'solid', color: FG }],
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
      fill: [{ type: 'solid', color: FG }],
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
      fill: [{ type: 'solid', color: MUTED }],
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

  rowChildren.push(buildTrailing(trailing));

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
