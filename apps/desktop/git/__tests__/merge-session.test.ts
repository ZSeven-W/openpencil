// apps/desktop/git/__tests__/merge-session.test.ts
import { describe, it, expect } from 'vitest';
import {
  encodeNodeConflictId,
  encodeDocFieldConflictId,
  parseConflictId,
  buildConflictBag,
} from '../merge-session';
import type { NodeConflict, DocFieldConflict, MergeResult } from '@zseven-w/pen-core';

describe('conflict id codec', () => {
  it('encodes node conflicts with the page-id placeholder for legacy single-page docs', () => {
    const a: NodeConflict = {
      pageId: 'page-1',
      nodeId: 'rect-7',
      reason: 'both-modified-same-field',
      base: null,
      ours: null,
      theirs: null,
    };
    expect(encodeNodeConflictId(a)).toBe('node:page-1:rect-7');

    const b: NodeConflict = {
      pageId: null,
      nodeId: 'frame-root',
      reason: 'both-modified-same-field',
      base: null,
      ours: null,
      theirs: null,
    };
    expect(encodeNodeConflictId(b)).toBe('node:_:frame-root');
  });

  it('encodes doc-field conflicts with field:path format', () => {
    const a: DocFieldConflict = {
      field: 'variables',
      path: 'variables.color-1.value',
      base: null,
      ours: null,
      theirs: null,
    };
    expect(encodeDocFieldConflictId(a)).toBe('field:variables:variables.color-1.value');
  });

  it('parses node and field ids back into structured form', () => {
    expect(parseConflictId('node:page-1:rect-7')).toEqual({
      kind: 'node',
      pageId: 'page-1',
      nodeId: 'rect-7',
    });
    expect(parseConflictId('node:_:frame-root')).toEqual({
      kind: 'node',
      pageId: null,
      nodeId: 'frame-root',
    });
    expect(parseConflictId('field:variables:variables.color-1.value')).toEqual({
      kind: 'field',
      field: 'variables',
      path: 'variables.color-1.value',
    });
  });

  it('buildConflictBag attaches ids and builds the lookup map in one pass', () => {
    const merged: MergeResult = {
      merged: { version: '1.0.0', name: 'doc', children: [] },
      nodeConflicts: [
        {
          pageId: null,
          nodeId: 'frame-root',
          reason: 'both-modified-same-field',
          base: null,
          ours: null,
          theirs: null,
        },
      ],
      docFieldConflicts: [
        {
          field: 'variables',
          path: 'variables.color-1.value',
          base: 'red',
          ours: 'blue',
          theirs: 'green',
        },
      ],
    };
    const { bag, conflictMap } = buildConflictBag(merged);
    expect(bag.nodeConflicts).toHaveLength(1);
    expect(bag.nodeConflicts[0].id).toBe('node:_:frame-root');
    expect(bag.docFieldConflicts[0].id).toBe('field:variables:variables.color-1.value');
    expect(conflictMap.size).toBe(2);
    expect(conflictMap.get('node:_:frame-root')).toBeDefined();
    expect(conflictMap.get('field:variables:variables.color-1.value')).toBeDefined();
  });
});
