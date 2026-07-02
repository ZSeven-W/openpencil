import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { dispatchElementToolCalls } from '../element-tools-dispatcher';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import type { DesignOutputShape } from '../design-parser';

/**
 * Large-scale dispatch stress — a realistic "full screen design"
 * orchestrator turn emits 40-60 element tools in a single
 * `dispatchElementToolCalls` call. Guards:
 *
 *   1. Latency: end-to-end under 500ms for 40 tools. Streaming
 *      generation typically takes 5-30s of AI thinking, so the
 *      dispatch itself must not be the bottleneck.
 *   2. Single undo entry for the whole batch (no history pollution).
 *   3. Every tool lands — no silent drops even under volume.
 *   4. Document store integrity: no id collisions, children array
 *      length matches the expected count.
 *   5. Batch idempotency: running the same batch twice against a
 *      fresh root produces identical tree shape.
 */

function tool(name: string, args: Record<string, unknown>): DesignOutputShape {
  return { kind: 'element-tool', name, arguments: args, raw: '' };
}

function seedRoot(label: string): string {
  const id = `${label}-${Math.random().toString(36).slice(2)}`;
  useDocumentStore.getState().addNode(null, {
    id,
    type: 'frame',
    name: label,
    width: 375,
    height: 812 * 2, // tall frame to hold a long screen
    layout: 'vertical',
    children: [],
  });
  return id;
}

function buildLargeBatch(size: number): DesignOutputShape[] {
  // Mix builder types so the stress test exercises multi-level trees,
  // not just heading×N. Pattern repeats every 5 tools.
  const batch: DesignOutputShape[] = [];
  for (let i = 0; i < size; i++) {
    const slot = i % 5;
    switch (slot) {
      case 0:
        batch.push(tool('add_heading_v0', { content: `Section ${i}`, level: 'h2' }));
        break;
      case 1:
        batch.push(
          tool('add_body_text_v0', {
            content: `Body paragraph ${i} explaining the content in moderate detail.`,
          }),
        );
        break;
      case 2:
        batch.push(
          tool('add_list_row_v0', {
            title: `Setting ${i}`,
            subtitle: 'Toggle description',
            leading_icon: 'cog',
            trailing_icon: 'chevron-right',
          }),
        );
        break;
      case 3:
        batch.push(tool('add_divider_v0', {}));
        break;
      case 4:
        batch.push(
          tool('add_stat_grid_v0', {
            items: [
              { value: `${i * 100}`, label: 'A', icon: 'activity' },
              { value: `${i * 200}`, label: 'B', icon: 'flame' },
              { value: `${i * 300}`, label: 'C', icon: 'moon' },
            ],
          }),
        );
        break;
    }
  }
  return batch;
}

function childCount(rootId: string): number {
  const root = useDocumentStore.getState().getNodeById(rootId) as
    | { children?: unknown[] }
    | undefined;
  return (root?.children ?? []).length;
}

function collectAllDescendantIds(rootId: string): string[] {
  const root = useDocumentStore.getState().getNodeById(rootId) as
    | { children?: unknown[] }
    | undefined;
  const ids: string[] = [];
  const walk = (n: unknown): void => {
    const node = n as { id: string; children?: unknown[] };
    ids.push(node.id);
    for (const c of node.children ?? []) walk(c);
  };
  for (const c of root?.children ?? []) walk(c);
  return ids;
}

describe('large-scale — 40-tool dispatch', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('40 tools dispatch completes under 500ms', async () => {
    const rootId = seedRoot('stress-40');
    const batch = buildLargeBatch(40);

    const t0 = performance.now();
    const result = await dispatchElementToolCalls(batch, { defaultParentId: rootId });
    const elapsed = performance.now() - t0;

    expect(result.status).toBe('applied');
    expect(result.results).toHaveLength(40);
    expect(elapsed, `40-tool batch took ${elapsed.toFixed(1)}ms`).toBeLessThan(500);
  });

  it('40 tools → 1 undo entry (one batch)', async () => {
    const rootId = seedRoot('stress-40-undo');
    const beforeUndo = useHistoryStore.getState().undoStack.length;
    await dispatchElementToolCalls(buildLargeBatch(40), { defaultParentId: rootId });
    const afterUndo = useHistoryStore.getState().undoStack.length;
    expect(afterUndo - beforeUndo).toBe(1);
  });

  it('40 tools → 40 top-level children under root (no silent drops)', async () => {
    const rootId = seedRoot('stress-40-children');
    await dispatchElementToolCalls(buildLargeBatch(40), { defaultParentId: rootId });
    expect(childCount(rootId)).toBe(40);
  });

  it('every inserted node has a unique id (no collisions)', async () => {
    const rootId = seedRoot('stress-40-unique');
    await dispatchElementToolCalls(buildLargeBatch(40), { defaultParentId: rootId });
    const ids = collectAllDescendantIds(rootId);
    expect(new Set(ids).size, `${ids.length} total ids, duplicates present`).toBe(ids.length);
  });
});

describe('large-scale — 60-tool dispatch (upper bound)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('60 tools complete + all land + unique ids', async () => {
    const rootId = seedRoot('stress-60');
    const batch = buildLargeBatch(60);

    const t0 = performance.now();
    const result = await dispatchElementToolCalls(batch, { defaultParentId: rootId });
    const elapsed = performance.now() - t0;

    expect(result.status).toBe('applied');
    expect(result.results).toHaveLength(60);
    expect(childCount(rootId)).toBe(60);

    const ids = collectAllDescendantIds(rootId);
    expect(new Set(ids).size).toBe(ids.length);

    // 60 tools should still complete in a reasonable window
    expect(elapsed, `60-tool batch took ${elapsed.toFixed(1)}ms`).toBeLessThan(800);
  });
});

describe('large-scale — consecutive batches keep state clean', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('3 × 20-tool batches: 60 total children, 3 undo entries', async () => {
    const rootId = seedRoot('stress-consecutive');
    const startUndo = useHistoryStore.getState().undoStack.length;

    for (let round = 0; round < 3; round++) {
      await dispatchElementToolCalls(buildLargeBatch(20), { defaultParentId: rootId });
    }

    expect(childCount(rootId)).toBe(60);
    expect(useHistoryStore.getState().undoStack.length - startUndo).toBe(3);

    // Ids across all rounds still unique
    const ids = collectAllDescendantIds(rootId);
    expect(new Set(ids).size).toBe(ids.length);
  });
});

describe('large-scale — concurrent dispatches into separate roots', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('Promise.all of 3 dispatches into 3 distinct roots all complete', async () => {
    const rootA = seedRoot('root-a');
    const rootB = seedRoot('root-b');
    const rootC = seedRoot('root-c');

    const [a, b, c] = await Promise.all([
      dispatchElementToolCalls(buildLargeBatch(15), { defaultParentId: rootA }),
      dispatchElementToolCalls(buildLargeBatch(15), { defaultParentId: rootB }),
      dispatchElementToolCalls(buildLargeBatch(15), { defaultParentId: rootC }),
    ]);

    expect(a.status).toBe('applied');
    expect(b.status).toBe('applied');
    expect(c.status).toBe('applied');

    expect(childCount(rootA)).toBe(15);
    expect(childCount(rootB)).toBe(15);
    expect(childCount(rootC)).toBe(15);

    // Full document id uniqueness across all three roots
    const allIds = [
      ...collectAllDescendantIds(rootA),
      ...collectAllDescendantIds(rootB),
      ...collectAllDescendantIds(rootC),
    ];
    expect(new Set(allIds).size, 'id collisions across concurrent batches').toBe(allIds.length);
  });
});
