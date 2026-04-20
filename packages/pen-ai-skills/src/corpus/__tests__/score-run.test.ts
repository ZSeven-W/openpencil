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
  kind: 'tool_call',
  name: 'add_card_row_v0',
  arguments: {},
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
    expect(row.outputKind).toBe('tool_call');
    expect(row.toolName).toBe('add_card_row_v0');
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

  it('wrong-tool when treatment obvious + tool routed but mis-matched', async () => {
    const wrongTool: ParsedOutput = {
      kind: 'tool_call',
      name: 'add_metric_row_v0',
      arguments: {},
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
    expect(row.toolName).toBe('add_metric_row_v0');
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
