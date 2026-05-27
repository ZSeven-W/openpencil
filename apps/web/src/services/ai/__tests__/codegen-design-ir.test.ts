import { describe, expect, it } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { buildCodegenDesignIR } from '../codegen-design-ir';

describe('buildCodegenDesignIR', () => {
  it('preserves layout, text, visual facts, assets, and semantic hints', () => {
    const nodes: PenNode[] = [
      {
        id: 'page',
        type: 'frame',
        name: 'Mobile Checkout Page',
        x: 0,
        y: 0,
        width: 390,
        height: 844,
        layout: 'vertical',
        gap: 16,
        padding: [24, 20, 24, 20],
        fill: [{ type: 'solid', color: '#ffffff' }],
        children: [
          {
            id: 'title',
            type: 'text',
            name: 'Title',
            content: 'Confirm order',
            x: 20,
            y: 32,
            width: 200,
            height: 32,
            fontFamily: 'Inter',
            fontSize: 24,
            fontWeight: 700,
            fill: [{ type: 'solid', color: '#111827' }],
          },
          {
            id: 'button',
            type: 'frame',
            name: 'Primary Button',
            role: 'button',
            x: 20,
            y: 760,
            width: 350,
            height: 48,
            cornerRadius: 8,
            fill: [{ type: 'solid', color: '#2563eb' }],
            children: [
              {
                id: 'button-text',
                type: 'text',
                content: 'Pay now',
                width: 80,
                height: 20,
              } as PenNode,
            ],
          },
          {
            id: 'hero',
            type: 'image',
            name: 'Product image',
            src: './assets/product-1.png',
            width: 120,
            height: 120,
          },
        ],
      } as PenNode,
    ];

    const ir = buildCodegenDesignIR(nodes, [
      {
        relativePath: './assets/product-1.png',
        sourceNodeId: 'hero',
        sourceNodeName: 'Product image',
        sourceKind: 'image-node',
      },
    ]);

    expect(ir.target.platformHint).toBe('mobile');
    expect(ir.summary.textContent).toEqual(expect.arrayContaining(['Confirm order', 'Pay now']));
    expect(ir.summary.assetCount).toBe(1);
    expect(ir.summary.semanticKinds.button).toBeGreaterThan(0);
    expect(ir.nodes[0].layout).toMatchObject({ mode: 'vertical', gap: 16 });
    expect(ir.nodes[0].children?.[0].text).toMatchObject({
      content: 'Confirm order',
      fontFamily: 'Inter',
      fontSize: 24,
      fontWeight: 700,
    });
    expect(ir.nodes[0].children?.[2].assetRefs).toEqual(['./assets/product-1.png']);
  });
});
