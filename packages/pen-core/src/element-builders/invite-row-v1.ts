import { coerceEnum } from './coerce-params.js';
import type { ElementTree } from './helpers.js';
import { resolveTheme, type V1Theme } from './resolve-theme.js';

export type InviteV1Status = 'pending' | 'expired' | 'accepted';

export interface InviteRowV1Params {
  /** Invitee email (e.g. "sarah@acme.com"). */
  email: string;
  /** Optional invited role (e.g. "Editor", "Viewer"). */
  role?: string;
  /** Status. Default "pending". */
  status?: InviteV1Status;
  /** Trailing primary action label (e.g. "Resend", "Revoke"). Default "Resend". */
  action_label?: string;
  /**
   * Theme mode.
   * - `'light'` (default): byte-parity with add_invite_row_v0.
   * - `'dark'`: email/initial text → textPrimary, role → textMuted,
   *             avatar bg → surface2, action → accent,
   *             status pills use alertColors tokens.
   * - `'system'`: emits `$color-*` refs for all fill fields.
   */
  theme?: V1Theme;
}

// Light-mode hex (must match v0 byte-for-byte)
const NAME_FG_LIGHT = '#0F172A';
const SUB_FG_LIGHT = '#64748B';
const AVATAR_BG_LIGHT = '#E2E8F0';
const ACTION_FG_LIGHT = '#2563EB';

type InviteStatusTone = Record<InviteV1Status, { bg: string; fg: string; text: string }>;

const STATUS_TONE_LIGHT: InviteStatusTone = {
  pending: { bg: '#FEF3C7', fg: '#92400E', text: 'Pending' },
  expired: { bg: '#FEE2E2', fg: '#991B1B', text: 'Expired' },
  accepted: { bg: '#DCFCE7', fg: '#166534', text: 'Accepted' },
};

/**
 * Pending invite list row — theme-aware variant of buildInviteRow.
 * Light mode is byte-equal to add_invite_row_v0.
 *
 * Color mapping:
 *   avatar bg    (#E2E8F0 slate-200) → surface2
 *   avatar init. (#64748B slate-500) → textMuted
 *   email text   (#0F172A slate-950) → textPrimary
 *   role text    (#64748B slate-500) → textMuted
 *   action CTA   (#2563EB blue-600)  → accent (brand-invariant)
 *   status pills: pending bg  (#FEF3C7) → alertColors.warningBg
 *                 pending fg  (#92400E) → alertColors.warningText
 *                 expired bg  (#FEE2E2) → alertColors.dangerBg
 *                 expired fg  (#991B1B) → alertColors.dangerText
 *                 accepted bg (#DCFCE7) → alertColors.successBg
 *                 accepted fg (#166534) → alertColors.successText
 */
export function buildInviteRowV1(params: InviteRowV1Params): ElementTree {
  const status = coerceEnum<InviteV1Status>(
    params.status,
    ['pending', 'expired', 'accepted'],
    'pending',
    'buildInviteRowV1',
    'status',
  );
  const actionLabel = params.action_label ?? 'Resend';
  const initial = (params.email.charAt(0) ?? '?').toUpperCase();
  const theme = params.theme ?? 'light';
  const isLight = theme === 'light';
  const t = resolveTheme(theme);

  // Text / action colors
  const nameFg = isLight ? NAME_FG_LIGHT : t.colors.textPrimary;
  const subFg = isLight ? SUB_FG_LIGHT : t.colors.textMuted;
  const avatarBg = isLight ? AVATAR_BG_LIGHT : t.colors.surface2;
  const actionFg = isLight ? ACTION_FG_LIGHT : t.colors.accent;

  // Status pill colors
  let pillBg: string;
  let pillFg: string;
  const pillText = STATUS_TONE_LIGHT[status].text;
  if (isLight) {
    pillBg = STATUS_TONE_LIGHT[status].bg;
    pillFg = STATUS_TONE_LIGHT[status].fg;
  } else if (status === 'accepted') {
    pillBg = t.alertColors.successBg;
    pillFg = t.alertColors.successText;
  } else if (status === 'expired') {
    pillBg = t.alertColors.dangerBg;
    pillFg = t.alertColors.dangerText;
  } else {
    // pending
    pillBg = t.alertColors.warningBg;
    pillFg = t.alertColors.warningText;
  }

  const avatar: ElementTree = {
    type: 'frame',
    name: 'Avatar',
    role: 'invite-row-avatar',
    width: 36,
    height: 36,
    cornerRadius: 18,
    fill: [{ type: 'solid', color: avatarBg }],
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'text',
        name: 'Initial',
        role: 'invite-row-avatar-initial',
        content: initial,
        fontSize: 14,
        fontWeight: 600,
        fill: [{ type: 'solid', color: subFg }],
      },
    ],
  };

  const textStackChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Email',
      role: 'invite-row-email',
      content: params.email,
      fontSize: 14,
      fontWeight: 500,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: nameFg }],
    },
  ];
  if (params.role) {
    textStackChildren.push({
      type: 'text',
      name: 'Role',
      role: 'invite-row-role',
      content: params.role,
      fontSize: 12,
      fontWeight: 400,
      width: 'fill_container',
      textGrowth: 'fixed-width',
      fill: [{ type: 'solid', color: subFg }],
    });
  }

  const statusPill: ElementTree = {
    type: 'frame',
    name: 'Status Pill',
    role: 'invite-row-status',
    width: 'fit_content',
    height: 'fit_content',
    cornerRadius: 12,
    fill: [{ type: 'solid', color: pillBg }],
    padding: [3, 10],
    children: [
      {
        type: 'text',
        name: 'Status Text',
        role: 'invite-row-status-text',
        content: pillText,
        fontSize: 12,
        fontWeight: 600,
        fill: [{ type: 'solid', color: pillFg }],
      },
    ],
  };

  const action: ElementTree = {
    type: 'text',
    name: 'Action',
    role: 'invite-row-action',
    content: actionLabel,
    fontSize: 14,
    fontWeight: 600,
    fill: [{ type: 'solid', color: actionFg }],
  };

  return {
    type: 'frame',
    name: 'Invite Row',
    role: 'invite-row',
    width: 'fill_container',
    height: 'fit_content',
    layout: 'horizontal',
    alignItems: 'center',
    gap: 12,
    padding: [12, 16],
    children: [
      avatar,
      {
        type: 'frame',
        name: 'Text Stack',
        role: 'invite-row-text',
        width: 'fill_container',
        height: 'fit_content',
        layout: 'vertical',
        gap: 2,
        children: textStackChildren,
      },
      statusPill,
      action,
    ],
  };
}
