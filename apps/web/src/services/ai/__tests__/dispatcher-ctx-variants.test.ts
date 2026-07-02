import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { dispatchElementToolCall } from '../element-tools-dispatcher';
import { useHistoryStore } from '@/stores/history-store';
import { useDocumentStore } from '@/stores/document-store';
import type { DesignOutputShape } from '../design-parser';

/**
 * Dispatcher ctx variants — full cross-product of where the parent
 * id can come from:
 *
 *   payload.parent_id      present | absent
 *   ctx.defaultParentId    set      | null | undefined
 *   target node in store   exists   | missing
 *
 * The resolution order the dispatcher documents (element-tools-
 * dispatcher.ts:320-326):
 *
 *   payload.parent_id > ctx.defaultParentId > page root
 *
 * With a fast-fail if ctx.defaultParentId is set but the node it
 * references has been removed (the 2026-04-21 Codex regression
 * scenario).
 *
 * These tests cover the combinatorial matrix. The happy path is
 * already exercised in ai-pipeline-e2e.test.ts + orchestrator-full-
 * screen-pipeline.test.ts, but those don't walk every branch.
 */

function seedFrame(id: string): string {
  useDocumentStore.getState().addNode(null, {
    id,
    type: 'frame',
    name: id,
    width: 400,
    height: 300,
    layout: 'vertical',
    children: [],
  });
  return id;
}

function tool(name: string, args: Record<string, unknown>): DesignOutputShape {
  return { kind: 'element-tool', name, arguments: args, raw: '' };
}

function getChildrenLength(parentId: string): number {
  const node = useDocumentStore.getState().getNodeById(parentId) as
    | { children?: unknown[] }
    | undefined;
  return (node?.children ?? []).length;
}

describe('dispatcher ctx — parent_id resolution matrix', () => {
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

  describe('payload.parent_id present', () => {
    it('parent_id exists + defaultParentId unset → land under parent_id', async () => {
      const pid = seedFrame('pid-only');
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Heading', parent_id: pid }),
        {},
      );
      expect(result.status).toBe('applied');
      expect(getChildrenLength(pid)).toBe(1);
    });

    it('parent_id exists + defaultParentId set → parent_id WINS', async () => {
      const explicit = seedFrame('explicit-parent');
      const fallback = seedFrame('fallback-parent');
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Explicit', parent_id: explicit }),
        { defaultParentId: fallback },
      );
      expect(result.status).toBe('applied');
      expect(getChildrenLength(explicit)).toBe(1);
      expect(getChildrenLength(fallback)).toBe(0);
    });

    it('parent_id does not exist → failed, no document write', async () => {
      const docBefore = useDocumentStore.getState().document;
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Ghost', parent_id: 'no-such-id-2026' }),
        {},
      );
      expect(result.status).toBe('failed');
      expect(useDocumentStore.getState().document).toBe(docBefore);
    });
  });

  describe('payload.parent_id absent', () => {
    it('defaultParentId exists → land under default', async () => {
      const fallback = seedFrame('only-default');
      const result = await dispatchElementToolCall(tool('add_heading_v0', { content: 'Default' }), {
        defaultParentId: fallback,
      });
      expect(result.status).toBe('applied');
      expect(getChildrenLength(fallback)).toBe(1);
    });

    it('defaultParentId set but node missing → fails fast, no document write', async () => {
      const docBefore = useDocumentStore.getState().document;
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Stale Parent' }),
        { defaultParentId: 'stale-parent-id-that-never-existed' },
      );
      expect(result.status).toBe('failed');
      expect(result.message).toContain('stale');
      expect(useDocumentStore.getState().document).toBe(docBefore);
    });

    it('defaultParentId null → insert at page root (no generation parent)', async () => {
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Page Root' }),
        { defaultParentId: null },
      );
      expect(result.status).toBe('applied');
      // At least one node was inserted somewhere in the document
      expect(result.insertedNodes.length).toBeGreaterThan(0);
    });

    it('defaultParentId undefined (ctx={}) → insert at page root', async () => {
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Undefined Ctx' }),
        {},
      );
      expect(result.status).toBe('applied');
      expect(result.insertedNodes.length).toBeGreaterThan(0);
    });
  });

  describe('parent_id existence takes precedence over ctx staleness', () => {
    it('valid parent_id + stale defaultParentId → parent_id wins, no fail-fast', async () => {
      const explicit = seedFrame('valid-explicit');
      // The dispatcher should never inspect defaultParentId when
      // parent_id is valid. Stale default must not contaminate the
      // happy path.
      const result = await dispatchElementToolCall(
        tool('add_heading_v0', { content: 'Valid wins', parent_id: explicit }),
        { defaultParentId: 'stale-never-existed-12345' },
      );
      expect(result.status).toBe('applied');
      expect(getChildrenLength(explicit)).toBe(1);
    });
  });
});

describe('dispatcher ctx — result shape sanity', () => {
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

  it('applied result carries insertedNodes for orchestrator accounting', async () => {
    const pid = seedFrame('applied-result');
    const result = await dispatchElementToolCall(
      tool('add_list_row_v0', {
        title: 'Notifications',
        subtitle: 'Push, email',
        parent_id: pid,
      }),
      {},
    );
    expect(result.status).toBe('applied');
    expect(result.insertedNodes).toHaveLength(1);
    expect(result.insertedNodes[0].id).toBeTypeOf('string');
    // Route and toolName are logged for metrics
    expect(result.route).toBe('element-tool');
    expect(result.toolName).toBe('add_list_row_v0');
  });

  it('failed result carries empty insertedNodes + diagnostic message', async () => {
    const result = await dispatchElementToolCall(
      tool('add_heading_v0', { content: 'X', parent_id: 'nope' }),
      {},
    );
    expect(result.status).toBe('failed');
    expect(result.insertedNodes).toEqual([]);
    expect(result.message.length).toBeGreaterThan(0);
  });

  it('unsupported result is distinct from failed (short-circuit for unknown tools)', async () => {
    const result = await dispatchElementToolCall(tool('add_nonexistent_v0', {}), {});
    expect(result.status).toBe('unsupported');
    expect(result.message).toContain('add_nonexistent_v0');
    expect(result.insertedNodes).toEqual([]);
  });
});
