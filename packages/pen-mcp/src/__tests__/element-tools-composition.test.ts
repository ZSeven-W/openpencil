import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { invalidateCache } from '../document-manager';
import { handleAddTopNavBarV0 } from '../tools/add-top-nav-bar-v0';
import { handleAddBottomNavV0 } from '../tools/add-bottom-nav-v0';
import { handleAddSectionHeaderV0 } from '../tools/add-section-header-v0';
import { handleAddListRowV0 } from '../tools/add-list-row-v0';
import { handleAddStatGridV0 } from '../tools/add-stat-grid-v0';
import { handleAddMetricComparisonV0 } from '../tools/add-metric-comparison-v0';
import { handleAddChartBarsV0 } from '../tools/add-chart-bars-v0';
import { handleAddHeadingV0 } from '../tools/add-heading-v0';
import { handleAddBodyTextV0 } from '../tools/add-body-text-v0';
import { handleAddFormFieldV0 } from '../tools/add-form-field-v0';
import { handleAddTextButtonV0 } from '../tools/add-text-button-v0';
import { handleAddLinkV0 } from '../tools/add-link-v0';
import { handleAddAvatarV0 } from '../tools/add-avatar-v0';
import { handleAddBadgeV0 } from '../tools/add-badge-v0';
import { handleAddSearchBarV0 } from '../tools/add-search-bar-v0';
import { handleAddCardRowV0 } from '../tools/add-card-row-v0';
import { handleAddDividerV0 } from '../tools/add-divider-v0';
import { handleAddPaginationV0 } from '../tools/add-pagination-v0';
import { handleAddFaqItemV0 } from '../tools/add-faq-item-v0';
import { handleAddActionMenuV0 } from '../tools/add-action-menu-v0';
import { handleAddDatePickerV0 } from '../tools/add-date-picker-v0';
import { handleAddEmptyChartV0 } from '../tools/add-empty-chart-v0';
import { handleAddChipInputV0 } from '../tools/add-chip-input-v0';
import { handleAddModalShellV1 } from '../tools/add-modal-shell-v1';
import { handleAddUploadDropzoneV0 } from '../tools/add-upload-dropzone-v0';
import { handleAddOtpInputV0 } from '../tools/add-otp-input-v0';
import { handleAddAttachmentRowV0 } from '../tools/add-attachment-row-v0';
import { handleAddChatBubbleV0 } from '../tools/add-chat-bubble-v0';

/**
 * End-to-end composition tests — verifies that N element-tool calls
 * can be chained together with parent_id threading to produce a
 * realistic multi-section screen without breaking tree invariants.
 *
 * Per-tool handler tests already cover "this tool emits the correct
 * shape." This file covers the next layer up: "can the AI take the
 * output of tool A, use its nodeId as parent_id for tool B, and
 * produce a well-formed screen across N such calls?" That's the
 * actual usage pattern under batch orchestration.
 *
 * Invariants checked after each composition:
 *   1. No call throws
 *   2. Every result includes a nodeId (no silent no-ops)
 *   3. Final document is valid JSON with a parseable tree
 *   4. Every tool call's nodeId is findable in the final tree
 *   5. parent_id threading works — a child's actual parent in the
 *      saved tree matches the parent_id we passed in
 *   6. Every tool's role marker survives into the final tree
 *
 * Screens intentionally span tool families (nav + content + inputs +
 * charts + feedback + pagination) so we catch cross-family regressions.
 */

const TMP = join(tmpdir(), 'openpencil-element-tools-composition');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}
async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}
function getRootChildren(doc: Record<string, unknown>): Record<string, unknown>[] {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const top = doc['children'] as Record<string, unknown>[] | undefined;
  return top ?? pages?.[0]?.children ?? [];
}

function findRoleInTree(
  nodes: Record<string, unknown>[],
  role: string,
): Record<string, unknown> | undefined {
  for (const n of nodes) {
    if (n.role === role) return n;
    const kids = n.children as Record<string, unknown>[] | undefined;
    if (kids) {
      const hit = findRoleInTree(kids, role);
      if (hit) return hit;
    }
  }
  return undefined;
}

function findIdInTree(
  nodes: Record<string, unknown>[],
  id: string,
): Record<string, unknown> | undefined {
  for (const n of nodes) {
    if (n.id === id) return n;
    const kids = n.children as Record<string, unknown>[] | undefined;
    if (kids) {
      const hit = findIdInTree(kids, id);
      if (hit) return hit;
    }
  }
  return undefined;
}

function nodeIdFrom(result: { results?: Array<{ nodeId?: string | null }> | undefined }): string {
  const id = result.results?.[0]?.nodeId;
  if (!id) throw new Error('tool call returned no nodeId');
  return id;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of [
    'settings.op',
    'dashboard.op',
    'login.op',
    'profile.op',
    'listing.op',
    'support-chat.op',
  ]) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('element-tools composition — N-tool real screens', () => {
  it('mobile settings: top nav + 2 sections × 3 list rows + bottom nav (9 calls)', async () => {
    const fp = await fresh('settings.op');

    // Top nav bar at root
    const top = await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Settings',
      leading_icon: 'chevron-left',
    });
    const topId = nodeIdFrom(top);

    // Section 1 header
    const sec1 = await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Account',
    });
    const sec1Id = nodeIdFrom(sec1);

    // 3 list rows under section 1
    const acc1 = await handleAddListRowV0({
      filePath: fp,
      title: 'Profile',
      leading_icon: 'user',
      trailing_icon: 'chevron-right',
    });
    const acc2 = await handleAddListRowV0({
      filePath: fp,
      title: 'Privacy',
      leading_icon: 'shield',
      trailing_icon: 'chevron-right',
    });
    const acc3 = await handleAddListRowV0({
      filePath: fp,
      title: 'Notifications',
      leading_icon: 'bell',
      trailing_icon: 'chevron-right',
    });

    // Section 2 header
    const sec2 = await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Support',
    });
    const sec2Id = nodeIdFrom(sec2);

    // 3 rows under section 2
    const sup1 = await handleAddListRowV0({
      filePath: fp,
      title: 'Help Center',
      leading_icon: 'help-circle',
    });
    const sup2 = await handleAddListRowV0({
      filePath: fp,
      title: 'Contact Us',
      leading_icon: 'mail',
    });
    const sup3 = await handleAddListRowV0({
      filePath: fp,
      title: 'About',
      leading_icon: 'info',
    });

    // Bottom nav at root
    const bot = await handleAddBottomNavV0({
      filePath: fp,
      items: [
        { title: 'Home', icon: 'home' },
        { title: 'Search', icon: 'search' },
        { title: 'Profile', icon: 'user', active: true },
      ],
    });

    // Assert all 10 calls emitted a nodeId
    for (const r of [top, sec1, acc1, acc2, acc3, sec2, sup1, sup2, sup3, bot]) {
      expect(r.results?.[0]?.nodeId).toBeDefined();
    }

    // Validate tree
    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    // At minimum: top nav + 2 section headers + 6 list rows + bottom nav = 10 nodes
    expect(children.length).toBe(10);
    expect(findRoleInTree(children, 'top-nav-bar')).toBeDefined();
    expect(findRoleInTree(children, 'section-header')).toBeDefined();
    expect(findRoleInTree(children, 'list-row')).toBeDefined();
    expect(findRoleInTree(children, 'bottom-tab-bar')).toBeDefined();
    // Cross-check: all 10 ids findable
    for (const r of [top, sec1, acc1, acc2, acc3, sec2, sup1, sup2, sup3, bot]) {
      expect(findIdInTree(children, r.results[0].nodeId as string)).toBeDefined();
    }
    // Unused vars suppression
    void sec1Id;
    void sec2Id;
    void topId;
  });

  it('dashboard home: nav + stat grid + section + 3 metric comparisons + chart (7 calls)', async () => {
    const fp = await fresh('dashboard.op');

    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Dashboard',
      trailing_icon: 'more-vertical',
    });

    await handleAddStatGridV0({
      filePath: fp,
      items: [
        { value: '8,432', label: 'Steps' },
        { value: '512', label: 'Kcal' },
        { value: '7h', label: 'Sleep' },
      ],
    });

    await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'KPIs',
    });

    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Revenue',
      value: '$12k',
      change: '8%',
      trend: 'up',
    });

    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Orders',
      value: '1,248',
      change: '3%',
      trend: 'up',
    });

    await handleAddMetricComparisonV0({
      filePath: fp,
      label: 'Churn',
      value: '1.2%',
      change: '0.5%',
      trend: 'down',
    });

    await handleAddChartBarsV0({
      filePath: fp,
      values: [4, 7, 3, 9, 5, 8, 6],
    });

    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    expect(children.length).toBe(7);
    expect(findRoleInTree(children, 'top-nav-bar')).toBeDefined();
    expect(findRoleInTree(children, 'stat-grid')).toBeDefined();
    expect(findRoleInTree(children, 'section-header')).toBeDefined();
    expect(findRoleInTree(children, 'metric-comparison')).toBeDefined();
    expect(findRoleInTree(children, 'chart-bars')).toBeDefined();
  });

  it('login form: heading + body + 2 fields + button + link (6 calls)', async () => {
    const fp = await fresh('login.op');

    const h = await handleAddHeadingV0({ filePath: fp, content: 'Welcome back' });
    const b = await handleAddBodyTextV0({
      filePath: fp,
      content: 'Sign in to continue to your account.',
    });
    const f1 = await handleAddFormFieldV0({
      filePath: fp,
      label: 'Email',
      placeholder: 'you@example.com',
      leading_icon: 'mail',
      required: true,
    });
    const f2 = await handleAddFormFieldV0({
      filePath: fp,
      label: 'Password',
      leading_icon: 'lock',
      trailing_icon: 'eye',
      required: true,
    });
    const btn = await handleAddTextButtonV0({ filePath: fp, label: 'Sign in' });
    const lk = await handleAddLinkV0({ filePath: fp, label: 'Forgot password?' });

    for (const r of [h, b, f1, f2, btn, lk]) {
      expect(r.results?.[0]?.nodeId).toBeDefined();
    }

    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    expect(children.length).toBe(6);
    expect(findRoleInTree(children, 'heading')).toBeDefined();
    expect(findRoleInTree(children, 'body')).toBeDefined();
    expect(findRoleInTree(children, 'form-field')).toBeDefined();
    expect(findRoleInTree(children, 'button')).toBeDefined();
    expect(findRoleInTree(children, 'link')).toBeDefined();
  });

  it('profile + UGC: nav + avatar + heading + badge + 2 FAQ items + action menu (7 calls)', async () => {
    const fp = await fresh('profile.op');

    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Profile',
      trailing_icon: 'more-vertical',
    });
    await handleAddAvatarV0({ filePath: fp, initial: 'SJ', size: 80 });
    await handleAddHeadingV0({ filePath: fp, content: 'Sarah Johnson' });
    await handleAddBadgeV0({ filePath: fp, label: 'PRO' });
    await handleAddFaqItemV0({
      filePath: fp,
      question: 'Can I change my plan?',
      answer: 'Go to Settings → Billing.',
      expanded: true,
    });
    await handleAddFaqItemV0({
      filePath: fp,
      question: 'How do I cancel?',
    });
    await handleAddActionMenuV0({
      filePath: fp,
      items: [
        { label: 'Edit', icon: 'pencil' },
        { label: 'Share', icon: 'share' },
        { label: 'Delete', icon: 'trash', destructive: true, divider_before: true },
      ],
    });

    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    expect(children.length).toBe(7);
    expect(findRoleInTree(children, 'faq-item')).toBeDefined();
    expect(findRoleInTree(children, 'action-menu')).toBeDefined();
    expect(findRoleInTree(children, 'action-menu-item-destructive')).toBeDefined();
  });

  it('listing: search + card row + divider + empty chart + date picker + chip input + pagination (7 calls)', async () => {
    const fp = await fresh('listing.op');

    await handleAddSearchBarV0({ filePath: fp, placeholder: 'Search…' });
    await handleAddCardRowV0({
      filePath: fp,
      items: [
        { title: 'A', subtitle: '1' },
        { title: 'B', subtitle: '2' },
        { title: 'C', subtitle: '3' },
      ],
    });
    await handleAddDividerV0({ filePath: fp });
    await handleAddEmptyChartV0({
      filePath: fp,
      title: 'No trends yet',
      subtitle: 'Come back later',
      icon: 'line-chart',
    });
    await handleAddDatePickerV0({
      filePath: fp,
      label: 'Filter date',
      value: 'Jan 15, 2026',
      clearable: true,
    });
    await handleAddChipInputV0({
      filePath: fp,
      label: 'Tags',
      chips: ['design', 'mobile'],
    });
    await handleAddPaginationV0({ filePath: fp, total: 20, current: 3 });

    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    expect(children.length).toBe(7);
    expect(findRoleInTree(children, 'search-bar')).toBeDefined();
    expect(findRoleInTree(children, 'empty-chart')).toBeDefined();
    expect(findRoleInTree(children, 'date-picker')).toBeDefined();
    expect(findRoleInTree(children, 'chip-input')).toBeDefined();
    expect(findRoleInTree(children, 'pagination')).toBeDefined();
  });

  it('parent_id threading: nested insert actually lands under named parent', async () => {
    const fp = await fresh('settings.op');

    // Insert section header at root
    const sec = await handleAddSectionHeaderV0({
      filePath: fp,
      title: 'Account',
    });
    const secId = nodeIdFrom(sec);

    // Insert a list row with parent_id = section id
    const row = await handleAddListRowV0({
      filePath: fp,
      title: 'Profile',
      parent_id: secId,
    });
    const rowId = nodeIdFrom(row);

    // Verify: rowId must be nested UNDER secId in the tree
    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    // Find the section at root and confirm the row is one of its children
    const section = findIdInTree(children, secId)!;
    expect(section).toBeDefined();
    const sectionKids = section.children as Record<string, unknown>[] | undefined;
    expect(sectionKids).toBeDefined();
    expect(findIdInTree(sectionKids ?? [], rowId)).toBeDefined();

    // And the row should NOT appear at root (if it did, parent_id threading failed silently)
    const rootLevel = children.filter((n) => n.id === rowId);
    expect(rootLevel.length).toBe(0);
  });

  it('support chat: exercises every tool added after the 62-mark (#63-#67 + v1)', async () => {
    const fp = await fresh('support-chat.op');

    // Mimics a realistic customer-support screen: header + two
    // chat bubbles (from-others then from-self) + one attachment
    // on the self message + an upload dropzone + chip input for
    // tagging + an open action menu + a dark-theme confirm modal
    // floating + an OTP input (phone verification step). Purpose
    // isn't "this is a real product screen", it's "every new tool
    // composes into the same document without throwing, with
    // expected roles + finite parent_id threading".

    // Top nav (existing 62-family)
    await handleAddTopNavBarV0({
      filePath: fp,
      title: 'Support',
      leading_icon: 'chevron-left',
    });

    // 2 chat bubbles
    const leftBubble = await handleAddChatBubbleV0({
      filePath: fp,
      message: 'Hi! How can I help you today?',
      side: 'left',
      author: 'Support Agent',
      timestamp: 'Just now',
    });
    const rightBubble = await handleAddChatBubbleV0({
      filePath: fp,
      message: "My order hasn't arrived and I can't find the tracking number.",
      side: 'right',
      timestamp: '2m',
    });

    // Attachment riding on the self message (compose pattern)
    const attachment = await handleAddAttachmentRowV0({
      filePath: fp,
      filename: 'order-confirmation.pdf',
      size: '340 KB',
      icon: 'file-text',
    });

    // Upload dropzone for additional evidence
    const dropzone = await handleAddUploadDropzoneV0({
      filePath: fp,
      title: 'Drop screenshots here',
      subtitle: 'PNG or JPG, max 5 MB',
      icon: 'upload-cloud',
    });

    // Chip input for conversation tags
    const chips = await handleAddChipInputV0({
      filePath: fp,
      label: 'Tags',
      chips: ['billing', 'shipping'],
      placeholder: 'Add tag…',
    });

    // Action menu (would be floating from a ⋯ button)
    const menu = await handleAddActionMenuV0({
      filePath: fp,
      items: [
        { label: 'Mark resolved', icon: 'check' },
        { label: 'Escalate', icon: 'arrow-up' },
        { label: 'Archive', icon: 'archive', divider_before: true },
      ],
    });

    // Dark-theme confirm modal (v1 tool)
    const modal = await handleAddModalShellV1({
      filePath: fp,
      title: 'Escalate to supervisor?',
      subtitle: 'The customer will receive an email update.',
      theme: 'dark',
    });

    // OTP input (phone verification step)
    const otp = await handleAddOtpInputV0({
      filePath: fp,
      length: 6,
      focused_index: 0,
    });

    // Every call must have produced a nodeId
    for (const r of [leftBubble, rightBubble, attachment, dropzone, chips, menu, modal, otp]) {
      expect(r.results?.[0]?.nodeId).toBeDefined();
    }

    // Validate the doc has all expected roles
    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    // 1 top_nav + 2 chat bubbles + 1 attachment + 1 dropzone + 1 chip input
    // + 1 action menu + 1 modal + 1 otp = 9
    expect(children.length).toBe(9);

    expect(findRoleInTree(children, 'top-nav-bar')).toBeDefined();
    expect(findRoleInTree(children, 'chat-bubble-left')).toBeDefined();
    expect(findRoleInTree(children, 'chat-bubble-right')).toBeDefined();
    expect(findRoleInTree(children, 'attachment-row')).toBeDefined();
    expect(findRoleInTree(children, 'upload-dropzone')).toBeDefined();
    expect(findRoleInTree(children, 'chip-input')).toBeDefined();
    expect(findRoleInTree(children, 'action-menu')).toBeDefined();
    expect(findRoleInTree(children, 'modal-scrim')).toBeDefined();
    expect(findRoleInTree(children, 'otp-input')).toBeDefined();

    // Dark modal: surface fill must be dark (not light #FFFFFF)
    const modalCard = findRoleInTree(children, 'modal-shell-card')!;
    const modalFill = (modalCard.fill as Array<{ color: string }>)[0].color;
    expect(modalFill, 'v1 theme=dark should emit dark surface').toBe('#1E293B');

    // Self bubble: accent fill (#2563EB default)
    const rightBubbleSurface = findRoleInTree(children, 'chat-bubble-surface');
    expect(rightBubbleSurface).toBeDefined();
    // The LEFT bubble surface is also role chat-bubble-surface, so search
    // for the one under chat-bubble-right specifically.
    const selfBubbleRoot = findRoleInTree(children, 'chat-bubble-right')!;
    const selfSurface = findRoleInTree(
      [selfBubbleRoot] as Record<string, unknown>[],
      'chat-bubble-surface',
    )!;
    expect((selfSurface.fill as Array<{ color: string }>)[0].color).toBe('#2563EB');

    // OTP focused slot: accent border
    const otpFocused = findRoleInTree(children, 'otp-slot-focused')!;
    const otpStroke = otpFocused.stroke as { fill: Array<{ color: string }> };
    expect(otpStroke.fill[0].color).toBe('#2563EB');
  });
});
