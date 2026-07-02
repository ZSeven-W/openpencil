import { coerceEnum } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export interface ActivityLogV1Params {
  /** Actor name (rendered bold). E.g. "Sarah Lee". */
  actor: string;
  /** Action verb-phrase (rendered as plain text after actor). E.g. "merged pull request #142". */
  action: string;
  /** Right-aligned relative timestamp (e.g. "2h ago", "Just now"). */
  timestamp: string;
  /** Optional leading lucide icon slug (16×16, in a small tinted dot). */
  icon?: string;
  /** Icon dot tone. Default "info". */
  tone?: 'info' | 'success' | 'warning' | 'danger' | 'neutral';
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_activity_log_v0.
   * - `'dark'`: dark-mode hex fills via semantic palette.
   * - `'system'`: emits `$color-*` ref strings for all fill fields.
   */
  theme?: V1Theme;
}

// Light-mode constants (must match v0 exactly for byte-parity)
const ACTOR_FG_LIGHT = '#0F172A';
const ACTION_FG_LIGHT = '#475569';
const TS_FG_LIGHT = '#94A3B8';

type Tone = NonNullable<ActivityLogV1Params['tone']>;

const TONE_BG_LIGHT: Record<Tone, string> = {
  info: '#DBEAFE',
  success: '#DCFCE7',
  warning: '#FEF3C7',
  danger: '#FEE2E2',
  neutral: '#E2E8F0',
};
const TONE_FG_LIGHT: Record<Tone, string> = {
  info: '#1D4ED8',
  success: '#166534',
  warning: '#92400E',
  danger: '#991B1B',
  neutral: '#475569',
};

// Dark / system fallback for 'neutral' tone (no alertColors token for neutral)
const NEUTRAL_BG_DARK = '#1E293B';
const NEUTRAL_FG_DARK = '#94A3B8';
const NEUTRAL_BG_SYSTEM = '$color-surface'; // best-effort: surface as neutral container
const NEUTRAL_FG_SYSTEM = '$color-text-muted';

/**
 * Audit / activity log entry — theme-aware version of buildActivityLog.
 * Light mode is byte-equal to add_activity_log_v0.
 * Dark mode: uses semantic palette alertColors for info/success/warning/danger;
 * neutral tone gets a surface-based dark fill.
 * System mode: info/success/warning/danger use $color-*-bg / $color-*-text refs;
 * neutral uses $color-surface / $color-text-muted.
 */
export function buildActivityLogV1(params: ActivityLogV1Params): ElementTree {
  const tone = coerceEnum<Tone>(
    params.tone,
    ['info', 'success', 'warning', 'danger', 'neutral'],
    'info',
    'buildActivityLogV1',
    'tone',
  );
  const theme = params.theme ?? 'light';
  const t = resolveTheme(theme);
  const isLight = theme === 'light';
  const isSystem = theme === 'system';

  function getToneBg(tn: Tone): string {
    if (isLight) return TONE_BG_LIGHT[tn];
    if (tn === 'neutral') return isSystem ? NEUTRAL_BG_SYSTEM : NEUTRAL_BG_DARK;
    if (isSystem) {
      if (tn === 'info') return t.alertColors.infoBg;
      if (tn === 'success') return t.alertColors.successBg;
      if (tn === 'warning') return t.alertColors.warningBg;
      return t.alertColors.dangerBg;
    }
    // dark — use resolveTheme('dark').alertColors
    if (tn === 'info') return t.alertColors.infoBg;
    if (tn === 'success') return t.alertColors.successBg;
    if (tn === 'warning') return t.alertColors.warningBg;
    return t.alertColors.dangerBg;
  }
  function getToneFg(tn: Tone): string {
    if (isLight) return TONE_FG_LIGHT[tn];
    if (tn === 'neutral') return isSystem ? NEUTRAL_FG_SYSTEM : NEUTRAL_FG_DARK;
    if (isSystem) {
      if (tn === 'info') return t.alertColors.infoText;
      if (tn === 'success') return t.alertColors.successText;
      if (tn === 'warning') return t.alertColors.warningText;
      return t.alertColors.dangerText;
    }
    // dark
    if (tn === 'info') return t.alertColors.infoText;
    if (tn === 'success') return t.alertColors.successText;
    if (tn === 'warning') return t.alertColors.warningText;
    return t.alertColors.dangerText;
  }

  // Text colors
  const actorFg = isLight ? ACTOR_FG_LIGHT : t.colors.textPrimary;
  const actionFg = isLight ? ACTION_FG_LIGHT : t.colors.textBody;
  const tsFg = isLight ? TS_FG_LIGHT : t.colors.textSubtle;

  const rowChildren: ElementTree[] = [];

  if (params.icon) {
    rowChildren.push({
      type: 'frame',
      name: 'Icon Dot',
      role: 'activity-log-icon-dot',
      width: 28,
      height: 28,
      cornerRadius: 14,
      fill: [{ type: 'solid', color: getToneBg(tone) }],
      layout: 'horizontal',
      alignItems: 'center',
      justifyContent: 'center',
      children: [
        {
          type: 'icon_font',
          name: 'Icon',
          role: 'activity-log-icon',
          iconFontName: params.icon,
          iconFontFamily: 'lucide',
          width: 14,
          height: 14,
          fill: [{ type: 'solid', color: getToneFg(tone) }],
        },
      ],
    });
  }

  rowChildren.push({
    type: 'frame',
    name: 'Body',
    role: 'activity-log-body',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'vertical',
    gap: 0,
    children: [
      {
        type: 'text',
        name: 'Line',
        role: 'activity-log-line',
        content: [
          { text: params.actor, fontWeight: 600, fill: actorFg },
          { text: ` ${params.action}`, fontWeight: 400, fill: actionFg },
        ],
        fontSize: 14,
        fontWeight: 400,
        width: 'fill_container',
        textGrowth: 'fixed-width',
        fill: [{ type: 'solid', color: actionFg }],
      },
    ],
  });

  rowChildren.push({
    type: 'text',
    name: 'Timestamp',
    role: 'activity-log-timestamp',
    content: params.timestamp,
    fontSize: 13,
    fontWeight: 400,
    fill: [{ type: 'solid', color: tsFg }],
  });

  return {
    type: 'frame',
    name: 'Activity Log Entry',
    role: 'activity-log',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [10, 0],
    children: rowChildren,
  };
}
