import type { ElementTree } from './helpers.js';

export type InviteStatus = 'pending' | 'expired' | 'accepted';

export interface InviteRowParams {
  /** Invitee email (e.g. "sarah@acme.com"). */
  email: string;
  /** Optional invited role (e.g. "Editor", "Viewer"). */
  role?: string;
  /** Status. Default "pending". */
  status?: InviteStatus;
  /** Trailing primary action label (e.g. "Resend", "Revoke"). Default "Resend". */
  action_label?: string;
}

const NAME_FG = '#0F172A';
const SUB_FG = '#64748B';
const STATUS_TONE: Record<InviteStatus, { bg: string; fg: string; text: string }> = {
  pending: { bg: '#FEF3C7', fg: '#92400E', text: 'Pending' },
  expired: { bg: '#FEE2E2', fg: '#991B1B', text: 'Expired' },
  accepted: { bg: '#DCFCE7', fg: '#166534', text: 'Accepted' },
};
const ACTION_FG = '#2563EB';

/**
 * Pending invite list row — avatar (initial from email) + (email over
 * role) + status pill + trailing action label. Distinct from
 * `add_member_row_v0` (an actual joined member with full name + role
 * badge) and `add_list_row_v0` (no avatar, no status pill, no action
 * label slot).
 *
 * Use for "pending invites list", "invitations table row", "team
 * invite row", "邀请列表行", "待接受邀请".
 */
export function buildInviteRow(params: InviteRowParams): ElementTree {
  const status = params.status ?? 'pending';
  const tone = STATUS_TONE[status];
  const actionLabel = params.action_label ?? 'Resend';
  const initial = (params.email.charAt(0) ?? '?').toUpperCase();

  const avatar: ElementTree = {
    type: 'frame',
    name: 'Avatar',
    role: 'invite-row-avatar',
    width: 36,
    height: 36,
    cornerRadius: 18,
    fill: [{ type: 'solid', color: '#E2E8F0' }],
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
        fill: [{ type: 'solid', color: SUB_FG }],
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
      fill: [{ type: 'solid', color: NAME_FG }],
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
      fill: [{ type: 'solid', color: SUB_FG }],
    });
  }

  const statusPill: ElementTree = {
    type: 'frame',
    name: 'Status Pill',
    role: 'invite-row-status',
    width: 'fit_content',
    height: 'fit_content',
    cornerRadius: 12,
    fill: [{ type: 'solid', color: tone.bg }],
    padding: [3, 10],
    children: [
      {
        type: 'text',
        name: 'Status Text',
        role: 'invite-row-status-text',
        content: tone.text,
        fontSize: 12,
        fontWeight: 600,
        fill: [{ type: 'solid', color: tone.fg }],
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
    fill: [{ type: 'solid', color: ACTION_FG }],
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
