// apps/desktop/git/__tests__/merge-orchestrator.test.ts
import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { mkTempDir, writeOpFile } from './test-helpers';
import { initSingleFile, commitFile, type IsoRepoHandle } from '../git-iso';
import { runMerge, applyResolutions } from '../merge-orchestrator';
import { buildConflictBag } from '../merge-session';
import type { PenDocument } from '@zseven-w/pen-types';
import { mergeDocuments } from '@zseven-w/pen-core';

const author = { name: 't', email: 't@example.com' };

async function commitDocument(
  handle: IsoRepoHandle,
  doc: PenDocument,
  ref: string,
  message: string,
  parents?: string[],
): Promise<string> {
  // Write to disk first (commitFile reads from disk, not from the input).
  const fsp = await import('node:fs/promises');
  const path = `${handle.dir}/login.op`;
  await fsp.writeFile(path, JSON.stringify(doc), 'utf-8');
  const { hash } = await commitFile({
    handle,
    filepath: 'login.op',
    ref,
    message,
    author,
    parents,
  });
  return hash;
}

describe('merge-orchestrator', () => {
  let temp: { dir: string; dispose: () => Promise<void> };
  let handle: IsoRepoHandle;

  beforeEach(async () => {
    temp = await mkTempDir();
    const opFile = await writeOpFile(temp.dir, 'login.op', {
      version: '1.0.0',
      name: 'login',
      children: [],
    });
    handle = await initSingleFile({ filePath: opFile });
  });

  afterEach(async () => {
    await temp.dispose();
  });

  it('runMerge produces an empty conflict bag for a clean merge', async () => {
    const base: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [{ id: 'rect-1', type: 'rectangle', x: 0, y: 0, width: 10, height: 10 } as never],
    };
    const ours: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [
        { id: 'rect-1', type: 'rectangle', x: 0, y: 0, width: 10, height: 10 } as never,
        { id: 'rect-2', type: 'rectangle', x: 20, y: 0, width: 10, height: 10 } as never,
      ],
    };
    const theirs: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [
        { id: 'rect-1', type: 'rectangle', x: 0, y: 0, width: 10, height: 10 } as never,
        { id: 'rect-3', type: 'rectangle', x: 40, y: 0, width: 10, height: 10 } as never,
      ],
    };

    const baseHash = await commitDocument(handle, base, 'refs/heads/main', 'base');
    const oursHash = await commitDocument(handle, ours, 'refs/heads/feature-a', 'ours', [baseHash]);
    const theirsHash = await commitDocument(handle, theirs, 'refs/heads/feature-b', 'theirs', [
      baseHash,
    ]);

    const merged = await runMerge({
      handle,
      filepath: 'login.op',
      oursCommit: oursHash,
      theirsCommit: theirsHash,
      baseCommit: baseHash,
    });

    expect(merged.bag.nodeConflicts).toHaveLength(0);
    expect(merged.bag.docFieldConflicts).toHaveLength(0);
    expect(merged.conflictMap.size).toBe(0);
  });

  it('runMerge surfaces a node conflict when both sides modify the same field', async () => {
    const base: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [
        {
          id: 'rect-1',
          type: 'rectangle',
          x: 0,
          y: 0,
          width: 10,
          height: 10,
          fill: [{ type: 'solid', color: '#ff0000' }],
        } as never,
      ],
    };
    const ours: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [
        {
          id: 'rect-1',
          type: 'rectangle',
          x: 0,
          y: 0,
          width: 10,
          height: 10,
          fill: [{ type: 'solid', color: '#00ff00' }],
        } as never,
      ],
    };
    const theirs: PenDocument = {
      version: '1.0.0',
      name: 'doc',
      children: [
        {
          id: 'rect-1',
          type: 'rectangle',
          x: 0,
          y: 0,
          width: 10,
          height: 10,
          fill: [{ type: 'solid', color: '#0000ff' }],
        } as never,
      ],
    };

    const baseHash = await commitDocument(handle, base, 'refs/heads/main', 'base');
    const oursHash = await commitDocument(handle, ours, 'refs/heads/feature-a', 'ours', [baseHash]);
    const theirsHash = await commitDocument(handle, theirs, 'refs/heads/feature-b', 'theirs', [
      baseHash,
    ]);

    const merged = await runMerge({
      handle,
      filepath: 'login.op',
      oursCommit: oursHash,
      theirsCommit: theirsHash,
      baseCommit: baseHash,
    });
    expect(merged.bag.nodeConflicts.length).toBeGreaterThan(0);
    const c = merged.bag.nodeConflicts[0];
    expect(c.id).toBe('node:_:rect-1');
    expect(c.reason).toBe('both-modified-same-field');
  });

  it('applyResolutions with kind=ours leaves the merged tree unchanged', () => {
    const base: PenDocument = { version: '1.0.0', name: 'd', children: [] };
    const ours: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 } as never],
    };
    const theirs: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 } as never],
    };
    const result = mergeDocuments({ base, ours, theirs });
    const { conflictMap } = buildConflictBag(result);

    const out = applyResolutions({
      merged: result.merged,
      conflictMap,
      resolutions: new Map([['node:_:a', { kind: 'ours' }]]),
    });
    // pen-core's merge places ours as the placeholder, so the result is
    // identical to ours.
    expect(((out.children ?? [])[0] as { x: number }).x).toBe(0);
  });

  it('applyResolutions with kind=theirs replaces the node with the theirs version', () => {
    const base: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 } as never],
    };
    const ours: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 } as never],
    };
    const theirs: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 9, y: 9, width: 1, height: 1 } as never],
    };
    const result = mergeDocuments({ base, ours, theirs });
    const { conflictMap } = buildConflictBag(result);
    expect(result.nodeConflicts.length).toBeGreaterThan(0);

    const out = applyResolutions({
      merged: result.merged,
      conflictMap,
      resolutions: new Map([['node:_:a', { kind: 'theirs' }]]),
    });
    expect(((out.children ?? [])[0] as { x: number }).x).toBe(9);
  });

  it('applyResolutions with kind=manual-node replaces the node with the user-supplied version', () => {
    const base: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 0, y: 0, width: 1, height: 1 } as never],
    };
    const ours: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 5, y: 5, width: 1, height: 1 } as never],
    };
    const theirs: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [{ id: 'a', type: 'rectangle', x: 9, y: 9, width: 1, height: 1 } as never],
    };
    const result = mergeDocuments({ base, ours, theirs });
    const { conflictMap } = buildConflictBag(result);

    const manualNode = {
      id: 'a',
      type: 'rectangle',
      x: 100,
      y: 100,
      width: 1,
      height: 1,
    } as never;
    const out = applyResolutions({
      merged: result.merged,
      conflictMap,
      resolutions: new Map([['node:_:a', { kind: 'manual-node', node: manualNode }]]),
    });
    expect(((out.children ?? [])[0] as { x: number }).x).toBe(100);
  });

  it('applyResolutions handles a doc-field conflict via setDocFieldByPath', () => {
    // Synthesize a doc-field conflict directly. We don't go through pen-core
    // because constructing a real variable conflict requires more PenDocument
    // boilerplate than this test needs to verify the path-set helper.
    const merged: PenDocument = {
      version: '1.0.0',
      name: 'd',
      children: [],
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ...({ variables: { 'color-1': { value: 'red' } } } as any),
    };
    const conflictMap = new Map([
      [
        'field:variables:variables.color-1.value',
        {
          field: 'variables' as const,
          path: 'variables.color-1.value',
          base: 'red',
          ours: 'green',
          theirs: 'blue',
        },
      ],
    ]);

    const out = applyResolutions({
      merged,
      conflictMap,
      resolutions: new Map([['field:variables:variables.color-1.value', { kind: 'theirs' }]]),
    });
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    expect((out as any).variables['color-1'].value).toBe('blue');
  });
});
