import { describe, expect, it, vi } from 'vitest';

vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({
      getNodeById: () => undefined,
      updateNode: () => {},
    }),
  },
}));

import type { PenNode } from '@zseven-w/pen-types';
import { resolveTreePostPass, resolveTreeRoles } from '../role-resolver';
import '../role-definitions/index';

describe('icon role defaults', () => {
  it('centers single-child icon frames so weaker models do not leave icons top-left', () => {
    const iconContainer = {
      id: 'icon-wrap',
      type: 'frame',
      role: 'icon',
      width: 44,
      height: 44,
      children: [
        {
          id: 'icon',
          type: 'icon_font',
          iconFontName: 'activity',
          width: 22,
          height: 22,
        },
      ],
    } as unknown as PenNode;

    resolveTreeRoles(iconContainer, 375);

    const frame = iconContainer as PenNode & {
      layout?: string;
      alignItems?: string;
      justifyContent?: string;
    };

    expect(frame.layout).toBe('horizontal');
    expect(frame.alignItems).toBe('center');
    expect(frame.justifyContent).toBe('center');
  });

  it('repairs placeholder circle icons using nearby semantic text', () => {
    const card = {
      id: 'card',
      type: 'frame',
      layout: 'horizontal',
      children: [
        {
          id: 'icon-wrap',
          type: 'frame',
          role: 'icon',
          width: 48,
          height: 48,
          children: [
            {
              id: 'icon',
              type: 'path',
              name: 'WC1 Icon',
              d: 'M 2 12 a 10 10 0 1 0 20 0 a 10 10 0 1 0 -20 0 Z',
              width: 24,
              height: 24,
              fill: [],
              iconId: 'lucide:circle',
              stroke: {
                thickness: 2,
                fill: [{ type: 'solid', color: '#22C55E' }],
              },
            },
          ],
        },
        {
          id: 'info',
          type: 'frame',
          layout: 'vertical',
          children: [
            {
              id: 'title',
              type: 'text',
              content: 'Morning Run',
            },
          ],
        },
      ],
    } as unknown as PenNode;

    resolveTreeRoles(card, 375);
    resolveTreePostPass(card, 375);

    const icon = (
      (card as PenNode & { children: PenNode[] }).children[0] as PenNode & {
        children: PenNode[];
      }
    ).children[0] as PenNode & { iconId?: string };

    expect(icon.iconId).toBe('lucide:activity');
  });
});
