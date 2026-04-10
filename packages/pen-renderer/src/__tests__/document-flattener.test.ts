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

  it('root frame clipRect matches absW/absH, not a divergent nodeW/nodeH', () => {
    // Root frame (depth=0) creates a clipRect for its children.
    // clipRect must use the same dimensions as the RenderNode's absW/absH.
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

    // Root frame itself has no clipRect (it IS the clip source)
    expect(rootRN.clipRect).toBeUndefined();

    // Child inherits root's clip — must match root's rendered dimensions
    expect(t1.clipRect).toBeDefined();
    expect(t1.clipRect!.w).toBe(rootRN.absW);
    expect(t1.clipRect!.h).toBe(rootRN.absH);
    expect(t1.clipRect!.x).toBe(rootRN.absX);
    expect(t1.clipRect!.y).toBe(rootRN.absY);
  });

  it('root frame clipRect matches absW/absH for frame without explicit height', () => {
    // Frame with fit_content height — getNodeHeight computes from children.
    // clipRect.h must equal the RenderNode's absH, not a stale fallback.
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

    // absH should be computed from content, not 100 fallback
    expect(rootRN.absH).toBeGreaterThan(0);

    // clipRect must match absH
    expect(t1.clipRect).toBeDefined();
    expect(t1.clipRect!.h).toBe(rootRN.absH);
    expect(t1.clipRect!.w).toBe(rootRN.absW);
  });
});
