import { describe, it, expect, vi } from 'vitest';

vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import {
  buildActionMenu,
  buildActivityRing,
  buildAlert,
  buildAvatar,
  buildBadge,
  buildBodyText,
  buildBottomNav,
  buildBreadcrumb,
  buildCalendarGrid,
  buildCardRow,
  buildCarouselDots,
  buildChartBars,
  buildChartLine,
  buildChartPie,
  buildCheckbox,
  buildChipInput,
  buildComment,
  buildCodeBlock,
  buildColorSwatch,
  buildDatePicker,
  buildDivider,
  buildEmptyChart,
  buildEmptyState,
  buildFab,
  buildFaqItem,
  buildFormField,
  buildHeading,
  buildIconButton,
  buildIconLabel,
  buildImagePlaceholder,
  buildVideoPlaceholder,
  buildModalShell,
  buildModalShellV1,
  buildUploadDropzone,
  buildOtpInput,
  buildKbd,
  buildLink,
  buildListRow,
  buildMetricComparison,
  buildMetricRow,
  buildNavChipRow,
  buildNotificationRow,
  buildPagination,
  buildPrice,
  buildProgressBar,
  buildQuoteBlock,
  buildRadio,
  buildRatingStars,
  buildSearchBar,
  buildSectionHeader,
  buildSegmentedControl,
  buildSelect,
  buildSkeleton,
  buildStatGrid,
  buildStatusBadge,
  buildSpinner,
  buildStepper,
  buildSwitch,
  buildTabs,
  buildTextarea,
  buildTextButton,
  buildTimeline,
  buildToast,
  buildTooltip,
  buildTopNavBar,
} from '@zseven-w/pen-core';
import { ELEMENT_TOOL_NAMES } from '@zseven-w/pen-mcp';
import { ELEMENT_SHIMS, SUPPORTED_EMBEDDED_ELEMENT_TOOLS } from '../element-tool-shims';

/**
 * Bilateral drift guard: the tool name must exist identically in
 * three registries at runtime, and the two executable paths (client
 * shim + server SERVER_BUILDERS, both at apps/web) must produce
 * structurally equivalent trees for the same args.
 *
 *   Registry 1: ELEMENT_SHIMS keys (apps/web client shim)
 *   Registry 2: SUPPORTED_EMBEDDED_ELEMENT_TOOLS (exported list)
 *   Registry 3: ELEMENT_TOOL_NAMES (pen-mcp canonical list)
 *
 *   Executable path A: shim → buildX pen-core
 *   Executable path B: server /api/mcp/exec-tool → buildX pen-core
 *
 * The shim and server BOTH delegate to the same pen-core buildX
 * function. So the parity assertion is really "running buildX
 * directly and running buildX through the shim produce the same
 * structural tree" — which makes the shim a pure delegation layer
 * (plus id stamping + meta-param extraction).
 *
 * If any of these diverge, a real production bug becomes possible:
 *   - Name in pen-mcp but not in ELEMENT_SHIMS → client shim skips,
 *     HTTP fallback fires to Nitro, extra latency per tool call
 *   - Name in ELEMENT_SHIMS but not in pen-mcp → external MCP
 *     clients (Claude Code, Codex) don't know the tool exists
 *   - Shim output differs from buildX direct output → canvas render
 *     differs depending on which path was taken (the A/B nightmare)
 */

interface BuilderCase {
  toolName: string;
  args: Record<string, unknown>;
  build: (args: Record<string, unknown>) => unknown;
}

/**
 * For each of the 42 tools, a canonical test args object + a
 * pointer to the raw buildX function. The `args` are the same
 * shape the shim would receive minus meta-fields (parent_id,
 * pageId, filePath) — those only affect the wrapper, not the build.
 */
const CASES: BuilderCase[] = [
  {
    toolName: 'add_card_row_v0',
    args: { items: [{ title: 'A' }, { title: 'B' }] },
    build: (a) => buildCardRow(a as unknown as Parameters<typeof buildCardRow>[0]),
  },
  {
    toolName: 'add_metric_row_v0',
    args: { items: [{ label: 'X', value: '1' }] },
    build: (a) => buildMetricRow(a as unknown as Parameters<typeof buildMetricRow>[0]),
  },
  {
    toolName: 'add_bottom_nav_v0',
    args: { items: [{ title: 'Home', icon: 'home' }] },
    build: (a) => buildBottomNav(a as unknown as Parameters<typeof buildBottomNav>[0]),
  },
  {
    toolName: 'add_section_header_v0',
    args: { title: 'Recent' },
    build: (a) => buildSectionHeader(a as unknown as Parameters<typeof buildSectionHeader>[0]),
  },
  {
    toolName: 'add_top_nav_bar_v0',
    args: { title: 'Home' },
    build: (a) => buildTopNavBar(a as unknown as Parameters<typeof buildTopNavBar>[0]),
  },
  {
    toolName: 'add_heading_v0',
    args: { content: 'Title' },
    build: (a) => buildHeading(a as unknown as Parameters<typeof buildHeading>[0]),
  },
  {
    toolName: 'add_body_text_v0',
    args: { content: 'Body text' },
    build: (a) => buildBodyText(a as unknown as Parameters<typeof buildBodyText>[0]),
  },
  {
    toolName: 'add_text_button_v0',
    args: { label: 'Go' },
    build: (a) => buildTextButton(a as unknown as Parameters<typeof buildTextButton>[0]),
  },
  {
    toolName: 'add_search_bar_v0',
    args: {},
    build: (a) => buildSearchBar(a as unknown as Parameters<typeof buildSearchBar>[0]),
  },
  {
    toolName: 'add_list_row_v0',
    args: { title: 'Item' },
    build: (a) => buildListRow(a as unknown as Parameters<typeof buildListRow>[0]),
  },
  {
    toolName: 'add_divider_v0',
    args: {},
    build: (a) => buildDivider(a as unknown as Parameters<typeof buildDivider>[0]),
  },
  {
    toolName: 'add_badge_v0',
    args: { label: 'NEW' },
    build: (a) => buildBadge(a as unknown as Parameters<typeof buildBadge>[0]),
  },
  {
    toolName: 'add_avatar_v0',
    args: { initial: 'JD' },
    build: (a) => buildAvatar(a as unknown as Parameters<typeof buildAvatar>[0]),
  },
  {
    toolName: 'add_icon_button_v0',
    args: { icon: 'search' },
    build: (a) => buildIconButton(a as unknown as Parameters<typeof buildIconButton>[0]),
  },
  {
    toolName: 'add_icon_label_v0',
    args: { icon: 'info', label: 'Info' },
    build: (a) => buildIconLabel(a as unknown as Parameters<typeof buildIconLabel>[0]),
  },
  {
    toolName: 'add_stat_grid_v0',
    args: { items: [{ value: '1', label: 'A' }] },
    build: (a) => buildStatGrid(a as unknown as Parameters<typeof buildStatGrid>[0]),
  },
  {
    toolName: 'add_switch_v0',
    args: { active: true },
    build: (a) => buildSwitch(a as unknown as Parameters<typeof buildSwitch>[0]),
  },
  {
    toolName: 'add_checkbox_v0',
    args: { label: 'A', checked: true },
    build: (a) => buildCheckbox(a as unknown as Parameters<typeof buildCheckbox>[0]),
  },
  {
    toolName: 'add_radio_v0',
    args: { label: 'A', selected: true },
    build: (a) => buildRadio(a as unknown as Parameters<typeof buildRadio>[0]),
  },
  {
    toolName: 'add_tabs_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) => buildTabs(a as unknown as Parameters<typeof buildTabs>[0]),
  },
  {
    toolName: 'add_segmented_control_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) =>
      buildSegmentedControl(a as unknown as Parameters<typeof buildSegmentedControl>[0]),
  },
  {
    toolName: 'add_empty_state_v0',
    args: { title: 'Empty' },
    build: (a) => buildEmptyState(a as unknown as Parameters<typeof buildEmptyState>[0]),
  },
  {
    toolName: 'add_alert_v0',
    args: { message: 'Saved' },
    build: (a) => buildAlert(a as unknown as Parameters<typeof buildAlert>[0]),
  },
  {
    toolName: 'add_toast_v0',
    args: { message: 'Copied' },
    build: (a) => buildToast(a as unknown as Parameters<typeof buildToast>[0]),
  },
  {
    toolName: 'add_progress_bar_v0',
    args: { value: 60 },
    build: (a) => buildProgressBar(a as unknown as Parameters<typeof buildProgressBar>[0]),
  },
  {
    toolName: 'add_fab_v0',
    args: { icon: 'plus' },
    build: (a) => buildFab(a as unknown as Parameters<typeof buildFab>[0]),
  },
  {
    toolName: 'add_breadcrumb_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) => buildBreadcrumb(a as unknown as Parameters<typeof buildBreadcrumb>[0]),
  },
  {
    toolName: 'add_stepper_v0',
    args: { total: 3, current: 1 },
    build: (a) => buildStepper(a as unknown as Parameters<typeof buildStepper>[0]),
  },
  {
    toolName: 'add_textarea_v0',
    args: { label: 'Bio', placeholder: 'Tell us about yourself', rows: 5 },
    build: (a) => buildTextarea(a as unknown as Parameters<typeof buildTextarea>[0]),
  },
  {
    toolName: 'add_skeleton_v0',
    args: { rows: 3 },
    build: (a) => buildSkeleton(a as unknown as Parameters<typeof buildSkeleton>[0]),
  },
  {
    toolName: 'add_select_v0',
    args: { label: 'Country', value: 'US' },
    build: (a) => buildSelect(a as unknown as Parameters<typeof buildSelect>[0]),
  },
  {
    toolName: 'add_chart_line_v0',
    args: { values: [1, 3, 2, 5, 4, 6] },
    build: (a) => buildChartLine(a as unknown as Parameters<typeof buildChartLine>[0]),
  },
  {
    toolName: 'add_chart_pie_v0',
    args: { values: [40, 30, 20, 10] },
    build: (a) => buildChartPie(a as unknown as Parameters<typeof buildChartPie>[0]),
  },
  {
    toolName: 'add_image_placeholder_v0',
    args: { width: 200, height: 140, label: 'Upload' },
    build: (a) =>
      buildImagePlaceholder(a as unknown as Parameters<typeof buildImagePlaceholder>[0]),
  },
  {
    toolName: 'add_video_placeholder_v0',
    args: { width: 320, height: 180, label: 'Coming soon' },
    build: (a) =>
      buildVideoPlaceholder(a as unknown as Parameters<typeof buildVideoPlaceholder>[0]),
  },
  {
    toolName: 'add_comment_v0',
    args: { author: 'Alice', body: 'Nice!', avatar_initial: 'A', timestamp: '2h ago' },
    build: (a) => buildComment(a as unknown as Parameters<typeof buildComment>[0]),
  },
  {
    toolName: 'add_modal_shell_v0',
    args: { title: 'Confirm', subtitle: 'Are you sure?' },
    build: (a) => buildModalShell(a as unknown as Parameters<typeof buildModalShell>[0]),
  },
  {
    toolName: 'add_status_badge_v0',
    args: { label: 'Online', tone: 'success' },
    build: (a) => buildStatusBadge(a as unknown as Parameters<typeof buildStatusBadge>[0]),
  },
  {
    toolName: 'add_spinner_v0',
    args: {},
    build: (a) => buildSpinner(a as unknown as Parameters<typeof buildSpinner>[0]),
  },
  {
    toolName: 'add_tooltip_v0',
    args: { text: 'Help' },
    build: (a) => buildTooltip(a as unknown as Parameters<typeof buildTooltip>[0]),
  },
  {
    toolName: 'add_metric_comparison_v0',
    args: { label: 'Revenue', value: '$12k', change: '8%', trend: 'up' },
    build: (a) =>
      buildMetricComparison(a as unknown as Parameters<typeof buildMetricComparison>[0]),
  },
  {
    toolName: 'add_notification_row_v0',
    args: { title: 'New follower', body: 'Alice is following you.', timestamp: '2m', unread: true },
    build: (a) => buildNotificationRow(a as unknown as Parameters<typeof buildNotificationRow>[0]),
  },
  {
    toolName: 'add_form_field_v0',
    args: { label: 'Email' },
    build: (a) => buildFormField(a as unknown as Parameters<typeof buildFormField>[0]),
  },
  {
    toolName: 'add_nav_chip_row_v0',
    args: { items: [{ label: 'A' }] },
    build: (a) => buildNavChipRow(a as unknown as Parameters<typeof buildNavChipRow>[0]),
  },
  {
    toolName: 'add_activity_ring_v0',
    args: { center_text: '50%' },
    build: (a) => buildActivityRing(a as unknown as Parameters<typeof buildActivityRing>[0]),
  },
  {
    toolName: 'add_rating_stars_v0',
    args: { filled: 4 },
    build: (a) => buildRatingStars(a as unknown as Parameters<typeof buildRatingStars>[0]),
  },
  {
    toolName: 'add_carousel_dots_v0',
    args: { total: 5, current: 2 },
    build: (a) => buildCarouselDots(a as unknown as Parameters<typeof buildCarouselDots>[0]),
  },
  {
    toolName: 'add_link_v0',
    args: { label: 'Read more' },
    build: (a) => buildLink(a as unknown as Parameters<typeof buildLink>[0]),
  },
  {
    toolName: 'add_kbd_v0',
    args: { keys: ['⌘', 'K'] },
    build: (a) => buildKbd(a as unknown as Parameters<typeof buildKbd>[0]),
  },
  {
    toolName: 'add_price_v0',
    args: { amount: '29' },
    build: (a) => buildPrice(a as unknown as Parameters<typeof buildPrice>[0]),
  },
  {
    toolName: 'add_quote_block_v0',
    args: { quote: 'Stay hungry.' },
    build: (a) => buildQuoteBlock(a as unknown as Parameters<typeof buildQuoteBlock>[0]),
  },
  {
    toolName: 'add_code_block_v0',
    args: { code: 'const x = 1;' },
    build: (a) => buildCodeBlock(a as unknown as Parameters<typeof buildCodeBlock>[0]),
  },
  {
    toolName: 'add_color_swatch_v0',
    args: { color: '#2563EB' },
    build: (a) => buildColorSwatch(a as unknown as Parameters<typeof buildColorSwatch>[0]),
  },
  {
    toolName: 'add_chart_bars_v0',
    args: { values: [1, 2, 3] },
    build: (a) => buildChartBars(a as unknown as Parameters<typeof buildChartBars>[0]),
  },
  {
    toolName: 'add_timeline_v0',
    args: { items: [{ title: 'A' }, { title: 'B' }] },
    build: (a) => buildTimeline(a as unknown as Parameters<typeof buildTimeline>[0]),
  },
  {
    toolName: 'add_calendar_grid_v0',
    args: {},
    build: (a) => buildCalendarGrid(a as unknown as Parameters<typeof buildCalendarGrid>[0]),
  },
  {
    toolName: 'add_pagination_v0',
    args: { total: 10, current: 5 },
    build: (a) => buildPagination(a as unknown as Parameters<typeof buildPagination>[0]),
  },
  {
    toolName: 'add_faq_item_v0',
    args: { question: 'How do I cancel?', answer: 'Email support.', expanded: true },
    build: (a) => buildFaqItem(a as unknown as Parameters<typeof buildFaqItem>[0]),
  },
  {
    toolName: 'add_chip_input_v0',
    args: { label: 'Tags', chips: ['design', 'mobile'], placeholder: 'Add tag…' },
    build: (a) => buildChipInput(a as unknown as Parameters<typeof buildChipInput>[0]),
  },
  {
    toolName: 'add_empty_chart_v0',
    args: { width: 320, height: 200, icon: 'line-chart', title: 'No data yet' },
    build: (a) => buildEmptyChart(a as unknown as Parameters<typeof buildEmptyChart>[0]),
  },
  {
    toolName: 'add_action_menu_v0',
    args: {
      items: [
        { label: 'Edit', icon: 'pencil' },
        { label: 'Share', icon: 'share' },
        { label: 'Delete', icon: 'trash', destructive: true, divider_before: true },
      ],
    },
    build: (a) => buildActionMenu(a as unknown as Parameters<typeof buildActionMenu>[0]),
  },
  {
    toolName: 'add_date_picker_v0',
    args: { label: 'Due date', value: 'Jan 15, 2026', clearable: true },
    build: (a) => buildDatePicker(a as unknown as Parameters<typeof buildDatePicker>[0]),
  },
  {
    toolName: 'add_modal_shell_v1',
    args: { title: 'Delete item?', subtitle: 'This cannot be undone.', theme: 'dark' },
    build: (a) => buildModalShellV1(a as unknown as Parameters<typeof buildModalShellV1>[0]),
  },
  {
    toolName: 'add_upload_dropzone_v0',
    args: { width: 480, title: 'Drop PDFs here', subtitle: 'Max 10 MB', icon: 'upload' },
    build: (a) => buildUploadDropzone(a as unknown as Parameters<typeof buildUploadDropzone>[0]),
  },
  {
    toolName: 'add_otp_input_v0',
    args: { length: 6, digits: ['1', '2', '3'], focused_index: 3 },
    build: (a) => buildOtpInput(a as unknown as Parameters<typeof buildOtpInput>[0]),
  },
];

/**
 * Strip ids recursively — shims call `assignIdsRecursively` which
 * stamps random nanoids; direct buildX output has no ids yet. We
 * compare structurally on everything BUT the id field.
 */
function stripIds(n: unknown): unknown {
  if (Array.isArray(n)) return n.map(stripIds);
  if (n && typeof n === 'object') {
    const out: Record<string, unknown> = {};
    for (const [k, v] of Object.entries(n as Record<string, unknown>)) {
      if (k === 'id') continue;
      out[k] = stripIds(v);
    }
    return out;
  }
  return n;
}

describe('registry parity — ELEMENT_SHIMS ⇄ ELEMENT_TOOL_NAMES ⇄ CASES', () => {
  it('CASES covers every ELEMENT_SHIMS key (no missing test)', () => {
    const caseNames = new Set(CASES.map((c) => c.toolName));
    const shimNames = Object.keys(ELEMENT_SHIMS);
    const missing = shimNames.filter((n) => !caseNames.has(n));
    expect(missing).toEqual([]);
  });

  it('CASES covers every SUPPORTED_EMBEDDED_ELEMENT_TOOLS entry', () => {
    const caseNames = new Set(CASES.map((c) => c.toolName));
    const missing = [...SUPPORTED_EMBEDDED_ELEMENT_TOOLS].filter((n) => !caseNames.has(n));
    expect(missing).toEqual([]);
  });

  it('SUPPORTED_EMBEDDED_ELEMENT_TOOLS is subset of ELEMENT_TOOL_NAMES (pen-mcp canonical)', () => {
    const canonical = new Set(ELEMENT_TOOL_NAMES);
    const missing = [...SUPPORTED_EMBEDDED_ELEMENT_TOOLS].filter((n) => !canonical.has(n));
    if (missing.length > 0) {
      throw new Error(
        `Shim registry names not present in pen-mcp canonical list: ${missing.join(', ')}. ` +
          `Either remove from shims or add the tool to pen-mcp.`,
      );
    }
    expect(missing).toEqual([]);
  });

  it('No duplicate entries in ELEMENT_SHIMS keys', () => {
    const keys = Object.keys(ELEMENT_SHIMS);
    expect(new Set(keys).size).toBe(keys.length);
  });
});

describe('structural parity — shim output === buildX direct output', () => {
  for (const c of CASES) {
    it(`${c.toolName}: shim(args) === buildX(args) (ids aside)`, () => {
      const shim = ELEMENT_SHIMS[c.toolName];
      expect(shim, `shim for ${c.toolName}`).toBeDefined();

      // Shim path
      const shimResult = shim(c.args);
      const shimTree = stripIds(shimResult.node);

      // Direct buildX path (same args the shim would strip meta from
      // — but `args` has no meta fields, so it's identical to what
      // the shim would pass to the builder)
      const directTree = stripIds(c.build(c.args));

      expect(shimTree).toEqual(directTree);
    });
  }
});

describe('shim meta-param extraction', () => {
  it('parent_id extracted before builder invocation, not passed to buildX', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const result = shim({ content: 'Hello', parent_id: 'some-parent-id' });
    expect(result.parentId).toBe('some-parent-id');
    // Verify the heading node itself has no spurious parent_id leak
    expect((result.node as unknown as Record<string, unknown>).parent_id).toBeUndefined();
  });

  it('pageId extracted, defaults to null when absent', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const a = shim({ content: 'X', pageId: 'page-1' });
    expect(a.pageId).toBe('page-1');
    const b = shim({ content: 'Y' });
    expect(b.pageId).toBeNull();
  });

  it('filePath extracted, live://canvas sentinel normalized to null', () => {
    const shim = ELEMENT_SHIMS['add_heading_v0'];
    const a = shim({ content: 'X', filePath: 'live://canvas' });
    expect(a.filePath).toBeNull();
    const b = shim({ content: 'Y', filePath: '/tmp/real.pen' });
    expect(b.filePath).toBe('/tmp/real.pen');
  });
});
