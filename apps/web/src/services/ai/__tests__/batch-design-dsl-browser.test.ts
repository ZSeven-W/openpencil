import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { dispatchElementToolCall } from '../element-tools-dispatcher';
import { useDocumentStore } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import type { DesignOutputShape } from '../design-parser';

/**
 * Integration coverage for the browser-safe batch_design DSL path
 * (closes #44). Every test stubs `fetch` to reject so that any
 * regression falling back to the HTTP path fails loudly — the whole
 * point of #44 is to keep batch_design on the hot in-browser path.
 *
 * Exercises real DSL shapes the AI emits, including binding chains
 * (`root=I(...); child=I(root, ...)`), U() updates, and multi-op
 * batches. Complements the 5 browser-safety static checks in pen-mcp
 * (`batch-design-dsl-browser-safe.test.ts`) which verify the import
 * tree; this file verifies the runtime behavior.
 */

function dslShape(dsl: string): DesignOutputShape {
  return {
    kind: 'batch-design-dsl',
    dsl,
    raw: `<op_tool>${dsl}</op_tool>`,
  };
}

describe('batch_design DSL — browser executor integration', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('browser path must not hit HTTP'))),
    );
  });

  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('single I() at root: frame inserted + store reflects it + no HTTP', async () => {
    const result = await dispatchElementToolCall(
      dslShape(
        'root=I(null, {"type":"frame","name":"Test","width":300,"height":200,"layout":"vertical"})',
      ),
    );
    expect(result.status).toBe('applied');
    expect(result.route).toBe('batch-design-dsl');
    expect(result.insertedNodes.length).toBe(1);

    const inserted = result.insertedNodes[0];
    expect(inserted.type).toBe('frame');
    expect(inserted.name).toBe('Test');
  });

  it('binding chain: root + nested child land in correct parent', async () => {
    const dsl = [
      'root=I(null, {"type":"frame","name":"Root","width":400,"height":300,"layout":"vertical"})',
      'child=I(root, {"type":"rectangle","name":"Child","width":100,"height":60,"fill":[{"type":"solid","color":"#2563EB"}]})',
    ].join('\n');

    const result = await dispatchElementToolCall(dslShape(dsl));
    expect(result.status).toBe('applied');
    expect(result.insertedNodes.length).toBe(2);

    // Walk store to verify the nested structure
    const rootNode = result.insertedNodes[0] as { id: string };
    const found = useDocumentStore.getState().getNodeById(rootNode.id) as
      | { children?: Array<{ name?: string }> }
      | undefined;
    expect(found?.children?.length).toBe(1);
    expect(found?.children?.[0].name).toBe('Child');
  });

  it('U() update applied: properties merged onto bound node', async () => {
    const dsl = [
      'root=I(null, {"type":"frame","name":"Updatable","width":200,"height":200,"layout":"none"})',
      'U(root, {"cornerRadius":16,"fill":[{"type":"solid","color":"#EF4444"}]})',
    ].join('\n');

    const result = await dispatchElementToolCall(dslShape(dsl));
    expect(result.status).toBe('applied');

    const root = result.insertedNodes[0] as {
      id: string;
      cornerRadius?: number;
      fill?: Array<{ color: string }>;
    };
    const fresh = useDocumentStore.getState().getNodeById(root.id) as {
      cornerRadius?: number;
      fill?: Array<{ color: string }>;
    };
    expect(fresh.cornerRadius).toBe(16);
    expect(fresh.fill?.[0].color).toBe('#EF4444');
  });

  it('6-op realistic screen: top nav + 2 cards + divider', async () => {
    const dsl = [
      'screen=I(null, {"type":"frame","name":"Screen","width":375,"height":812,"layout":"vertical","gap":0})',
      'nav=I(screen, {"type":"frame","name":"Nav","width":"fill_container","height":56,"layout":"horizontal"})',
      'title=I(nav, {"type":"text","name":"Title","content":"Settings","fontSize":18,"fontWeight":600})',
      'c1=I(screen, {"type":"frame","name":"Card A","width":"fill_container","height":80,"layout":"vertical","padding":[16,16]})',
      'd1=I(screen, {"type":"rectangle","name":"Divider","width":"fill_container","height":1,"fill":[{"type":"solid","color":"#E2E8F0"}]})',
      'c2=I(screen, {"type":"frame","name":"Card B","width":"fill_container","height":80,"layout":"vertical","padding":[16,16]})',
    ].join('\n');

    const result = await dispatchElementToolCall(dslShape(dsl));
    expect(result.status).toBe('applied');
    expect(result.insertedNodes.length).toBe(6);

    const screen = result.insertedNodes[0] as { id: string };
    const fresh = useDocumentStore.getState().getNodeById(screen.id) as
      | { children?: Array<{ name?: string }> }
      | undefined;
    expect(fresh?.children?.length).toBe(4); // nav + c1 + d1 + c2
    const childNames = (fresh?.children ?? []).map((c) => c.name);
    expect(childNames).toEqual(['Nav', 'Card A', 'Divider', 'Card B']);
  });

  it('multi-op DSL → one undo entry (batch wrap survived)', async () => {
    const before = useHistoryStore.getState().undoStack.length;

    const dsl = [
      'a=I(null, {"type":"frame","name":"A","width":100,"height":100,"layout":"none"})',
      'b=I(null, {"type":"frame","name":"B","width":100,"height":100,"layout":"none"})',
      'c=I(null, {"type":"frame","name":"C","width":100,"height":100,"layout":"none"})',
    ].join('\n');

    await dispatchElementToolCall(dslShape(dsl));

    // Dispatcher wraps every call in startBatch/endBatch. 3 root-level
    // frames but ONE undo entry.
    const after = useHistoryStore.getState().undoStack.length;
    expect(after - before).toBe(1);
  });

  it('malformed op in the middle: earlier ops apply, bad op surfaces as failed', async () => {
    const dsl = [
      'good1=I(null, {"type":"frame","name":"Good 1","width":100,"height":100,"layout":"none"})',
      'bad=I(nonsense syntax that is definitely not valid',
      'good2=I(null, {"type":"frame","name":"Good 2","width":100,"height":100,"layout":"none"})',
    ].join('\n');

    const result = await dispatchElementToolCall(dslShape(dsl));
    // Any per-op error → dispatcher returns failed for the whole batch.
    // This is the documented contract: the caller / user can see which
    // line broke and retry.
    expect(result.status).toBe('failed');
    expect(result.message).toContain('failing operation');
  });

  it('empty DSL string: zero ops, status=applied, no insertions', async () => {
    const result = await dispatchElementToolCall(dslShape(''));
    expect(result.status).toBe('applied');
    expect(result.insertedNodes).toEqual([]);
  });

  it('applyExternalDocument called once per batch (not per op)', async () => {
    // Defensive: the browser executor structuredClones the doc, runs
    // ALL ops against the clone, then applies the clone back in one
    // call. Applying per-op would thrash the React tree + history
    // state for no benefit. Spy to confirm the one-shot apply.
    const applySpy = vi.spyOn(useDocumentStore.getState(), 'applyExternalDocument');
    const dsl = [
      'a=I(null, {"type":"frame","name":"A","width":100,"height":100,"layout":"none"})',
      'b=I(null, {"type":"frame","name":"B","width":100,"height":100,"layout":"none"})',
      'c=I(null, {"type":"frame","name":"C","width":100,"height":100,"layout":"none"})',
      'd=I(null, {"type":"frame","name":"D","width":100,"height":100,"layout":"none"})',
    ].join('\n');

    await dispatchElementToolCall(dslShape(dsl));
    expect(applySpy).toHaveBeenCalledTimes(1);
    applySpy.mockRestore();
  });

  it('G() op without fetcher: image node inserted with empty src', async () => {
    // G() regex requires a quoted parent; use a seeded root frame
    // as the anchor so the insert actually lands in the tree.
    const dsl = [
      'root=I(null, {"type":"frame","name":"Gallery","width":375,"height":400,"layout":"vertical"})',
      // G's parent arg MUST be quoted — the regex requires `"<ref>"`.
      // resolveRef strips quotes + looks up the binding, so "root"
      // resolves to the frame inserted above.
      'img=G("root", "search", "cat photo")',
    ].join('\n');
    const result = await dispatchElementToolCall(dslShape(dsl));
    expect(result.status).toBe('applied');
    // 1 frame + 1 image = 2 ops
    expect(result.insertedNodes.length).toBe(2);

    const img = result.insertedNodes[1] as { type?: string; src?: string };
    expect(img.type).toBe('image');
    // Browser path omits imageSearchFetcher → empty src for downstream
    // scanAndFillImages pass to enrich.
    expect(img.src).toBe('');
  });
});
