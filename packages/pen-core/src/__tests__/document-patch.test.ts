import { describe, expect, it } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import type { NodePatch } from '../merge/node-diff';
import { applyDocumentPatches, diffPenDocuments } from '../merge/document-patch';

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

const doc = (children: PenNode[], name = 'Design'): PenDocument => ({
  version: '1.0.0',
  name,
  pages: [{ id: 'page-1', name: 'Page 1', children }],
  children: [],
});

describe('document patches', () => {
  it('round-trips node add, remove, modify, move, and reorder changes', () => {
    const base = doc([frame('a', [rect('b')]), rect('c'), rect('remove-me')]);
    const next = doc([
      frame('a', [rect('new-child', { width: 40 })]),
      rect('c', { width: 20 }),
      frame('new-frame', [rect('b', { x: 12 })]),
    ]);

    const patches = diffPenDocuments(base, next);

    expect(patches.find((patch) => patch.op === 'remove' && patch.nodeId === 'remove-me')).toBeDefined();
    expect(patches.find((patch) => patch.op === 'modify' && patch.nodeId === 'c')).toBeDefined();
    expect(patches.find((patch) => patch.op === 'move' && patch.nodeId === 'b')).toBeDefined();
    expect(applyDocumentPatches(base, patches)).toEqual(next);
  });

  it('includes document field patches for name, variables, themes, and version', () => {
    const base = doc([]);
    const next: PenDocument = {
      ...base,
      name: 'Renamed',
      version: '1.1.0',
      variables: {
        accent: { type: 'color', value: '#ff0000' },
      },
      themes: {
        mode: ['light', 'dark'],
      },
    };

    const patches = diffPenDocuments(base, next);

    expect(patches.filter((patch) => patch.op === 'set-doc-field').map((patch) => patch.field)).toEqual([
      'name',
      'version',
      'variables',
      'themes',
    ]);
    expect(applyDocumentPatches(base, patches)).toEqual(next);
  });

  it('applies page metadata rename, creation, deletion, and reorder', () => {
    const base = doc([rect('a')]);
    const next: PenDocument = {
      ...base,
      pages: [
        { id: 'page-2', name: 'Cover', children: [] },
        { id: 'page-1', name: 'Canvas', children: [rect('a')] },
      ],
    };

    const patches = diffPenDocuments(base, next);

    expect(patches.find((patch) => patch.op === 'set-pages-meta')).toEqual({
      op: 'set-pages-meta',
      pages: [
        { id: 'page-2', name: 'Cover' },
        { id: 'page-1', name: 'Canvas' },
      ],
      before: [{ id: 'page-1', name: 'Page 1' }],
    });
    expect(applyDocumentPatches(base, patches)).toEqual(next);
  });

  it('applies shuffled patches in a stable order', () => {
    const base = doc([frame('parent', []), rect('old')]);
    const next = doc([
      frame('parent', [frame('child-parent', [rect('leaf', { width: 22 })])]),
      rect('old', { width: 50 }),
    ]);
    const shuffled = diffPenDocuments(base, next).reverse();

    expect(applyDocumentPatches(base, shuffled)).toEqual(next);
  });

  it('round-trips top-level reorder-only changes', () => {
    const base = doc([rect('a'), rect('b'), rect('c')]);
    const next = doc([rect('b'), rect('a'), rect('c')]);

    expect(applyDocumentPatches(base, diffPenDocuments(base, next))).toEqual(next);
  });

  it('collapses added subtrees into a single node patch', () => {
    const base = doc([]);
    const next = doc([frame('root', [frame('nested', [rect('leaf')])])]);

    const addPatches = diffPenDocuments(base, next).filter(
      (patch): patch is NodePatch => patch.op === 'add',
    );

    expect(addPatches).toHaveLength(1);
    const addPatch = addPatches[0];
    expect(addPatch.op).toBe('add');
    expect((addPatch.fields as PenNode & { children?: PenNode[] }).children?.[0].id).toBe('nested');
    expect(applyDocumentPatches(base, addPatches)).toEqual(next);
  });

  it('rejects invalid parent targets and cycles', () => {
    const base = doc([frame('parent', [frame('child', [])])]);

    expect(() =>
      applyDocumentPatches(base, [
        {
          op: 'move',
          pageId: 'page-1',
          nodeId: 'parent',
          parentId: 'child',
          index: 0,
        },
      ]),
    ).toThrow(/descendant/);

    expect(() =>
      applyDocumentPatches(base, [
        {
          op: 'add',
          pageId: 'page-1',
          nodeId: 'bad',
          parentId: 'missing-parent',
          index: 0,
          fields: rect('bad') as Partial<PenNode>,
        },
      ]),
    ).toThrow(/Parent node not found/);
  });
});
