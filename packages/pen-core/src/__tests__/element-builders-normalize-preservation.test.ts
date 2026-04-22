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
} from '../element-builders/index.js';
import { normalizePenDocument } from '../normalize.js';

/**
 * normalizePenDocument preservation gate for every builder output.
 *
 * `normalizePenDocument` is format-only normalization (type field
 * renames, fill shorthand expansion, gradient stop offset migration,
 * sizing keyword fallback). Builders already produce canonical
 * format output — running normalize over builder trees should be a
 * no-op EXCEPT for incidental already-canonical normalizations.
 *
 * We verify THREE things per builder:
 *   1. Critical semantic fields survive unchanged: role, type,
 *      content, layout, children count at every level. These drive
 *      renderer/layout/role-resolver downstream; a loss would be
 *      silent breakage.
 *   2. Tree node count is preserved (no nodes dropped).
 *   3. Running normalize TWICE produces the same output as once
 *      (idempotency — same property as post-processing but for the
 *      format-normalization layer).
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

function wrapInDoc(tree: PenNode): PenDocument {
  const rootFrame: PenNode = {
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
  return {
    name: 'test',
    version: '1.0.0',
    children: [rootFrame],
  } as unknown as PenDocument;
}

function countNodes(n: PenNode): number {
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  return 1 + children.reduce((s, c) => s + countNodes(c), 0);
}

function collectSemanticFields(
  n: PenNode,
  path = 'root',
  out: Array<{ path: string; type: string; role?: string; content?: string; layout?: string }> = [],
): typeof out {
  const node = n as PenNode & { role?: string; content?: string; layout?: string };
  out.push({
    path,
    type: node.type,
    role: node.role,
    content: node.content,
    layout: node.layout,
  });
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  children.forEach((c, i) => collectSemanticFields(c, `${path}/${i}`, out));
  return out;
}

describe('normalizePenDocument — semantic preservation over builder trees', () => {
  for (const c of CASES) {
    it(`${c.name}: role / type / content / layout preserved`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);

      const docBefore = wrapInDoc(tree);
      const before = collectSemanticFields(docBefore.children[0]);
      const nodeCountBefore = countNodes(docBefore.children[0]);

      const docAfter = normalizePenDocument(docBefore);
      const after = collectSemanticFields(docAfter.children[0]);
      const nodeCountAfter = countNodes(docAfter.children[0]);

      // Same node count — no nodes created or dropped
      expect(nodeCountAfter).toBe(nodeCountBefore);
      // Same semantic-field signature at every position in the tree
      expect(after).toEqual(before);
    });
  }
});

describe('normalizePenDocument — idempotency over builder trees', () => {
  for (const c of CASES) {
    it(`${c.name}: normalize(x) === normalize(normalize(x))`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const doc = wrapInDoc(tree);

      const once = normalizePenDocument(doc);
      const twice = normalizePenDocument(once);
      expect(twice).toEqual(once);
    });
  }
});
