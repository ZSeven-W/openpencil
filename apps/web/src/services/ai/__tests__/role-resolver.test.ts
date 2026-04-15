import { describe, it, expect, vi } from 'vitest';

// Mock canvas-text-measure to avoid CanvasKit WASM dependency in tests
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

import {
  hexLuminance,
  hasFill,
  hasVisibleFill,
  resolveNodeRole,
  resolveTreePostPass,
  resolveTreeRoles,
} from '../role-resolver';
import type { RoleContext } from '../role-resolver';
import type { PenNode } from '@zseven-w/pen-types';

// Ensure role definitions are registered
import '../role-definitions/index';

describe('hexLuminance', () => {
  it('returns 0 for black', () => {
    expect(hexLuminance('#000000')).toBeCloseTo(0, 2);
  });

  it('returns 1 for white', () => {
    expect(hexLuminance('#FFFFFF')).toBeCloseTo(1, 2);
  });

  it('returns ~0.5 for mid-gray', () => {
    const lum = hexLuminance('#808080');
    expect(lum).toBeGreaterThan(0.2);
    expect(lum).toBeLessThan(0.6);
  });

  it('handles lowercase hex', () => {
    expect(hexLuminance('#ffffff')).toBeCloseTo(1, 2);
  });

  it('handles 8-digit hex (with alpha)', () => {
    expect(hexLuminance('#000000FF')).toBeCloseTo(0, 2);
  });

  it('returns < 0.5 for dark blue (#2563EB)', () => {
    expect(hexLuminance('#2563EB')).toBeLessThan(0.5);
  });

  it('returns > 0.5 for light gray (#F8FAFC)', () => {
    expect(hexLuminance('#F8FAFC')).toBeGreaterThan(0.5);
  });
});

describe('hasFill', () => {
  // hasFill answers "has the AI declared any fill entry?" — it's used by
  // post-pass heuristics that must NOT overwrite an explicitly-chosen
  // fill, even if that fill is transparent. Use `hasVisibleFill` when
  // you need "will this draw a visible color?".

  it('returns false for node without fill', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = {
      id: 'n1',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [],
    } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns true for node with solid fill', () => {
    const node = {
      id: 'n1',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  // hasFill must report transparent fills as "has fill" so the overwrite-
  // protection callers (fixOrphanContainerContrast, fixSectionAlternation)
  // leave them alone. A frame whose AI author explicitly set
  // fill=#00000000 is making a deliberate no-background choice.
  it('returns true for explicit-transparent hex (#00000000) — overwrite protection', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#00000000' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  it('returns true for CSS keyword "transparent"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: 'transparent' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });

  it('returns true for opacity-0 fill — still a deliberate author choice', () => {
    // opacity: 0 is a legitimate "I want a transparent background"
    // declaration. hasFill exists to stop post-pass heuristics from
    // overwriting such choices, so it must keep reporting true.
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });
});

describe('hasVisibleFill', () => {
  // hasVisibleFill answers "will this draw a visible color on screen?".
  // Used by the button foreground contrast pass to decide whether a
  // child node needs a color supplied. Transparent fills must report
  // as false so contrast can paint in a visible foreground color.

  it('returns false for node without fill', () => {
    const node = { id: 'n', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for node with opaque solid fill', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  it('returns false for 8-digit transparent hex (#00000000)', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: '#00000000' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for any 8-digit hex with 00 alpha', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: '#FF00FF00' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for CSS keyword "transparent"', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: 'transparent' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for CSS keyword "none"', () => {
    const node = {
      id: 'p',
      type: 'path',
      x: 0,
      y: 0,
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: 'none' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for a partially-transparent 8-digit hex (non-zero alpha)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FF000080' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  // --- opacity field handling ---------------------------------------
  // PenFill variants all carry an optional `opacity` field. A solid fill
  // with opacity=0 renders as fully transparent regardless of its color
  // hex, and downstream contrast logic must treat it as "no visible
  // fill" so the foreground color can still be supplied.
  it('returns false for a solid fill with opacity: 0', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for a linear gradient fill with opacity: 0', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [
        {
          type: 'linear_gradient',
          angle: 90,
          stops: [
            { offset: 0, color: '#FF0000' },
            { offset: 1, color: '#0000FF' },
          ],
          opacity: 0,
        },
      ],
    } as unknown as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns false for negative opacity (treated as zero)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: -0.5 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(false);
  });

  it('returns true for a solid fill with opacity: 0.5', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0.5 }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });

  it('returns true for a solid fill without an opacity field (default opaque)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasVisibleFill(node)).toBe(true);
  });
});

describe('resolveTreePostPass — transparent fill overwrite protection', () => {
  // Regression: the post-pass heuristics that DON'T want to overwrite an
  // explicit fill choice must respect a transparent fill the same way
  // they respect any other color. Only the button contrast pass needs
  // to treat transparent as "no visible fill".

  it('fixOrphanContainerContrast does NOT overwrite a card with opacity: 0 fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Opacity Zero Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: '#FFFFFF', opacity: 0 }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF', opacity: 0 }]);
    expect((card as any).effects).toBeUndefined();
  });

  it('fixOrphanContainerContrast does NOT overwrite a card with explicit transparent fill', () => {
    // An author wrote a card with cornerRadius + children but chose a
    // transparent background intentionally. The orphan-contrast pass
    // must not suddenly paint it white and add shadows.
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Hollow Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: '#00000000' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    // Fill is exactly what the author set — not #FFFFFF.
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#00000000' }]);
    // No shadow added.
    expect((card as any).effects).toBeUndefined();
  });

  it('fixSectionAlternation does NOT repaint a section with explicit transparent fill', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
        fill: [{ type: 'solid', color: '#00000000' }],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's3',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 1600,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    // The transparent hero stays transparent — alternation does not
    // overwrite an explicit fill of any color.
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#00000000' }]);
  });
});

describe('resolveTreePostPass — button foreground contrast with transparent-hex path icon', () => {
  // The real failure the normalizeStrokeFillSchema fix had to address:
  // an AI-generated stroke-style line icon inside a button, where the
  // AI wrote `fill: [{color: "none"}]` and the normalizer substituted
  // `#00000000` to preserve hollow intent. The button contrast pass
  // must still see "no visible fill" and supply a visible stroke color.
  it('paints stroke on a path icon whose fill is 8-digit transparent hex', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Icon Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#1E293B' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Arrow',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          fill: [{ type: 'solid', color: '#00000000' }],
          stroke: { thickness: 2 },
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.stroke.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('paints fill on a path icon whose only fill is transparent and has no stroke', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Icon Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Star',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          fill: [{ type: 'solid', color: '#00000000' }],
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });
});

describe('resolveTreePostPass — button foreground contrast', () => {
  it('sets white text on dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets dark text on light button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#DBEAFE' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#0F172A' }]);
  });

  it('does not overwrite explicit text fill', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Label',
          x: 0,
          y: 0,
          width: 80,
          height: 20,
          content: 'Sign In',
          fill: [{ type: 'solid', color: '#FDE047' }],
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const txt = (root as any).children[0].children[0];
    expect(txt.fill).toEqual([{ type: 'solid', color: '#FDE047' }]);
  });

  it('sets fill on icon_font child in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'icon-button',
      fill: [{ type: 'solid', color: '#1E293B' }],
      children: [
        {
          id: 'ico',
          type: 'icon_font',
          name: 'Icon',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const ico = (root as any).children[0].children[0];
    expect(ico.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets stroke.fill on stroke-style path in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        {
          id: 'p',
          type: 'path',
          name: 'Arrow',
          x: 0,
          y: 0,
          width: 24,
          height: 24,
          stroke: { thickness: 2 },
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.stroke.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('sets fill on unstyled path (no stroke, no fill) in dark button', () => {
    const button: PenNode = {
      id: 'btn',
      type: 'frame',
      name: 'Button',
      x: 0,
      y: 0,
      width: 44,
      height: 44,
      role: 'button',
      fill: [{ type: 'solid', color: '#2563EB' }],
      children: [
        { id: 'p', type: 'path', name: 'Arrow', x: 0, y: 0, width: 24, height: 24 } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      children: [button],
    } as PenNode;
    resolveTreePostPass(root, 375);
    const p = (root as any).children[0].children[0];
    expect(p.fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });
});

describe('resolveTreePostPass — section background alternation', () => {
  it('alternates fills on 3+ consecutive unfilled sections', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 2400,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((children[1] as any).fill).toEqual([{ type: 'solid', color: '#F8FAFC' }]);
    expect((children[2] as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
  });

  it('only alternates within contiguous runs — non-section children break the run', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 'card',
        type: 'frame' as const,
        name: 'Card',
        x: 0,
        y: 0,
        width: 300,
        height: 200,
        role: 'card',
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's3',
        type: 'frame' as const,
        name: 'Section2',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 3000,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
    expect((children[3] as any).fill).toBeUndefined();
    expect((children[4] as any).fill).toBeUndefined();
  });

  it('skips sections with existing fills', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        fill: [{ type: 'solid', color: '#1E293B' }],
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Features',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 2400,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toEqual([{ type: 'solid', color: '#1E293B' }]);
    expect((children[1] as any).fill).toBeUndefined();
  });

  it('does nothing with fewer than 3 consecutive sections', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Footer',
        x: 0,
        y: 0,
        width: 1200,
        height: 400,
        role: 'footer',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 1200,
      layout: 'vertical',
      children,
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
  });

  // Regression guard for 2026-04-15: when design.md forces a dark rootFrame,
  // the hardcoded #FFFFFF/#F8FAFC alternation painted visible white strips
  // over the dark page background. On dark pages sections must stay
  // transparent — internal card contrast already groups them visually.
  it('does NOT alternate on a dark-themed parent (luminance < 0.5)', () => {
    const children = [
      {
        id: 's0',
        type: 'frame' as const,
        name: 'Hero',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'hero',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's1',
        type: 'frame' as const,
        name: 'Stats',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'stats-section',
        layout: 'vertical' as const,
        children: [],
      },
      {
        id: 's2',
        type: 'frame' as const,
        name: 'CTA',
        x: 0,
        y: 0,
        width: 375,
        height: 400,
        role: 'cta-section',
        layout: 'vertical' as const,
        children: [],
      },
    ] as PenNode[];
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 375,
      height: 1200,
      layout: 'vertical',
      fill: [{ type: 'solid', color: '#111111' }],
      children,
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((children[0] as any).fill).toBeUndefined();
    expect((children[1] as any).fill).toBeUndefined();
    expect((children[2] as any).fill).toBeUndefined();
  });
});

describe('resolveTreePostPass — orphan container contrast', () => {
  it('adds fill + shadow to untagged rounded frame when parent has no fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toEqual([{ type: 'solid', color: '#FFFFFF' }]);
    expect((card as any).effects).toHaveLength(2);
    expect((card as any).effects[0].type).toBe('shadow');
  });

  it('does not apply to structural roles like section', () => {
    const section: PenNode = {
      id: 'sec',
      type: 'frame',
      name: 'Section',
      x: 0,
      y: 0,
      width: 1200,
      height: 400,
      role: 'section',
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [section],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((section as any).fill).toBeUndefined();
  });

  it('does not apply when parent has fill', () => {
    const card: PenNode = {
      id: 'card',
      type: 'frame',
      name: 'Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [
        {
          id: 'txt',
          type: 'text',
          name: 'Title',
          x: 0,
          y: 0,
          width: 200,
          height: 20,
          content: 'Hello',
        } as PenNode,
      ],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      fill: [{ type: 'solid', color: '#F8FAFC' }],
      children: [card],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((card as any).fill).toBeUndefined();
  });

  it('does not apply to empty frames', () => {
    const empty: PenNode = {
      id: 'e',
      type: 'frame',
      name: 'Empty',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
      cornerRadius: 12,
      children: [],
    } as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Root',
      x: 0,
      y: 0,
      width: 1200,
      height: 800,
      children: [empty],
    } as PenNode;
    resolveTreePostPass(root, 1200);
    expect((empty as any).fill).toBeUndefined();
  });
});

describe('resolveTreePostPass — input sibling consistency', () => {
  it('propagates first input fill/stroke to mismatched siblings', () => {
    const input1: PenNode = {
      id: 'i1',
      type: 'frame',
      name: 'Email',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'form-input',
      fill: [{ type: 'solid', color: '#E0F2FE' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#0EA5E9' }] },
    } as unknown as PenNode;
    const input2: PenNode = {
      id: 'i2',
      type: 'frame',
      name: 'Password',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'form-input',
      fill: [{ type: 'solid', color: '#F8FAFC' }],
      stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    } as unknown as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Form',
      x: 0,
      y: 0,
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [input1, input2],
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((input2 as any).fill).toEqual([{ type: 'solid', color: '#E0F2FE' }]);
    expect((input2 as any).stroke.fill).toEqual([{ type: 'solid', color: '#0EA5E9' }]);
  });

  it('skips when all inputs already match', () => {
    const fill = [{ type: 'solid' as const, color: '#F8FAFC' }];
    const stroke = { thickness: 1, fill: [{ type: 'solid' as const, color: '#E2E8F0' }] };
    const input1: PenNode = {
      id: 'i1',
      type: 'frame',
      name: 'Email',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'input',
      fill: [...fill],
      stroke: { ...stroke },
    } as unknown as PenNode;
    const input2: PenNode = {
      id: 'i2',
      type: 'frame',
      name: 'Password',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
      role: 'input',
      fill: [...fill],
      stroke: { ...stroke },
    } as unknown as PenNode;
    const root: PenNode = {
      id: 'root',
      type: 'frame',
      name: 'Form',
      x: 0,
      y: 0,
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [input1, input2],
    } as PenNode;
    resolveTreePostPass(root, 375);
    expect((input1 as any).fill[0].color).toBe('#F8FAFC');
    expect((input2 as any).fill[0].color).toBe('#F8FAFC');
  });
});

// ---------------------------------------------------------------------------
// Name-based role inference
// ---------------------------------------------------------------------------

describe('resolveNodeRole — name-based role inference', () => {
  const ctx: RoleContext = { canvasWidth: 375 };

  it('infers button role from name "Sign In Button"', () => {
    const node = {
      id: 'b',
      type: 'frame',
      name: 'Sign In Button',
      x: 0,
      y: 0,
      width: 120,
      height: 44,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('button');
    // Button role should NOT inject a hardcoded fill color — fill comes from AI/design system
    expect((node as any).fill).toBeUndefined();
  });

  it('does not infer button role for container names like "Button Group"', () => {
    const node = {
      id: 'bg',
      type: 'frame',
      name: 'Button Group',
      x: 0,
      y: 0,
      width: 300,
      height: 60,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('does not infer button role for container names like "Buttons Row"', () => {
    const node = {
      id: 'br',
      type: 'frame',
      name: 'Buttons Row',
      x: 0,
      y: 0,
      width: 300,
      height: 60,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('infers card role from name "Restaurant Card"', () => {
    const node = {
      id: 'c',
      type: 'frame',
      name: 'Restaurant Card',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('card');
    expect((node as any).fill).toBeDefined();
    expect((node as any).effects).toHaveLength(2);
  });

  it('infers input role from name "Email Input"', () => {
    const node = {
      id: 'i',
      type: 'frame',
      name: 'Email Input',
      x: 0,
      y: 0,
      width: 300,
      height: 48,
    } as PenNode;
    resolveNodeRole(node, { ...ctx, parentLayout: 'vertical' });
    expect(node.role).toBe('input');
    expect((node as any).fill[0].color).toBe('#F8FAFC');
    expect((node as any).stroke).toBeDefined();
  });

  it('infers navbar from exact name "Navigation"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Navigation',
      x: 0,
      y: 0,
      width: 375,
      height: 56,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('navbar');
    expect((node as any).fill[0].color).toBe('#FFFFFF');
  });

  it('infers search-bar from name "Search"', () => {
    const node = {
      id: 's',
      type: 'frame',
      name: 'Search',
      x: 0,
      y: 0,
      width: 300,
      height: 44,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('search-bar');
  });

  it('infers hero from exact name "Hero"', () => {
    const node = {
      id: 'h',
      type: 'frame',
      name: 'Hero',
      x: 0,
      y: 0,
      width: 375,
      height: 400,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('hero');
  });

  it('infers footer from exact name "Footer"', () => {
    const node = {
      id: 'f',
      type: 'frame',
      name: 'Footer',
      x: 0,
      y: 0,
      width: 375,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('footer');
  });

  it('does not infer role for non-frame nodes', () => {
    const node = {
      id: 't',
      type: 'text',
      name: 'Button Label',
      x: 0,
      y: 0,
      width: 80,
      height: 20,
      content: 'Click',
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  it('does not override explicit role', () => {
    const node = {
      id: 'b',
      type: 'frame',
      name: 'Search',
      x: 0,
      y: 0,
      width: 300,
      height: 44,
      role: 'input',
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('input'); // keeps explicit role, not inferred search-bar
  });

  it('does not infer role for generic names', () => {
    const node = {
      id: 'g',
      type: 'frame',
      name: 'Container',
      x: 0,
      y: 0,
      width: 300,
      height: 200,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBeUndefined();
  });

  // ---------------------------------------------------------------------
  // Regression coverage for the 2026-04-06 part-word / modifier fixes
  // (commits 5e2e6f9, d45f1a5, f842853). These cases locked down the
  // final behavior of ROLE_PART_WORDS / first-word-after-match scan.
  // ---------------------------------------------------------------------

  // --- "Card X" where X is a structural piece: must NOT become role=card ---
  it.each([
    ['Card Header'],
    ['Card Body'],
    ['Card Footer'],
    ['Card Title'],
    ['Card Content'],
    ['Card Image'],
    ['Card Media'],
    ['Card Label'],
    ['Card Action'],
    ['Card Actions'],
    ['Card Meta'],
    ['Card Caption'],
    ['Card Description'],
    ['Card Wrapper'],
  ])('does not infer card role from structural part name "%s"', (name) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).not.toBe('card');
    // Must not inherit card default fill/shadow
    expect((node as { fill?: unknown }).fill).toBeUndefined();
    expect((node as { effects?: unknown }).effects).toBeUndefined();
  });

  // --- Punctuation between role word and part word still counts as part ---
  it.each([['Card - Header'], ['Card: Body'], ['Card / Footer']])(
    'does not infer card role when punctuation separates it from part word: "%s"',
    (name) => {
      const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
      resolveNodeRole(node, ctx);
      expect(node.role).not.toBe('card');
    },
  );

  // --- Numeric index between role word and part word must NOT leak role ---
  // Regression: sub-agents name nodes "Card 1 Content", "Card 2 Header",
  // "Button 3 Label". The naive \w+ word scan grabbed the numeric "1" as
  // the first token, missed the trailing part word, and wrongly inferred
  // role=card on the content wrapper — which then got the white card
  // default fill and hid all the text inside (the "Upcoming card title
  // invisible" bug from the 2026-04-06 health-tracker dump).
  it.each([
    ['Card 1 Content'],
    ['Card 2 Header'],
    ['Card 3 Footer'],
    ['Card 1 Body'],
    ['Card 10 Title'],
    ['Button 1 Label'],
    ['Button 2 Icon'],
  ])(
    'does not infer card/button role when a numeric index precedes the part word: "%s"',
    (name) => {
      const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
      resolveNodeRole(node, ctx);
      // The node should not inherit the card white fill + shadow. Role is
      // either undefined or something other than card/button.
      expect(node.role).not.toBe('card');
      expect(node.role).not.toBe('button');
      expect((node as { fill?: unknown }).fill).toBeUndefined();
      expect((node as { effects?: unknown }).effects).toBeUndefined();
    },
  );

  // --- Modifier BEFORE the role word: must STILL infer the role ---
  it.each([
    ['Icon Button', 'button'],
    ['Primary Button', 'button'],
    ['Submit Button', 'button'],
    ['Text Button', 'button'],
    ['Image Card', 'card'],
    ['Icon Card', 'card'],
    ['Media Card', 'card'],
    ['User Card', 'card'],
    ['Product Card', 'card'],
  ])('infers %s role from modifier-before name "%s"', (name, expected) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe(expected);
  });

  // --- Prepositional variants: "X with Y" means a variant of X, keep role ---
  it.each([
    ['Card with Icon', 'card'],
    ['Card with Image', 'card'],
    ['Card with Header', 'card'],
    ['Card with Label', 'card'],
    ['Button with Icon', 'button'],
    ['Button with Image', 'button'],
    ['Button with Label', 'button'],
  ])('keeps role on prepositional variant "%s" → %s', (name, expected) => {
    const node = { id: 'n', type: 'frame', name, x: 0, y: 0, width: 300, height: 60 } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe(expected);
  });

  // --- "X Icon" / "X Label": role pattern skipped by part word, the icon
  //     pattern (later in NAME_PATTERN_MAP) then takes over ---
  it('falls through to icon role for "Card Icon" (icon is a part word after card)', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Card Icon',
      x: 0,
      y: 0,
      width: 40,
      height: 40,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('icon');
  });

  it('falls through to icon role for "Button Icon"', () => {
    const node = {
      id: 'n',
      type: 'frame',
      name: 'Button Icon',
      x: 0,
      y: 0,
      width: 40,
      height: 40,
    } as PenNode;
    resolveNodeRole(node, ctx);
    expect(node.role).toBe('icon');
  });
});

// ---------------------------------------------------------------------------
// Size-sanity guard for card-family roles
// ---------------------------------------------------------------------------

describe('resolveNodeRole — absurd-size guard for card-family roles', () => {
  const ctx: RoleContext = { canvasWidth: 375 };

  it('refuses stat-card inference on a 6×6 status dot (name "Status Dot")', () => {
    // Regression: name-based inference saw `/\bstat/` in "Status Dot"
    // and tagged it stat-card. That role injects padding:[24,24] +
    // cornerRadius + shadow, inflating a 6-pixel dot into an
    // oversized card. Guard strips the role entirely when the node
    // is too small to plausibly be a card container.
    const node = {
      id: 'dot',
      type: 'frame',
      name: 'Status Dot',
      x: 0,
      y: 0,
      width: 6,
      height: 6,
      cornerRadius: 3,
      fill: [{ type: 'solid', color: '#22C55E' }],
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { padding?: unknown }).padding).toBeUndefined();
    expect((node as { effects?: unknown[] }).effects).toBeUndefined();
  });

  it('refuses card inference on a tiny swatch (width=20)', () => {
    // A 20×20 color swatch named "Card Swatch" also shouldn't become
    // a card — 20px is below the CARD_LIKE_MIN_DIMENSION threshold.
    const node = {
      id: 'swatch',
      type: 'frame',
      name: 'Card Swatch',
      x: 0,
      y: 0,
      width: 20,
      height: 20,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
  });

  it('refuses LLM-emitted card role on a 16×16 node (not just name-inferred)', () => {
    // The LLM can also emit `role: "card"` directly on a tiny node.
    // Guard applies either way.
    const node = {
      id: 'pill',
      type: 'frame',
      role: 'card',
      x: 0,
      y: 0,
      width: 16,
      height: 16,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBeUndefined();
  });

  it('applies card defaults normally on a normal-sized card (200×120)', () => {
    // Counter-test: a real card-sized frame must still get the
    // card defaults applied. The guard should only strip the role
    // on tiny elements.
    const node = {
      id: 'real-card',
      type: 'frame',
      name: 'Restaurant Card',
      x: 0,
      y: 0,
      width: 200,
      height: 120,
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBe('card');
    expect((node as { cornerRadius?: number }).cornerRadius).toBeDefined();
    expect((node as { effects?: unknown[] }).effects).toBeDefined();
  });

  it('leaves the role alone when width/height are fill_container (unknown pixel size)', () => {
    // When dimensions are sizing keywords we can't tell if the final
    // render will be small, so the guard refuses to decide and the
    // role is applied normally. This prevents the guard from
    // accidentally stripping card roles on responsive layouts.
    const node = {
      id: 'responsive-card',
      type: 'frame',
      role: 'card',
      width: 'fill_container',
      height: 'fit_content',
    } as unknown as PenNode;
    resolveNodeRole(node, ctx);
    expect((node as { role?: string }).role).toBe('card');
    expect((node as { effects?: unknown[] }).effects).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// Page-chrome-in-card guard
// ---------------------------------------------------------------------------

describe('resolveNodeRole — page-chrome roles must not infer inside a card-family parent', () => {
  it('does NOT infer navbar from "Header" when parent is a card', () => {
    // Regression: heart-rate card had a child named "Header" (its
    // title row with the heart icon and "Heart Rate" label). The
    // name `header` lexically matches NAME_EXACT_MAP → 'navbar',
    // which then injected navbarFill (#FFFFFF on light) + bottom
    // border, turning the inner section into a glaring white bar
    // that didn't belong inside the dark heart-rate card.
    const node = {
      id: 'card-header',
      type: 'frame',
      name: 'Header',
      width: 343,
      height: 48,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { fill?: unknown }).fill).toBeUndefined();
    expect((node as { stroke?: unknown }).stroke).toBeUndefined();
  });

  it('does NOT infer footer from "Footer" when parent is a stat-card', () => {
    const node = {
      id: 'card-footer',
      type: 'frame',
      name: 'Footer',
      width: 343,
      height: 36,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'stat-card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBeUndefined();
    expect((node as { padding?: unknown }).padding).toBeUndefined();
  });

  it('STILL infers navbar from "Header" at the page top level (no card parent)', () => {
    // Counter-test: at the page top level (no parent role, or
    // parent is a layout container like section), "Header" still
    // legitimately means a page navbar. Don't over-strip.
    const node = {
      id: 'page-header',
      type: 'frame',
      name: 'Header',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const ctxAtTop: RoleContext = { canvasWidth: 375 };
    resolveNodeRole(node, ctxAtTop);
    expect((node as { role?: string }).role).toBe('navbar');
    expect((node as { fill?: Array<{ color?: string }> }).fill).toBeDefined();
  });

  it('STILL infers footer from "Footer" at the page top level (no card parent)', () => {
    const node = {
      id: 'page-footer',
      type: 'frame',
      name: 'Footer',
      width: 375,
      height: 200,
    } as unknown as PenNode;
    const ctxAtTop: RoleContext = { canvasWidth: 375 };
    resolveNodeRole(node, ctxAtTop);
    expect((node as { role?: string }).role).toBe('footer');
  });

  it('does NOT strip an EXPLICIT navbar role even when parent is a card (LLM was deliberate)', () => {
    // Guard only applies to NAME-INFERRED page-chrome roles. If the
    // LLM explicitly emitted `role: navbar` on a node inside a card,
    // we trust the author intent and apply navbar defaults. (Edge
    // case: a card containing a mini search/nav header. Probably
    // wrong but not our place to override.)
    const node = {
      id: 'explicit-nav',
      type: 'frame',
      name: 'Some Inner Nav',
      role: 'navbar',
      width: 343,
      height: 48,
    } as unknown as PenNode;
    const ctxInCard: RoleContext = { canvasWidth: 375, parentRole: 'card' };
    resolveNodeRole(node, ctxInCard);
    expect((node as { role?: string }).role).toBe('navbar');
  });
});

// ---------------------------------------------------------------------------
// Theme detection (resolveTreeRoles)
// ---------------------------------------------------------------------------

describe('resolveTreeRoles — theme-aware role defaults', () => {
  it('paints navbar default with WHITE on a light page (theme inferred light)', () => {
    // Baseline: page bg is white, the navbar role default should pick the
    // light-theme fill (#FFFFFF). Locks in the original behavior so the
    // theme switch is purely additive.
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('paints navbar default with DARK fill on a dark page (theme inferred dark)', () => {
    // The bug: a navbar with no LLM-set fill on a #111111 dark page used
    // to inherit the hardcoded #FFFFFF default, leaving a glaring white
    // bar across the top of the dark design. The theme detector reads
    // the root fill, classifies the page as dark, and the navbar role
    // function picks navbarFill('dark') = #111111.
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#111111' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });

  it('paints card default with DARK fill on a dark page', () => {
    const card = {
      id: 'card',
      type: 'frame',
      role: 'card',
      width: 300,
      height: 200,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      children: [card],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (card as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('does not paint white orphan-container fill over stroked activity rings', () => {
    const ringCircle = {
      id: 'activity_rings-steps-circle',
      type: 'frame',
      name: 'Steps Circle',
      width: 100,
      height: 100,
      cornerRadius: 50,
      stroke: { thickness: 10, fill: [{ type: 'solid', color: '#00D09C' }] },
      layout: 'vertical',
      children: [
        {
          id: 'steps-value',
          type: 'text',
          content: '8,432',
        },
      ],
    } as unknown as PenNode;
    const ring = {
      id: 'steps-ring',
      type: 'frame',
      name: 'Steps Ring',
      width: 100,
      height: 100,
      children: [ringCircle],
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 812,
      fill: [{ type: 'solid', color: '#1A1A2E' }],
      children: [ring],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);
    resolveTreePostPass(page, 375);

    const fill = (ringCircle as unknown as { fill?: Array<{ color?: string }> }).fill;
    const effects = (ringCircle as unknown as { effects?: unknown[] }).effects;
    expect(fill).toBeUndefined();
    expect(effects).toBeUndefined();
  });

  it('does NOT overwrite an LLM-supplied fill (only fills missing defaults)', () => {
    // The applyDefaults rule must still hold: if the LLM emits any fill,
    // the resolver leaves it alone — even if theme-aware defaults would
    // pick something else. This guards against the resolver becoming a
    // destructive layer.
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
      fill: [{ type: 'solid', color: '#FF00AA' }],
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#111111' }],
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FF00AA');
  });

  it('falls back to LIGHT theme when the root fill is missing or unparseable', () => {
    const navbar = {
      id: 'nav',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
    } as unknown as PenNode;
    const page = {
      id: 'page',
      type: 'frame',
      width: 375,
      height: 800,
      // No fill on root → defaults to light theme
      children: [navbar],
    } as unknown as PenNode;

    resolveTreeRoles(page, 375);

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('honors EXPLICIT theme override even when called on a sub-tree without its own fill', () => {
    // Locks in the fix Codex caught: `sanitizeNodesForInsert` and
    // `sanitizeNodesForUpsert` call `resolveTreeRoles` on an arbitrary
    // sub-tree (a card or navbar that has no fill of its own — the LLM
    // omitted it expecting the dark page bg to show through). Without
    // an explicit theme parameter, theme detection would read the
    // sub-tree root's missing fill, fall back to 'light', and stamp
    // #FFFFFF on top of the dark page.
    //
    // The fix is to look up the actual page root from the document
    // store at the call site and pass its detected theme through this
    // last positional argument. This test simulates that path.
    const navbar = {
      id: 'nav-subtree',
      type: 'frame',
      role: 'navbar',
      width: 375,
      height: 56,
      // No fill on this navbar — the LLM expected the page bg to show.
    } as unknown as PenNode;

    // Call resolveTreeRoles on the navbar DIRECTLY (no parent passed),
    // but supply an explicit dark theme — this is what the sanitize
    // path does after looking up the live page root.
    resolveTreeRoles(navbar, 375, undefined, undefined, undefined, false, 'dark');

    const fill = (navbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });

  it('explicit theme override beats auto-detection from a non-page sub-tree root', () => {
    // Stronger version of the previous test: the sub-tree root HAS a
    // fill (e.g. a card with #1A1A1A) which auto-detection would
    // classify as 'dark' anyway. This test makes sure the explicit
    // 'light' override still wins, proving the explicit parameter is
    // not silently ignored.
    const card = {
      id: 'card-subtree',
      type: 'frame',
      role: 'card',
      width: 300,
      height: 200,
      fill: [{ type: 'solid', color: '#FAFAFA' }],
      children: [
        {
          id: 'inner-navbar',
          type: 'frame',
          role: 'navbar',
          width: 300,
          height: 56,
        },
      ],
    } as unknown as PenNode;

    // Force theme = 'dark' even though the card itself looks light
    resolveTreeRoles(card, 375, undefined, undefined, undefined, false, 'dark');

    const innerNavbar = (card as unknown as { children: PenNode[] }).children[0];
    const fill = (innerNavbar as unknown as { fill?: Array<{ color?: string }> }).fill;
    expect(fill?.[0]?.color).toBe('#111111');
  });
});
