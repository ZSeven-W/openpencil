import { describe, it, expect } from 'vitest';
import { aggregate } from '../aggregate';
import type { ScoreRow } from '../types';

function row(partial: Partial<ScoreRow>): ScoreRow {
  return {
    promptId: 'x',
    category: 'mobile',
    difficulty: 'obvious',
    model: 'm1',
    variant: 'B',
    outputKind: 'batch_design',
    toolName: '',
    expectedToolIfAny: '',
    routing: 'n/a',
    m1_legal: true,
    m3_success: true,
    m3_failure_reason: '',
    issues: [],
    applyError: '',
    rawOutput: '',
    promptTokens: 0,
    completionTokens: 0,
    ...partial,
  };
}

describe('aggregate — per-model summary', () => {
  it('computes M1 rates and delta per model', () => {
    const rows: ScoreRow[] = [
      row({ model: 'a', variant: 'B', m1_legal: true }),
      row({ model: 'a', variant: 'B', m1_legal: false }),
      row({ model: 'a', variant: 'T', m1_legal: true }),
      row({ model: 'a', variant: 'T', m1_legal: true }),
    ];
    const r = aggregate(rows);
    const a = r.byModel.find((m) => m.model === 'a');
    expect(a).toBeDefined();
    expect(a!.m1_baseline).toBeCloseTo(0.5);
    expect(a!.m1_treatment).toBeCloseTo(1.0);
    expect(a!.m1_delta_pp).toBeCloseTo(50);
  });

  it('routing breakdown splits 4-way right/wrong/fallback/garbage on obvious treatment', () => {
    // 4 obvious-treatment runs: 2 right, 1 wrong, 1 fallback → rates 0.5 / 0.25 / 0.25 / 0
    const rows: ScoreRow[] = [
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_link_v0',
        expectedToolIfAny: 'add_link_v0',
        routing: 'right-tool',
      }),
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_link_v0',
        expectedToolIfAny: 'add_link_v0',
        routing: 'right-tool',
      }),
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_divider_v0',
        expectedToolIfAny: 'add_link_v0',
        routing: 'wrong-tool',
      }),
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'batch_design',
        expectedToolIfAny: 'add_link_v0',
        routing: 'fallback',
      }),
      row({
        difficulty: 'optional',
        variant: 'T',
        outputKind: 'batch_design',
        routing: 'n/a',
      }),
      row({ variant: 'B', routing: 'n/a' }),
    ];
    const r = aggregate(rows);
    expect(r.byModel[0].m5_right_tool).toBeCloseTo(0.5);
    expect(r.byModel[0].m5_wrong_tool).toBeCloseTo(0.25);
    expect(r.byModel[0].m5_fallback).toBeCloseTo(0.25);
    expect(r.byModel[0].m5_garbage).toBeCloseTo(0);
  });

  it('garbage in denominator prevents inflated right-tool rate (the codex fix)', () => {
    // 10 garbage + 1 right + 1 wrong; naively computing right / (right+wrong)
    // would yield 50% "routing success" — but only 1/12 runs was actually
    // right. 4-way breakdown forces the denominator to include garbage.
    const rows: ScoreRow[] = [
      ...Array.from({ length: 10 }, () =>
        row({
          difficulty: 'obvious',
          variant: 'T',
          outputKind: 'garbage',
          routing: 'garbage',
        }),
      ),
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_link_v0',
        expectedToolIfAny: 'add_link_v0',
        routing: 'right-tool',
      }),
      row({
        difficulty: 'obvious',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_fab_v0',
        expectedToolIfAny: 'add_link_v0',
        routing: 'wrong-tool',
      }),
    ];
    const r = aggregate(rows);
    expect(r.byModel[0].m5_right_tool).toBeCloseTo(1 / 12);
    expect(r.byModel[0].m5_wrong_tool).toBeCloseTo(1 / 12);
    expect(r.byModel[0].m5_fallback).toBeCloseTo(0);
    expect(r.byModel[0].m5_garbage).toBeCloseTo(10 / 12);
  });

  it('four routing rates sum to 1 (obvious-treatment denominator)', () => {
    const rows: ScoreRow[] = [
      row({ difficulty: 'obvious', variant: 'T', routing: 'right-tool' }),
      row({ difficulty: 'obvious', variant: 'T', routing: 'wrong-tool' }),
      row({ difficulty: 'obvious', variant: 'T', routing: 'fallback' }),
      row({ difficulty: 'obvious', variant: 'T', routing: 'garbage' }),
    ];
    const r = aggregate(rows);
    const sum =
      r.byModel[0].m5_right_tool +
      r.byModel[0].m5_wrong_tool +
      r.byModel[0].m5_fallback +
      r.byModel[0].m5_garbage;
    expect(sum).toBeCloseTo(1);
  });

  it('returns NaN for rates with zero samples in a cell', () => {
    const rows: ScoreRow[] = [row({ model: 'empty-treatment', variant: 'B', m1_legal: true })];
    const r = aggregate(rows);
    const m = r.byModel.find((x) => x.model === 'empty-treatment');
    expect(m).toBeDefined();
    expect(Number.isNaN(m!.m1_treatment)).toBe(true);
    expect(Number.isNaN(m!.m1_delta_pp)).toBe(true);
    // All three routing rates NaN when there are no obvious-treatment
    // runs at all (denominator is zero).
    expect(Number.isNaN(m!.m5_right_tool)).toBe(true);
    expect(Number.isNaN(m!.m5_wrong_tool)).toBe(true);
    expect(Number.isNaN(m!.m5_fallback)).toBe(true);
    expect(Number.isNaN(m!.m5_garbage)).toBe(true);
  });
});

describe('aggregate — per-category + per-tool', () => {
  it('splits M1 by category, both variants', () => {
    const rows: ScoreRow[] = [
      row({ category: 'mobile', variant: 'B', m1_legal: false }),
      row({ category: 'mobile', variant: 'T', m1_legal: true }),
      row({ category: 'landing', variant: 'B', m1_legal: true }),
      row({ category: 'landing', variant: 'T', m1_legal: true }),
    ];
    const r = aggregate(rows);
    const mobile = r.byCategory.find((c) => c.category === 'mobile')!;
    expect(mobile.m1_baseline).toBe(0);
    expect(mobile.m1_treatment).toBe(1);
    const landing = r.byCategory.find((c) => c.category === 'landing')!;
    expect(landing.m1_baseline).toBe(1);
    expect(landing.m1_treatment).toBe(1);
  });

  it('ranks tools by invocation count, descending', () => {
    const rows: ScoreRow[] = [
      row({ variant: 'T', outputKind: 'tool_call', toolName: 'add_link_v0', m1_legal: true }),
      row({ variant: 'T', outputKind: 'tool_call', toolName: 'add_link_v0', m1_legal: false }),
      row({ variant: 'T', outputKind: 'tool_call', toolName: 'add_divider_v0', m1_legal: true }),
    ];
    const r = aggregate(rows);
    expect(r.byTool).toHaveLength(2);
    expect(r.byTool[0]).toEqual({
      tool: 'add_link_v0',
      invocations: 2,
      successfulInvocations: 1,
    });
    expect(r.byTool[1]).toEqual({
      tool: 'add_divider_v0',
      invocations: 1,
      successfulInvocations: 1,
    });
  });

  it('ignores baseline runs in tool usage', () => {
    const rows: ScoreRow[] = [
      row({ variant: 'B', outputKind: 'batch_design' }),
      row({ variant: 'T', outputKind: 'tool_call', toolName: 'add_fab_v0' }),
    ];
    const r = aggregate(rows);
    expect(r.byTool).toHaveLength(1);
    expect(r.byTool[0].tool).toBe('add_fab_v0');
  });

  it('totalRuns reflects input length', () => {
    const r = aggregate([row({}), row({}), row({})]);
    expect(r.totalRuns).toBe(3);
  });
});

describe('aggregate — composite difficulty', () => {
  it('m6 splits multi-tool / fallback / garbage on composite-treatment runs only', () => {
    const rows: ScoreRow[] = [
      row({
        difficulty: 'composite',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_link_v0',
        routing: 'multi-tool',
      }),
      row({
        difficulty: 'composite',
        variant: 'T',
        outputKind: 'tool_call',
        toolName: 'add_card_row_v0',
        routing: 'multi-tool',
      }),
      row({
        difficulty: 'composite',
        variant: 'T',
        outputKind: 'batch_design',
        routing: 'fallback',
      }),
      row({ difficulty: 'composite', variant: 'T', outputKind: 'garbage', routing: 'garbage' }),
      // Obvious row should not pollute composite buckets
      row({ difficulty: 'obvious', variant: 'T', routing: 'right-tool' }),
    ];
    const r = aggregate(rows);
    const m = r.byModel[0];
    expect(m.m6_multi_tool).toBeCloseTo(0.5);
    expect(m.m6_fallback).toBeCloseTo(0.25);
    expect(m.m6_garbage).toBeCloseTo(0.25);
    // Three composite buckets sum to 1
    expect(m.m6_multi_tool + m.m6_fallback + m.m6_garbage).toBeCloseTo(1);
    // Obvious row keeps right-tool counted in m5
    expect(m.m5_right_tool).toBeCloseTo(1);
  });

  it('m6 returns NaN when no composite-treatment rows', () => {
    const rows: ScoreRow[] = [row({ difficulty: 'obvious', variant: 'T', routing: 'right-tool' })];
    const r = aggregate(rows);
    const m = r.byModel[0];
    expect(Number.isNaN(m.m6_multi_tool)).toBe(true);
    expect(Number.isNaN(m.m6_fallback)).toBe(true);
    expect(Number.isNaN(m.m6_garbage)).toBe(true);
  });
});

describe('aggregate — token cost averages', () => {
  it('averages prompt + completion tokens, baseline vs treatment', () => {
    const rows: ScoreRow[] = [
      row({ variant: 'B', promptTokens: 1000, completionTokens: 200 }),
      row({ variant: 'B', promptTokens: 1200, completionTokens: 300 }),
      row({ variant: 'T', promptTokens: 800, completionTokens: 150 }),
      row({ variant: 'T', promptTokens: 600, completionTokens: 100 }),
    ];
    const r = aggregate(rows);
    const m = r.byModel[0];
    expect(m.avgPromptTokensBaseline).toBeCloseTo(1100);
    expect(m.avgPromptTokensTreatment).toBeCloseTo(700);
    expect(m.avgCompletionTokensBaseline).toBeCloseTo(250);
    expect(m.avgCompletionTokensTreatment).toBeCloseTo(125);
  });

  it('skips rows with 0/0 usage so codex/harness-error rows do not deflate average', () => {
    const rows: ScoreRow[] = [
      row({ variant: 'T', promptTokens: 1000, completionTokens: 200 }),
      row({ variant: 'T', promptTokens: 0, completionTokens: 0 }), // codex-cli or harness error
    ];
    const r = aggregate(rows);
    const m = r.byModel[0];
    expect(m.avgPromptTokensTreatment).toBeCloseTo(1000);
    expect(m.avgCompletionTokensTreatment).toBeCloseTo(200);
  });

  it('returns NaN when no rows in a cell have usage data', () => {
    const rows: ScoreRow[] = [
      row({ variant: 'B', promptTokens: 0, completionTokens: 0 }),
      row({ variant: 'T', promptTokens: 0, completionTokens: 0 }),
    ];
    const r = aggregate(rows);
    const m = r.byModel[0];
    expect(Number.isNaN(m.avgPromptTokensBaseline)).toBe(true);
    expect(Number.isNaN(m.avgPromptTokensTreatment)).toBe(true);
    expect(Number.isNaN(m.avgCompletionTokensBaseline)).toBe(true);
    expect(Number.isNaN(m.avgCompletionTokensTreatment)).toBe(true);
  });
});
