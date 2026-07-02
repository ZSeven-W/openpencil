import { describe, it, expect, beforeEach, vi } from 'vitest';
import { tryParseAllElementToolOutputs } from '../design-parser';
import { dispatchElementToolCalls } from '../element-tools-dispatcher';
import { useHistoryStore } from '@/stores/history-store';
import { useDocumentStore } from '@/stores/document-store';

/**
 * Orchestrator-drive "full screen" pipeline integration. Each test
 * simulates one orchestrator round that emits 5-10 `<op_tool>` tags
 * in a single AI response — a realistic shape for N-tool streaming
 * where the model produces the entire screen at once.
 *
 * This is the strongest integration signal we have short of a real
 * browser: it exercises parser → dispatcher → shim → store for a
 * complete screen, and verifies user-visible invariants (one undo
 * entry, every node landed under the generation root, final tree is
 * valid). Complements ai-pipeline-e2e.test.ts which exercises the
 * same chain but one tag at a time.
 *
 * Scope:
 *   - fetch is stubbed to reject (no HTTP fallback path exercised)
 *   - only client-side shim-branch tools are used; this constrains
 *     the tool set but is the real production path for the 42
 *     embedded tools
 *   - defaultParentId is set to a seeded "generation root" frame
 *     exactly as the orchestrator does for every subtask
 */

const SCREEN_ROOT_PREFIX = 'gen-root-test-';

function seedGenerationRoot(name: string): string {
  const id = `${SCREEN_ROOT_PREFIX}${Math.random().toString(36).slice(2)}`;
  useDocumentStore.getState().addNode(null, {
    id,
    type: 'frame',
    name,
    width: 375,
    height: 812,
    layout: 'vertical',
    padding: [0, 0],
    gap: 0,
    children: [],
  });
  return id;
}

function countNodesUnder(rootId: string): number {
  const root = useDocumentStore.getState().getNodeById(rootId) as
    | { children?: unknown[] }
    | undefined;
  const walk = (n: unknown): number => {
    const node = n as { children?: unknown[] };
    return 1 + (node.children ?? []).reduce((s: number, c: unknown) => s + walk(c), 0);
  };
  // Exclude the root itself from the count
  return (root?.children ?? []).reduce((s: number, c: unknown) => s + walk(c), 0);
}

function collectDirectChildRoles(rootId: string): string[] {
  const root = useDocumentStore.getState().getNodeById(rootId) as
    | { children?: Array<{ role?: string; type?: string }> }
    | undefined;
  return (root?.children ?? []).map((c) => c.role ?? c.type ?? 'unknown');
}

describe('orchestrator — full screen pipeline (login)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  it('8 tools for a login screen → single undo entry + all nodes landed', async () => {
    const rootId = seedGenerationRoot('Login');

    // What the orchestrator would emit for a login screen
    const rawAi = [
      `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Welcome back","level":"h1"}}</op_tool>`,
      `<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Sign in to continue."}}</op_tool>`,
      `<op_tool>{"name":"add_form_field_v0","arguments":{"label":"Email","leading_icon":"mail"}}</op_tool>`,
      `<op_tool>{"name":"add_form_field_v0","arguments":{"label":"Password","leading_icon":"lock","trailing_icon":"eye","required":true}}</op_tool>`,
      `<op_tool>{"name":"add_checkbox_v0","arguments":{"label":"Remember me","checked":true}}</op_tool>`,
      `<op_tool>{"name":"add_text_button_v0","arguments":{"label":"Sign in"}}</op_tool>`,
      `<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>`,
      `<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Forgot password?"}}</op_tool>`,
    ].join('\n');

    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(8);

    const startBatchSpy = vi.spyOn(useHistoryStore.getState(), 'startBatch');
    const endBatchSpy = vi.spyOn(useHistoryStore.getState(), 'endBatch');

    const result = await dispatchElementToolCalls(shapes, { defaultParentId: rootId });

    expect(result.status).toBe('applied');
    expect(result.results).toHaveLength(8);
    // Every tool should have applied status, not partial/failed
    for (const r of result.results) {
      expect(r.status).toBe('applied');
    }

    // One batch (one undo entry) for the whole screen
    expect(startBatchSpy).toHaveBeenCalledTimes(1);
    expect(endBatchSpy).toHaveBeenCalledTimes(1);

    // Every child should have landed under the generation root
    const directRoles = collectDirectChildRoles(rootId);
    expect(directRoles.length).toBe(8);

    // Sanity: node count under root grew to match the screen tree
    expect(countNodesUnder(rootId)).toBeGreaterThan(8); // includes sub-children

    startBatchSpy.mockRestore();
    endBatchSpy.mockRestore();
  });
});

describe('orchestrator — full screen pipeline (dashboard)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  it('dashboard: top nav + stat grid + card row + bottom nav land in order', async () => {
    const rootId = seedGenerationRoot('Dashboard');

    const rawAi = [
      `<op_tool>{"name":"add_top_nav_bar_v0","arguments":{"title":"Home","trailing_icon":"bell"}}</op_tool>`,
      `<op_tool>{"name":"add_stat_grid_v0","arguments":{"items":[{"value":"8,432","label":"Steps","icon":"activity"},{"value":"512","label":"Kcal","icon":"flame"},{"value":"7h","label":"Sleep","icon":"moon"}]}}</op_tool>`,
      `<op_tool>{"name":"add_section_header_v0","arguments":{"title":"Recent workouts"}}</op_tool>`,
      `<op_tool>{"name":"add_card_row_v0","arguments":{"items":[{"title":"HIIT","subtitle":"30 min","icon":"flame"},{"title":"Strength","subtitle":"45 min","icon":"dumbbell"},{"title":"Yoga","subtitle":"25 min","icon":"leaf"}]}}</op_tool>`,
      `<op_tool>{"name":"add_bottom_nav_v0","arguments":{"items":[{"title":"Home","icon":"home","active":true},{"title":"Search","icon":"search"},{"title":"Profile","icon":"user"}]}}</op_tool>`,
    ].join('\n');

    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(5);

    const result = await dispatchElementToolCalls(shapes, { defaultParentId: rootId });
    expect(result.status).toBe('applied');

    // Ordering: children under root should be in the order the tools
    // were emitted — this matters for vertical layout visual output
    const roles = collectDirectChildRoles(rootId);
    expect(roles.length).toBe(5);
    expect(roles[0]).toBe('top-nav-bar');
    expect(roles[1]).toBe('stat-grid');
    expect(roles[2]).toBe('section-header');
    // The card_row wrapper is scroll-row-wrapper (per builder)
    expect(roles[3]).toBe('scroll-row-wrapper');
    expect(roles[4]).toBe('bottom-tab-bar');
  });
});

describe('orchestrator — full screen pipeline (settings)', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  it('settings: interleaved list rows + dividers preserve emission order', async () => {
    const rootId = seedGenerationRoot('Settings');

    const rawAi = [
      `<op_tool>{"name":"add_top_nav_bar_v0","arguments":{"title":"Settings","leading_icon":"chevron-left"}}</op_tool>`,
      `<op_tool>{"name":"add_list_row_v0","arguments":{"title":"Notifications","subtitle":"Push, email","leading_icon":"bell","trailing_icon":"chevron-right"}}</op_tool>`,
      `<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>`,
      `<op_tool>{"name":"add_list_row_v0","arguments":{"title":"Privacy","leading_icon":"shield","trailing_icon":"chevron-right"}}</op_tool>`,
      `<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>`,
      `<op_tool>{"name":"add_list_row_v0","arguments":{"title":"Dark mode","leading_icon":"moon"}}</op_tool>`,
      `<op_tool>{"name":"add_switch_v0","arguments":{"active":true}}</op_tool>`,
    ].join('\n');

    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(7);

    const result = await dispatchElementToolCalls(shapes, { defaultParentId: rootId });
    expect(result.status).toBe('applied');

    const roles = collectDirectChildRoles(rootId);
    expect(roles).toEqual([
      'top-nav-bar',
      'list-row',
      'divider',
      'list-row',
      'divider',
      'list-row',
      'switch',
    ]);
  });
});

describe('orchestrator — failure handling', () => {
  beforeEach(() => {
    useHistoryStore.getState().clear();
    vi.stubGlobal(
      'fetch',
      vi.fn(() => Promise.reject(new Error('fetch stubbed'))),
    );
  });

  it('mixed known + unknown tools: known apply, unknowns short-circuit, one undo batch', async () => {
    const rootId = seedGenerationRoot('Mixed');

    // Use `_v0` suffix so the parser regex accepts the name; only the
    // dispatcher short-circuits on unknown tools. A different suffix
    // (`_v9`) would be filtered at parse time, masking this failure.
    const rawAi = [
      `<op_tool>{"name":"add_heading_v0","arguments":{"content":"Real heading"}}</op_tool>`,
      `<op_tool>{"name":"add_fake_component_v0","arguments":{}}</op_tool>`,
      `<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Real body."}}</op_tool>`,
    ].join('\n');

    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(3);

    const startSpy = vi.spyOn(useHistoryStore.getState(), 'startBatch');
    const result = await dispatchElementToolCalls(shapes, { defaultParentId: rootId });

    // Partial: some applied, some unsupported
    expect(result.results).toHaveLength(3);
    const statuses = result.results.map((r) => r.status);
    expect(statuses.filter((s) => s === 'applied').length).toBe(2);
    expect(statuses.filter((s) => s === 'unsupported').length).toBe(1);

    // Only the two real ones should be under the root
    const count = collectDirectChildRoles(rootId).length;
    expect(count).toBe(2);

    // Still one undo batch for the whole emission even with one failure
    expect(startSpy).toHaveBeenCalledTimes(1);
    startSpy.mockRestore();
  });

  it('empty op_tool emission: parser returns [], dispatcher reports empty status', async () => {
    const rawAi = `This is just chat text, no tool tags at all.`;
    const shapes = tryParseAllElementToolOutputs(rawAi);
    expect(shapes).toHaveLength(0);

    const result = await dispatchElementToolCalls(shapes, {});
    expect(result.status).toBe('empty');
    expect(result.results).toHaveLength(0);
  });
});
