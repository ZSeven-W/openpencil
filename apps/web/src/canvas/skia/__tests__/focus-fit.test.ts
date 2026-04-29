import { describe, expect, it } from 'vitest';

import type { RenderNode } from '@zseven-w/pen-renderer';

import { fitSceneBoundsToViewport, getFocusBounds } from '../focus-fit';

function renderNode(
  partial: Partial<RenderNode> & { node: { id: string; type: string } },
): RenderNode {
  return {
    absX: 0,
    absY: 0,
    absW: 0,
    absH: 0,
    ...partial,
  } as RenderNode;
}

describe('focus fit helpers', () => {
  it('uses selected render-node bounds when selection exists', () => {
    const renderNodes = [
      renderNode({
        node: { id: 'page', type: 'frame' },
        absX: 0,
        absY: 0,
        absW: 1000,
        absH: 800,
      }),
      renderNode({
        node: { id: 'shape-1', type: 'rectangle' },
        absX: 120,
        absY: 80,
        absW: 240,
        absH: 100,
      }),
      renderNode({
        node: { id: 'shape-2', type: 'rectangle' },
        absX: 420,
        absY: 200,
        absW: 180,
        absH: 160,
      }),
    ];

    expect(getFocusBounds(renderNodes, ['shape-1', 'shape-2'])).toEqual({
      minX: 120,
      minY: 80,
      maxX: 600,
      maxY: 360,
    });
  });

  it('falls back to top-level content bounds when nothing is selected', () => {
    const renderNodes = [
      renderNode({
        node: { id: 'page', type: 'frame' },
        absX: 0,
        absY: 0,
        absW: 1000,
        absH: 800,
      }),
      renderNode({
        node: { id: 'child', type: 'rectangle' },
        absX: 80,
        absY: 60,
        absW: 120,
        absH: 90,
        clipRect: { x: 0, y: 0, w: 1000, h: 800, rx: 0 },
      }),
    ];

    expect(getFocusBounds(renderNodes, [])).toEqual({
      minX: 0,
      minY: 0,
      maxX: 1000,
      maxY: 800,
    });
  });

  it('computes a centered fit viewport and honors max zoom', () => {
    const viewport = fitSceneBoundsToViewport(
      {
        minX: 100,
        minY: 50,
        maxX: 300,
        maxY: 150,
      },
      1000,
      800,
      { padding: 100, maxZoom: 2 },
    );

    expect(viewport).toEqual({
      zoom: 2,
      panX: 100,
      panY: 200,
    });
  });
});
