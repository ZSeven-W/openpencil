import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, readFile, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { handleBatchDesign } from '../tools/batch-design';
import { invalidateCache } from '../document-manager';

/**
 * Large-scale stress test for batch_design. AI orchestrators
 * occasionally emit very large batches (one sub-agent produces a
 * whole section in a single call). This test pins three things:
 *
 *   1. 100+ sequential `I(...)` ops in a single batch don't throw,
 *      don't silently drop ops, and land all N nodes in the tree
 *   2. Wall-clock is bounded (<5s for 100 ops — each op is pure
 *      tree-mutation, no disk I/O between them)
 *   3. parent_id threading works across 10+ levels of nesting
 *      (the AI's `foo=I(...)` then `bar=I(foo, ...)` pattern)
 *
 * These are NOT perf benchmarks — the budget is "reasonable" not
 * "fast." If the wall-clock threshold trips we've probably
 * regressed the DSL parser (O(n²) scan), the insert-node path
 * (full tree walk per op), or the save loop (flushing to disk
 * after every op instead of once at the end).
 */

const TMP = join(tmpdir(), 'openpencil-batch-design-stress');
const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

async function fresh(name: string): Promise<string> {
  const fp = join(TMP, name);
  await writeFile(fp, EMPTY, 'utf-8');
  return fp;
}
async function readDoc(fp: string): Promise<Record<string, unknown>> {
  return JSON.parse(await readFile(fp, 'utf-8'));
}
function getRootChildren(doc: Record<string, unknown>): Record<string, unknown>[] {
  const pages = doc['pages'] as Array<{ children?: Record<string, unknown>[] }> | undefined;
  const top = doc['children'] as Record<string, unknown>[] | undefined;
  return top ?? pages?.[0]?.children ?? [];
}

function countNodes(nodes: Record<string, unknown>[]): number {
  let n = 0;
  for (const node of nodes) {
    n += 1;
    const kids = node.children as Record<string, unknown>[] | undefined;
    if (kids) n += countNodes(kids);
  }
  return n;
}

beforeEach(async () => {
  await mkdir(TMP, { recursive: true });
});
afterEach(async () => {
  for (const f of ['stress.op']) {
    try {
      const fp = join(TMP, f);
      invalidateCache(fp);
      await unlink(fp);
    } catch {}
  }
});

describe('batch_design — large-scale stress (100+ ops)', () => {
  it('100 sibling I() ops at root: all land, no silent drops', async () => {
    const fp = await fresh('stress.op');

    const ops: string[] = [];
    for (let i = 0; i < 100; i += 1) {
      const body = JSON.stringify({
        type: 'text',
        name: `Row ${i}`,
        role: 'body',
        content: `Row ${i} content`,
        fontSize: 14,
      });
      ops.push(`row${i}=I(null, ${body})`);
    }
    const dsl = ops.join('\n');

    const t0 = Date.now();
    const result = await handleBatchDesign({
      operations: dsl,
      filePath: fp,
      postProcess: false,
    });
    const elapsed = Date.now() - t0;

    expect(result.errors ?? []).toEqual([]);
    expect(result.results.length).toBe(100);
    // Every result should have a nodeId
    for (const r of result.results) {
      expect(r.nodeId).toBeTruthy();
    }

    const children = getRootChildren(await readDoc(fp));
    expect(children.length).toBe(100);
    // Perf budget: 5s wall-clock. Disk I/O + 100 tree inserts + save.
    // If this trips, the tree-insert path is probably O(n) per insert
    // (rather than O(1) append), giving O(n²) batch-wide.
    expect(elapsed, `100-op batch took ${elapsed}ms (budget 5000ms)`).toBeLessThan(5000);
  });

  it('200 sibling ops: 2x more nodes, still bounded perf', async () => {
    const fp = await fresh('stress.op');
    const ops: string[] = [];
    for (let i = 0; i < 200; i += 1) {
      ops.push(
        `row${i}=I(null, ${JSON.stringify({
          type: 'rectangle',
          name: `r${i}`,
          width: 100,
          height: 20,
        })})`,
      );
    }
    const t0 = Date.now();
    const result = await handleBatchDesign({
      operations: ops.join('\n'),
      filePath: fp,
      postProcess: false,
    });
    const elapsed = Date.now() - t0;
    expect(result.errors ?? []).toEqual([]);
    expect(result.results.length).toBe(200);
    expect(getRootChildren(await readDoc(fp)).length).toBe(200);
    // 2x ops → should be roughly 2x time, NOT 4x (which would indicate O(n²))
    expect(elapsed, `200-op batch took ${elapsed}ms (budget 10000ms)`).toBeLessThan(10000);
  });

  it('parent_id threading: 30-level nested I() chain resolves correctly', async () => {
    const fp = await fresh('stress.op');

    // Build: root frame > nested frame > nested frame > ... × 30
    const ops: string[] = [];
    ops.push(
      `n0=I(null, ${JSON.stringify({
        type: 'frame',
        name: 'L0',
        layout: 'vertical',
        width: 300,
      })})`,
    );
    for (let i = 1; i < 30; i += 1) {
      ops.push(
        `n${i}=I(n${i - 1}, ${JSON.stringify({
          type: 'frame',
          name: `L${i}`,
          layout: 'vertical',
          width: 280,
        })})`,
      );
    }
    const result = await handleBatchDesign({
      operations: ops.join('\n'),
      filePath: fp,
      postProcess: false,
    });
    expect(result.errors ?? []).toEqual([]);
    expect(result.results.length).toBe(30);

    const children = getRootChildren(await readDoc(fp));
    expect(children.length).toBe(1);
    // Each level has exactly one child (the next level)
    let cursor: Record<string, unknown> | undefined = children[0];
    for (let i = 0; i < 29; i += 1) {
      expect(cursor, `level ${i} missing`).toBeDefined();
      const kids = cursor?.children as Record<string, unknown>[] | undefined;
      expect(kids?.length, `level ${i} should have 1 child`).toBe(1);
      cursor = kids?.[0];
    }
    // Total node count: 30
    expect(countNodes(children)).toBe(30);
  });

  it('mixed batch: 50 sections × (header + 3 rows) = 200 ops across 50 parents', async () => {
    const fp = await fresh('stress.op');

    const ops: string[] = [];
    for (let s = 0; s < 50; s += 1) {
      // Section frame
      ops.push(
        `sec${s}=I(null, ${JSON.stringify({
          type: 'frame',
          name: `Section ${s}`,
          layout: 'vertical',
          width: 'fill_container',
          gap: 8,
        })})`,
      );
      // Header inside the section
      ops.push(
        `h${s}=I(sec${s}, ${JSON.stringify({
          type: 'text',
          name: 'Header',
          role: 'heading',
          content: `Section ${s} Title`,
          fontSize: 18,
          fontWeight: 600,
        })})`,
      );
      // 3 rows inside the section
      for (let r = 0; r < 3; r += 1) {
        ops.push(
          `r${s}_${r}=I(sec${s}, ${JSON.stringify({
            type: 'text',
            name: `Row ${r}`,
            role: 'body',
            content: `Row ${r}`,
            fontSize: 14,
          })})`,
        );
      }
    }
    // 50 sections + 50 headers + 150 rows = 250 ops total
    expect(ops.length).toBe(250);

    const t0 = Date.now();
    const result = await handleBatchDesign({
      operations: ops.join('\n'),
      filePath: fp,
      postProcess: false,
    });
    const elapsed = Date.now() - t0;

    expect(result.errors ?? []).toEqual([]);
    expect(result.results.length).toBe(250);

    const doc = await readDoc(fp);
    const children = getRootChildren(doc);
    expect(children.length).toBe(50); // 50 sections at root
    for (const sec of children) {
      const kids = sec.children as Record<string, unknown>[] | undefined;
      expect(kids?.length).toBe(4); // header + 3 rows
    }
    expect(countNodes(children)).toBe(250);
    expect(elapsed, `250-op mixed batch took ${elapsed}ms (budget 15000ms)`).toBeLessThan(15000);
  });

  it('partial failure: 1 bogus parent among 100 good ops is isolated (rest still land)', async () => {
    const fp = await fresh('stress.op');

    const ops: string[] = [];
    for (let i = 0; i < 100; i += 1) {
      if (i === 50) {
        // This one references a parent that doesn't exist
        ops.push(
          `bad=I(nonexistent_parent, ${JSON.stringify({
            type: 'text',
            name: 'Bad',
            content: 'x',
          })})`,
        );
      } else {
        ops.push(
          `row${i}=I(null, ${JSON.stringify({
            type: 'text',
            name: `row${i}`,
            content: `row ${i}`,
          })})`,
        );
      }
    }

    const result = await handleBatchDesign({
      operations: ops.join('\n'),
      filePath: fp,
      postProcess: false,
    });

    // The 99 good ops should have landed; bogus parent produces an
    // error entry. Policy is "don't let one bad op poison the batch"
    // per batch_design's design.
    const children = getRootChildren(await readDoc(fp));
    expect(children.length).toBe(99); // 100 minus the 1 bad
    // The bad op may end up as an error OR land at root (depending on
    // batch_design's resolve-null-parent fallback). Either behavior is
    // acceptable — the key invariant is: good ops still land.
    const totalAttempted = 100;
    const errorCount = result.errors?.length ?? 0;
    const landedCount = result.results.filter((r) => r.nodeId).length;
    // landed + error rows covers all 100 (may overlap if bad op falls
    // through to root): at minimum, ≥ 99 good ones in the tree
    expect(landedCount + errorCount).toBeGreaterThanOrEqual(totalAttempted);
  });
});
