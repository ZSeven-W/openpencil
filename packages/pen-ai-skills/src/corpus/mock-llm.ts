import type { CorpusPrompt, ParsedOutput } from './types';

/**
 * Deterministic mock-LLM used by the local A/B harness. The real
 * LLM runs in CI / staging against the full model zoo; the mock is
 * for local unit-test coverage of the harness plumbing (scorer +
 * output-parser + apply pipeline) without burning API dollars or
 * waiting on network round-trips.
 *
 * Behavior per variant:
 *
 * - Treatment (`variant: 'T'`): for prompts with `expected_tool_if_any`,
 *   emit a single `<op_tool>{"name": ..., "arguments": {}}</op_tool>`
 *   block naming that tool. This simulates the "ideal" routing outcome
 *   (what we hope the real model does). For prompts WITHOUT a hint
 *   (`difficulty: 'optional'`), fall through to the baseline path.
 *
 * - Baseline (`variant: 'B'`): emit a minimal batch_design DSL that
 *   creates a single frame (sufficient to pass through the apply
 *   pipeline). The point of baseline is "no element tools" — the real
 *   model would produce whatever DSL it picked; the mock just needs
 *   something that parses and applies.
 *
 * The mock does NOT try to produce correct / passing output. The
 * harness test's job is to verify the SCORER correctly classifies
 * the mocked output — M1/M3/M5 results will match what the scorer
 * would produce on the same raw output from a real model.
 *
 * To simulate specific failure modes (garbage / wrong-tool / empty
 * payload), pass `overrides` — the caller can pin a prompt to emit
 * anything. Without an override, the defaults described above apply.
 */
export interface MockLlmOverride {
  /** Prompt id to override. */
  promptId: string;
  /** Variant this override applies to. */
  variant: 'B' | 'T';
  /** The raw string the mock should return. */
  raw: string;
}

export interface MockLlmArgs {
  prompt: CorpusPrompt;
  variant: 'B' | 'T';
  /** When the prompt id + variant matches an override, return its raw. */
  overrides?: MockLlmOverride[];
}

/**
 * Return the raw string a real LLM would emit for this prompt.
 * Does NOT parse — feed the result through `parseModelOutput()` from
 * `output-parser.ts` to get a `ParsedOutput` for the scorer.
 */
export function mockLlmRaw(args: MockLlmArgs): string {
  const { prompt, variant, overrides = [] } = args;
  const hit = overrides.find((o) => o.promptId === prompt.id && o.variant === variant);
  if (hit) return hit.raw;

  if (variant === 'T' && prompt.expected_tool_if_any) {
    const name = prompt.expected_tool_if_any;
    return `<op_tool>${JSON.stringify({ name, arguments: {} })}</op_tool>`;
  }

  // Baseline: emit a minimal batch_design DSL. Creates a single frame
  // stamped with one of the expected roles when we have one, otherwise
  // a generic frame. This is enough to pass apply but typically FAILS
  // the expected-shape check for multi-role prompts — which is fine,
  // that's the whole point of baseline vs treatment comparison.
  const roleHint = prompt.expected.must_contain_roles?.[0] ?? 'frame';
  const body = JSON.stringify({
    type: 'frame',
    name: 'Baseline',
    role: roleHint,
    width: 200,
    height: 100,
    layout: 'vertical',
  });
  return `root=I(null, ${body})`;
}

/**
 * Convenience: mock LLM returning an already-parsed output. Useful
 * when the caller wants to short-circuit the raw-string → parser
 * round-trip (e.g. to pin a specific ParsedOutput kind regardless of
 * what the parser would've inferred).
 */
export function mockLlmParsed(args: MockLlmArgs): ParsedOutput {
  const raw = mockLlmRaw(args);
  // Intentionally NOT importing parseModelOutput here — callers that
  // want the parser pipeline should call it themselves, and callers
  // that want to pin a parsed shape use this. Keeps the two sides
  // of the mock decoupled (raw-level vs parsed-level).
  if (raw.startsWith('<op_tool>')) {
    // Collect every tag, not just the first — composite-prompt mocks
    // may emit multi-call raw strings.
    const tagRe = /<op_tool>\s*([\s\S]*?)\s*<\/op_tool>/g;
    const calls: { name: string; arguments: Record<string, unknown> }[] = [];
    for (const m of raw.matchAll(tagRe)) {
      try {
        const parsed = JSON.parse(m[1]) as { name?: string; arguments?: Record<string, unknown> };
        if (parsed.name) {
          calls.push({ name: parsed.name, arguments: parsed.arguments ?? {} });
        }
      } catch {
        // skip malformed tag
      }
    }
    if (calls.length === 0) {
      return { kind: 'garbage', reason: 'malformed op_tool block', raw };
    }
    return { kind: 'tool_calls', calls, raw };
  }
  if (/^\s*(?:\w+\s*=\s*[ICRMG]\(|[ICRGUDM]\()/.test(raw)) {
    return { kind: 'batch_design', dsl: raw, raw };
  }
  return { kind: 'garbage', reason: 'no recognizable output kind', raw };
}
