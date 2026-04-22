import { describe, it, expect } from 'vitest';
import {
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
  buildMetricComparison,
  buildMetricRow,
  buildNavChipRow,
  buildNotificationRow,
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
} from '../element-builders/index.js';

/**
 * Performance floor for every builder. Element tools run inside
 * streaming AI generation — a single screen may emit 5-10 tools,
 * and a complex multi-section generation may emit 40+ across
 * subtasks. Each builder running in >5ms compounds into visible
 * stutter on the canvas.
 *
 * The thresholds here are loose enough to tolerate CI cold-start
 * noise (V8 warm-up, module resolution) but tight enough to flag
 * a regression that makes a builder genuinely expensive — e.g.
 * someone adding a font-metrics round-trip, an async icon lookup,
 * or an N² layout pre-pass.
 *
 * Per-builder budget: 5ms average over 100 invocations (includes
 * V8 optimization settling). Single slow cold-run allowed since
 * first invocation pays JIT cost.
 */

interface PerfCase {
  name: string;
  build: () => ElementTree;
}

const CASES: PerfCase[] = [
  { name: 'divider', build: () => buildDivider({}) },
  { name: 'badge', build: () => buildBadge({ label: 'NEW' }) },
  { name: 'avatar', build: () => buildAvatar({ initial: 'JD' }) },
  { name: 'icon-button', build: () => buildIconButton({ icon: 'search' }) },
  { name: 'icon-label', build: () => buildIconLabel({ icon: 'info', label: 'Hint' }) },
  { name: 'link', build: () => buildLink({ label: 'Learn more' }) },
  { name: 'kbd', build: () => buildKbd({ keys: ['⌘', 'K'] }) },
  { name: 'price', build: () => buildPrice({ amount: '29' }) },
  { name: 'color-swatch', build: () => buildColorSwatch({ color: '#2563EB' }) },
  { name: 'fab', build: () => buildFab({ icon: 'plus' }) },
  { name: 'toast', build: () => buildToast({ message: 'Copied' }) },
  { name: 'heading', build: () => buildHeading({ content: 'Welcome' }) },
  { name: 'body-text', build: () => buildBodyText({ content: 'Body copy.' }) },
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
  { name: 'section-header', build: () => buildSectionHeader({ title: 'Recent' }) },
  { name: 'top-nav-bar', build: () => buildTopNavBar({ title: 'Home' }) },
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
    build: () => buildSegmentedControl({ items: [{ label: 'D' }, { label: 'W', active: true }] }),
  },
  {
    name: 'breadcrumb',
    build: () => buildBreadcrumb({ items: [{ label: 'Home' }, { label: 'Docs' }] }),
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
  {
    name: 'metric-comparison',
    build: () =>
      buildMetricComparison({ label: 'Revenue', value: '$12k', change: '8%', trend: 'up' }),
  },
  {
    name: 'notification-row',
    build: () =>
      buildNotificationRow({
        title: 'New follower',
        body: 'Alice is now following you.',
        timestamp: '2m',
        unread: true,
      }),
  },
  { name: 'chart-bars', build: () => buildChartBars({ values: [4, 7, 3, 9, 5, 8] }) },
  {
    name: 'empty-state',
    build: () =>
      buildEmptyState({
        title: 'No items',
        subtitle: 'Add one to start',
        icon: 'inbox',
        cta_label: 'Create',
      }),
  },
  { name: 'alert', build: () => buildAlert({ message: 'Saved', icon: 'check' }) },
  { name: 'checkbox', build: () => buildCheckbox({ label: 'Accept', checked: true }) },
  { name: 'radio', build: () => buildRadio({ label: 'Small', selected: true }) },
  { name: 'switch', build: () => buildSwitch({ active: true }) },
  { name: 'activity-ring', build: () => buildActivityRing({ center_text: '42%' }) },
  { name: 'progress-bar', build: () => buildProgressBar({ value: 60 }) },
  { name: 'quote-block', build: () => buildQuoteBlock({ quote: 'Stay hungry.', author: 'SJ' }) },
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

const ITERATIONS = 100;
// Per-builder budget in ms, averaged over ITERATIONS runs.
// 5ms is 20x the observed median for mechanical builders; a
// regression to actually-expensive code (e.g. synchronous network
// call, font round-trip) would exceed this by 10-100x.
const BUDGET_MS_AVG = 5;
// Absolute worst single-call threshold (cold JIT invocation).
const BUDGET_MS_COLD = 50;

describe('builder performance — under 5ms average per call', () => {
  for (const c of CASES) {
    it(`${c.name}: avg(${ITERATIONS} runs) < ${BUDGET_MS_AVG}ms`, () => {
      // Warm-up: 5 calls before measurement to let V8 settle
      for (let i = 0; i < 5; i++) c.build();

      const t0 = performance.now();
      for (let i = 0; i < ITERATIONS; i++) c.build();
      const totalMs = performance.now() - t0;
      const avgMs = totalMs / ITERATIONS;

      expect(
        avgMs,
        `${c.name}: avg=${avgMs.toFixed(3)}ms total=${totalMs.toFixed(1)}ms`,
      ).toBeLessThan(BUDGET_MS_AVG);
    });
  }
});

describe('builder performance — cold first call under 50ms', () => {
  // Each case isolated so first-call JIT cost doesn't leak between
  // builders. Still tight enough to catch a truly slow builder.
  for (const c of CASES) {
    it(`${c.name}: first call < ${BUDGET_MS_COLD}ms`, () => {
      const t0 = performance.now();
      c.build();
      const elapsed = performance.now() - t0;
      expect(elapsed, `${c.name}: cold=${elapsed.toFixed(3)}ms`).toBeLessThan(BUDGET_MS_COLD);
    });
  }
});

describe('builder performance — full 42-tool batch under 100ms', () => {
  it('running every builder once takes under 100ms', () => {
    // Simulates a realistic "AI emits a full screen" case where the
    // orchestrator builds 40+ trees in sequence. If this blows past
    // 100ms, the streaming path will visibly stutter.
    const t0 = performance.now();
    for (const c of CASES) c.build();
    const elapsed = performance.now() - t0;

    expect(
      elapsed,
      `full-batch elapsed ${elapsed.toFixed(1)}ms for ${CASES.length} builders`,
    ).toBeLessThan(100);
  });
});
