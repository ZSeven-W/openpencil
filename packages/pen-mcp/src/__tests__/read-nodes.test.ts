import { describe, it, expect } from 'vitest';
import { handleReadNodes } from '../tools/read-nodes';
import type { PenDocument } from '@zseven-w/pen-types';

const mockDoc: PenDocument = {
  version: '0.6.0',
  children: [
    {
      id: 'f1',
      type: 'frame',
      name: 'Hero',
      x: 0,
      y: 0,
      width: 800,
      height: 600,
      children: [
        { id: 'r1', type: 'rectangle', name: 'BG', x: 0, y: 0, width: 800, height: 600 },
        {
          id: 'f2',
          type: 'frame',
          name: 'Content',
          x: 100,
          y: 100,
          width: 600,
          height: 400,
          children: [{ id: 't1', type: 'text', name: 'Title', x: 0, y: 0, width: 200, height: 40 }],
        },
      ],
    },
  ],
  variables: { 'color-1': { type: 'color', value: '#FF0000' } },
} as unknown as PenDocument;

describe('handleReadNodes', () => {
  it('returns all page children when nodeIds omitted, depth=-1', async () => {
    const result = await handleReadNodes({}, mockDoc);
    expect(result.nodes).toHaveLength(1);
    expect(result.nodes[0].id).toBe('f1');
    expect(Array.isArray(result.nodes[0].children)).toBe(true);
  });

  it('returns depth-limited nodes with depth=0', async () => {
    const result = await handleReadNodes({ nodeIds: ['f1'], depth: 0 }, mockDoc);
    expect(result.nodes).toHaveLength(1);
    expect(result.nodes[0].children).toBe('...');
  });

  it('returns depth=1 with direct children only', async () => {
    const result = await handleReadNodes({ nodeIds: ['f1'], depth: 1 }, mockDoc);
    expect(result.nodes).toHaveLength(1);
    const children = result.nodes[0].children;
    expect(Array.isArray(children)).toBe(true);
    if (Array.isArray(children)) {
      expect(children).toHaveLength(2);
      // f2's children should be truncated at depth 1
      const f2 = children.find((c: any) => c.id === 'f2');
      expect(f2?.children).toBe('...');
    }
  });

  it('includes variables when includeVariables=true', async () => {
    const result = await handleReadNodes({ includeVariables: true }, mockDoc);
    expect(result.variables).toBeDefined();
    expect(result.variables!['color-1']).toBeDefined();
  });

  it('omits variables by default', async () => {
    const result = await handleReadNodes({}, mockDoc);
    expect(result.variables).toBeUndefined();
  });

  it('filters by nodeIds', async () => {
    const result = await handleReadNodes({ nodeIds: ['r1'] }, mockDoc);
    expect(result.nodes).toHaveLength(1);
    expect(result.nodes[0].id).toBe('r1');
  });

  it('returns empty for unknown nodeIds', async () => {
    const result = await handleReadNodes({ nodeIds: ['unknown'] }, mockDoc);
    expect(result.nodes).toHaveLength(0);
  });
});
