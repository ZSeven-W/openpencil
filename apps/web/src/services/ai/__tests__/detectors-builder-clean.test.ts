import { describe, it, expect } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import {
  assignIdsRecursively,
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
  buildComment,
  buildCodeBlock,
  buildColorSwatch,
  buildDivider,
  buildEmptyState,
  buildFab,
  buildFormField,
  buildHeading,
  buildIconButton,
  buildIconLabel,
  buildImagePlaceholder,
  buildModalShell,
  buildKbd,
  buildLink,
  buildListRow,
  buildMetricRow,
  buildNavChipRow,
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
  type ElementTree,
} from '@zseven-w/pen-core';
import { detectAllIssues } from '@zseven-w/pen-ai-skills';

/**
 * Drift guard: our builders should never emit a tree that trips a
 * pre-validation detector at `warning` or `error` severity. If any
 * detector fires on a clean builder tree, one of two things is true:
 *
 *   a) The builder has regressed into producing a problematic shape
 *      (e.g. explicit text height, empty path, invisible container).
 *      Fix the builder.
 *
 *   b) The detector has a false positive on a valid builder output.
 *      Tighten the detector so builder output is excluded.
 *
 * Either way, this test is the tripwire. `info` severity is allowed
 * because those are detect-only diagnostics (the pre-validation
 * pipeline explicitly skips them for auto-fix), and triggering one
 * doesn't mean the builder is wrong.
 */

interface BuilderCase {
  name: string;
  build: () => ElementTree;
}

const CASES: BuilderCase[] = [
  { name: 'divider', build: () => buildDivider({}) },
  { name: 'badge', build: () => buildBadge({ label: 'NEW' }) },
  { name: 'avatar', build: () => buildAvatar({ initial: 'JD' }) },
  { name: 'icon-button', build: () => buildIconButton({ icon: 'search' }) },
  { name: 'icon-label', build: () => buildIconLabel({ icon: 'info', label: 'Hint' }) },
  { name: 'link', build: () => buildLink({ label: 'Learn more', trailing_icon: 'arrow-right' }) },
  { name: 'kbd', build: () => buildKbd({ keys: ['⌘', 'K'] }) },
  { name: 'price', build: () => buildPrice({ amount: '29', period: '/month' }) },
  { name: 'color-swatch', build: () => buildColorSwatch({ color: '#2563EB', label: 'Primary' }) },
  { name: 'fab', build: () => buildFab({ icon: 'plus' }) },
  { name: 'toast', build: () => buildToast({ message: 'Copied', icon: 'check' }) },
  { name: 'heading', build: () => buildHeading({ content: 'Welcome' }) },
  { name: 'body-text', build: () => buildBodyText({ content: 'Lorem ipsum.' }) },
  { name: 'text-button', build: () => buildTextButton({ label: 'Sign in' }) },
  {
    name: 'card-row',
    build: () => buildCardRow({ items: [{ title: 'A' }, { title: 'B' }, { title: 'C' }] }),
  },
  {
    name: 'metric-row',
    build: () =>
      buildMetricRow({
        items: [
          { label: 'Steps', value: '8,432' },
          { label: 'Kcal', value: '512' },
        ],
      }),
  },
  {
    name: 'nav-chip-row',
    build: () => buildNavChipRow({ items: [{ label: 'All', active: true }, { label: 'Videos' }] }),
  },
  {
    name: 'bottom-nav',
    build: () =>
      buildBottomNav({
        items: [
          { title: 'Home', icon: 'home', active: true },
          { title: 'Search', icon: 'search' },
          { title: 'Profile', icon: 'user' },
        ],
      }),
  },
  {
    name: 'section-header',
    build: () =>
      buildSectionHeader({ title: 'Recent', action: { label: 'See all', icon: 'arrow-right' } }),
  },
  {
    name: 'top-nav-bar',
    build: () =>
      buildTopNavBar({
        title: 'Settings',
        leading_icon: 'chevron-left',
        trailing_icon: 'more-vertical',
      }),
  },
  {
    name: 'stat-grid',
    build: () =>
      buildStatGrid({
        items: [
          { value: '1', label: 'A' },
          { value: '2', label: 'B' },
          { value: '3', label: 'C' },
        ],
      }),
  },
  {
    name: 'tabs',
    build: () => buildTabs({ items: [{ label: 'A', active: true }, { label: 'B' }] }),
  },
  {
    name: 'segmented-control',
    build: () =>
      buildSegmentedControl({
        items: [{ label: 'Day' }, { label: 'Week', active: true }, { label: 'Month' }],
      }),
  },
  {
    name: 'breadcrumb',
    build: () => buildBreadcrumb({ items: [{ label: 'Home' }, { label: 'Settings' }] }),
  },
  { name: 'stepper', build: () => buildStepper({ total: 3, current: 1 }) },
  { name: 'rating-stars', build: () => buildRatingStars({ filled: 4 }) },
  { name: 'carousel-dots', build: () => buildCarouselDots({ total: 5, current: 2 }) },
  {
    name: 'list-row',
    build: () => buildListRow({ title: 'Notifications', subtitle: 'Push, email' }),
  },
  { name: 'search-bar', build: () => buildSearchBar({}) },
  { name: 'form-field', build: () => buildFormField({ label: 'Email' }) },
  { name: 'textarea', build: () => buildTextarea({ label: 'Bio', rows: 5 }) },
  { name: 'skeleton', build: () => buildSkeleton({ rows: 3 }) },
  { name: 'select', build: () => buildSelect({ label: 'Country', value: 'US' }) },
  { name: 'chart-line', build: () => buildChartLine({ values: [1, 3, 2, 5, 4, 6] }) },
  { name: 'chart-pie', build: () => buildChartPie({ values: [40, 30, 20, 10] }) },
  { name: 'image-placeholder', build: () => buildImagePlaceholder({}) },
  {
    name: 'comment',
    build: () => buildComment({ author: 'Alice', body: 'Great post!', avatar_initial: 'A' }),
  },
  { name: 'modal-shell', build: () => buildModalShell({ title: 'Confirm' }) },
  { name: 'status-badge', build: () => buildStatusBadge({ label: 'Online', tone: 'success' }) },
  { name: 'spinner', build: () => buildSpinner({}) },
  { name: 'tooltip', build: () => buildTooltip({ text: 'Help' }) },
  { name: 'chart-bars', build: () => buildChartBars({ values: [4, 7, 3, 9, 5] }) },
  {
    name: 'empty-state',
    build: () =>
      buildEmptyState({
        title: 'No items',
        subtitle: 'Add one to get started',
        icon: 'inbox',
        cta_label: 'Create new',
      }),
  },
  { name: 'alert', build: () => buildAlert({ message: 'Saved', icon: 'check' }) },
  { name: 'checkbox', build: () => buildCheckbox({ label: 'Accept', checked: true }) },
  { name: 'radio', build: () => buildRadio({ label: 'Small', selected: true }) },
  { name: 'switch', build: () => buildSwitch({ active: true }) },
  { name: 'activity-ring', build: () => buildActivityRing({ center_text: '42%' }) },
  { name: 'progress-bar', build: () => buildProgressBar({ value: 60 }) },
  {
    name: 'quote-block',
    build: () => buildQuoteBlock({ quote: 'Stay hungry.', author: 'SJ' }),
  },
  { name: 'code-block', build: () => buildCodeBlock({ code: 'const x = 1;' }) },
  {
    name: 'timeline',
    build: () =>
      buildTimeline({
        items: [
          { title: 'Ordered', subtitle: '10:42 AM', active: true },
          { title: 'Preparing' },
          { title: 'Shipped' },
        ],
      }),
  },
  { name: 'calendar-grid', build: () => buildCalendarGrid({}) },
];

function wrapInDoc(tree: PenNode): { root: PenNode; doc: PenDocument } {
  const root: PenNode = {
    id: 'root',
    type: 'frame',
    name: 'Page',
    x: 0,
    y: 0,
    width: 375,
    height: 812,
    layout: 'vertical',
    children: [tree],
  } as unknown as PenNode;
  const doc = {
    name: 'test',
    children: [root],
  } as unknown as PenDocument;
  return { root, doc };
}

describe('detectors — 42 builders are pre-validation clean', () => {
  for (const c of CASES) {
    it(`${c.name}: no warning/error severity issues`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const { root, doc } = wrapInDoc(tree);

      const issues = detectAllIssues(root, doc);

      // Filter to actionable severities — `info` is detect-only and
      // skipped by the auto-fix pipeline, so triggering one is not a
      // regression.
      const actionable = issues.filter((i) => i.severity !== 'info');

      if (actionable.length > 0) {
        const summary = actionable
          .map((i) => `  - [${i.severity}] ${i.nodeId} ${i.property}: ${i.reason}`)
          .join('\n');
        throw new Error(
          `Builder "${c.name}" tripped ${actionable.length} actionable detector ` +
            `issue(s) — either fix the builder or tighten the detector:\n${summary}`,
        );
      }

      // Passing: zero actionable issues. Info-severity is tolerated.
      expect(actionable).toHaveLength(0);
    });
  }
});

describe('detectors — negative cases (sanity: detectors still fire)', () => {
  // Sanity: confirm the detectors actually fire on obviously bad input.
  // If these pass with zero issues, the detectors are broken and the
  // positive tests above would give false confidence.

  it('text with explicit pixel height → detector fires', () => {
    const badTree: PenNode = {
      id: 'bad-root',
      type: 'frame',
      name: 'root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      layout: 'vertical',
      children: [
        {
          id: 'bad-text',
          type: 'text',
          name: 'Bad text',
          content: 'Hello',
          fontSize: 16,
          height: 48, // explicit pixel height → detector should flag
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    const doc = { name: 'bad', children: [badTree] } as unknown as PenDocument;
    const issues = detectAllIssues(badTree, doc);
    const heightIssue = issues.find(
      (i) => i.nodeId === 'bad-text' && i.property === 'height' && i.severity !== 'info',
    );
    expect(heightIssue).toBeDefined();
  });

  it('same-fill-as-parent container with children → invisible-container detector fires', () => {
    // The invisible-container detector fires when a frame has: same
    // fill as parent, no stroke, layout set, and at least one child.
    const invisibleChild: PenNode = {
      id: 'invisible-child',
      type: 'frame',
      name: 'invisible',
      x: 0,
      y: 0,
      width: 300,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      layout: 'vertical',
      children: [
        {
          id: 'some-text',
          type: 'text',
          name: 'txt',
          content: 'Text',
          fontSize: 14,
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    const rootFrame: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Page',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      layout: 'vertical',
      fill: [{ type: 'solid', color: '#FFFFFF' }], // same as child
      children: [invisibleChild],
    } as unknown as PenNode;

    const doc = { name: 'bad-doc', children: [rootFrame] } as unknown as PenDocument;
    const issues = detectAllIssues(rootFrame, doc);
    const invisibleIssue = issues.find(
      (i) => i.nodeId === 'invisible-child' && i.category === 'invisible-container',
    );
    expect(invisibleIssue).toBeDefined();
  });
});
