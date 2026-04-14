import { describe, expect, it } from 'vitest';
import type { ContainerProps, FrameNode, PenDocument, PenNode } from '@/types/pen';
import { DEFAULT_PAGE_ID, findNodeInTree } from './document-tree-utils';
import { getNodeVisualPosition, moveNodePreservingVisualPosition } from './document-position-utils';

const rect = (id: string, x?: number, y?: number): PenNode => ({
  id,
  type: 'rectangle',
  x,
  y,
  width: 50,
  height: 20,
});

const frame = (
  id: string,
  props: Partial<FrameNode & ContainerProps> & { children?: PenNode[] } = {},
): PenNode => ({
  id,
  type: 'frame',
  x: 0,
  y: 0,
  width: 200,
  height: 200,
  ...props,
});

describe('document-position-utils', () => {
  it('reads visual position from auto-layout, not raw child x/y', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      pages: [
        {
          id: DEFAULT_PAGE_ID,
          name: 'Page 1',
          children: [
            frame('parent', {
              x: 100,
              y: 50,
              layout: 'vertical',
              padding: [10, 20],
              children: [rect('child', 0, 0)],
            }),
          ],
        },
      ],
    };

    expect(getNodeVisualPosition(doc, DEFAULT_PAGE_ID, 'child')).toEqual({ x: 120, y: 60 });
  });

  it('preserves visual position when moving a layout-positioned child to root', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      pages: [
        {
          id: DEFAULT_PAGE_ID,
          name: 'Page 1',
          children: [
            frame('parent', {
              x: 100,
              y: 50,
              layout: 'vertical',
              padding: [10, 20],
              children: [rect('child', 0, 0)],
            }),
            rect('root-sibling', 300, 300),
          ],
        },
      ],
    };

    const movedChildren = moveNodePreservingVisualPosition(doc, DEFAULT_PAGE_ID, 'child', null, 1);

    expect(movedChildren).toBeDefined();
    const moved = findNodeInTree(movedChildren ?? [], 'child');
    expect(moved?.x).toBe(120);
    expect(moved?.y).toBe(60);
  });

  it('promotes a root frame to explicit clipContent when nesting it', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [],
      pages: [
        {
          id: DEFAULT_PAGE_ID,
          name: 'Page 1',
          children: [
            frame('outer', {
              x: 300,
              y: 100,
              children: [],
            }),
            frame('root-frame', {
              x: 20,
              y: 30,
              cornerRadius: 16,
              children: [rect('child', 0, 0)],
            }),
          ],
        },
      ],
    };

    const movedChildren = moveNodePreservingVisualPosition(
      doc,
      DEFAULT_PAGE_ID,
      'root-frame',
      'outer',
      0,
    );

    const moved = findNodeInTree(movedChildren ?? [], 'root-frame') as PenNode | undefined;
    expect(moved).toBeDefined();
    expect(moved && 'clipContent' in moved ? moved.clipContent : undefined).toBe(true);
  });
});
