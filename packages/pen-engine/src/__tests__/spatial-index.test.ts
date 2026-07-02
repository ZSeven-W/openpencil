import { describe, it, expect, beforeEach } from 'vitest';
import { EngineSpatialIndex } from '../core/spatial-index';
import type { PenNode } from '@zseven-w/pen-types';

function makeRenderNode(id: string, x: number, y: number, w: number, h: number) {
  return {
    node: { id, type: 'rectangle', name: id } as PenNode,
    absX: x,
    absY: y,
    absW: w,
    absH: h,
  };
}

describe('EngineSpatialIndex', () => {
  let si: EngineSpatialIndex;

  beforeEach(() => {
    si = new EngineSpatialIndex();
    si.rebuild([
      makeRenderNode('a', 0, 0, 100, 100),
      makeRenderNode('b', 50, 50, 100, 100),
      makeRenderNode('c', 200, 200, 50, 50),
    ] as any);
  });

  it('hitTest should find nodes containing a point', () => {
    const results = si.hitTest(75, 75);
    const ids = results.map((r) => r.node.id);
    expect(ids).toContain('a');
    expect(ids).toContain('b');
    expect(ids).not.toContain('c');
  });

  it('hitTest should return topmost (highest z-index) first', () => {
    const results = si.hitTest(75, 75);
    // b was added after a, so it should be first (topmost)
    expect(results[0].node.id).toBe('b');
  });

  it('searchRect should find intersecting nodes', () => {
    const results = si.searchRect(0, 0, 110, 110);
    const ids = results.map((r) => r.node.id);
    expect(ids).toContain('a');
    expect(ids).toContain('b');
    expect(ids).not.toContain('c');
  });

  it('get should return a specific node', () => {
    const rn = si.get('c');
    expect(rn).toBeDefined();
    expect(rn!.absX).toBe(200);
  });

  it('hitTest on empty point should return empty', () => {
    const results = si.hitTest(500, 500);
    expect(results).toEqual([]);
  });
});
