import { describe, it, expect } from 'vitest';
import { resolve } from 'node:path';
import { loadCorpus } from '../corpus-loader';
import { mockLlmParsed, mockLlmRaw } from '../mock-llm';
import { parseModelOutput } from '../output-parser';
import { scoreRun } from '../score-run';
import type { ApplyFn, ApplyResult, CorpusPrompt, ParsedOutput } from '../types';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';

/**
 * Local A/B harness (mock-LLM driven). Exercises the full pipeline:
 *
 *   corpus file → mockLlmRaw → parseModelOutput → scoreRun.apply → ScoreRow
 *
 * without burning real LLM tokens. This is the unit-level gate on
 * the scorer + parser + mock integration. Real A/B runs against the
 * model zoo live in `scripts/ab-corpus/` and hit the network.
 *
 * What we're verifying:
 *   1. The mock produces output matching every corpus file's schema
 *      (no regression in the shape the parser expects)
 *   2. Treatment routing: for difficulty='obvious' prompts, the mock
 *      emits tool_call parsed output naming expected_tool_if_any
 *   3. Baseline path: emits batch_design parsed output, never a
 *      tool_call
 *   4. Overrides correctly pin specific outputs (garbage / wrong
 *      tool / empty)
 *   5. Scorer classifies all four routing outcomes correctly
 *      (right-tool / wrong-tool / fallback / garbage) when fed the
 *      mock's output
 */

function fakeDoc(roles: string[]): PenDocument {
  // Build a flat tree with one node per role. Minimal but enough to
  // pass role-counting for most small corpus expectations.
  const children: PenNode[] = roles.map((role, i) => ({
    id: `n${i}`,
    type: 'frame',
    name: role,
    role,
    children: [],
  })) as PenNode[];
  return { version: '1.0.0', children } as PenDocument;
}

function makeApply(roles: string[]): ApplyFn {
  return async () =>
    ({
      ok: true,
      error: '',
      doc: fakeDoc(roles),
    }) satisfies ApplyResult;
}

const FAILED_APPLY: ApplyFn = async () =>
  ({
    ok: false,
    error: 'mock apply failure',
    doc: null,
  }) satisfies ApplyResult;

// Load the real ab-v1 corpus from disk so this test tracks whatever
// the checked-in prompts look like. If the corpus grows a new
// field or role convention, this test is the first canary.
const CORPUS_DIR = resolve(__dirname, '../../../corpus/ab-v1');
const CORPUS: CorpusPrompt[] = loadCorpus(CORPUS_DIR);

describe('mock LLM + scorer end-to-end', () => {
  it('corpus loaded successfully (sanity)', () => {
    expect(CORPUS.length).toBeGreaterThan(0);
    // Every obvious prompt has the required hint (loader already enforces
    // this, but assert explicitly for documentation value).
    for (const p of CORPUS) {
      if (p.difficulty === 'obvious') {
        expect(p.expected_tool_if_any, `${p.id} obvious → hint required`).toBeTruthy();
      }
    }
  });

  describe('mock output shape', () => {
    it('T variant on obvious prompt → tool_call matching expected_tool_if_any', () => {
      const obvious = CORPUS.filter((p) => p.difficulty === 'obvious');
      expect(obvious.length).toBeGreaterThan(0);
      for (const p of obvious) {
        const parsed = mockLlmParsed({ prompt: p, variant: 'T' });
        expect(parsed.kind, `${p.id} T → tool_call`).toBe('tool_call');
        if (parsed.kind === 'tool_call') {
          expect(parsed.name).toBe(p.expected_tool_if_any);
        }
      }
    });

    it('B variant → batch_design (never tool_call) for every prompt', () => {
      for (const p of CORPUS) {
        const parsed = mockLlmParsed({ prompt: p, variant: 'B' });
        expect(parsed.kind, `${p.id} B → batch_design`).toBe('batch_design');
      }
    });

    it('T variant on optional prompt without hint → batch_design fallthrough', () => {
      const optional: CorpusPrompt = {
        id: 'hypothetical-optional',
        category: 'mobile',
        difficulty: 'optional',
        prompt: 'design something',
        expected: {},
      };
      const parsed = mockLlmParsed({ prompt: optional, variant: 'T' });
      expect(parsed.kind).toBe('batch_design');
    });
  });

  describe('raw → parser round-trip', () => {
    it('every mocked T output parses to the same kind as mockLlmParsed', () => {
      for (const p of CORPUS) {
        const raw = mockLlmRaw({ prompt: p, variant: 'T' });
        const direct = mockLlmParsed({ prompt: p, variant: 'T' });
        const viaParser = parseModelOutput(raw);
        expect(viaParser.kind, `${p.id}: parser kind mismatch`).toBe(direct.kind);
      }
    });

    it('every mocked B output parses to batch_design', () => {
      for (const p of CORPUS) {
        const raw = mockLlmRaw({ prompt: p, variant: 'B' });
        const viaParser = parseModelOutput(raw);
        expect(viaParser.kind, `${p.id} B should parse as batch_design`).toBe('batch_design');
      }
    });
  });

  describe('overrides', () => {
    it('garbage override produces parsed.kind=garbage', () => {
      const p = CORPUS[0];
      const parsed = mockLlmParsed({
        prompt: p,
        variant: 'T',
        overrides: [{ promptId: p.id, variant: 'T', raw: '<<< not valid anything >>>' }],
      });
      expect(parsed.kind).toBe('garbage');
    });

    it('malformed op_tool (missing name) → garbage', () => {
      const p = CORPUS[0];
      const parsed = mockLlmParsed({
        prompt: p,
        variant: 'T',
        overrides: [{ promptId: p.id, variant: 'T', raw: '<op_tool>{"arguments":{}}</op_tool>' }],
      });
      expect(parsed.kind).toBe('garbage');
    });

    it('wrong-tool override produces parsed.kind=tool_call with a different name', () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed = mockLlmParsed({
        prompt: p,
        variant: 'T',
        overrides: [
          {
            promptId: p.id,
            variant: 'T',
            raw: '<op_tool>{"name":"add_not_this_tool_v0","arguments":{}}</op_tool>',
          },
        ],
      });
      expect(parsed.kind).toBe('tool_call');
      if (parsed.kind === 'tool_call') {
        expect(parsed.name).not.toBe(p.expected_tool_if_any);
      }
    });

    it('override only applies to the matching (promptId, variant) tuple', () => {
      const p = CORPUS[0];
      // Set B override; T should still follow defaults.
      const bParsed = mockLlmParsed({
        prompt: p,
        variant: 'B',
        overrides: [{ promptId: p.id, variant: 'B', raw: '<op_tool>{"name":"x"}</op_tool>' }],
      });
      const tParsed = mockLlmParsed({
        prompt: p,
        variant: 'T',
        overrides: [{ promptId: p.id, variant: 'B', raw: '<op_tool>{"name":"x"}</op_tool>' }],
      });
      expect(bParsed.kind).toBe('tool_call');
      // T got no override → defaults (tool_call iff obvious else batch_design)
      expect(tParsed.kind).toBe(
        p.difficulty === 'obvious' && p.expected_tool_if_any ? 'tool_call' : 'batch_design',
      );
    });
  });

  describe('scorer classifies mock output correctly', () => {
    it('T + right tool + apply produces expected roles → routing=right-tool, m3=true', async () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed = mockLlmParsed({ prompt: p, variant: 'T' });
      const expectedRoles = p.expected.must_contain_roles ?? [];
      // Generate enough of each role to satisfy min_roles thresholds
      const allRoles: string[] = [];
      for (const r of expectedRoles) {
        const min = p.expected.min_roles?.[r] ?? 1;
        for (let i = 0; i < min; i += 1) allRoles.push(r);
      }
      // Plus the min_roles entries that aren't in must_contain_roles
      for (const [role, count] of Object.entries(p.expected.min_roles ?? {})) {
        if (!expectedRoles.includes(role)) {
          for (let i = 0; i < count; i += 1) allRoles.push(role);
        }
      }
      const row = await scoreRun({
        prompt: p,
        parsed,
        apply: makeApply(allRoles),
        model: 'mock',
        variant: 'T',
      });
      expect(row.routing).toBe('right-tool');
      expect(row.m1_legal).toBe(true);
      expect(row.m3_success).toBe(true);
    });

    it('T + wrong-tool override → routing=wrong-tool', async () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed: ParsedOutput = {
        kind: 'tool_call',
        name: 'add_definitely_wrong_v0',
        arguments: {},
        raw: '',
      };
      const row = await scoreRun({
        prompt: p,
        parsed,
        apply: makeApply([]),
        model: 'mock',
        variant: 'T',
      });
      expect(row.routing).toBe('wrong-tool');
    });

    it('T + garbage → routing=garbage, m1=false, m3=false', async () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed: ParsedOutput = {
        kind: 'garbage',
        reason: 'synthetic',
        raw: '',
      };
      const row = await scoreRun({
        prompt: p,
        parsed,
        apply: FAILED_APPLY,
        model: 'mock',
        variant: 'T',
      });
      expect(row.routing).toBe('garbage');
      expect(row.m1_legal).toBe(false);
      expect(row.m3_success).toBe(false);
    });

    it('T + batch_design fallback → routing=fallback on obvious prompt', async () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed: ParsedOutput = {
        kind: 'batch_design',
        dsl: 'root=I(null, {})',
        raw: '',
      };
      const row = await scoreRun({
        prompt: p,
        parsed,
        apply: makeApply([]),
        model: 'mock',
        variant: 'T',
      });
      expect(row.routing).toBe('fallback');
    });

    it('B variant → routing=n/a (baseline runs are not routing-scored)', async () => {
      const p = CORPUS.find((c) => c.difficulty === 'obvious')!;
      const parsed = mockLlmParsed({ prompt: p, variant: 'B' });
      const row = await scoreRun({
        prompt: p,
        parsed,
        apply: makeApply([]),
        model: 'mock',
        variant: 'B',
      });
      expect(row.routing).toBe('n/a');
    });
  });

  describe('bulk sweep: every corpus prompt × both variants scores without throwing', () => {
    it('B variant: all prompts score with default mock + zero-roles apply', async () => {
      for (const p of CORPUS) {
        const parsed = mockLlmParsed({ prompt: p, variant: 'B' });
        const row = await scoreRun({
          prompt: p,
          parsed,
          apply: makeApply([]),
          model: 'mock',
          variant: 'B',
        });
        expect(row.promptId, `${p.id} B row.promptId`).toBe(p.id);
      }
    });

    it('T variant: all obvious prompts score as right-tool with the ideal mock', async () => {
      const obvious = CORPUS.filter((c) => c.difficulty === 'obvious');
      for (const p of obvious) {
        const parsed = mockLlmParsed({ prompt: p, variant: 'T' });
        const row = await scoreRun({
          prompt: p,
          parsed,
          apply: makeApply([]),
          model: 'mock',
          variant: 'T',
        });
        expect(row.routing, `${p.id} T expected right-tool`).toBe('right-tool');
      }
    });
  });
});
