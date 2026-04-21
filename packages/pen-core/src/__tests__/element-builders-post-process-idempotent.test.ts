import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
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
  buildCheckbox,
  buildCodeBlock,
  buildColorSwatch,
  buildDivider,
  buildEmptyState,
  buildFab,
  buildFormField,
  buildHeading,
  buildIconButton,
  buildIconLabel,
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
  buildStatGrid,
  buildStepper,
  buildSwitch,
  buildTabs,
  buildTextarea,
  buildTextButton,
  buildTimeline,
  buildToast,
  buildTopNavBar,
  type ElementTree,
} from '../element-builders/index.js';
import { normalizeTreeLayout } from '../layout/normalize-tree.js';
import { unwrapFakePhoneMockups } from '../layout/unwrap-fake-phone-mockup.js';
import { stripRedundantSectionFills } from '../layout/strip-redundant-section-fills.js';
import { normalizeStrokeFillSchema } from '../normalize/normalize-stroke-fill-schema.js';

/**
 * Idempotency gate for every post-processing pass over every builder.
 *
 * Why idempotency matters: post-processing runs on the AI's raw tree
 * before insertion. When a user later edits the doc and the pipeline
 * re-runs (e.g. on undo/redo replay, or when a second AI round
 * operates on an already-normalized subtree), running each pass a
 * SECOND time must be a no-op — otherwise the tree keeps shifting
 * shape silently and undo/redo diverges from what the user sees.
 *
 * Passes under test:
 *   - normalizeTreeLayout           — infers layout on child-bearing frames
 *   - unwrapFakePhoneMockups        — repairs AI-generated fake phone wraps
 *   - stripRedundantSectionFills    — removes redundant dark fills
 *   - normalizeStrokeFillSchema     — fixes stroke/fill schema violations
 *
 * Each builder × each pass: run twice and assert deep-equal. If a pass
 * isn't idempotent on a specific builder shape, this test fails on
 * that row and points at the culprit directly.
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

/**
 * Deep clone via structuredClone — avoids referential shared state so
 * mutations in one pass call don't leak into the next.
 */
function snap(n: PenNode): unknown {
  return structuredClone(n);
}

function freshTree(c: BuilderCase): PenNode {
  const tree = c.build() as unknown as PenNode;
  assignIdsRecursively(tree as unknown as ElementTree);
  return tree;
}

/**
 * Wrap the builder tree in a page-root frame. Some passes (like
 * stripRedundantSectionFills) only behave meaningfully inside a root
 * container; passing a bare builder tree would skip logic we actually
 * want to exercise.
 */
function wrapInRoot(tree: PenNode): PenNode {
  return {
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
}

describe('post-processing idempotency — normalizeTreeLayout', () => {
  for (const c of CASES) {
    it(`${c.name}: normalizeTreeLayout(x); normalizeTreeLayout(x) === normalizeTreeLayout(x)`, () => {
      const tree = wrapInRoot(freshTree(c));
      normalizeTreeLayout(tree);
      const once = snap(tree);
      normalizeTreeLayout(tree);
      const twice = snap(tree);
      expect(twice).toEqual(once);
    });
  }
});

describe('post-processing idempotency — unwrapFakePhoneMockups', () => {
  for (const c of CASES) {
    it(`${c.name}: second unwrap pass is a no-op`, () => {
      const tree = wrapInRoot(freshTree(c));
      unwrapFakePhoneMockups(tree);
      const once = snap(tree);
      const changed = unwrapFakePhoneMockups(tree);
      const twice = snap(tree);
      expect(twice).toEqual(once);
      // Second invocation should also return false (no change made)
      expect(changed).toBe(false);
    });
  }
});

describe('post-processing idempotency — stripRedundantSectionFills', () => {
  for (const c of CASES) {
    it(`${c.name}: second strip pass is a no-op`, () => {
      const tree = wrapInRoot(freshTree(c));
      stripRedundantSectionFills(tree);
      const once = snap(tree);
      const changed = stripRedundantSectionFills(tree);
      const twice = snap(tree);
      expect(twice).toEqual(once);
      expect(changed).toBe(false);
    });
  }
});

describe('post-processing idempotency — normalizeStrokeFillSchema', () => {
  for (const c of CASES) {
    it(`${c.name}: normalizeStrokeFillSchema is idempotent`, () => {
      const tree = wrapInRoot(freshTree(c));
      normalizeStrokeFillSchema(tree);
      const once = snap(tree);
      normalizeStrokeFillSchema(tree);
      const twice = snap(tree);
      expect(twice).toEqual(once);
    });
  }
});

describe('post-processing idempotency — full chain', () => {
  /**
   * The canonical pipeline order (per post-processing order memory:
   * semantic passes before structural fallback passes). Running the
   * full chain twice must be fixed-point — the single strongest
   * guarantee because it catches cross-pass interactions that don't
   * show up when passes are tested in isolation.
   */
  const runChain = (tree: PenNode): void => {
    // Structural / format passes (no semantic interpretation yet)
    normalizeStrokeFillSchema(tree);
    stripRedundantSectionFills(tree);
    unwrapFakePhoneMockups(tree);
    // Layout fallback last — this is the "last safety net" per
    // normalize-tree.ts docstring. Any pass above that writes layout
    // semantically should run first.
    normalizeTreeLayout(tree);
  };

  for (const c of CASES) {
    it(`${c.name}: full chain × 2 === full chain × 1`, () => {
      const tree = wrapInRoot(freshTree(c));
      runChain(tree);
      const once = snap(tree);
      runChain(tree);
      const twice = snap(tree);
      expect(twice).toEqual(once);
    });
  }
});
