// packages/pen-core/src/__tests__/node-diff.test.ts
import { describe, it, expect } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import { diffDocuments } from '../merge/node-diff';

const rect = (id: string, props: Partial<PenNode> = {}): PenNode =>
  ({ id, type: 'rectangle', x: 0, y: 0, width: 10, height: 10, ...props }) as PenNode;

const frame = (id: string, children: PenNode[] = [], props: Partial<PenNode> = {}): PenNode =>
  ({
    id,
    type: 'frame',
    x: 0,
    y: 0,
    width: 100,
    height: 100,
    children,
    ...props,
  }) as PenNode;

const doc = (children: PenNode[]): PenDocument => ({
  version: '1.0.0',
  pages: [{ id: 'page-1', name: 'page-1', children }],
  children: [],
});

describe('diffDocuments', () => {
  it('returns empty for two empty documents', () => {
    const empty = doc([]);
    expect(diffDocuments(empty, empty)).toEqual([]);
  });

  it('emits one add for a new top-level node', () => {
    const before = doc([]);
    const after = doc([rect('r1')]);
    const patches = diffDocuments(before, after);
    expect(patches).toHaveLength(1);
    expect(patches[0].op).toBe('add');
    expect(patches[0].nodeId).toBe('r1');
    expect(patches[0].pageId).toBe('page-1');
    expect(patches[0].parentId).toBeNull();
    expect(patches[0].index).toBe(0);
  });

  it('emits one remove for a deleted top-level node', () => {
    const before = doc([rect('r1')]);
    const after = doc([]);
    const patches = diffDocuments(before, after);
    expect(patches).toHaveLength(1);
    expect(patches[0].op).toBe('remove');
    expect(patches[0].nodeId).toBe('r1');
  });

  it('returns empty for an unchanged single node', () => {
    const a = doc([rect('r1')]);
    const b = doc([rect('r1')]);
    expect(diffDocuments(a, b)).toEqual([]);
  });

  it('emits a modify with changed and before fields when an atomic field changes', () => {
    const before = doc([rect('r1', { width: 10 })]);
    const after = doc([rect('r1', { width: 20 })]);
    const patches = diffDocuments(before, after);
    expect(patches).toHaveLength(1);
    expect(patches[0].op).toBe('modify');
    expect(patches[0].nodeId).toBe('r1');
    expect(patches[0].fields).toEqual({ width: 20 });
    expect(patches[0].beforeFields).toEqual({ width: 10 });
  });

  it('emits one move when a node is reparented within the same page', () => {
    const inner = rect('inner');
    const f1 = frame('f1', [inner]);
    const f2 = frame('f2', []);
    const before = doc([f1, f2]);
    // After: inner moved into f2
    const after = doc([frame('f1', []), frame('f2', [rect('inner')])]);
    const patches = diffDocuments(before, after);
    // Expect at least one move op for inner (and no modify since fields unchanged)
    const moves = patches.filter((p) => p.op === 'move');
    expect(moves).toHaveLength(1);
    expect(moves[0].nodeId).toBe('inner');
    expect(moves[0].parentId).toBe('f2');
  });

  it('emits move when a node moves to a different page', () => {
    const inner = rect('inner');
    const before: PenDocument = {
      version: '1.0.0',
      pages: [
        { id: 'p1', name: 'p1', children: [inner] },
        { id: 'p2', name: 'p2', children: [] },
      ],
      children: [],
    };
    const after: PenDocument = {
      version: '1.0.0',
      pages: [
        { id: 'p1', name: 'p1', children: [] },
        { id: 'p2', name: 'p2', children: [rect('inner')] },
      ],
      children: [],
    };
    const patches = diffDocuments(before, after);
    const moves = patches.filter((p) => p.op === 'move');
    expect(moves).toHaveLength(1);
    expect(moves[0].nodeId).toBe('inner');
    expect(moves[0].pageId).toBe('p2');
  });

  it('emits add for a nested child added inside an existing frame', () => {
    const before = doc([frame('f1', [])]);
    const after = doc([frame('f1', [rect('r1')])]);
    const patches = diffDocuments(before, after);
    const adds = patches.filter((p) => p.op === 'add');
    expect(adds).toHaveLength(1);
    expect(adds[0].nodeId).toBe('r1');
    expect(adds[0].parentId).toBe('f1');
  });

  it('emits a move when a node is reordered within the same parent', () => {
    const before = doc([rect('a'), rect('b'), rect('c')]);
    const after = doc([rect('b'), rect('a'), rect('c')]);
    const patches = diffDocuments(before, after);
    const moves = patches.filter((p) => p.op === 'move');
    // a moves from index 0 to 1, b moves from index 1 to 0, c stays
    expect(moves.map((m) => m.nodeId).sort()).toEqual(['a', 'b']);
  });

  it('produces add + modify + remove in a single mixed diff', () => {
    const before = doc([rect('a', { width: 10 }), rect('b')]);
    const after = doc([rect('a', { width: 20 }), rect('c')]);
    const patches = diffDocuments(before, after);
    expect(patches.find((p) => p.op === 'modify' && p.nodeId === 'a')).toBeDefined();
    expect(patches.find((p) => p.op === 'remove' && p.nodeId === 'b')).toBeDefined();
    expect(patches.find((p) => p.op === 'add' && p.nodeId === 'c')).toBeDefined();
    expect(patches).toHaveLength(3);
  });
});
