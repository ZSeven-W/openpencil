import { describe, it, expect } from 'vitest';
import { scoreRun, countRoles } from '../score-run';
import type { ApplyFn, CorpusPrompt, ParsedOutput } from '../types';
import type { PenDocument } from '@zseven-w/pen-types';

const baseDoc: PenDocument = {
  version: '1.0.0',
  children: [
    {
      id: 'root',
      type: 'frame',
      name: 'Root',
      role: 'section',
      children: [
        {
          id: 'a',
          type: 'frame',
          name: 'A',
          role: 'card',
          children: [{ id: 'a1', type: 'text', role: 'heading', content: 'X' }],
        },
        {
          id: 'b',
          type: 'frame',
          name: 'B',
          role: 'card',
          children: [],
        },
      ],
    },
  ],
} as PenDocument;

const prompt: CorpusPrompt = {
  id: 'p1',
  category: 'mobile',
  difficulty: 'obvious',
  prompt: 'test',
  expected: {
    must_contain_roles: ['section'],
    min_roles: { card: 2 },
  },
  expected_tool_if_any: 'add_card_row_v0',
};

const okParsed: ParsedOutput = { kind: 'batch_design', dsl: 'root=I(null, {})', raw: '' };
const toolParsed: ParsedOutput = {
  kind: 'tool_calls',
  calls: [{ name: 'add_card_row_v0', arguments: {} }],
  raw: '',
};
const garbageParsed: ParsedOutput = { kind: 'garbage', reason: 'empty output', raw: '' };

function makeApply(result: { ok: boolean; doc: PenDocument | null; error?: string }): ApplyFn {
  return async () => ({ ok: result.ok, doc: result.doc, error: result.error ?? '' });
}

describe('countRoles', () => {
  it('counts roles across the full tree including nested', () => {
    const counts = countRoles(baseDoc);
    expect(counts.section).toBe(1);
    expect(counts.card).toBe(2);
    expect(counts.heading).toBe(1);
  });

  it('ignores nodes without a role', () => {
    const doc: PenDocument = {
      version: '1.0.0',
      children: [{ id: 'x', type: 'frame', name: 'X', children: [] }],
    } as PenDocument;
    expect(countRoles(doc)).toEqual({});
  });
});

describe('scoreRun — M1 / M3 gates', () => {
  it('passes M1 + M3 when apply succeeds and shape matches', async () => {
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m1_legal).toBe(true);
    expect(row.m3_success).toBe(true);
    expect(row.m3_failure_reason).toBe('');
  });

  it('fails M1 when apply throws', async () => {
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({ ok: false, doc: null, error: 'DSL parse failed' }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m1_legal).toBe(false);
    expect(row.m3_success).toBe(false);
    expect(row.applyError).toBe('DSL parse failed');
    expect(row.m3_failure_reason).toBe('apply failed before shape checks');
  });

  it('fails M3 when required role is missing', async () => {
    const shapeOnlyPrompt: CorpusPrompt = {
      ...prompt,
      expected: { must_contain_roles: ['hero'] },
    };
    const row = await scoreRun({
      prompt: shapeOnlyPrompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m1_legal).toBe(true);
    expect(row.m3_success).toBe(false);
    expect(row.m3_failure_reason).toMatch(/missing required role.*hero/);
  });

  it('fails M3 when min_roles not met', async () => {
    const strictPrompt: CorpusPrompt = {
      ...prompt,
      expected: { min_roles: { card: 5 } },
    };
    const row = await scoreRun({
      prompt: strictPrompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m3_success).toBe(false);
    expect(row.m3_failure_reason).toMatch(/role counts below minimum/);
  });

  it('partial apply (ok=false but doc populated): M1 false, M3 still scored on what landed', async () => {
    // ab-corpus's per-shape continuation surfaces a PenDocument even
    // when one of N composite tags throws. Scorer must run shape
    // checks against that partial doc — otherwise a model that
    // nailed 12/13 tags reads identical to one that crashed on tag 1.
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({
        ok: false,
        doc: baseDoc,
        error: '1/13 tag(s) failed: call 12/13 (add_heading_v0): invalid level "caption"',
      }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m1_legal).toBe(false);
    expect(row.m3_success).toBe(true);
    expect(row.m3_failure_reason).toMatch(/partial apply.*caption/);
    expect(row.applyError).toMatch(/caption/);
  });

  it('partial apply: M3 fails when shape miss ANYWAY (shape miss wins over partial-apply notice)', async () => {
    const shapeOnlyPrompt: CorpusPrompt = {
      ...prompt,
      expected: { must_contain_roles: ['hero'] },
    };
    const row = await scoreRun({
      prompt: shapeOnlyPrompt,
      parsed: okParsed,
      apply: makeApply({
        ok: false,
        doc: baseDoc,
        error: 'tag 5/8 failed',
      }),
      model: 'm1',
      variant: 'B',
    });
    expect(row.m1_legal).toBe(false);
    expect(row.m3_success).toBe(false);
    // Shape-miss reason wins because it's the structural verdict.
    expect(row.m3_failure_reason).toMatch(/missing required role.*hero/);
  });

  it('short-circuits to garbage when parse failed', async () => {
    const row = await scoreRun({
      prompt,
      parsed: garbageParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm1',
      variant: 'T',
    });
    expect(row.m1_legal).toBe(false);
    expect(row.m3_success).toBe(false);
    expect(row.outputKind).toBe('garbage');
    expect(row.m3_failure_reason).toMatch(/output unparseable/);
  });

  it('carries tool name through on tool_call output', async () => {
    const row = await scoreRun({
      prompt,
      parsed: toolParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm1',
      variant: 'T',
    });
    expect(row.outputKind).toBe('tool_calls');
    expect(row.toolNames).toEqual(['add_card_row_v0']);
  });
});

describe('scoreRun — routing classification (expected_tool_if_any)', () => {
  it('right-tool when treatment obvious + tool matches expected', async () => {
    const row = await scoreRun({
      prompt, // expected_tool_if_any = 'add_card_row_v0'
      parsed: toolParsed, // name = 'add_card_row_v0'
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('right-tool');
    expect(row.expectedToolIfAny).toBe('add_card_row_v0');
  });

  it('right-tool also matches when the expected tool appears alongside extras', async () => {
    // A model that emits `[expected_tool, extra_tool]` still routed
    // correctly to the expected tool — the extras are over-production,
    // not a routing miss. classifyRouting uses Array.includes.
    const overProduction: ParsedOutput = {
      kind: 'tool_calls',
      calls: [
        { name: 'add_card_row_v0', arguments: {} },
        { name: 'add_metric_row_v0', arguments: {} },
      ],
      raw: '',
    };
    const row = await scoreRun({
      prompt,
      parsed: overProduction,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('right-tool');
  });

  it('wrong-tool when treatment obvious + tool routed but mis-matched', async () => {
    const wrongTool: ParsedOutput = {
      kind: 'tool_calls',
      calls: [{ name: 'add_metric_row_v0', arguments: {} }],
      raw: '',
    };
    const row = await scoreRun({
      prompt,
      parsed: wrongTool,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('wrong-tool');
    expect(row.toolNames).toEqual(['add_metric_row_v0']);
    expect(row.expectedToolIfAny).toBe('add_card_row_v0');
  });

  it('fallback when treatment obvious + output is batch_design', async () => {
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('fallback');
  });

  it('n/a on baseline runs (routing is undefined there)', async () => {
    const row = await scoreRun({
      prompt,
      parsed: toolParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'B',
    });
    expect(row.routing).toBe('n/a');
  });

  it('n/a on optional difficulty (expected_tool_if_any undefined there)', async () => {
    const optionalPrompt: CorpusPrompt = {
      ...prompt,
      difficulty: 'optional',
      expected_tool_if_any: undefined,
    };
    const row = await scoreRun({
      prompt: optionalPrompt,
      parsed: toolParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('n/a');
    expect(row.expectedToolIfAny).toBe('');
  });

  it("'garbage' (not 'n/a') on garbage outputs — stays in M5 denominator so right-tool rate doesn't inflate when most runs fail to parse", async () => {
    const row = await scoreRun({
      prompt,
      parsed: garbageParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('garbage');
  });

  it('n/a on garbage outputs in baseline (routing is undefined for baseline regardless of parse status)', async () => {
    const row = await scoreRun({
      prompt,
      parsed: garbageParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'B',
    });
    expect(row.routing).toBe('n/a');
  });

  it('n/a on garbage outputs for optional prompts (routing is undefined regardless of parse status)', async () => {
    const optionalPrompt: CorpusPrompt = {
      ...prompt,
      difficulty: 'optional',
      expected_tool_if_any: undefined,
    };
    const row = await scoreRun({
      prompt: optionalPrompt,
      parsed: garbageParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('n/a');
  });
});

describe('scoreRun — composite difficulty', () => {
  const compositePrompt: CorpusPrompt = {
    ...prompt,
    difficulty: 'composite',
    expected_tool_if_any: undefined,
  };

  it('multi-tool when treatment emits a tool_call (no expected_tool match required)', async () => {
    const row = await scoreRun({
      prompt: compositePrompt,
      parsed: toolParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('multi-tool');
  });

  it('toolNames captures every emitted call from a multi-call output', async () => {
    const multi: ParsedOutput = {
      kind: 'tool_calls',
      calls: [
        { name: 'add_member_row_v0', arguments: {} },
        { name: 'add_member_row_v0', arguments: {} },
        { name: 'add_invite_row_v0', arguments: {} },
      ],
      raw: '',
    };
    const row = await scoreRun({
      prompt: compositePrompt,
      parsed: multi,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('multi-tool');
    expect(row.toolNames).toEqual(['add_member_row_v0', 'add_member_row_v0', 'add_invite_row_v0']);
  });

  it('fallback when treatment emits batch_design DSL', async () => {
    const row = await scoreRun({
      prompt: compositePrompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('fallback');
  });

  it('garbage when treatment output unparseable', async () => {
    const row = await scoreRun({
      prompt: compositePrompt,
      parsed: garbageParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'T',
    });
    expect(row.routing).toBe('garbage');
  });

  it('n/a on baseline (routing only meaningful for treatment)', async () => {
    const row = await scoreRun({
      prompt: compositePrompt,
      parsed: toolParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'B',
    });
    expect(row.routing).toBe('n/a');
  });
});

describe('scoreRun — token usage plumbing', () => {
  it('writes provided usage to row', async () => {
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'B',
      usage: { promptTokens: 1234, completionTokens: 56 },
    });
    expect(row.promptTokens).toBe(1234);
    expect(row.completionTokens).toBe(56);
  });

  it('defaults to 0/0 when usage omitted', async () => {
    const row = await scoreRun({
      prompt,
      parsed: okParsed,
      apply: makeApply({ ok: true, doc: baseDoc }),
      model: 'm',
      variant: 'B',
    });
    expect(row.promptTokens).toBe(0);
    expect(row.completionTokens).toBe(0);
  });
});
