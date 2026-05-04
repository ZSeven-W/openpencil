import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { flattenToRenderNodes } from '../document-flattener';

const frame = (props: Partial<PenNode> & { children?: PenNode[] }): PenNode =>
  ({
    id: 'f1',
    type: 'frame',
    x: 0,
    y: 0,
    ...props,
  }) as PenNode;

const text = (id: string, content: string, props: Partial<PenNode> = {}): PenNode =>
  ({
    id,
    type: 'text',
    x: 0,
    y: 0,
    content,
    fontSize: 16,
    ...props,
  }) as PenNode;

describe('flattenToRenderNodes — dimension consistency', () => {
  it('skips nodes disabled via enabled=false', () => {
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      children: [
        { id: 'visible', type: 'rectangle', x: 0, y: 0, width: 120, height: 80 } as PenNode,
        {
          id: 'disabled',
          type: 'rectangle',
          x: 20,
          y: 20,
          width: 120,
          height: 80,
          enabled: false,
        } as PenNode,
      ],
    });

    const nodes = flattenToRenderNodes([root]);

    expect(nodes.some((rn) => rn.node.id === 'visible')).toBe(true);
    expect(nodes.some((rn) => rn.node.id === 'disabled')).toBe(false);
  });

  it('absH uses getNodeHeight for text without height, not sizeToNumber 100 fallback', () => {
    // Simulates text after fixTextHeights deleted height
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      layout: 'vertical' as any,
      children: [
        // Text with no height property (deleted by fixTextHeights)
        {
          id: 't1',
          type: 'text',
          content: 'Hello world',
          fontSize: 16,
          width: 'fill_container' as any,
        } as PenNode,
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const t1 = nodes.find((rn) => rn.node.id === 't1')!;

    // absH should reflect estimated text height (~18-24px for single line at 16px),
    // NOT the 100px sizeToNumber fallback
    expect(t1.absH).toBeLessThan(50);
    expect(t1.absH).toBeGreaterThan(10);
  });

  it('absW matches child layout width for frame with no explicit width', () => {
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      layout: 'vertical' as any,
      children: [
        frame({
          id: 'inner',
          // No explicit width — getNodeWidth should compute from children
          height: 100,
          children: [
            { id: 'r1', type: 'rectangle', x: 0, y: 0, width: 200, height: 50 } as PenNode,
          ],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const inner = nodes.find((rn) => rn.node.id === 'inner')!;

    // inner absW should come from getNodeWidth (fitContentWidth → 200),
    // not the sizeToNumber fallback of 100
    expect(inner.absW).toBeGreaterThanOrEqual(200);
  });

  it('nested text nodes get correct positions and non-zero dimensions', () => {
    const root = frame({
      id: 'root',
      width: 375,
      height: 812,
      layout: 'vertical' as any,
      padding: [20, 16],
      gap: 8,
      children: [
        frame({
          id: 'card',
          width: 'fill_container' as any,
          height: 'fit_content' as any,
          layout: 'vertical' as any,
          padding: [16, 16],
          gap: 8,
          children: [
            text('title', 'Card Title', {
              width: 'fill_container' as any,
              fontSize: 18,
              fontWeight: '600',
            }),
            text('desc', 'Description text that may wrap.', {
              width: 'fill_container' as any,
              fontSize: 14,
            }),
          ],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);

    for (const rn of nodes) {
      expect(rn.absW, `${rn.node.id} width > 0`).toBeGreaterThan(0);
      expect(rn.absH, `${rn.node.id} height > 0`).toBeGreaterThan(0);
    }

    const card = nodes.find((rn) => rn.node.id === 'card')!;
    const title = nodes.find((rn) => rn.node.id === 'title')!;
    const desc = nodes.find((rn) => rn.node.id === 'desc')!;

    // title inside card
    expect(title.absX).toBeGreaterThan(card.absX);
    expect(title.absY).toBeGreaterThan(card.absY);

    // desc below title
    expect(desc.absY).toBeGreaterThan(title.absY);
  });

  it('absW/absH match nodeW/nodeH for frame without explicit dimensions', () => {
    // Frame has children but no explicit width — not inside a layout parent,
    // so computeLayoutPositions does NOT set width. This exposes the divergence
    // between sizeToNumber (fallback 100) and getNodeWidth (fitContent → 200).
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      // No layout, gap, padding, or fill_container children → inferLayout returns undefined
      children: [
        frame({
          id: 'inner',
          // No explicit width or height
          children: [
            { id: 'r1', type: 'rectangle', x: 10, y: 10, width: 200, height: 50 } as PenNode,
          ],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const inner = nodes.find((rn) => rn.node.id === 'inner')!;

    // getNodeWidth → fitContentWidth → 200 (from child rectangle)
    // Before fix: absW = 100 (sizeToNumber fallback). After fix: absW = 200.
    expect(inner.absW).toBeGreaterThanOrEqual(200);
    // getNodeHeight → fitContentHeight → 50 (from child rectangle)
    // Before fix: absH = 100 (fallback). After fix: absH = 50 (or greater).
    expect(inner.absH).toBeGreaterThanOrEqual(50);
    expect(inner.absH).toBeLessThan(100); // not the 100 fallback
  });

  it('children with stripped x/y in layout frame get correct positions', () => {
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      layout: 'vertical' as any,
      padding: [20, 16],
      gap: 12,
      children: [
        // x/y stripped by sanitizeLayoutChildPositions
        {
          id: 't1',
          type: 'text',
          content: 'First',
          fontSize: 16,
          width: 'fill_container' as any,
        } as PenNode,
        {
          id: 't2',
          type: 'text',
          content: 'Second',
          fontSize: 16,
          width: 'fill_container' as any,
        } as PenNode,
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const t1 = nodes.find((rn) => rn.node.id === 't1')!;
    const t2 = nodes.find((rn) => rn.node.id === 't2')!;

    // t1 at padding offset
    expect(t1.absX).toBe(16); // pad.left
    expect(t1.absY).toBe(20); // pad.top

    // t2 below t1 + gap
    expect(t2.absY).toBeGreaterThan(t1.absY + t1.absH);
  });

  it('root frame clip matches absW/absH, not a divergent nodeW/nodeH', () => {
    // Root frame (depth=0) creates a clipStack entry for its children.
    // The clip rect must use the same dimensions as the RenderNode's absW/absH.
    const root = frame({
      id: 'root',
      width: 400,
      height: 600,
      cornerRadius: 12,
      layout: 'vertical' as any,
      children: [text('t1', 'Hello', { width: 'fill_container' as any })],
    });

    const nodes = flattenToRenderNodes([root]);
    const rootRN = nodes.find((rn) => rn.node.id === 'root')!;
    const t1 = nodes.find((rn) => rn.node.id === 't1')!;

    // Root frame itself has empty clip stack (it IS the clip source for children)
    expect(rootRN.clipStack).toBeUndefined();

    // Child inherits root's clip — single entry on the stack
    expect(t1.clipStack).toBeDefined();
    expect(t1.clipStack!.length).toBe(1);
    expect(t1.clipStack![0].w).toBe(rootRN.absW);
    expect(t1.clipStack![0].h).toBe(rootRN.absH);
    expect(t1.clipStack![0].x).toBe(rootRN.absX);
    expect(t1.clipStack![0].y).toBe(rootRN.absY);
  });

  it('root frame clip matches absW/absH for frame without explicit height', () => {
    // Frame with fit_content height — getNodeHeight computes from children.
    // The artboard clip must equal the RenderNode's absH, not a stale fallback.
    const root = frame({
      id: 'root',
      width: 375,
      // No explicit height — relies on getNodeHeight → fitContentHeight
      layout: 'vertical' as any,
      padding: [20, 16],
      children: [text('t1', 'Card title', { width: 'fill_container' as any, fontSize: 18 })],
    });

    const nodes = flattenToRenderNodes([root]);
    const rootRN = nodes.find((rn) => rn.node.id === 'root')!;
    const t1 = nodes.find((rn) => rn.node.id === 't1')!;

    expect(rootRN.absH).toBeGreaterThan(0);

    expect(t1.clipStack).toBeDefined();
    expect(t1.clipStack!.length).toBe(1);
    expect(t1.clipStack![0].h).toBe(rootRN.absH);
    expect(t1.clipStack![0].w).toBe(rootRN.absW);
  });

  it('nested frame with clipContent appends its own clip on top of ancestor stack', () => {
    const root = frame({
      id: 'root',
      width: 400,
      height: 400,
      children: [
        frame({
          id: 'card',
          x: 40,
          y: 50,
          width: 200,
          height: 120,
          cornerRadius: 16,
          clipContent: true,
          children: [text('inner', 'Nested content', { width: 'fill_container' as any })],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const card = nodes.find((rn) => rn.node.id === 'card')!;
    const inner = nodes.find((rn) => rn.node.id === 'inner')!;

    // inner has 2-level stack: outer = root frame's clip, then card's own clip
    expect(inner.clipStack).toBeDefined();
    expect(inner.clipStack!.length).toBe(2);
    expect(inner.clipStack![1].x).toBe(card.absX);
    expect(inner.clipStack![1].y).toBe(card.absY);
    expect(inner.clipStack![1].w).toBe(card.absW);
    expect(inner.clipStack![1].h).toBe(card.absH);
    expect(inner.clipStack![1].rx).toBe(16);
  });

  it('preserves each ancestor rrect (rounded modal containing rounded card)', () => {
    // A rounded modal contains a rounded card. Both rrects must be enforced
    // on inner content — a single ClipInfo can't encode `rrect ∩ rrect` so
    // each level lives independently on the clipStack.
    const root = frame({
      id: 'root',
      width: 600,
      height: 600,
      children: [
        frame({
          id: 'modal',
          x: 100,
          y: 100,
          width: 400,
          height: 400,
          cornerRadius: 24,
          clipContent: true,
          children: [
            frame({
              id: 'card',
              x: 20,
              y: 20,
              width: 360,
              height: 80,
              cornerRadius: 12,
              clipContent: true,
              children: [text('card-inner', 'Title')],
            }),
          ],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const inner = nodes.find((rn) => rn.node.id === 'card-inner')!;

    // Inner sits under [root artboard, modal rrect, card rrect]
    expect(inner.clipStack).toBeDefined();
    expect(inner.clipStack!.length).toBe(3);
    expect(inner.clipStack![0].rx).toBe(0); // root artboard (no cornerRadius)
    expect(inner.clipStack![1].rx).toBe(24); // modal preserves its rounding
    expect(inner.clipStack![2].rx).toBe(12); // card preserves its rounding
  });

  it('overflowing horizontal scroll: 3rd card stack ends with row + card own clip', () => {
    // Reproduces the food-delivery brief regression: a horizontal scroll row
    // contains 3 rounded-corner cards (each `clipContent: true`) whose total
    // width exceeds the row's visible width. The renderer enforces the row's
    // rectangular clip AND the card's rounded clip independently — the 3rd
    // card's children paint within `(row clip) ∩ (card3 rrect clip)` even
    // though card3's bounds extend past the row's right edge.
    const root = frame({
      id: 'root',
      width: 400,
      height: 400,
      children: [
        frame({
          id: 'row',
          width: 'fill_container' as any,
          height: 200,
          layout: 'horizontal' as any,
          gap: 12,
          clipContent: true,
          children: [
            frame({
              id: 'c1',
              width: 150,
              height: 150,
              cornerRadius: 16,
              clipContent: true,
              children: [text('c1-inner', 'Sushi')],
            }),
            frame({
              id: 'c2',
              width: 150,
              height: 150,
              cornerRadius: 16,
              clipContent: true,
              children: [text('c2-inner', 'Burger')],
            }),
            frame({
              id: 'c3',
              width: 150,
              height: 150,
              cornerRadius: 16,
              clipContent: true,
              children: [text('c3-inner', 'Pizza')],
            }),
          ],
        }),
      ],
    });

    const nodes = flattenToRenderNodes([root]);
    const c3 = nodes.find((rn) => rn.node.id === 'c3')!;
    const c3inner = nodes.find((rn) => rn.node.id === 'c3-inner')!;

    // Layout sanity: c3 at 324 with right edge 474 > 400 (root width)
    expect(c3.absX).toBe(324);
    expect(c3.absW).toBe(150);

    // c3 RN paints under [root, row] — 2 ancestors
    expect(c3.clipStack).toBeDefined();
    expect(c3.clipStack!.length).toBe(2);
    // Row clip is full row width (400) at row position
    expect(c3.clipStack![1].w).toBe(400);

    // c3-inner sits under [root, row, c3] — 3 levels, every rrect preserved
    expect(c3inner.clipStack).toBeDefined();
    expect(c3inner.clipStack!.length).toBe(3);
    // Row clip is enforced as its OWN entry — not collapsed into c3's
    expect(c3inner.clipStack![1].x).toBe(0);
    expect(c3inner.clipStack![1].w).toBe(400);
    expect(c3inner.clipStack![1].rx).toBe(0);
    // c3 clip is enforced as its OWN entry — keeps its own bounds + rrect
    expect(c3inner.clipStack![2].x).toBe(324);
    expect(c3inner.clipStack![2].w).toBe(150);
    expect(c3inner.clipStack![2].rx).toBe(16);
  });
});
