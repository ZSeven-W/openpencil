import { describe, expect, it } from 'vitest';
import { buildSystemPrompt } from '../build-prompt';

// These tests work entirely from the prompt builder's observable output —
// length deltas + marker strings — so they don't need to import the
// internal pen-ai-skills registry (which doesn't resolve cleanly under
// vitest from a path outside apps/web). The relative cookbook size is
// large enough that structural assertions catch any regression where
// the diet silently no-ops.

const T_MULTI_TOOL_MARKER = 'Respond with one or more `<op_tool>` tags';
const T_STRATEGY_A_MARKER = 'STRATEGY A — element tools';
const T_STRATEGY_B_MARKER = 'STRATEGY B — batch_design fallback';
const T_NO_MIX_MARKER = 'Do not mix Strategy A and Strategy B';
const B_BATCH_MARKER =
  '<op_tool>{"name": "batch_design", "arguments": {"operations": "<DSL_STRING>"}}</op_tool>';
// Earlier versions of the T instructions explicitly forbade multi-tool
// output ("Respond with one tag" / "Do not combine multiple tags");
// that wording undercut the multi-tool teaching baked into elements.md.
// A subsequent revision flipped to allow chaining but quietly invited
// mixed batch_design + element tags per-component, which the corpus
// parser silently drops. Codex stop-time review caught both. Guard
// against re-introducing either failure mode.
const FORBIDDEN_SINGLE_TAG_PHRASE = 'one `<op_tool>` tag, nothing else';
const FORBIDDEN_NO_COMBINE_PHRASE = 'Do not combine multiple tags';
const FORBIDDEN_PER_COMPONENT_FALLBACK_PHRASE = 'when no element tool fits a given component shape';

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

  it('every T variant carries the multi-tool + Strategy A/B output-format markers', () => {
    for (const difficulty of ['obvious', 'optional', 'composite'] as const) {
      const built = buildSystemPrompt('T', { difficulty });
      expect(built.system).toContain(T_MULTI_TOOL_MARKER);
      expect(built.system).toContain(T_STRATEGY_A_MARKER);
      expect(built.system).toContain(T_STRATEGY_B_MARKER);
    }
  });

  it('T variant does NOT forbid multi-tool output (regression guard)', () => {
    for (const difficulty of ['obvious', 'optional', 'composite'] as const) {
      const built = buildSystemPrompt('T', { difficulty });
      expect(built.system).not.toContain(FORBIDDEN_SINGLE_TAG_PHRASE);
      expect(built.system).not.toContain(FORBIDDEN_NO_COMBINE_PHRASE);
    }
  });

  it('T variant explicitly forbids mixing Strategy A and Strategy B (parser drops mixed)', () => {
    for (const difficulty of ['obvious', 'optional', 'composite'] as const) {
      const built = buildSystemPrompt('T', { difficulty });
      expect(built.system).toContain(T_NO_MIX_MARKER);
      // Earlier wording invited per-component fallback ("when no element
      // tool fits a given component shape") which the parser silently
      // drops when element calls are also present. Guard the rephrase.
      expect(built.system).not.toContain(FORBIDDEN_PER_COMPONENT_FALLBACK_PHRASE);
    }
  });

  it('B variant carries the batch_design marker but not the Strategy A/B split', () => {
    const built = buildSystemPrompt('B');
    expect(built.system).toContain(B_BATCH_MARKER);
    expect(built.system).not.toContain(T_STRATEGY_A_MARKER);
    expect(built.system).not.toContain(T_STRATEGY_B_MARKER);
  });

  it('T + category=mobile drops dashboard/landing recipes from cookbook', () => {
    const mobile = buildSystemPrompt('T', { difficulty: 'composite', category: 'mobile' });
    // Dashboard-tagged recipe titles should be gone (Team / members list,
    // Audit / activity feed, KPI strip, Faceted search filter sidebar).
    expect(mobile.system).not.toContain('### Team / members list');
    expect(mobile.system).not.toContain('### Audit / activity feed');
    expect(mobile.system).not.toContain('### Dashboard KPI strip');
    expect(mobile.system).not.toContain('### Faceted search filter sidebar');
    // Landing-tagged recipe (Pricing) should be gone.
    expect(mobile.system).not.toContain('### Pricing section');
    // Mobile-tagged recipes should remain.
    expect(mobile.system).toContain('### Onboarding "How it works"');
    expect(mobile.system).toContain('### Support chat thread');
  });

  it('T + category=dashboard drops mobile/landing recipes from cookbook', () => {
    const dashboard = buildSystemPrompt('T', { difficulty: 'composite', category: 'dashboard' });
    // Mobile-tagged recipes should be gone.
    expect(dashboard.system).not.toContain('### Onboarding "How it works"');
    expect(dashboard.system).not.toContain('### Support chat thread');
    // Landing-tagged recipe should be gone.
    expect(dashboard.system).not.toContain('### Pricing section');
    // Dashboard-tagged recipes should remain.
    expect(dashboard.system).toContain('### Team / members list');
    expect(dashboard.system).toContain('### Audit / activity feed');
    expect(dashboard.system).toContain('### Dashboard KPI strip');
    expect(dashboard.system).toContain('### Faceted search filter sidebar');
  });

  it('T + category=landing drops mobile/dashboard recipes from cookbook', () => {
    const landing = buildSystemPrompt('T', { difficulty: 'composite', category: 'landing' });
    // Mobile-tagged recipes should be gone.
    expect(landing.system).not.toContain('### Onboarding "How it works"');
    expect(landing.system).not.toContain('### Support chat thread');
    // Dashboard-tagged recipes should be gone.
    expect(landing.system).not.toContain('### Team / members list');
    expect(landing.system).not.toContain('### Dashboard KPI strip');
    // Landing-tagged recipe should remain.
    expect(landing.system).toContain('### Pricing section');
  });

  it('T + category preserves untagged content (Empty inbox / Login etc. stay general)', () => {
    // Untagged recipes should be visible regardless of category.
    for (const category of ['mobile', 'dashboard', 'landing'] as const) {
      const built = buildSystemPrompt('T', { difficulty: 'composite', category });
      expect(built.system).toContain('### Empty inbox / first-run onboarding');
      // Login + Signup are untagged today (cross-domain ambiguity); they
      // load for every category until we either tag with a comma list
      // or split them into single-domain variants.
      expect(built.system).toContain('### Login screen');
      expect(built.system).toContain('### Signup form');
    }
  });

  it('T + category undefined = no filtering (every tagged recipe loads)', () => {
    const full = buildSystemPrompt('T', { difficulty: 'composite' });
    expect(full.system).toContain('### Team / members list');
    expect(full.system).toContain('### Onboarding "How it works"');
    expect(full.system).toContain('### Pricing section');
    expect(full.system).toContain('### Empty inbox / first-run onboarding');
  });

  it('T + category produces a smaller prompt than uncategorized (filter actually fires)', () => {
    const full = buildSystemPrompt('T', { difficulty: 'composite' }).system.length;
    for (const category of ['mobile', 'dashboard', 'landing'] as const) {
      const filtered = buildSystemPrompt('T', { difficulty: 'composite', category }).system.length;
      expect(filtered).toBeLessThan(full);
      // Each filter should drop at least 500 chars (loose floor — guards
      // against the regex silently no-oping, not a tight savings target).
      expect(full - filtered).toBeGreaterThan(500);
    }
  });
});
