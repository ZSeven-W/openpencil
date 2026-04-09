// packages/pen-core/src/__tests__/node-merge.test.ts
import { describe, it, expect } from 'vitest';
import type { PenDocument, PenNode, VariableDefinition } from '@zseven-w/pen-types';
import { mergeDocuments } from '../merge/node-merge';

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

const group = (id: string, children: PenNode[] = []): PenNode =>
  ({ id, type: 'group', x: 0, y: 0, width: 10, height: 10, children }) as PenNode;

const doc = (children: PenNode[]): PenDocument => ({
  version: '1.0.0',
  pages: [{ id: 'page-1', name: 'page-1', children }],
  children: [],
});

const docMulti = (pages: Array<{ id: string; children: PenNode[] }>): PenDocument => ({
  version: '1.0.0',
  pages: pages.map((p) => ({ id: p.id, name: p.id, children: p.children })),
  children: [],
});

const docLegacy = (children: PenNode[]): PenDocument => ({
  version: '1.0.0',
  children,
});

const findNode = (d: PenDocument, id: string): PenNode | undefined => {
  function walk(nodes: PenNode[]): PenNode | undefined {
    for (const n of nodes) {
      if (n.id === id) return n;
      const c = (n as { children?: PenNode[] }).children;
      if (c) {
        const found = walk(c);
        if (found) return found;
      }
    }
    return undefined;
  }
  for (const page of d.pages ?? [{ id: null, name: null, children: d.children ?? [] } as never]) {
    const found = walk((page as { children: PenNode[] }).children);
    if (found) return found;
  }
  return undefined;
};

describe('mergeDocuments', () => {
  // 1
  it('merges two empty documents to an empty document with no conflicts', () => {
    const empty = doc([]);
    const result = mergeDocuments({ base: empty, ours: empty, theirs: empty });
    expect(result.nodeConflicts).toEqual([]);
    expect(result.docFieldConflicts).toEqual([]);
    expect(result.merged.pages![0].children).toEqual([]);
  });

  // 2
  it('produces no conflicts when all three sides are identical', () => {
    const d = doc([rect('r1'), rect('r2')]);
    const result = mergeDocuments({ base: d, ours: d, theirs: d });
    expect(result.nodeConflicts).toEqual([]);
    expect(result.docFieldConflicts).toEqual([]);
    expect(result.merged.pages![0].children).toHaveLength(2);
  });

  // 3
  it('takes ours when only ours changed a field', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([rect('r1', { width: 20 })]);
    const theirs = doc([rect('r1', { width: 10 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(20);
  });

  // 4
  it('takes theirs when only theirs changed a field', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([rect('r1', { width: 10 })]);
    const theirs = doc([rect('r1', { width: 30 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(30);
  });

  // 5
  it('produces no conflict when both sides change a field to the same value', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([rect('r1', { width: 99 })]);
    const theirs = doc([rect('r1', { width: 99 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(99);
  });

  // 6
  it('emits both-modified-same-field when sides disagree on a field value', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([rect('r1', { width: 20 })]);
    const theirs = doc([rect('r1', { width: 30 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toHaveLength(1);
    expect(result.nodeConflicts[0].reason).toBe('both-modified-same-field');
    expect(result.nodeConflicts[0].nodeId).toBe('r1');
    // Merged document uses ours as placeholder
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(20);
  });

  // 7
  it('takes ours when ours adds a node not in base or theirs', () => {
    const base = doc([]);
    const ours = doc([rect('r1')]);
    const theirs = doc([]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(findNode(result.merged, 'r1')).toBeDefined();
  });

  // 8
  it('takes theirs when theirs adds a node not in base or ours', () => {
    const base = doc([]);
    const ours = doc([]);
    const theirs = doc([rect('r1')]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(findNode(result.merged, 'r1')).toBeDefined();
  });

  // 9
  it('takes ours without conflict when both sides add the same id with same content', () => {
    const base = doc([]);
    const ours = doc([rect('r1', { width: 50 })]);
    const theirs = doc([rect('r1', { width: 50 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(50);
  });

  // 10
  it('emits add-vs-add-different when both sides add the same id with different content', () => {
    const base = doc([]);
    const ours = doc([rect('r1', { width: 50 })]);
    const theirs = doc([rect('r1', { width: 75 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toHaveLength(1);
    expect(result.nodeConflicts[0].reason).toBe('add-vs-add-different');
    expect(result.nodeConflicts[0].base).toBeNull();
  });

  // 11
  it('cleanly deletes when ours deletes and theirs is unchanged', () => {
    const base = doc([rect('r1')]);
    const ours = doc([]);
    const theirs = doc([rect('r1')]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(findNode(result.merged, 'r1')).toBeUndefined();
  });

  // 12
  it('cleanly deletes when theirs deletes and ours is unchanged', () => {
    const base = doc([rect('r1')]);
    const ours = doc([rect('r1')]);
    const theirs = doc([]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(findNode(result.merged, 'r1')).toBeUndefined();
  });

  // 13
  it('cleanly deletes when both sides delete the same node', () => {
    const base = doc([rect('r1')]);
    const ours = doc([]);
    const theirs = doc([]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(findNode(result.merged, 'r1')).toBeUndefined();
  });

  // 14
  it('emits modify-vs-delete when ours modifies and theirs deletes', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([rect('r1', { width: 20 })]);
    const theirs = doc([]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toHaveLength(1);
    expect(result.nodeConflicts[0].reason).toBe('modify-vs-delete');
    expect(result.nodeConflicts[0].theirs).toBeNull();
  });

  // 15
  it('emits modify-vs-delete when ours deletes and theirs modifies', () => {
    const base = doc([rect('r1', { width: 10 })]);
    const ours = doc([]);
    const theirs = doc([rect('r1', { width: 30 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toHaveLength(1);
    expect(result.nodeConflicts[0].reason).toBe('modify-vs-delete');
    expect(result.nodeConflicts[0].ours).toBeNull();
  });

  // 16
  it('handles nested groups: deep modify keeps parents intact', () => {
    const base = doc([frame('f1', [group('g1', [rect('r1', { width: 10 })])])]);
    const ours = doc([frame('f1', [group('g1', [rect('r1', { width: 20 })])])]);
    const theirs = doc([frame('f1', [group('g1', [rect('r1', { width: 10 })])])]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect((findNode(result.merged, 'r1') as { width: number }).width).toBe(20);
    expect(findNode(result.merged, 'g1')).toBeDefined();
    expect(findNode(result.merged, 'f1')).toBeDefined();
  });

  // 17
  it('auto-merges reparent when both sides move to the same new parent', () => {
    const base = doc([frame('f1', [rect('r1')]), frame('f2', [])]);
    const ours = doc([frame('f1', []), frame('f2', [rect('r1')])]);
    const theirs = doc([frame('f1', []), frame('f2', [rect('r1')])]);
    const result = mergeDocuments({ base, ours, theirs });
    const reparentConflicts = result.nodeConflicts.filter((c) => c.reason === 'reparent-conflict');
    expect(reparentConflicts).toHaveLength(0);
    // r1 ends up under f2
    const f2 = findNode(result.merged, 'f2') as { children?: PenNode[] };
    expect(f2.children?.some((c) => c.id === 'r1')).toBe(true);
  });

  // 18
  it('emits reparent-conflict when sides move to different parents', () => {
    const base = doc([frame('f1', [rect('r1')]), frame('f2', []), frame('f3', [])]);
    const ours = doc([frame('f1', []), frame('f2', [rect('r1')]), frame('f3', [])]);
    const theirs = doc([frame('f1', []), frame('f2', []), frame('f3', [rect('r1')])]);
    const result = mergeDocuments({ base, ours, theirs });
    const reparentConflicts = result.nodeConflicts.filter((c) => c.reason === 'reparent-conflict');
    expect(reparentConflicts).toHaveLength(1);
    expect(reparentConflicts[0].nodeId).toBe('r1');
  });

  // 19
  it('auto-merges reparent when only one side moved', () => {
    const base = doc([frame('f1', [rect('r1')]), frame('f2', [])]);
    const ours = doc([frame('f1', []), frame('f2', [rect('r1')])]);
    const theirs = doc([frame('f1', [rect('r1')]), frame('f2', [])]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts.filter((c) => c.reason === 'reparent-conflict')).toHaveLength(0);
    const f2 = findNode(result.merged, 'f2') as { children?: PenNode[] };
    expect(f2.children?.some((c) => c.id === 'r1')).toBe(true);
  });

  // 20
  it('prefers ours order on sibling reorder when ours reorders and theirs does not', () => {
    const base = doc([rect('a'), rect('b'), rect('c')]);
    const ours = doc([rect('b'), rect('a'), rect('c')]);
    const theirs = doc([rect('a'), rect('b'), rect('c')]);
    const result = mergeDocuments({ base, ours, theirs });
    const order = result.merged.pages![0].children.map((n) => n.id);
    expect(order).toEqual(['b', 'a', 'c']);
  });

  // 21
  it('takes a new variable when ours adds it and theirs does not have it', () => {
    const baseDoc: PenDocument = {
      version: '1.0.0',
      pages: [{ id: 'p', name: 'p', children: [] }],
      children: [],
    };
    const oursDoc: PenDocument = {
      ...baseDoc,
      variables: { 'color-1': { type: 'color', value: '#ff0000' } as VariableDefinition },
    };
    const theirsDoc: PenDocument = { ...baseDoc };
    const result = mergeDocuments({ base: baseDoc, ours: oursDoc, theirs: theirsDoc });
    expect(result.docFieldConflicts).toEqual([]);
    expect(result.merged.variables?.['color-1']).toBeDefined();
  });

  // 22
  it('emits a variables doc-field conflict when both sides add the same variable name with different values', () => {
    const baseDoc: PenDocument = {
      version: '1.0.0',
      pages: [{ id: 'p', name: 'p', children: [] }],
      children: [],
    };
    const oursDoc: PenDocument = {
      ...baseDoc,
      variables: { 'color-1': { type: 'color', value: '#ff0000' } as VariableDefinition },
    };
    const theirsDoc: PenDocument = {
      ...baseDoc,
      variables: { 'color-1': { type: 'color', value: '#00ff00' } as VariableDefinition },
    };
    const result = mergeDocuments({ base: baseDoc, ours: oursDoc, theirs: theirsDoc });
    expect(result.docFieldConflicts).toHaveLength(1);
    expect(result.docFieldConflicts[0].field).toBe('variables');
    expect(result.docFieldConflicts[0].path).toBe('variables.color-1');
  });

  // 23
  it('emits a variables conflict when ours deletes a variable that theirs modified', () => {
    const baseDoc: PenDocument = {
      version: '1.0.0',
      pages: [{ id: 'p', name: 'p', children: [] }],
      children: [],
      variables: { 'color-1': { type: 'color', value: '#ff0000' } as VariableDefinition },
    };
    const oursDoc: PenDocument = { ...baseDoc, variables: {} };
    const theirsDoc: PenDocument = {
      ...baseDoc,
      variables: { 'color-1': { type: 'color', value: '#0000ff' } as VariableDefinition },
    };
    const result = mergeDocuments({ base: baseDoc, ours: oursDoc, theirs: theirsDoc });
    expect(result.docFieldConflicts).toHaveLength(1);
    expect(result.docFieldConflicts[0].field).toBe('variables');
    expect(result.docFieldConflicts[0].path).toBe('variables.color-1');
  });

  // 24
  it('auto-merges themes when only one side adds a theme axis', () => {
    const baseDoc: PenDocument = {
      version: '1.0.0',
      pages: [{ id: 'p', name: 'p', children: [] }],
      children: [],
      themes: { mode: ['light', 'dark'] },
    };
    const oursDoc: PenDocument = {
      ...baseDoc,
      themes: { mode: ['light', 'dark'], density: ['compact', 'comfortable'] },
    };
    const theirsDoc: PenDocument = { ...baseDoc };
    const result = mergeDocuments({ base: baseDoc, ours: oursDoc, theirs: theirsDoc });
    expect(result.docFieldConflicts).toEqual([]);
    expect(result.merged.themes?.['density']).toEqual(['compact', 'comfortable']);
  });

  // 25
  it('emits pages-order conflict when both sides reorder pages differently', () => {
    const baseDoc: PenDocument = {
      version: '1.0.0',
      pages: [
        { id: 'p1', name: 'p1', children: [] },
        { id: 'p2', name: 'p2', children: [] },
        { id: 'p3', name: 'p3', children: [] },
      ],
      children: [],
    };
    const oursDoc: PenDocument = {
      ...baseDoc,
      pages: [
        { id: 'p2', name: 'p2', children: [] },
        { id: 'p1', name: 'p1', children: [] },
        { id: 'p3', name: 'p3', children: [] },
      ],
    };
    const theirsDoc: PenDocument = {
      ...baseDoc,
      pages: [
        { id: 'p3', name: 'p3', children: [] },
        { id: 'p1', name: 'p1', children: [] },
        { id: 'p2', name: 'p2', children: [] },
      ],
    };
    const result = mergeDocuments({ base: baseDoc, ours: oursDoc, theirs: theirsDoc });
    const pageOrderConflicts = result.docFieldConflicts.filter((c) => c.field === 'pages-order');
    expect(pageOrderConflicts).toHaveLength(1);
  });

  // 26
  it('handles non-overlapping field changes on the same node (ours width, theirs height)', () => {
    const base = doc([rect('r1', { width: 10, height: 10 })]);
    const ours = doc([rect('r1', { width: 20, height: 10 })]);
    const theirs = doc([rect('r1', { width: 10, height: 30 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    const r1 = findNode(result.merged, 'r1') as { width: number; height: number };
    expect(r1.width).toBe(20);
    expect(r1.height).toBe(30);
  });

  // 27
  it('handles legacy children mode (no pages array) on all three sides', () => {
    const base = docLegacy([rect('r1', { width: 10 })]);
    const ours = docLegacy([rect('r1', { width: 20 })]);
    const theirs = docLegacy([rect('r1', { width: 10 })]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts).toEqual([]);
    expect(result.merged.children).toHaveLength(1);
    expect((result.merged.children[0] as { width: number }).width).toBe(20);
  });

  // 28
  it('moving a node across pages auto-merges when only one side moves', () => {
    const base = docMulti([
      { id: 'p1', children: [rect('r1')] },
      { id: 'p2', children: [] },
    ]);
    const ours = docMulti([
      { id: 'p1', children: [] },
      { id: 'p2', children: [rect('r1')] },
    ]);
    const theirs = docMulti([
      { id: 'p1', children: [rect('r1')] },
      { id: 'p2', children: [] },
    ]);
    const result = mergeDocuments({ base, ours, theirs });
    expect(result.nodeConflicts.filter((c) => c.reason === 'reparent-conflict')).toHaveLength(0);
    // r1 should now be on p2
    const p1 = result.merged.pages!.find((p) => p.id === 'p1');
    const p2 = result.merged.pages!.find((p) => p.id === 'p2');
    expect(p1!.children).toHaveLength(0);
    expect(p2!.children).toHaveLength(1);
    expect(p2!.children[0].id).toBe('r1');
  });
});
