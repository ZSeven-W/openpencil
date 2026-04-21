import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { insertElementTree, ensureParentExists } from '../tools/element-tool-helpers';
import { invalidateCache } from '../document-manager';

/**
 * Direct unit tests for `insertElementTree` — previously only
 * covered indirectly through handler integration tests. Pins the
 * safety invariants listed in the function's JSDoc: parent_id
 * quote-safety pre-check, rollback on error + bytes-exact file
 * restoration, post-insert verification catches silent-no-op paths,
 * wrong-parent-landing rollback.
 */

const TMP = join(tmpdir(), 'openpencil-insert-element-tree');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['a.op', 'quoted.op', 'backslash.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('insertElementTree — parent_id safety pre-check', () => {
  it('rejects parent_id containing literal quote (cannot round-trip DSL)', async () => {
    const fp = await fresh('quoted.op');
    await expect(
      insertElementTree({
        binding: 'x',
        tree: { type: 'frame', name: 'X', width: 10, height: 10 },
        parent_id: 'weird"id',
        filePath: fp,
      }),
    ).rejects.toThrow(/cannot be safely passed through batch_design's DSL parser/);
  });

  it('rejects parent_id containing backslash (same reason)', async () => {
    const fp = await fresh('backslash.op');
    await expect(
      insertElementTree({
        binding: 'x',
        tree: { type: 'frame', name: 'X', width: 10, height: 10 },
        parent_id: 'weird\\id',
        filePath: fp,
      }),
    ).rejects.toThrow(/cannot be safely passed/);
  });

  it('pre-check runs BEFORE any write — file bytes untouched on rejection', async () => {
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      insertElementTree({
        binding: 'x',
        tree: { type: 'frame', name: 'X', width: 10, height: 10 },
        parent_id: 'weird"id',
        filePath: fp,
      }),
    ).rejects.toThrow();
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});

describe('insertElementTree — post-insert verification', () => {
  it('rollback + throw when parent_id names a node that does not exist AT insert time', async () => {
    // `ensureParentExists` prevents this in the normal flow, but if a
    // handler skips that guard the post-insert check must still catch
    // the silent no-op. Here we call insertElementTree directly with a
    // parent_id that never existed.
    const fp = await fresh('a.op');
    const before = await readFile(fp, 'utf-8');
    await expect(
      insertElementTree({
        binding: 'x',
        tree: { type: 'frame', name: 'X', width: 10, height: 10 },
        parent_id: 'never-existed',
        filePath: fp,
      }),
    ).rejects.toThrow();
    // File restored to pre-insert bytes
    const after = await readFile(fp, 'utf-8');
    expect(after).toBe(before);
  });
});

describe('insertElementTree — happy path', () => {
  it('inserts tree at root when no parent_id + returns handleBatchDesign result', async () => {
    const fp = await fresh('a.op');
    const result = await insertElementTree({
      binding: 'root',
      tree: {
        type: 'frame',
        name: 'Root',
        width: 400,
        height: 300,
        layout: 'vertical',
        children: [],
      },
      filePath: fp,
    });
    expect(result.results.length).toBe(1);
    expect(result.results[0].nodeId).toBeTruthy();
    // Disk state reflects insert
    const saved = JSON.parse(await readFile(fp, 'utf-8')) as {
      children?: Array<{ name: string }>;
      pages?: Array<{ children: Array<{ name: string }> }>;
    };
    const rootChildren = saved.children ?? saved.pages?.[0]?.children ?? [];
    expect(rootChildren.length).toBeGreaterThan(0);
  });

  it('inserts tree under an existing parent + returns nodeId', async () => {
    // Seed doc with a named parent
    const fp = join(TMP, 'a.op');
    await writeFile(
      fp,
      JSON.stringify({
        version: '1.0.0',
        children: [
          {
            id: 'container-1',
            type: 'frame',
            name: 'Container',
            width: 400,
            height: 300,
            layout: 'vertical',
            children: [],
          },
        ],
      }),
      'utf-8',
    );
    const result = await insertElementTree({
      binding: 'child',
      tree: { type: 'text', name: 'Child', content: 'Hello' },
      parent_id: 'container-1',
      filePath: fp,
    });
    expect(result.results.length).toBe(1);
    // Verify the child landed under container-1, not at root
    const saved = JSON.parse(await readFile(fp, 'utf-8')) as {
      children: Array<{ id: string; children?: Array<{ id: string }> }>;
    };
    expect(saved.children[0].id).toBe('container-1');
    expect(saved.children[0].children?.length).toBe(1);
    expect(saved.children[0].children?.[0].id).toBe(result.results[0].nodeId);
  });
});

describe('ensureParentExists (sibling coverage)', () => {
  it('no parent_id → no-op (allows root insertion)', async () => {
    const fp = await fresh('a.op');
    await expect(ensureParentExists({ filePath: fp })).resolves.toBeUndefined();
  });

  it('missing parent_id → throws actionable error', async () => {
    const fp = await fresh('a.op');
    await expect(ensureParentExists({ filePath: fp, parent_id: 'not-there' })).rejects.toThrow(
      /parent_id "not-there" not found/,
    );
  });
});
