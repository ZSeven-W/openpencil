import { describe, it, expect, vi } from 'vitest';

// Mock canvas-text-measure to avoid CanvasKit WASM import.
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import type { PenNode } from '@zseven-w/pen-types';
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
  assignIdsRecursively,
  type ElementTree,
} from '@zseven-w/pen-core';
import { rewriteLlmAntiPatterns } from '../sanitize-llm-anti-patterns';

/**
 * Drift guard: builders are our OWN templates — they should never
 * trip an LLM anti-pattern detector. If any detector rewrites a
 * builder's tree, either:
 *   (a) the builder regressed into producing an anti-pattern
 *       (e.g. stacked ellipses for a ring, open-stroke path with
 *       duplicate fill); fix the builder, or
 *   (b) the detector has a false positive on valid builder output;
 *       tighten the detector so it excludes our shapes.
 *
 * Either way, this test is the tripwire. Snapshot-compare before and
 * after `rewriteLlmAntiPatterns` — if they diverge, the test fails
 * on the specific builder + dumps both snapshots for inspection.
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

describe('anti-patterns — 42 builders are clean by construction', () => {
  for (const c of CASES) {
    it(`${c.name}: rewriteLlmAntiPatterns is a no-op on builder output`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const wrapped = wrapInRoot(tree);
      const before = structuredClone(wrapped);

      rewriteLlmAntiPatterns(wrapped);

      // Exact equality: if any sub-detector mutated anything, the
      // snapshot diverges and the test fails on this specific builder
      // with a diff visible in the vitest report.
      expect(wrapped).toEqual(before);
    });
  }
});

describe('anti-patterns — activity-ring specifically (regression anchor)', () => {
  /**
   * The activity ring is the most adversarial case: it's a ring with
   * centered text, which is EXACTLY the "stacked ellipses" anti-pattern
   * template except done right (frame + cornerRadius instead of two
   * ellipses). If the builder accidentally regresses to the ellipse
   * version, the rewriter would fire — we catch it here directly.
   */
  it('builder emits frame+cornerRadius, NOT stacked ellipses', () => {
    const ring = buildActivityRing({ center_text: '42%' }) as unknown as PenNode & {
      type: string;
      cornerRadius?: number;
      children?: PenNode[];
    };
    expect(ring.type).toBe('frame');
    expect(ring.cornerRadius).toBeGreaterThan(0);
    // No ellipse children — the whole point
    const hasEllipse = (ring.children ?? []).some((c) => c.type === 'ellipse');
    expect(hasEllipse).toBe(false);
  });

  it('progress-bar uses rectangle children (not ellipses)', () => {
    // Progress bars are linear — rectangles are the correct primitive.
    // The anti-pattern would be stacking two ellipses to fake a pill
    // shape; rectangle avoids that entirely.
    const bar = buildProgressBar({ value: 60 }) as unknown as PenNode & {
      type: string;
      children?: PenNode[];
    };
    expect(bar.type).toBe('frame');
    const children = bar.children ?? [];
    // No ellipse children — that's the only real anti-pattern here
    const hasEllipse = children.some((c) => c.type === 'ellipse');
    expect(hasEllipse).toBe(false);
  });
});
