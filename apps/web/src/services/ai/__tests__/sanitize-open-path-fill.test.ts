import { describe, expect, it } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { rewriteOpenStrokePathsWithDuplicateFill } from '../sanitize-llm-anti-patterns';

describe('rewriteOpenStrokePathsWithDuplicateFill', () => {
  it('drops duplicate fill from an open stroked line path', () => {
    const node = {
      id: 'chart',
      type: 'path',
      d: 'M0 45 L30 40 L60 42 L90 35',
      fill: [{ type: 'solid', color: '#22C55E' }],
      stroke: {
        thickness: 2,
        fill: [{ type: 'solid', color: '#22C55E' }],
      },
    } as unknown as PenNode;

    rewriteOpenStrokePathsWithDuplicateFill(node);

    expect((node as PenNode & { fill?: unknown }).fill).toBeUndefined();
  });

  it('keeps fill for a closed stroked shape', () => {
    const node = {
      id: 'shape',
      type: 'path',
      d: 'M0 0 L10 0 L10 10 Z',
      fill: [{ type: 'solid', color: '#22C55E' }],
      stroke: {
        thickness: 2,
        fill: [{ type: 'solid', color: '#22C55E' }],
      },
    } as unknown as PenNode;

    rewriteOpenStrokePathsWithDuplicateFill(node);

    expect((node as PenNode & { fill?: unknown }).fill).toEqual([
      { type: 'solid', color: '#22C55E' },
    ]);
  });
});
