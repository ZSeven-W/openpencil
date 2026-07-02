import { describe, it, expect, beforeEach, vi, afterEach } from 'vitest';
import { dispatchElementToolCall, dispatchElementToolCalls } from '../element-tools-dispatcher';
import { useHistoryStore } from '@/stores/history-store';
import { useDocumentStore } from '@/stores/document-store';
import type { DesignOutputShape } from '../design-parser';

/**
 * Integration: undo/redo × element-tool dispatch.
 *
 * User-visible contract the dispatcher owes us:
 *   1. One dispatchElementToolCalls invocation = exactly ONE undo entry
 *      (even if N tools execute under the hood). The user doesn't want
 *      to tap Ctrl-Z 10 times to reverse one AI turn.
 *   2. Undo after a batch dispatch restores the document to pre-
 *      dispatch state — all tools revert atomically.
 *   3. Redo after undo re-applies the whole batch atomically.
 *   4. Consecutive separate dispatches produce separate undo entries;
 *      they don't silently merge into one mega-undo.
 *
 * These tests exercise the startBatch/endBatch plumbing in the
 * dispatcher against the actual history-store. Complements
 * element-tools-dispatcher.test.ts which spies on startBatch but
 * doesn't walk the full history-store round-trip.
 */

function tool(name: string, args: Record<string, unknown>): DesignOutputShape {
  return { kind: 'element-tool', name, arguments: args, raw: '' };
}

function seedRoot(): string {
  const id = `root-${Math.random().toString(36).slice(2)}`;
  useDocumentStore.getState().addNode(null, {
    id,
    type: 'frame',
    name: 'root',
    width: 375,
    height: 812,
    layout: 'vertical',
    children: [],
  });
  return id;
}

function childCount(rootId: string): number {
  const root = useDocumentStore.getState().getNodeById(rootId) as
    | { children?: unknown[] }
    | undefined;
  return (root?.children ?? []).length;
}

describe('undo/redo — single dispatch produces exactly one undo entry', () => {
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

  it('3-tool batch → 1 undoStack entry (not 3)', async () => {
    const rootId = seedRoot();
    const undoBefore = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'A' }),
        tool('add_body_text_v0', { content: 'B paragraph longer than 15 chars' }),
        tool('add_divider_v0', {}),
      ],
      { defaultParentId: rootId },
    );

    const undoAfter = useHistoryStore.getState().undoStack.length;
    expect(undoAfter - undoBefore).toBe(1);
    expect(childCount(rootId)).toBe(3);
  });

  it('8-tool batch → 1 undoStack entry', async () => {
    const rootId = seedRoot();
    const undoBefore = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCalls(
      [
        tool('add_top_nav_bar_v0', { title: 'A' }),
        tool('add_form_field_v0', { label: 'Email' }),
        tool('add_form_field_v0', { label: 'Password' }),
        tool('add_checkbox_v0', { label: 'Remember', checked: true }),
        tool('add_text_button_v0', { label: 'Sign in' }),
        tool('add_divider_v0', {}),
        tool('add_body_text_v0', { content: 'Forgot password? Read me' }),
        tool('add_link_v0', { label: 'Reset' }),
      ],
      { defaultParentId: rootId },
    );

    expect(useHistoryStore.getState().undoStack.length - undoBefore).toBe(1);
    expect(childCount(rootId)).toBe(8);
  });
});

describe('undo/redo — separate dispatches produce separate undo entries', () => {
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

  it('3 consecutive single-tool dispatches → 3 undo entries (not 1 mega-undo)', async () => {
    const rootId = seedRoot();
    const before = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCall(tool('add_heading_v0', { content: 'A' }), {
      defaultParentId: rootId,
    });
    await dispatchElementToolCall(tool('add_heading_v0', { content: 'B' }), {
      defaultParentId: rootId,
    });
    await dispatchElementToolCall(tool('add_heading_v0', { content: 'C' }), {
      defaultParentId: rootId,
    });

    expect(useHistoryStore.getState().undoStack.length - before).toBe(3);
    expect(childCount(rootId)).toBe(3);
  });

  it('2 batch dispatches (3 + 2 tools) → 2 undo entries', async () => {
    const rootId = seedRoot();
    const before = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'Section 1' }),
        tool('add_body_text_v0', { content: 'First intro paragraph body.' }),
        tool('add_divider_v0', {}),
      ],
      { defaultParentId: rootId },
    );
    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'Section 2' }),
        tool('add_body_text_v0', { content: 'Second intro paragraph body.' }),
      ],
      { defaultParentId: rootId },
    );

    expect(useHistoryStore.getState().undoStack.length - before).toBe(2);
    expect(childCount(rootId)).toBe(5);
  });
});

describe('undo/redo — round trip restores atomically', () => {
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

  it('after batch dispatch, one undo reverts all inserts', async () => {
    const rootId = seedRoot();
    expect(childCount(rootId)).toBe(0);

    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'X' }),
        tool('add_body_text_v0', { content: 'Body paragraph long enough' }),
        tool('add_divider_v0', {}),
      ],
      { defaultParentId: rootId },
    );
    expect(childCount(rootId)).toBe(3);

    // One undo should revert all three at once
    const history = useHistoryStore.getState();
    const current = useDocumentStore.getState().document;
    const reverted = history.undo(current);
    expect(reverted).not.toBeNull();
    if (reverted) {
      useDocumentStore.setState({ document: reverted });
    }
    // Root frame should no longer have any children (pre-dispatch state)
    expect(childCount(rootId)).toBe(0);
  });

  it('undo → redo cycle re-applies batch atomically', async () => {
    const rootId = seedRoot();
    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'Cycle A' }),
        tool('add_divider_v0', {}),
        tool('add_body_text_v0', { content: 'Middle body paragraph.' }),
      ],
      { defaultParentId: rootId },
    );
    expect(childCount(rootId)).toBe(3);

    const history = useHistoryStore.getState();
    // Undo
    const post1 = useDocumentStore.getState().document;
    const reverted = history.undo(post1);
    if (reverted) useDocumentStore.setState({ document: reverted });
    expect(childCount(rootId)).toBe(0);

    // Redo
    const post2 = useDocumentStore.getState().document;
    const redone = history.redo(post2);
    expect(redone).not.toBeNull();
    if (redone) useDocumentStore.setState({ document: redone });
    expect(childCount(rootId)).toBe(3);
  });
});

describe('undo/redo — unsupported tools do NOT create undo noise', () => {
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

  it('all-unsupported batch → no undo entry (nothing actually mutated)', async () => {
    const rootId = seedRoot();
    const before = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCalls(
      [
        tool('add_nonexistent_one_v0', {}),
        tool('add_nonexistent_two_v0', {}),
        tool('add_nonexistent_three_v0', {}),
      ],
      { defaultParentId: rootId },
    );

    // Per endBatch logic: if no changes happened in the batch window,
    // the batch is discarded without creating an undo entry. This
    // prevents "ghost undos" that jump the UI back to identical state.
    const delta = useHistoryStore.getState().undoStack.length - before;
    expect(delta).toBe(0);
    expect(childCount(rootId)).toBe(0);
  });

  it('mixed batch: 2 valid + 1 unsupported → 1 undo entry for the 2 valid', async () => {
    const rootId = seedRoot();
    const before = useHistoryStore.getState().undoStack.length;

    await dispatchElementToolCalls(
      [
        tool('add_heading_v0', { content: 'Valid A' }),
        tool('add_nonexistent_v0', {}),
        tool('add_body_text_v0', { content: 'Valid B body paragraph.' }),
      ],
      { defaultParentId: rootId },
    );

    expect(useHistoryStore.getState().undoStack.length - before).toBe(1);
    expect(childCount(rootId)).toBe(2);
  });
});
