import { describe, it, expect } from 'vitest';
import { handleValidationReport } from '../tools/debug-validation-report';
import type { PenDocument } from '@zseven-w/pen-types';

describe('handleValidationReport', () => {
  it('returns empty result for an empty document', async () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{ id: 'root', type: 'frame', children: [] } as never],
    } as unknown as PenDocument;

    const json = await handleValidationReport({}, doc);
    const parsed = JSON.parse(json);
    expect(parsed.totalIssues).toBe(0);
    expect(parsed.issues).toEqual([]);
    expect(parsed.byCategory).toEqual({});
  });

  it('reports invisible container issues without applying fixes', async () => {
    const doc: PenDocument = {
      version: '1.0.0',
      variables: {},
      children: [
        {
          id: 'root',
          type: 'frame',
          fill: [{ type: 'solid', color: '#FAFAFA' }],
          layout: 'vertical',
          children: [
            {
              id: 'card',
              type: 'frame',
              fill: [{ type: 'solid', color: '#FAFAFA' }],
              layout: 'horizontal',
              children: [{ id: 't', type: 'text', text: 'x' }],
            },
          ],
        } as never,
      ],
    } as unknown as PenDocument;

    const json = await handleValidationReport({}, doc);
    const parsed = JSON.parse(json);
    expect(parsed.totalIssues).toBeGreaterThanOrEqual(1);
    expect(parsed.byCategory['invisible-container']).toBeGreaterThanOrEqual(1);
    expect(parsed.issues[0]).toHaveProperty('nodeId');
    expect(parsed.issues[0]).toHaveProperty('category');
    expect(parsed.issues[0]).toHaveProperty('reason');
  });

  it('respects the maxIssues cap', async () => {
    const doc: PenDocument = {
      version: '1.0.0',
      variables: {},
      children: [
        {
          id: 'root',
          type: 'frame',
          children: Array.from({ length: 10 }, (_, i) => ({
            id: `p${i}`,
            type: 'path',
          })),
        } as never,
      ],
    } as unknown as PenDocument;

    const json = await handleValidationReport({ maxIssues: 3 }, doc);
    const parsed = JSON.parse(json);
    expect(parsed.issues.length).toBeLessThanOrEqual(3);
  });
});
