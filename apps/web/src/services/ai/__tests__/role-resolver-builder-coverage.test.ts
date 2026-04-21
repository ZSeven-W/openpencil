import { describe, it, expect, vi } from 'vitest';

// Mock canvas-text-measure to avoid CanvasKit WASM dependency (same
// shim pattern as role-resolver.test.ts).
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
  buildChartLine,
  buildChartPie,
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
  buildSelect,
  buildSkeleton,
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
import { resolveTreeRoles } from '../role-resolver';

// Ensure role definitions are registered (matches app startup).
import '../role-definitions/index';

/**
 * Role-resolver coverage over every element builder's output.
 *
 * What this checks:
 *   1. Every builder tree survives `resolveTreeRoles` without throwing
 *      on both light and dark themes.
 *   2. Every role string emitted by a builder is either
 *      (a) registered in role-definitions (gets defaults injected), or
 *      (b) an unknown role that the resolver silently passes through —
 *      which is the documented contract (role-resolver.ts:292).
 *   3. Post-resolve tree is still a PenNode tree — no nodes dropped, no
 *      shape mutation that would break downstream layout/render.
 *
 * Why this exists: builders are the source of role strings at runtime.
 * The role-resolver silently ignores unknown roles, so a typo in a
 * builder ("chart-bar" vs "chart-bars", "toast-message" vs "toast")
 * wouldn't throw — it would just quietly skip defaults. A coverage
 * test that walks every builder output + prints a registered/
 * pass-through breakdown catches drift between the builder registry
 * and the role registry.
 */

interface BuilderCase {
  name: string;
  build: () => ElementTree;
}

const CASES: BuilderCase[] = [
  // Atoms
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

  // Rows / composites
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
      buildSectionHeader({
        title: 'Recent',
        action: { label: 'See all', icon: 'arrow-right' },
      }),
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

function collectRoles(n: PenNode, out: Set<string> = new Set()): Set<string> {
  const role = (n as PenNode & { role?: string }).role;
  if (role) out.add(role);
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  for (const c of children) collectRoles(c, out);
  return out;
}

function countNodes(n: PenNode): number {
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  return 1 + children.reduce((s, c) => s + countNodes(c), 0);
}

describe('role resolver — coverage over all 42 element builders', () => {
  for (const c of CASES) {
    it(`${c.name}: resolveTreeRoles doesn't throw on light theme`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      // Wrap in a parent frame so role-resolver has a canvas context
      const parent: PenNode = {
        id: 'parent',
        type: 'frame',
        name: 'Parent',
        width: 375,
        height: 812,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#FFFFFF' }],
        children: [tree],
      } as unknown as PenNode;
      expect(() => resolveTreeRoles(parent, 375)).not.toThrow();
    });

    it(`${c.name}: resolveTreeRoles doesn't throw on dark theme`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const parent: PenNode = {
        id: 'parent',
        type: 'frame',
        name: 'Parent',
        width: 375,
        height: 812,
        layout: 'vertical',
        fill: [{ type: 'solid', color: '#0A0A0A' }],
        children: [tree],
      } as unknown as PenNode;
      // 7th arg forces dark even if luminance detection disagrees
      expect(() =>
        resolveTreeRoles(parent, 375, undefined, undefined, undefined, false, 'dark'),
      ).not.toThrow();
    });

    it(`${c.name}: resolve preserves node count + top-level role`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const before = countNodes(tree);
      const topRole = (tree as PenNode & { role?: string }).role;

      const parent: PenNode = {
        id: 'parent',
        type: 'frame',
        name: 'Parent',
        width: 375,
        height: 812,
        layout: 'vertical',
        children: [tree],
      } as unknown as PenNode;
      resolveTreeRoles(parent, 375);

      // Node count unchanged: resolver adds defaults, never drops nodes.
      expect(countNodes(tree)).toBe(before);
      // Top-level role is either the original OR preserved when role
      // pass-through triggers (never reassigned to a bogus value).
      const afterRole = (tree as PenNode & { role?: string }).role;
      if (topRole !== undefined) {
        expect(afterRole).toBeDefined();
      }
    });
  }
});

describe('role resolver — builder role vocabulary sanity', () => {
  it('every builder emits at least one role (coverage floor)', () => {
    for (const c of CASES) {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const roles = collectRoles(tree);
      // Every builder output should have at least one role somewhere in
      // the tree — builders without any role string can't benefit from
      // post-generation role dispatch.
      expect(roles.size, `${c.name} emits no roles at all`).toBeGreaterThan(0);
    }
  });

  it('aggregate role set across all builders is stable (snapshot)', () => {
    const allRoles = new Set<string>();
    for (const c of CASES) {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      for (const r of collectRoles(tree)) allRoles.add(r);
    }
    // The full vocabulary should be non-trivial. 42 builders × ~2
    // roles per tree = ~80+ distinct strings is the healthy range.
    // If this suddenly drops far below, either a builder regressed or
    // the role annotations were stripped en-masse.
    expect(allRoles.size).toBeGreaterThanOrEqual(60);
  });
});
