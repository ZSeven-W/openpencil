import { describe, expect, it } from 'vitest';
import { buildSystemPrompt } from '../build-prompt';

// These tests work entirely from the prompt builder's observable output —
// length deltas + marker strings — so they don't need to import the
// internal pen-ai-skills registry (which doesn't resolve cleanly under
// vitest from a path outside apps/web). The relative cookbook size is
// large enough that structural assertions catch any regression where
// the diet silently no-ops.

const T_PRIMARY_MARKER = 'PRIMARY: when your intent matches an add_*_v0 element tool above';
const B_BATCH_MARKER =
  '<op_tool>{"name": "batch_design", "arguments": {"operations": "<DSL_STRING>"}}</op_tool>';

describe('buildSystemPrompt', () => {
  it('T + composite returns the largest prompt (cookbook present)', () => {
    const composite = buildSystemPrompt('T', { difficulty: 'composite' });
    const obvious = buildSystemPrompt('T', { difficulty: 'obvious' });
    expect(composite.system.length).toBeGreaterThan(obvious.system.length);
  });

  it('T + undefined difficulty matches the composite size (safe default)', () => {
    const undef = buildSystemPrompt('T');
    const composite = buildSystemPrompt('T', { difficulty: 'composite' });
    expect(undef.system.length).toBe(composite.system.length);
  });

  it("T + difficulty='optional' matches the composite size", () => {
    const optional = buildSystemPrompt('T', { difficulty: 'optional' });
    const composite = buildSystemPrompt('T', { difficulty: 'composite' });
    expect(optional.system.length).toBe(composite.system.length);
  });

  it("T + difficulty='obvious' shaves at least 10kb off the prompt", () => {
    const obvious = buildSystemPrompt('T', { difficulty: 'obvious' }).system.length;
    const composite = buildSystemPrompt('T', { difficulty: 'composite' }).system.length;
    // Cookbook is ~18kb; expect at least 10kb savings to guard against
    // accidental regressions where the strip silently no-ops.
    expect(composite - obvious).toBeGreaterThan(10_000);
  });

  it('B prompt is smaller than every T variant (both skills stripped)', () => {
    const b = buildSystemPrompt('B').system.length;
    const tComposite = buildSystemPrompt('T', { difficulty: 'composite' }).system.length;
    const tObvious = buildSystemPrompt('T', { difficulty: 'obvious' }).system.length;
    expect(b).toBeLessThan(tObvious);
    expect(b).toBeLessThan(tComposite);
  });

  it('B is the same regardless of difficulty (variant comparison stays clean)', () => {
    const sizes = (['obvious', 'optional', 'composite', undefined] as const).map((d) => {
      const built = buildSystemPrompt('B', d ? { difficulty: d } : {});
      return built.system.length;
    });
    const distinct = new Set(sizes);
    expect(distinct.size).toBe(1);
  });

  it('every T variant carries the PRIMARY/FALLBACK output-format marker', () => {
    for (const difficulty of ['obvious', 'optional', 'composite'] as const) {
      const built = buildSystemPrompt('T', { difficulty });
      expect(built.system).toContain(T_PRIMARY_MARKER);
    }
  });

  it('B variant carries the batch_design marker but not the PRIMARY split', () => {
    const built = buildSystemPrompt('B');
    expect(built.system).toContain(B_BATCH_MARKER);
    expect(built.system).not.toContain(T_PRIMARY_MARKER);
  });
});
