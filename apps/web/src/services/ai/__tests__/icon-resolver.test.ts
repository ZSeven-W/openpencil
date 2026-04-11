import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';

// Mock canvas-text-measure to avoid alias resolution pulling browser-only deps.
import { vi } from 'vitest';
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      getNodeById: () => undefined,
      updateNode: () => {},
    }),
  },
}));

import { applyIconPathResolution } from '../icon-resolver';

// A minimal valid SVG path data used as a placeholder for tests where we
// want to assert the resolver left `d` untouched.
const CUSTOM_GEOMETRY_D = 'M0 0 L100 0 L100 50 Z';

function makePath(props: Partial<PenNode> & { name: string }): PenNode {
  return {
    id: 'p',
    type: 'path',
    x: 0,
    y: 0,
    width: 100,
    height: 50,
    d: CUSTOM_GEOMETRY_D,
    ...props,
  } as unknown as PenNode;
}

describe('applyIconPathResolution — opt-in marker gate', () => {
  // Regression coverage for commit 5e2e6f9.
  //
  // Before the fix, the resolver ran a prefix/substring lookup over the
  // icon dictionary on EVERY path node. Data-viz path names that happen
  // to share substrings with icon keys got hijacked:
  //   - "Heart Rate Chart" → lucide:circle
  //   - "Chart Fill"       → lucide:bar-chart-2
  //   - "Steps Progress"   → lucide:circle
  //   - "Calories Progress" → lucide:circle
  //   - "Distance Progress" → lucide:circle
  // The fix gates the resolver so only path nodes whose name carries an
  // explicit icon / logo / symbol / glyph marker are considered.

  // --- Descriptive geometry names MUST be left alone ---
  it.each([
    ['Heart Rate Chart'],
    ['Heart Rate Waveform'],
    ['Steps Progress'],
    ['Calories Progress'],
    ['Distance Progress'],
    ['Chart Fill'],
    ['Bar Chart'],
    ['Line Chart'],
    ['Sparkline'],
    ['Activity Curve'],
    ['Custom Illustration'],
    ['Decorative Shape'],
  ])('does not touch custom geometry path named "%s"', (name) => {
    const node = makePath({ name });
    applyIconPathResolution(node);
    // The resolver writes node.d and node.iconId on a successful match —
    // both must remain unchanged for data-viz paths.
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- Explicit icon markers MUST still resolve ---
  // "Search Icon" is in the icon dictionary as exact match after the
  // trailing "icon" suffix is stripped → "search" → lucide:search.
  it('still resolves a legitimate "Search Icon" path', () => {
    const node = makePath({ name: 'Search Icon' });
    applyIconPathResolution(node);
    // d is replaced with the lucide search path, iconId is set.
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves camelCase "SearchIcon"', () => {
    const node = makePath({ name: 'SearchIcon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves kebab-case "search-icon"', () => {
    const node = makePath({ name: 'search-icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves snake_case "search_icon"', () => {
    const node = makePath({ name: 'search_icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  it('still resolves "Brand Logo" via the logo marker', () => {
    const node = makePath({ name: 'Brand Logo' });
    applyIconPathResolution(node);
    // "brand" is not a known icon dictionary key; without an exact hit
    // the resolver still recognizes the logo intent and queues the node
    // for async iconify lookup, leaving a placeholder d behind — so the
    // custom geometry IS overwritten (which is correct: the caller asked
    // for a logo, not custom geometry).
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
  });

  it('does not replace generic scaffold names like "WC1 Icon" with a fallback circle', () => {
    const node = makePath({ name: 'WC1 Icon' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- "chart" alone (word with explicit icon marker) still resolves ---
  it('resolves "Chart Icon" via the dictionary', () => {
    const node = makePath({ name: 'Chart Icon' });
    applyIconPathResolution(node);
    // After stripping the "icon" suffix, rawName is "chart" which maps
    // to the bar-chart entry in the dictionary.
    expect((node as { d?: string }).d).not.toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeDefined();
  });

  // --- "Bar Chart" alone (NO marker) must NOT resolve ---
  it('does NOT resolve bare "Bar Chart" (no icon/logo marker)', () => {
    const node = makePath({ name: 'Bar Chart' });
    applyIconPathResolution(node);
    expect((node as { d?: string }).d).toBe(CUSTOM_GEOMETRY_D);
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });

  // --- Non-path node types are a no-op ---
  it('ignores non-path nodes', () => {
    const node = {
      id: 'f',
      type: 'frame',
      name: 'Heart Icon',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
    } as unknown as PenNode;
    expect(() => applyIconPathResolution(node)).not.toThrow();
    expect((node as { iconId?: string }).iconId).toBeUndefined();
  });
});
