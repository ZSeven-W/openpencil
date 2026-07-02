// packages/pen-core/src/__tests__/merge-helpers.test.ts
import { describe, it, expect } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import {
  indexNodesById,
  getAllPages,
  nodeFieldsEqual,
  jsonEqual,
  stripChildren,
} from '../merge/merge-helpers';

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

const docWithPages = (pages: Array<{ id: string; children: PenNode[] }>): PenDocument => ({
  version: '1.0.0',
  pages: pages.map((p) => ({ id: p.id, name: p.id, children: p.children })),
  children: [],
});

const docLegacy = (children: PenNode[]): PenDocument => ({
  version: '1.0.0',
  children,
});

describe('merge-helpers', () => {
  describe('indexNodesById', () => {
    it('indexes a flat top-level node with pageId and null parentId', () => {
      const doc = docWithPages([{ id: 'page-1', children: [rect('r1')] }]);
      const idx = indexNodesById(doc);
      expect(idx.size).toBe(1);
      const r1 = idx.get('r1');
      expect(r1).toBeDefined();
      expect(r1!.pageId).toBe('page-1');
      expect(r1!.parentId).toBeNull();
      expect(r1!.index).toBe(0);
    });

    it('indexes nested nodes with the correct parent id', () => {
      const inner = rect('inner');
      const outer = frame('outer', [inner]);
      const doc = docWithPages([{ id: 'page-1', children: [outer] }]);
      const idx = indexNodesById(doc);
      expect(idx.size).toBe(2);
      expect(idx.get('outer')!.parentId).toBeNull();
      expect(idx.get('inner')!.parentId).toBe('outer');
      expect(idx.get('inner')!.pageId).toBe('page-1');
    });

    it('uses pageId === null for legacy single-page documents', () => {
      const doc = docLegacy([rect('r1')]);
      const idx = indexNodesById(doc);
      const r1 = idx.get('r1');
      expect(r1!.pageId).toBeNull();
      expect(r1!.parentId).toBeNull();
    });

    it('records correct index within parent.children', () => {
      const doc = docWithPages([{ id: 'page-1', children: [rect('a'), rect('b'), rect('c')] }]);
      const idx = indexNodesById(doc);
      expect(idx.get('a')!.index).toBe(0);
      expect(idx.get('b')!.index).toBe(1);
      expect(idx.get('c')!.index).toBe(2);
    });
  });

  describe('getAllPages', () => {
    it('returns explicit pages when the document has them', () => {
      const doc = docWithPages([
        { id: 'p1', children: [rect('a')] },
        { id: 'p2', children: [rect('b')] },
      ]);
      const pages = getAllPages(doc);
      expect(pages).toHaveLength(2);
      expect(pages[0].id).toBe('p1');
      expect(pages[1].id).toBe('p2');
    });

    it('returns one synthetic page with id null for legacy documents', () => {
      const doc = docLegacy([rect('r1'), rect('r2')]);
      const pages = getAllPages(doc);
      expect(pages).toHaveLength(1);
      expect(pages[0].id).toBeNull();
      expect(pages[0].children).toHaveLength(2);
    });
  });

  describe('nodeFieldsEqual / stripChildren', () => {
    it('returns true for two nodes that differ only in children', () => {
      const a = frame('f', [rect('a')]);
      const b = frame('f', [rect('b'), rect('c')]);
      expect(nodeFieldsEqual(a, b)).toBe(true);
    });

    it('returns false when an atomic field differs', () => {
      const a = rect('r', { width: 10 });
      const b = rect('r', { width: 20 });
      expect(nodeFieldsEqual(a, b)).toBe(false);
    });

    it('stripChildren removes the children field without mutating the input', () => {
      const original = frame('f', [rect('child')]);
      const stripped = stripChildren(original);
      expect((stripped as { children?: unknown }).children).toBeUndefined();
      // Original is untouched
      expect((original as { children?: unknown }).children).toBeDefined();
    });
  });

  describe('jsonEqual', () => {
    it('returns true for primitives that are ===', () => {
      expect(jsonEqual(1, 1)).toBe(true);
      expect(jsonEqual('a', 'a')).toBe(true);
      expect(jsonEqual(null, null)).toBe(true);
    });

    it('returns true for deeply equal objects regardless of key order', () => {
      const a = { x: 1, y: 2 };
      const b = { y: 2, x: 1 };
      expect(jsonEqual(a, b)).toBe(true);
    });

    it('returns false for objects with different values', () => {
      expect(jsonEqual({ x: 1 }, { x: 2 })).toBe(false);
    });

    it('returns true for nested objects with same content', () => {
      expect(jsonEqual({ a: { b: [1, 2] } }, { a: { b: [1, 2] } })).toBe(true);
    });
  });
});
