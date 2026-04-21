import { describe, it, expect, beforeEach, vi } from 'vitest';
import { tryParseElementToolOutput, tryParseAllElementToolOutputs } from '../design-parser';
import { dispatchElementToolCall, dispatchElementToolCalls } from '../element-tools-dispatcher';
import { useHistoryStore } from '@/stores/history-store';
import { useDocumentStore } from '@/stores/document-store';

/**
 * End-to-end tests: raw AI response string → parser → dispatcher →
 * document-store. These are the "nothing mocked in between" tests
 * that prove the full pipeline works as the embedded orchestrator
 * would drive it. Complements the isolated dispatcher + parser +
 * shim tests by exercising them together.
 *
 * Scope note: fetch is stubbed to fail so HTTP-fallback paths
 * resolve deterministically; all assertions here are for the
 * client-side shim branch or explicit failure modes.
 */

describe('AI output → dispatcher → store (end-to-end)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch not available in test env'))),
    );
  });

  it('happy path: raw <op_tool> string → parsed → shim applied → node in store', async () => {
    const rawAi = `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Welcome"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    expect(shape).not.toBeNull();
    expect(shape?.kind).toBe('element-tool');

    const result = await dispatchElementToolCall(shape!, {});
    expect(result.status).toBe('applied');
    expect(result.insertedNodes).toHaveLength(1);
    expect(result.insertedNodes[0].type).toBe('text');
    expect((result.insertedNodes[0] as { content?: string }).content).toBe('Welcome');
  });

  it('parent_id targeted: AI names an existing container, insert lands inside', async () => {
    // Seed a container first
    const containerId = `ctr-${Math.random().toString(36).slice(2)}`;
    useDocumentStore.getState().addNode(null, {
      id: containerId,
      type: 'frame',
      name: 'Container',
      width: 400,
      height: 300,
      layout: 'vertical',
      children: [],
    });

    const rawAi = `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Nested","parent_id":"${containerId}"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    const result = await dispatchElementToolCall(shape!, {});
    expect(result.status).toBe('applied');

    const container = useDocumentStore.getState().getNodeById(containerId) as
      | { children?: Array<{ id: string }> }
      | undefined;
    expect(container?.children?.length).toBe(1);
    expect(container?.children?.[0].id).toBe(result.insertedNodes[0].id);
  });

  it('stale parent_id: AI points at missing node → failed, no write', async () => {
    const docBefore = useDocumentStore.getState().document;
    const rawAi = `<op_tool>{"name":"add_heading_v0","arguments":{"content":"X","parent_id":"does-not-exist-2026"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    const result = await dispatchElementToolCall(shape!, {});
    expect(result.status).toBe('failed');
    expect(useDocumentStore.getState().document).toBe(docBefore);
  });

  it('batch_design fallback: AI emits DSL-wrapped tag → route detected, HTTP attempted', async () => {
    const fetchFn = vi.fn(() => Promise.reject(new Error('network down')));
    vi.stubGlobal('fetch', fetchFn);

    const rawAi = `<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {\\"type\\":\\"frame\\",\\"name\\":\\"X\\",\\"width\\":100,\\"height\\":100})"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    expect(shape?.kind).toBe('batch-design-dsl');

    const result = await dispatchElementToolCall(shape!, {});
    // fetch failed → unsupported (not silent drop)
    expect(result.status).toBe('unsupported');
    expect(fetchFn).toHaveBeenCalledTimes(1);
  });

  it('multi-tag response: all tags parse + batch dispatch one undo entry', async () => {
    const rawAi = [
      `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Section A"}}</op_tool>`,
      `<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Intro paragraph."}}</op_tool>`,
      `<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>`,
    ].join('\n');

    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(3);
    expect(shapes.map((s) => (s.kind === 'element-tool' ? s.name : s.kind))).toEqual([
      'add_heading_v0',
      'add_body_text_v0',
      'add_divider_v0',
    ]);

    const startSpy = vi.spyOn(useHistoryStore.getState(), 'startBatch');
    const result = await dispatchElementToolCalls(shapes, {});
    expect(result.status).toBe('applied');
    expect(result.results).toHaveLength(3);
    expect(startSpy).toHaveBeenCalledTimes(1);
    startSpy.mockRestore();
  });

  it('malformed tag: JSON parse fails → parser returns null (caller falls to legacy)', async () => {
    const rawAi = `<op_tool>{bad json not valid}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    // Single-tag path returns null for unparseable → orchestrator
    // falls through to extractJsonFromResponse (legacy JSONL path).
    expect(shape).toBeNull();
  });

  it('tool name outside registry: parser emits shape but dispatcher short-circuits', async () => {
    const rawAi = `<op_tool>{"name":"add_fake_component_v9","arguments":{}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    expect(shape?.kind).toBe('element-tool');

    const fetchFn = vi.fn(() => Promise.reject(new Error('should not be called')));
    vi.stubGlobal('fetch', fetchFn);
    const result = await dispatchElementToolCall(shape!, {});
    expect(result.status).toBe('unsupported');
    expect(result.message).toContain('add_fake_component_v9');
    // Critical: short-circuit skipped HTTP — the Nitro endpoint
    // would also 404 this name, so no round-trip waste.
    expect(fetchFn).not.toHaveBeenCalled();
  });

  it('<think> wrapper stripped before parse (reasoning models)', async () => {
    const rawAi = `<think>Let me think about the design...</think>
<op_tool>{"name":"add_heading_v0","arguments":{"content":"After thinking"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    expect(shape?.kind).toBe('element-tool');

    const result = await dispatchElementToolCall(shape!, {});
    expect(result.status).toBe('applied');
    expect((result.insertedNodes[0] as { content?: string }).content).toBe('After thinking');
  });

  it('ctx.defaultParentId honored for rootless payloads from AI', async () => {
    // Seed a generation root to simulate orchestrator's subtask parent
    const rootId = `gen-root-${Math.random().toString(36).slice(2)}`;
    useDocumentStore.getState().addNode(null, {
      id: rootId,
      type: 'frame',
      name: 'Generation Root',
      width: 800,
      height: 600,
      layout: 'vertical',
      children: [],
    });

    const rawAi = `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Rootless"}}</op_tool>`;
    const shape = tryParseElementToolOutput(rawAi);
    const result = await dispatchElementToolCall(shape!, { defaultParentId: rootId });
    expect(result.status).toBe('applied');

    // The node should have landed under the generation root, not at page root
    const root = useDocumentStore.getState().getNodeById(rootId) as
      | { children?: unknown[] }
      | undefined;
    expect((root?.children ?? []).length).toBeGreaterThan(0);
  });
});
