import { describe, expect, it } from 'vitest';

import type { PenDocument } from '@/types/pen';
import { parseAndPrepareImportedDocument, prepareImportedDocument } from '../import-pen-document';

function getChildrenNames(node: PenDocument['children'][number]): string[] {
  if (!('children' in node) || !Array.isArray(node.children)) return [];
  return node.children.map((child) => child.name ?? '');
}

function getChildAt(
  node: PenDocument['children'][number],
  index: number,
): PenDocument['children'][number] | null {
  if (!('children' in node) || !Array.isArray(node.children)) return null;
  return node.children[index] ?? null;
}

function makeLegacyPen(): PenDocument {
  const legacyDoc = {
    version: '2.10',
    children: [
      {
        id: 'page-1',
        type: 'frame',
        name: 'Page 01',
        x: 0,
        y: 0,
        width: 600,
        height: 800,
        clip: true,
        children: [
          {
            id: 'content-1',
            type: 'frame',
            name: 'Content',
            x: 0,
            y: 0,
            width: 600,
            height: 800,
            children: [
              {
                id: 'bg-1',
                type: 'rectangle',
                name: 'Background',
                x: 0,
                y: 0,
                width: 600,
                height: 800,
                fill: [{ type: 'solid', color: '#ffffff' }],
              },
              {
                id: 'title-1',
                type: 'text',
                name: 'Title',
                x: 24,
                y: 24,
                content: 'Hello',
                fill: [{ type: 'solid', color: '#111111' }],
              },
            ],
          },
          {
            id: 'qa-1',
            type: 'frame',
            name: 'QA',
            x: 0,
            y: 0,
            width: 600,
            height: 800,
            children: [],
          },
        ],
      },
      {
        id: 'page-2',
        type: 'frame',
        name: 'Page 02',
        x: 0,
        y: 900,
        width: 600,
        height: 800,
        clip: true,
        children: [
          {
            id: 'content-2',
            type: 'frame',
            name: 'Content',
            x: 0,
            y: 0,
            width: 600,
            height: 800,
            children: [
              {
                id: 'bg-2',
                type: 'rectangle',
                name: 'Background',
                x: 0,
                y: 0,
                width: 600,
                height: 800,
                fill: [{ type: 'solid', color: '#eeeeee' }],
              },
              {
                id: 'title-2',
                type: 'text',
                name: 'Title',
                x: 24,
                y: 24,
                content: 'World',
                fill: [{ type: 'solid', color: '#222222' }],
              },
            ],
          },
        ],
      },
    ],
  };
  return legacyDoc as unknown as PenDocument;
}

describe('prepareImportedDocument', () => {
  it('forces legacy .pen compatibility for pencil-style page stacks', () => {
    const prepared = prepareImportedDocument(makeLegacyPen(), {
      fileName: 'layout.pen',
    });

    expect(prepared).not.toBeNull();
    expect(prepared!.appliedLegacyPenCompatibility).toBe(true);
    expect(prepared!.doc.children.map((node) => node.name)).toEqual(['Page 01', 'Page 02']);
    expect(getChildrenNames(prepared!.doc.children[0])).toEqual(['QA', 'Content']);
    const contentNode = getChildAt(prepared!.doc.children[0], 1);
    expect(contentNode).not.toBeNull();
    expect(getChildrenNames(contentNode!)).toEqual(['Title', 'Background']);
  });

  it('does not apply legacy stack reversal to .op files', () => {
    const prepared = prepareImportedDocument(makeLegacyPen(), {
      fileName: 'layout.op',
    });

    expect(prepared).not.toBeNull();
    expect(prepared!.appliedLegacyPenCompatibility).toBe(false);
    expect(getChildrenNames(prepared!.doc.children[0])).toEqual(['Content', 'QA']);
    const contentNode = getChildAt(prepared!.doc.children[0], 0);
    expect(contentNode).not.toBeNull();
    expect(getChildrenNames(contentNode!)).toEqual(['Background', 'Title']);
  });

  it('parses text input and skips compatibility once the document is already page-based', () => {
    const pageBasedDoc: PenDocument = {
      version: '2.10',
      pages: [
        {
          id: 'page-1',
          name: 'Page 1',
          children: [
            {
              id: 'shape-1',
              type: 'rectangle',
              x: 0,
              y: 0,
              width: 100,
              height: 100,
              fill: [{ type: 'solid', color: '#ffffff' }],
            },
          ],
        },
      ],
      children: [],
    };

    const prepared = parseAndPrepareImportedDocument(JSON.stringify(pageBasedDoc), {
      fileName: 'layout.pen',
    });

    expect(prepared).not.toBeNull();
    expect(prepared!.appliedLegacyPenCompatibility).toBe(false);
    expect(prepared!.doc.pages?.[0].children[0].id).toBe('shape-1');
  });
});
