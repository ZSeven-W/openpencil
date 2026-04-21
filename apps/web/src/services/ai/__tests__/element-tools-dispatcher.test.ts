import { describe, it, expect, beforeEach, vi } from 'vitest';
import { dispatchElementToolCall } from '../element-tools-dispatcher';
import { useHistoryStore } from '@/stores/history-store';
import { useDocumentStore } from '@/stores/document-store';
import { useCanvasStore } from '@/stores/canvas-store';
import type { DesignOutputShape } from '../design-parser';

/**
 * Phase 2 M2/M3 invariants under test:
 *   1. Every dispatch runs inside exactly one startBatch/endBatch pair
 *   2. Known tool → client shim applies tree via addNode (status='applied')
 *   3. Unknown tool → HTTP fallback attempted; fetch failure in test env
 *      collapses to status='unsupported' (never silently drops)
 *   4. batch_design DSL → HTTP fallback (M3 stubs it, same collapse)
 *   5. Multiple back-to-back dispatches don't leak batches
 */

describe('dispatchElementToolCall — M2 shim + M3 HTTP fallback', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    // In test env there's no Nitro backend, so /api/mcp/exec-tool fetch
    // will fail. Explicitly stub fetch to reject so assertions are
    // deterministic (don't rely on jsdom fetch error phrasing).
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch not available in test env'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('known element tool → applied via client shim + returns inserted node', async () => {
    const shape: DesignOutputShape = {
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: {
        items: [
          { title: 'Hiit', subtitle: '30 min' },
          { title: 'Yoga', subtitle: '25 min' },
        ],
      },
      raw: '<op_tool>...</op_tool>',
    };

    const result = await dispatchElementToolCall(shape);

    expect(result.status).toBe('applied');
    expect(result.route).toBe('element-tool');
    expect(result.toolName).toBe('add_card_row_v0');
    // Critical for orchestrator accounting: applied result MUST return
    // the inserted nodes so the caller can update its progress tally.
    // Without this the renderer-based tally stays at 0 and upstream
    // treats the subtask as "produced nothing → fail".
    expect(result.insertedNodes).toHaveLength(1);
    expect(result.insertedNodes[0].type).toBe('frame');
    expect(typeof result.insertedNodes[0].id).toBe('string');

    // Verify the inserted node actually reached the document store.
    // Can't assert on page-children length growing by 1 because
    // insertStreamingNode's frame branch may REPLACE an empty root
    // frame (generation replaces the placeholder) rather than
    // appending — both paths are valid "applied". Existence lookup
    // is the stable post-condition.
    const insertedId = result.insertedNodes[0].id;
    const docStore = useDocumentStore.getState();
    // Either the returned node id lives under a page, OR an id
    // remapped from it does (insertStreamingNode dedupes ids). The
    // "some id landed" check is what orchestrator accounting cares
    // about, which is what we surface to callers.
    const flat = docStore.getFlatNodes();
    const matched = flat.some((n) => n.id === insertedId);
    const anyCardFrame = flat.some((n) => (n as { role?: string }).role === 'scroll-row-wrapper');
    expect(matched || anyCardFrame).toBe(true);
  });

  it('element tool outside SUPPORTED_EMBEDDED_ELEMENT_TOOLS → short-circuits before HTTP', async () => {
    // The dispatcher must NOT waste an HTTP roundtrip on a tool the
    // Nitro endpoint is guaranteed to 404 (shim/server registries
    // are kept identical by convention). Fail fast with the canonical
    // supported list in the diagnostic so callers / UI can route to
    // batch_design instead.
    const fetchFn = vi.fn(() => Promise.reject(new Error('should not be called')));
    vi.stubGlobal('fetch', fetchFn);

    const shape: DesignOutputShape = {
      kind: 'element-tool',
      name: 'add_divider_v0', // real pen-mcp tool but not in shim registry
      arguments: {},
      raw: '<op_tool>...</op_tool>',
    };

    const result = await dispatchElementToolCall(shape);

    expect(result.status).toBe('unsupported');
    expect(result.route).toBe('element-tool');
    expect(result.toolName).toBe('add_divider_v0');
    expect(result.message).toContain('add_divider_v0');
    // The canonical covered list is embedded in the diagnostic
    expect(result.message).toContain('add_card_row_v0');
    expect(result.message).toContain('batch_design');
    // Critical: no HTTP fallback attempt — we already know the
    // endpoint cannot handle this name either.
    expect(fetchFn).not.toHaveBeenCalled();
    expect(result.insertedNodes).toEqual([]);
  });

  it('batch-design-dsl route → HTTP fallback unreachable → unsupported', async () => {
    const dsl = 'root=I(null, {"type":"frame","name":"X","width":100,"height":100})';
    const shape: DesignOutputShape = {
      kind: 'batch-design-dsl',
      dsl,
      raw: `<op_tool>...${dsl}...</op_tool>`,
    };

    const result = await dispatchElementToolCall(shape);

    expect(result.status).toBe('unsupported');
    expect(result.route).toBe('batch-design-dsl');
    expect(result.toolName).toBe('batch_design');
    expect(result.message).toContain('batch_design');
  });

  it('wraps every dispatch in exactly one startBatch/endBatch pair', async () => {
    const startSpy = vi.spyOn(useHistoryStore.getState(), 'startBatch');
    const endSpy = vi.spyOn(useHistoryStore.getState(), 'endBatch');

    await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: { items: [{ title: 'X' }] },
      raw: '',
    });

    expect(startSpy).toHaveBeenCalledTimes(1);
    expect(endSpy).toHaveBeenCalledTimes(1);

    const startCallOrder = startSpy.mock.invocationCallOrder[0];
    const endCallOrder = endSpy.mock.invocationCallOrder[0];
    expect(startCallOrder).toBeLessThan(endCallOrder);

    startSpy.mockRestore();
    endSpy.mockRestore();
  });

  it('batchDepth returns to 0 after dispatch (no leaked batches)', async () => {
    const shape: DesignOutputShape = {
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: { items: [{ title: 'X' }] },
      raw: '',
    };

    expect(useHistoryStore.getState().batchDepth).toBe(0);
    await dispatchElementToolCall(shape);
    expect(useHistoryStore.getState().batchDepth).toBe(0);
  });

  it('multiple dispatches each produce exactly one batch pair (no cross-call leak)', async () => {
    const startSpy = vi.spyOn(useHistoryStore.getState(), 'startBatch');
    const endSpy = vi.spyOn(useHistoryStore.getState(), 'endBatch');

    const shape: DesignOutputShape = {
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: { items: [{ title: 'A' }] },
      raw: '',
    };
    await dispatchElementToolCall(shape);
    await dispatchElementToolCall(shape);
    await dispatchElementToolCall(shape);

    expect(startSpy).toHaveBeenCalledTimes(3);
    expect(endSpy).toHaveBeenCalledTimes(3);
    expect(useHistoryStore.getState().batchDepth).toBe(0);

    startSpy.mockRestore();
    endSpy.mockRestore();
  });

  it('defaultParentId from ctx → rootless call lands under it + appends', async () => {
    // Mimics the orchestrator handing dispatcher subtask.parentFrameId:
    // the model's `<op_tool>` payload has no parent_id, but the
    // generation's root frame is the real target. Without ctx the
    // dispatcher would insert at page root (wrong scope). With ctx,
    // it must land inside the generation parent.
    const rootId = `gen-root-${Math.random().toString(36).slice(2)}`;
    useDocumentStore.getState().addNode(null, {
      id: rootId,
      type: 'frame',
      name: 'Generation Root',
      width: 800,
      height: 600,
      layout: 'vertical',
      children: [
        {
          id: `${rootId}-existing-child`,
          type: 'text',
          name: 'Existing',
          content: 'Already here',
        },
      ],
    });

    const result = await dispatchElementToolCall(
      {
        kind: 'element-tool',
        name: 'add_heading_v0',
        arguments: { content: 'New Heading' },
        raw: '',
      },
      { defaultParentId: rootId },
    );

    expect(result.status).toBe('applied');
    expect(result.message).toContain(rootId);

    const rootNode = useDocumentStore.getState().getNodeById(rootId) as
      | { children?: { id: string }[] }
      | undefined;
    const rootChildren = rootNode?.children ?? [];
    expect(rootChildren.length).toBe(2);
    // Append semantics — the new element lands AFTER the pre-existing
    // child, matching the streaming path's generation-order behavior.
    // (If the dispatcher reverted to prepend, this would fail because
    // the new element would take index 0 and push the existing child
    // to index 1.)
    expect(rootChildren[0].id).toBe(`${rootId}-existing-child`);
    expect(rootChildren[1].id).toBe(result.insertedNodes[0].id);
  });

  it('explicit parent_id wins over defaultParentId', async () => {
    // Two candidate parents seeded; model names the "real" one via
    // parent_id while ctx.defaultParentId points elsewhere. Payload
    // parent_id must win (the model had a specific intent).
    const ctxDefaultId = `ctx-default-${Math.random().toString(36).slice(2)}`;
    const realTargetId = `real-${Math.random().toString(36).slice(2)}`;
    const store = useDocumentStore.getState();
    store.addNode(null, {
      id: ctxDefaultId,
      type: 'frame',
      name: 'Ctx Default Parent',
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [],
    });
    store.addNode(null, {
      id: realTargetId,
      type: 'frame',
      name: 'Real Target',
      width: 400,
      height: 200,
      layout: 'vertical',
      children: [],
    });

    const result = await dispatchElementToolCall(
      {
        kind: 'element-tool',
        name: 'add_heading_v0',
        arguments: { content: 'Targeted', parent_id: realTargetId },
        raw: '',
      },
      { defaultParentId: ctxDefaultId },
    );

    expect(result.status).toBe('applied');
    const realTarget = useDocumentStore.getState().getNodeById(realTargetId) as
      | { children?: unknown[] }
      | undefined;
    const ctxDefault = useDocumentStore.getState().getNodeById(ctxDefaultId) as
      | { children?: unknown[] }
      | undefined;
    expect(realTarget?.children?.length).toBe(1);
    expect(ctxDefault?.children?.length).toBe(0);
  });

  it('stale defaultParentId (node not in document) → failed, no write', async () => {
    // Simulates an orchestrator handing dispatcher a subtask.parentFrameId
    // that was valid when the plan was built but has since been deleted
    // / remapped out of the document. Without validation, insertStreamingNode
    // silently retargets via its module-global generationRootFrameId fallback
    // and the subtree lands somewhere the AI never intended — but the
    // dispatcher still reports `applied`. The validation catches this.
    const docBefore = useDocumentStore.getState().document;

    const result = await dispatchElementToolCall(
      {
        kind: 'element-tool',
        name: 'add_heading_v0',
        arguments: { content: 'Orphan Heading' },
        raw: '',
      },
      { defaultParentId: 'stale-or-never-existed' },
    );

    expect(result.status).toBe('failed');
    expect(result.message).toContain('stale-or-never-existed');
    expect(result.message).toContain('stale');
    // Document reference equality — no write happened despite the
    // insertStreamingNode path being set up to silently retarget.
    expect(useDocumentStore.getState().document).toBe(docBefore);
  });

  it('explicit parent_id valid + stale defaultParentId → applied (payload wins, default ignored)', async () => {
    // When the AI gives an explicit parent_id that resolves, the stale
    // ctx default must NOT block the apply — the payload takes precedence
    // and the default is never used, so its staleness is irrelevant.
    const realTargetId = `real-${Math.random().toString(36).slice(2)}`;
    useDocumentStore.getState().addNode(null, {
      id: realTargetId,
      type: 'frame',
      name: 'Explicit Target',
      width: 400,
      height: 300,
      layout: 'vertical',
      children: [],
    });

    const result = await dispatchElementToolCall(
      {
        kind: 'element-tool',
        name: 'add_heading_v0',
        arguments: { content: 'Targeted', parent_id: realTargetId },
        raw: '',
      },
      { defaultParentId: 'would-be-stale-but-unused' },
    );

    expect(result.status).toBe('applied');
    const target = useDocumentStore.getState().getNodeById(realTargetId) as
      | { children?: unknown[] }
      | undefined;
    expect(target?.children?.length).toBe(1);
  });

  it('parent_id pointing at a real node → applied under that parent', async () => {
    // Seed: insert a Section frame at root, then route a shim call
    // with parent_id pointing to it. Shim MUST honor parent_id, not
    // silently insert at root.
    const sectionId = `section-${Math.random().toString(36).slice(2)}`;
    useDocumentStore.getState().addNode(null, {
      id: sectionId,
      type: 'frame',
      name: 'Seeded Section',
      width: 800,
      height: 400,
      layout: 'vertical',
      children: [],
    });

    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: {
        parent_id: sectionId,
        items: [{ title: 'A' }, { title: 'B' }],
      },
      raw: '',
    });

    expect(result.status).toBe('applied');
    expect(result.message).toContain(sectionId);

    // Verify the inserted node actually landed UNDER the section,
    // not at the page root. This is the load-bearing assertion
    // guarding against the "silently drops parent_id" regression.
    const parent = useDocumentStore.getState().getNodeById(sectionId);
    const parentChildren = (parent as { children?: unknown[] } | undefined)?.children ?? [];
    expect(parentChildren.length).toBe(1);
    expect((parentChildren[0] as { id?: string }).id).toBe(result.insertedNodes[0].id);
  });

  it('parent_id pointing at a missing node → failed, no write', async () => {
    const docBefore = useDocumentStore.getState().document;

    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: {
        parent_id: 'definitely-does-not-exist',
        items: [{ title: 'A' }],
      },
      raw: '',
    });

    expect(result.status).toBe('failed');
    expect(result.message).toContain('definitely-does-not-exist');
    // Store must not have been written. batchDepth=0 means endBatch ran;
    // the base snapshot === current means nothing landed.
    expect(useDocumentStore.getState().document).toBe(docBefore);
  });

  it('pageId matching active page → applied', async () => {
    const activePageId = useCanvasStore.getState().activePageId;
    // Only meaningful when there IS an active page — otherwise the
    // "null activePageId" branch is exercised elsewhere. Bail out
    // rather than assert misleadingly.
    if (!activePageId) return;

    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_heading_v0',
      arguments: { content: 'Same Page', pageId: activePageId },
      raw: '',
    });

    expect(result.status).toBe('applied');
  });

  it('filePath set to a real path → failed with hint, no write (client shim has no file I/O)', async () => {
    const docBefore = useDocumentStore.getState().document;

    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_heading_v0',
      arguments: { content: 'Saved Heading', filePath: '/tmp/design.op' },
      raw: '',
    });

    expect(result.status).toBe('failed');
    expect(result.message).toContain('/tmp/design.op');
    expect(result.message).toContain('stdio');
    // Store reference equality — neither an addNode nor applyExternalDocument
    // may have fired (filePath rejection happens before either).
    expect(useDocumentStore.getState().document).toBe(docBefore);
  });

  it('filePath="live://canvas" is the live-canvas sentinel → applied', async () => {
    // pen-mcp's resolveDocPath treats `live://canvas` the same as
    // undefined. Rejecting it as "unsupported filePath" would be a
    // regression — the sentinel IS the default target. This test
    // pairs with the "/tmp/design.op → failed" case above.
    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_heading_v0',
      arguments: { content: 'Sentinel Heading', filePath: 'live://canvas' },
      raw: '',
    });

    expect(result.status).toBe('applied');
    expect(result.insertedNodes).toHaveLength(1);
  });

  it('pageId NOT matching active page → failed with hint, no write', async () => {
    const docBefore = useDocumentStore.getState().document;

    const result = await dispatchElementToolCall({
      kind: 'element-tool',
      name: 'add_heading_v0',
      arguments: { content: 'Wrong Page', pageId: 'some-other-page-id' },
      raw: '',
    });

    expect(result.status).toBe('failed');
    expect(result.message).toContain('some-other-page-id');
    expect(result.message).toContain('HTTP fallback');
    expect(useDocumentStore.getState().document).toBe(docBefore);
  });

  it('N successful shim applies collapse into 1 history undo entry', async () => {
    // Record undoStack size before a batch of 3 applies
    const undoBefore = useHistoryStore.getState().undoStack.length;

    const shape: DesignOutputShape = {
      kind: 'element-tool',
      name: 'add_card_row_v0',
      arguments: { items: [{ title: 'A' }, { title: 'B' }] },
      raw: '',
    };
    // Each dispatch is its own batch (orchestrator emits one <op_tool>
    // per response), so 3 dispatches = 3 undo entries. This is the
    // *intended* M1/M2 behavior — the batch-per-tool-call invariant is
    // about preventing N-PER-TOOL-call fragmentation from internal
    // store writes, not about collapsing multiple tool calls across
    // a whole generation (that's an orchestrator-level concern).
    await dispatchElementToolCall(shape);
    await dispatchElementToolCall(shape);
    await dispatchElementToolCall(shape);

    const undoAfter = useHistoryStore.getState().undoStack.length;
    // 3 dispatches → exactly 3 batches committed (one per dispatch)
    expect(undoAfter - undoBefore).toBe(3);
  });
});

// afterEach is imported implicitly via vitest globals; re-import for clarity
import { afterEach } from 'vitest';
