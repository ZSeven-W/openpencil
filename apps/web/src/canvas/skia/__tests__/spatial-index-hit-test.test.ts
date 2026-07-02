import { describe, expect, it } from 'vitest';
import { SpatialIndex } from '@zseven-w/pen-renderer';
import type { PenNode } from '@/types/pen';
import type { RenderNode } from '@zseven-w/pen-renderer';

function renderNode(node: PenNode): RenderNode {
  return {
    node,
    absX: node.x ?? 0,
    absY: node.y ?? 0,
    absW:
      typeof (node as { width?: unknown }).width === 'number'
        ? ((node as { width?: number }).width ?? 0)
        : 0,
    absH:
      typeof (node as { height?: unknown }).height === 'number'
        ? ((node as { height?: number }).height ?? 0)
        : 0,
  };
}

describe('SpatialIndex hitTest', () => {
  it('skips transparent QA-style container overlays without self paint', () => {
    const index = new SpatialIndex();
    const content = renderNode({
      id: 'content',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      fill: [{ type: 'solid', color: '#88A750' }],
    } as PenNode);
    const qa = renderNode({
      id: 'qa',
      type: 'frame',
      name: 'QA',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
    } as PenNode);

    index.rebuild([content, qa]);

    expect(index.hitTest(100, 100).map((rn) => rn.node.id)).toEqual(['content']);
  });

  it('keeps visibly painted containers hittable', () => {
    const index = new SpatialIndex();
    const content = renderNode({
      id: 'content',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      fill: [{ type: 'solid', color: '#88A750' }],
    } as PenNode);
    const overlay = renderNode({
      id: 'overlay',
      type: 'frame',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      fill: [{ type: 'solid', color: '#ffffff10' }],
    } as PenNode);

    index.rebuild([content, overlay]);

    expect(index.hitTest(100, 100).map((rn) => rn.node.id)).toEqual(['overlay', 'content']);
  });

  it('keeps stroke-only containers hittable', () => {
    const index = new SpatialIndex();
    const content = renderNode({
      id: 'content',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      fill: [{ type: 'solid', color: '#88A750' }],
    } as PenNode);
    const outlinedFrame = renderNode({
      id: 'outlined',
      type: 'frame',
      x: 0,
      y: 0,
      width: 200,
      height: 200,
      stroke: {
        thickness: 1,
        fill: [{ type: 'solid', color: '#4969A8' }],
      },
    } as PenNode);

    index.rebuild([content, outlinedFrame]);

    expect(index.hitTest(100, 100).map((rn) => rn.node.id)).toEqual(['outlined', 'content']);
  });
});
