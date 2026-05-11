import { describe, expect, it } from 'vitest';
import { buildCodegenTarget } from '../codegen-history';

describe('buildCodegenTarget', () => {
  it('builds a page target when no nodes are selected', () => {
    const target = buildCodegenTarget({ pageId: 'page-1', selectedIds: [] });

    expect(target).toMatchObject({
      pageId: 'page-1',
      targetKind: 'page',
      nodeIds: [],
    });
    expect(target.targetHash).toHaveLength(8);
  });

  it('sorts selected node ids before hashing', () => {
    const a = buildCodegenTarget({ pageId: 'page-1', selectedIds: ['b', 'a'] });
    const b = buildCodegenTarget({ pageId: 'page-1', selectedIds: ['a', 'b'] });

    expect(a.nodeIds).toEqual(['a', 'b']);
    expect(a.targetHash).toBe(b.targetHash);
  });
});

