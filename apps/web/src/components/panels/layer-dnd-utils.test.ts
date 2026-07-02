import { describe, expect, it } from 'vitest';
import type { PenNode } from '@/types/pen';
import { resolveLayerDropMove } from './layer-dnd-utils';

const frame = (id: string, children: PenNode[] = []): PenNode => ({
  id,
  type: 'frame',
  x: 0,
  y: 0,
  width: 100,
  height: 100,
  children,
});

const rect = (id: string): PenNode => ({
  id,
  type: 'rectangle',
  x: 0,
  y: 0,
  width: 10,
  height: 10,
});

function findParent(nodes: PenNode[], id: string): PenNode | undefined {
  for (const node of nodes) {
    if ('children' in node && node.children?.some((child) => child.id === id)) return node;
    if ('children' in node && node.children) {
      const nested = findParent(node.children, id);
      if (nested) return nested;
    }
  }
  return undefined;
}

describe('resolveLayerDropMove', () => {
  it('preserves absolute position when moving a nested node to root', () => {
    const tree = [frame('parent', [rect('child')]), rect('root-sibling')];
    expect(
      resolveLayerDropMove('child', 'root-sibling', 'above', tree, (id) => findParent(tree, id)),
    ).toEqual({
      parentId: null,
      index: 1,
      preserveAbsolutePosition: true,
    });
  });

  it('does not preserve absolute position for same-parent reorder', () => {
    const tree = [frame('parent', [rect('a'), rect('b')])];
    expect(resolveLayerDropMove('a', 'b', 'below', tree, (id) => findParent(tree, id))).toEqual({
      parentId: 'parent',
      index: 2,
      preserveAbsolutePosition: false,
    });
  });

  it('preserves absolute position when moving into a different container', () => {
    const tree = [frame('source', [rect('child')]), frame('target')];
    expect(
      resolveLayerDropMove('child', 'target', 'inside', tree, (id) => findParent(tree, id)),
    ).toEqual({
      parentId: 'target',
      index: 0,
      preserveAbsolutePosition: true,
    });
  });
});
