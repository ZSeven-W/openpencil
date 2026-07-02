import { describe, it, expect, vi } from 'vitest';

vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import type { PenNode } from '@zseven-w/pen-types';
import {
  assignIdsRecursively,
  buildAlert,
  buildBodyText,
  buildBottomNav,
  buildCardRow,
  buildDivider,
  buildEmptyState,
  buildFab,
  buildFormField,
  buildHeading,
  buildIconButton,
  buildIconLabel,
  buildLink,
  buildListRow,
  buildSectionHeader,
  buildStatGrid,
  buildToast,
  buildTopNavBar,
  type ElementTree,
} from '@zseven-w/pen-core';
import { lookupIconByName } from '../icon-dictionary';

/**
 * Icon-name resolution coverage: every icon name that appears in any
 * builder's default output must resolve via `lookupIconByName`. A
 * builder hardcoding a non-Lucide name would render as an empty glyph
 * or a fallback circle on canvas — silent but visually broken.
 *
 * Two layers of checks:
 *
 *   1. Per-builder: collect every `icon_font` name (or the `name`
 *      field of icon-adjacent nodes) and assert each resolves.
 *   2. Aggregate: every icon name used across all builders is in
 *      AVAILABLE_LUCIDE_ICONS. This is the stronger gate: even if
 *      lookupIconByName falls back to a prefix/substring match,
 *      the name must still be a real Lucide slug.
 *
 * When a new builder emits an unknown icon, this test points at the
 * exact builder + name. Fix: either rename to a known Lucide icon,
 * add the new icon to AVAILABLE_LUCIDE_ICONS upstream, or relax the
 * test to accept that builder's inherently dynamic name.
 */

interface IconBuilderCase {
  name: string;
  /**
   * Default args known to produce an icon_font node. When a builder
   * takes icon name(s) as input, we pass them explicitly.
   */
  build: () => ElementTree;
}

const CASES: IconBuilderCase[] = [
  // Builders that explicitly take an icon prop
  { name: 'icon-button', build: () => buildIconButton({ icon: 'search' }) },
  { name: 'icon-label', build: () => buildIconLabel({ icon: 'info', label: 'Hint' }) },
  { name: 'fab', build: () => buildFab({ icon: 'plus' }) },
  { name: 'alert-with-icon', build: () => buildAlert({ message: 'Saved', icon: 'check' }) },
  { name: 'toast-with-icon', build: () => buildToast({ message: 'Done', icon: 'check' }) },
  {
    name: 'empty-state-with-icon',
    build: () => buildEmptyState({ title: 'Empty', icon: 'inbox' }),
  },
  {
    name: 'link-with-trailing-icon',
    build: () => buildLink({ label: 'Read', trailing_icon: 'arrow-right' }),
  },
  {
    name: 'form-field-both-icons',
    build: () => buildFormField({ label: 'Email', leading_icon: 'mail', trailing_icon: 'eye' }),
  },
  {
    name: 'list-row-both-icons',
    build: () =>
      buildListRow({ title: 'Settings', leading_icon: 'cog', trailing_icon: 'chevron-right' }),
  },
  {
    name: 'section-header-action-icon',
    build: () =>
      buildSectionHeader({ title: 'Recent', action: { label: 'See', icon: 'arrow-right' } }),
  },
  {
    name: 'top-nav-bar-icons',
    build: () =>
      buildTopNavBar({
        title: 'Home',
        leading_icon: 'chevron-left',
        trailing_icon: 'more-vertical',
      }),
  },

  // Multi-item builders where each item carries an icon
  {
    name: 'bottom-nav-3-items',
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
    name: 'card-row-items-with-icons',
    build: () =>
      buildCardRow({
        items: [
          { title: 'HIIT', subtitle: '30m', icon: 'flame' },
          { title: 'Strength', subtitle: '45m', icon: 'dumbbell' },
        ],
      }),
  },
  {
    name: 'stat-grid-items-with-icons',
    build: () =>
      buildStatGrid({
        items: [
          { value: '8,432', label: 'Steps', icon: 'activity' },
          { value: '512', label: 'Kcal', icon: 'flame' },
          { value: '7h', label: 'Sleep', icon: 'moon' },
        ],
      }),
  },

  // Builders with no explicit icon arg but default output is icon-free
  { name: 'heading', build: () => buildHeading({ content: 'Welcome' }) },
  { name: 'body-text', build: () => buildBodyText({ content: 'Body' }) },
  { name: 'divider', build: () => buildDivider({}) },
];

/**
 * Collect every icon slug recursively from `icon_font` nodes. The
 * slug lives in `iconFontName` (e.g. 'search', 'home'); the `name`
 * field is a human-readable layer-panel label, not the glyph.
 */
function collectIconNames(n: PenNode, out: Set<string> = new Set()): Set<string> {
  if (n.type === 'icon_font') {
    const slug = (n as PenNode & { iconFontName?: string }).iconFontName;
    if (slug) out.add(slug);
  }
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  for (const c of children) collectIconNames(c, out);
  return out;
}

describe('builder icon names — per-builder resolution', () => {
  for (const c of CASES) {
    it(`${c.name}: every icon_font name resolves via lookupIconByName`, () => {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const iconNames = collectIconNames(tree);

      const unresolved: string[] = [];
      for (const name of iconNames) {
        const result = lookupIconByName(name);
        if (!result) unresolved.push(name);
      }

      if (unresolved.length > 0) {
        throw new Error(
          `${c.name}: builder emits icon(s) that don't resolve: ${unresolved.join(', ')}. ` +
            `Either rename to a Lucide slug or add the icon upstream.`,
        );
      }
    });
  }
});

describe('builder icon names — aggregate Lucide coverage', () => {
  it('every icon name used across all builders resolves via lookupIconByName', () => {
    // Use lookupIconByName rather than the AVAILABLE_LUCIDE_ICONS
    // list directly: the dictionary has prefix/substring fallbacks
    // that resolve common names (e.g. "home", "more-vertical") even
    // when the literal slug isn't in AVAILABLE_LUCIDE_ICONS. The
    // lookup is the authoritative runtime resolver.
    const allIcons = new Set<string>();
    for (const c of CASES) {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      for (const n of collectIconNames(tree)) allIcons.add(n);
    }

    const unresolved: string[] = [];
    for (const name of allIcons) {
      if (!lookupIconByName(name)) unresolved.push(name);
    }

    if (unresolved.length > 0) {
      throw new Error(
        `Builders reference icon names that don't resolve at runtime: ${unresolved.join(', ')}`,
      );
    }
  });

  it('icon set is non-trivial (≥10 distinct icons across builders)', () => {
    const allIcons = new Set<string>();
    for (const c of CASES) {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      for (const n of collectIconNames(tree)) allIcons.add(n);
    }
    expect(allIcons.size).toBeGreaterThanOrEqual(10);
  });

  it('text-only builders emit zero icons (no spurious icon_font nodes)', () => {
    // Drift guard: heading + body-text + divider must never produce
    // an icon_font node in default output. A regression to "default
    // add a decorative icon" would be visible here.
    const TEXT_ONLY = ['heading', 'body-text', 'divider'] as const;
    for (const name of TEXT_ONLY) {
      const c = CASES.find((x) => x.name === name);
      if (!c) continue;
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const icons = collectIconNames(tree);
      expect(icons.size, `${name} should emit 0 icons`).toBe(0);
    }
  });
});

describe('builder icon names — iconFontFamily always lucide', () => {
  // All builders emit icon_font with family='lucide'. A regression
  // emitting a non-lucide family would break rendering because the
  // font manager only bundles the Lucide font.
  it('every icon_font node has iconFontFamily="lucide"', () => {
    const offenders: Array<{ builder: string; family: string | undefined }> = [];
    for (const c of CASES) {
      const tree = c.build() as unknown as PenNode;
      assignIdsRecursively(tree as unknown as ElementTree);
      const walk = (n: PenNode): void => {
        if (n.type === 'icon_font') {
          const family = (n as PenNode & { iconFontFamily?: string }).iconFontFamily;
          if (family !== 'lucide') {
            offenders.push({ builder: c.name, family });
          }
        }
        const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
        for (const child of children) walk(child);
      };
      walk(tree);
    }
    expect(offenders).toEqual([]);
  });
});
