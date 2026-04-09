import { describe, it, expect } from 'vitest';
import { readNodeWithDepth } from '../utils/node-operations';
import type { PenNode, PenDocument } from '@zseven-w/pen-types';

describe('readNodeWithDepth', () => {
  it('returns raw $ref values when resolveRefs is not set', () => {
    const node: PenNode = {
      id: 'n',
      type: 'frame',
      fill: [{ type: 'solid', color: '$color-1' }],
      children: [],
    } as unknown as PenNode;
    const out = readNodeWithDepth(node, 1);
    expect((out.fill as Array<{ color: string }>)[0].color).toBe('$color-1');
  });

  it('resolves $ref values when resolveRefs=true', () => {
    const node: PenNode = {
      id: 'n',
      type: 'frame',
      fill: [{ type: 'solid', color: '$color-1' }],
      children: [],
    } as unknown as PenNode;
    const doc: PenDocument = {
      version: '1.0.0',
      variables: { 'color-1': { type: 'color', value: '#FAFAFA' } },
      children: [],
    } as unknown as PenDocument;
    const out = readNodeWithDepth(node, 1, { resolveRefs: true, doc });
    expect((out.fill as Array<{ color: string }>)[0].color).toBe('#FAFAFA');
  });

  it('preserves depth-truncation behavior with resolveRefs', () => {
    const node: PenNode = {
      id: 'n',
      type: 'frame',
      children: [{ id: 'c', type: 'text', text: 'x' } as unknown as PenNode],
    } as unknown as PenNode;
    const doc: PenDocument = {
      version: '1.0.0',
      variables: {},
      children: [],
    } as unknown as PenDocument;
    const out = readNodeWithDepth(node, 0, { resolveRefs: true, doc });
    expect(out.children).toBe('...');
  });
});
