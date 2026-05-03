import type { ElementTree } from './helpers.js';

export interface ActivityLogParams {
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
}

const VALID_ACTIVITY_LOG_TONES = new Set<string>([
  'info',
  'success',
  'warning',
  'danger',
  'neutral',
]);

const ACTOR_FG = '#0F172A';
const ACTION_FG = '#475569';
const TS_FG = '#94A3B8';

const TONE_BG: Record<NonNullable<ActivityLogParams['tone']>, string> = {
  info: '#DBEAFE',
  success: '#DCFCE7',
  warning: '#FEF3C7',
  danger: '#FEE2E2',
  neutral: '#E2E8F0',
};
const TONE_FG: Record<NonNullable<ActivityLogParams['tone']>, string> = {
  info: '#1D4ED8',
  success: '#166534',
  warning: '#92400E',
  danger: '#991B1B',
  neutral: '#475569',
};

/**
 * Audit / activity log entry — single line with optional small tinted
 * icon dot, then "<actor> <action>" in mixed weights, then a right-
 * aligned relative timestamp. Designed for stacking under a heading
 * to form a feed (e.g. "Recent activity", "Audit log").
 *
 * Distinct from `add_timeline_v0` (vertical multi-event with
 * connecting line + per-event title + body) and
 * `add_notification_row_v0` (title + body, no actor focus).
 *
 * Use for "audit log row", "activity feed entry", "recent activity",
 * "审计日志条目", "活动记录".
 */
export function buildActivityLog(params: ActivityLogParams): ElementTree {
  const requestedTone = (params.tone ?? 'info') as string;
  if (!VALID_ACTIVITY_LOG_TONES.has(requestedTone)) {
    throw new Error(
      `add_activity_log_v0: invalid tone "${requestedTone}"; expected one of: info, success, warning, danger, neutral`,
    );
  }
  const tone = requestedTone as NonNullable<ActivityLogParams['tone']>;
  const rowChildren: ElementTree[] = [];

  if (params.icon) {
    rowChildren.push({
      type: 'frame',
      name: 'Icon Dot',
      role: 'activity-log-icon-dot',
      width: 28,
      height: 28,
      cornerRadius: 14,
      fill: [{ type: 'solid', color: TONE_BG[tone] }],
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
          fill: [{ type: 'solid', color: TONE_FG[tone] }],
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
          { text: params.actor, fontWeight: 600, fill: ACTOR_FG },
          { text: ` ${params.action}`, fontWeight: 400, fill: ACTION_FG },
        ],
        fontSize: 14,
        fontWeight: 400,
        width: 'fill_container',
        textGrowth: 'fixed-width',
        fill: [{ type: 'solid', color: ACTION_FG }],
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
    fill: [{ type: 'solid', color: TS_FG }],
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
