import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { normalizeTreeLayout } from '../layout/normalize-tree';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f1',
    type: 'frame',
    ...props,
  }) as PenNode;

const rect = (id: string, props: Partial<PenNode> = {}): PenNode =>
  ({
    id,
    type: 'rectangle',
    width: 50,
    height: 30,
    ...props,
  }) as PenNode;

describe('normalizeTreeLayout', () => {
  it('leaves leaf rectangle nodes unchanged', () => {
    const node = rect('a', { x: 10, y: 20 });
    const before = JSON.stringify(node);
    normalizeTreeLayout(node);
    expect(JSON.stringify(node)).toBe(before);
  });

  it('preserves an explicit vertical layout', () => {
    const node = frame({
      layout: 'vertical',
      children: [rect('a'), rect('b')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does not overwrite horizontal layout set by a prior role resolver', () => {
    // Simulates the navbar case: role resolver wrote layout='horizontal',
    // children have no layout hints (inferLayout would return undefined),
    // so normalize would otherwise fall back to 'vertical' and clobber the
    // semantically correct value. It must not.
    const node = frame({
      layout: 'horizontal',
      children: [rect('logo'), rect('links'), rect('cta')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('horizontal');
  });

  it('preserves absolute positioning when all children carry x/y', () => {
    // Frame with no layout and children that all carry explicit x/y is a
    // deliberate absolute-positioning container (phone mockup internals,
    // hero image with floating overlays, etc.). The fallback must not
    // write `vertical` or strip the x/y.
    const node = frame({
      width: 300,
      height: 600,
      children: [
        rect('statusBar', { x: 0, y: 0, width: 300, height: 44 }),
        rect('content', { x: 16, y: 60, width: 268, height: 500 }),
        rect('homeIndicator', { x: 100, y: 580, width: 100, height: 4 }),
      ],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[0].x).toBe(0);
    expect(kids[0].y).toBe(0);
    expect(kids[1].x).toBe(16);
    expect(kids[1].y).toBe(60);
    expect(kids[2].x).toBe(100);
    expect(kids[2].y).toBe(580);
  });

  it('preserves absolute positioning when even one child carries x/y', () => {
    // If the model tagged at least one child with explicit coordinates, we
    // treat the container as absolute-positioned. This is conservative but
    // avoids destroying hand-placed overlays.
    const node = frame({
      children: [rect('base'), rect('overlay', { x: 120, y: 80 })],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[1].x).toBe(120);
    expect(kids[1].y).toBe(80);
  });

  it('writes explicit horizontal layout when gap is the only hint', () => {
    const node = frame({
      gap: 12,
      children: [rect('a'), rect('b')],
    });
    normalizeTreeLayout(node);
    // inferLayout() currently treats gap as a horizontal signal — preserve that behavior.
    expect((node as PenNode & { layout?: string }).layout).toBe('horizontal');
  });

  it('falls back to vertical for multi-child frames with no layout hints', () => {
    const node = frame({
      children: [rect('a'), rect('b'), rect('c')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('leaves single-child frame without a layout untouched', () => {
    const node = frame({
      children: [rect('a')],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('strips x and y from children of an active layout frame', () => {
    const node = frame({
      layout: 'vertical',
      children: [rect('a', { x: 99, y: 33 }), rect('b', { x: 12, y: 77 })],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect('x' in kids[0]).toBe(false);
    expect('y' in kids[0]).toBe(false);
    expect('x' in kids[1]).toBe(false);
    expect('y' in kids[1]).toBe(false);
  });

  it('keeps x and y on badge overlay children', () => {
    const badge: PenNode = {
      id: 'badge1',
      type: 'rectangle',
      role: 'badge',
      width: 8,
      height: 8,
      x: 40,
      y: 0,
    } as PenNode;
    const node = frame({
      layout: 'horizontal',
      children: [rect('a', { x: 5, y: 5 }), badge],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect('x' in kids[0]).toBe(false);
    expect('y' in kids[0]).toBe(false);
    expect(kids[1].x).toBe(40);
    expect(kids[1].y).toBe(0);
  });

  it('recurses into nested frames', () => {
    // Neither inner nor outer children carry x/y, so both get the vertical
    // fallback; the recursion visits the inner frame and writes its layout.
    const inner = frame({
      id: 'inner',
      children: [rect('c'), rect('d')],
    });
    const outer = frame({
      id: 'outer',
      children: [inner, rect('b')],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBe('vertical');
    expect((inner as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does not strip x/y when layout is "none"', () => {
    const node = frame({
      layout: 'none',
      children: [rect('a', { x: 10, y: 20 }), rect('b', { x: 30, y: 40 })],
    });
    normalizeTreeLayout(node);
    const kids = (node as PenNode & { children: PenNode[] }).children;
    expect(kids[0].x).toBe(10);
    expect(kids[0].y).toBe(20);
    expect(kids[1].x).toBe(30);
    expect(kids[1].y).toBe(40);
  });

  it('overlay-only children do not block the vertical fallback', () => {
    // A container whose only positioned children are overlays (badges) is
    // still considered "model forgot layout" — the overlays retain their
    // coordinates while the base frame gets a vertical layout for the rest.
    const badge: PenNode = {
      id: 'badge1',
      type: 'rectangle',
      role: 'badge',
      width: 8,
      height: 8,
      x: 40,
      y: 0,
    } as PenNode;
    const node = frame({
      children: [rect('a'), rect('b'), badge],
    });
    normalizeTreeLayout(node);
    expect((node as PenNode & { layout?: string }).layout).toBe('vertical');
    const kids = (node as PenNode & { children: PenNode[] }).children;
    // badge still carries its absolute coordinates
    expect(kids[2].x).toBe(40);
    expect(kids[2].y).toBe(0);
  });

  it('does NOT verticalize when ALL non-overlay children are frames (overlay composition)', () => {
    // This is the bug subagent diagnosed: AI emits a layout-less parent
    // containing structured nested-frame children (e.g. ring + value-wrap,
    // hero + floating-card overlay, badge stack). Children have no x/y
    // because the AI assumed (0,0) would just work. The previous behavior
    // would silently rewrite the parent to layout='vertical', stacking
    // the frames into a list and breaking the intended overlay.
    //
    // New behavior: when every non-overlay child is a frame, treat it as
    // an overlay container and leave layout undefined so the renderer
    // respects the children's positions.
    const inner1 = frame({ id: 'inner1', width: 80, height: 80 });
    const inner2 = frame({ id: 'inner2', width: 80, height: 80 });
    const outer = frame({
      id: 'outer',
      width: 80,
      height: 80,
      children: [inner1, inner2],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('still verticalizes mixed text+shape stacks (frame + text) where vertical is the right call', () => {
    // Counter-test for the widened composition-primitive rule. The rule
    // must not over-trigger: a frame containing a nested frame plus a
    // text node is a content stack and SHOULD be verticalized.
    // Homogeneous shape-only children (frame+frame, frame+rect,
    // ellipse+ellipse) signal composition; text or icon_font mixed in
    // breaks the heuristic.
    const inner = frame({ id: 'inner', children: [rect('c'), rect('d')] });
    const outer = frame({
      id: 'outer',
      children: [inner, { id: 'label', type: 'text', content: 'Hello' } as unknown as PenNode],
    });
    normalizeTreeLayout(outer);
    expect((outer as PenNode & { layout?: string }).layout).toBe('vertical');
  });

  it('does NOT verticalize all-ellipse concentric stacks (progress-ring pattern)', () => {
    // The activity-rings regression: an LLM emits a layout-less parent
    // with N concentric ellipses at (0,0), expecting them to render
    // centered. The old all-FRAME-children heuristic missed this because
    // the children were ellipses (not frames) and fell through to the
    // vertical fallback, stacking the rings as a list. The widened
    // composition-primitive rule now recognises ellipse-only children
    // as a composition and leaves layout undefined.
    const ellipse = (id: string, w: number, h: number): PenNode =>
      ({ id, type: 'ellipse', width: w, height: h }) as PenNode;
    const parent = frame({
      id: 'rings',
      width: 120,
      height: 120,
      children: [ellipse('outer', 120, 120), ellipse('middle', 84, 84), ellipse('inner', 52, 52)],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('does NOT verticalize frame + ellipse (ring wrapped in a frame)', () => {
    // A common composition: an outer frame background with a nested
    // ellipse ring on top. Both are composition primitives, so the
    // heuristic keeps layout undefined and lets the renderer use the
    // children's own positioning.
    const parent = frame({
      id: 'avatar',
      width: 80,
      height: 80,
      children: [
        { id: 'bg', type: 'frame', width: 80, height: 80 } as unknown as PenNode,
        { id: 'ring', type: 'ellipse', width: 80, height: 80 } as unknown as PenNode,
      ],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBeUndefined();
  });

  it('STILL verticalizes rectangle-only stacks (list of cards, not an overlay)', () => {
    // Rectangles are intentionally NOT in the composition-primitive set.
    // 3 rectangles with no layout hints is almost always a content
    // stack (card list, nav buttons, divider rows), so we keep the
    // vertical fallback for it. Authors who really want a rectangle
    // overlay composition must declare x/y on at least one child.
    const parent = frame({
      id: 'list',
      children: [rect('a'), rect('b'), rect('c')],
    });
    normalizeTreeLayout(parent);
    expect((parent as PenNode & { layout?: string }).layout).toBe('vertical');
  });
});
