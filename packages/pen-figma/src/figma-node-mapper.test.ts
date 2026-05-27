import { describe, expect, it } from 'vitest';
import {
  figmaToPenDocument,
  getFigmaPageLayers,
  getFigmaPages,
  type FigmaLayerSummary,
} from './figma-node-mapper';
import type { FigmaDecodedFile, FigmaGUID, FigmaNodeChange } from './figma-types';

function guid(sessionID: number, localID: number): FigmaGUID {
  return { sessionID, localID };
}

function node(
  localID: number,
  type: FigmaNodeChange['type'],
  name: string,
  parent?: FigmaGUID,
  position = `p${localID}`,
  children: Partial<FigmaNodeChange> = {},
): FigmaNodeChange {
  return {
    guid: guid(1, localID),
    type,
    name,
    parentIndex: parent ? { guid: parent, position } : undefined,
    size: type === 'DOCUMENT' || type === 'CANVAS' ? undefined : { x: 100, y: 80 },
    transform:
      type === 'DOCUMENT' || type === 'CANVAS'
        ? undefined
        : { m00: 1, m01: 0, m02: localID * 10, m10: 0, m11: 1, m12: localID * 5 },
    ...children,
  };
}

function decodedFile(): FigmaDecodedFile {
  const documentGuid = guid(1, 1);
  const pageGuid = guid(1, 2);
  const frameGuid = guid(1, 3);
  const groupGuid = guid(1, 4);

  return {
    blobs: [],
    imageFiles: new Map(),
    nodeChanges: [
      node(1, 'DOCUMENT', 'Document'),
      node(2, 'CANVAS', 'Page A', documentGuid, 'a'),
      node(3, 'FRAME', 'Checkout flow', pageGuid, 'c'),
      node(4, 'GROUP', 'Marketing leftovers', pageGuid, 'b'),
      node(5, 'RECTANGLE', 'Primary CTA', frameGuid),
      node(6, 'RECTANGLE', 'Unused mock', groupGuid),
    ],
  };
}

describe('figma document mapping', () => {
  it('summarizes top-level layers for a page before conversion', () => {
    expect(getFigmaPages(decodedFile())).toEqual([
      {
        id: '1:2',
        name: 'Page A',
        childCount: 2,
      },
    ]);

    expect(getFigmaPageLayers(decodedFile(), 0)).toEqual<FigmaLayerSummary[]>([
      {
        id: '1:3',
        name: 'Checkout flow',
        type: 'FRAME',
        childCount: 1,
        visible: true,
      },
      {
        id: '1:4',
        name: 'Marketing leftovers',
        type: 'GROUP',
        childCount: 1,
        visible: true,
      },
    ]);
  });

  it('converts only selected top-level layers when layer ids are provided', () => {
    const { document } = figmaToPenDocument(decodedFile(), 'selective', 0, 'preserve', {
      topLevelNodeIds: ['1:3'],
    });

    const children = document.pages?.[0]?.children ?? [];
    expect(children).toHaveLength(1);
    expect(children[0]).toMatchObject({
      type: 'frame',
      name: 'Checkout flow',
    });
    expect(JSON.stringify(document)).toContain('Primary CTA');
    expect(JSON.stringify(document)).not.toContain('Marketing leftovers');
    expect(JSON.stringify(document)).not.toContain('Unused mock');
  });

  it('keeps full page conversion when no layer selection is provided', () => {
    const { document } = figmaToPenDocument(decodedFile(), 'full', 0, 'preserve');

    const children = document.pages?.[0]?.children ?? [];
    expect(children.map((child) => child.name)).toEqual(['Checkout flow', 'Marketing leftovers']);
  });

  it('treats an explicit empty layer selection as importing no top-level layers', () => {
    const { document } = figmaToPenDocument(decodedFile(), 'empty', 0, 'preserve', {
      topLevelNodeIds: [],
    });

    expect(document.pages?.[0]?.children ?? []).toEqual([]);
  });
});
