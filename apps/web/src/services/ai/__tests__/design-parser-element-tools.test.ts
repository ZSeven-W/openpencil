import { describe, it, expect } from 'vitest';
import { tryParseElementToolOutput } from '../design-parser';

/**
 * Element-tool output-shape detection. Sits in front of the legacy
 * JSONL parser when the N-tool feature flag is on. Contract mirrors
 * the corpus A/B parser so production behavior matches the measured
 * results in openpencil-docs
 * superpowers/notes/2026-04-20-ab-v1-results.md.
 */
describe('tryParseElementToolOutput — element-tool path', () => {
  it('detects a single <op_tool> with an add_*_v0 name', () => {
    const raw =
      '<op_tool>{"name":"add_card_row_v0","arguments":{"items":[{"title":"Hiit"}]}}</op_tool>';
    const out = tryParseElementToolOutput(raw);
    expect(out).not.toBeNull();
    if (!out) throw new Error();
    expect(out.kind).toBe('element-tool');
    if (out.kind !== 'element-tool') throw new Error();
    expect(out.name).toBe('add_card_row_v0');
    expect(out.arguments).toEqual({ items: [{ title: 'Hiit' }] });
  });

  it('prefers element-tool over batch_design when both appear', () => {
    // Observed weak-model pattern (MiniMax M2.7): emits batch_design
    // scaffold + add_nav_chip_row_v0 call + another batch_design.
    // Element-tool intent must win the dispatch.
    const raw =
      '<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {})"}}</op_tool>\n' +
      '<op_tool>{"name":"add_nav_chip_row_v0","arguments":{"items":[{"label":"All"}]}}</op_tool>';
    const out = tryParseElementToolOutput(raw);
    if (!out || out.kind !== 'element-tool') throw new Error();
    expect(out.name).toBe('add_nav_chip_row_v0');
  });

  it('strips <think> chain-of-thought before detection', () => {
    const raw =
      '<think>Let me choose the right element tool...</think>\n' +
      '<op_tool>{"name":"add_empty_state_v0","arguments":{"title":"No items"}}</op_tool>';
    const out = tryParseElementToolOutput(raw);
    if (!out || out.kind !== 'element-tool') throw new Error();
    expect(out.name).toBe('add_empty_state_v0');
  });
});

describe('tryParseElementToolOutput — batch_design path', () => {
  it('unwraps <op_tool>{name:"batch_design"} and returns the DSL string', () => {
    const raw =
      '<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {\\"type\\":\\"frame\\"})"}}</op_tool>';
    const out = tryParseElementToolOutput(raw);
    if (!out || out.kind !== 'batch-design-dsl') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });
});

describe('tryParseElementToolOutput — legacy passthrough', () => {
  it('returns null for JSONL-looking output so the legacy parser runs', () => {
    const raw = '{"type":"frame","name":"Root","children":[]}';
    expect(tryParseElementToolOutput(raw)).toBeNull();
  });

  it('returns null for a bare PenNode tree', () => {
    const raw = '[{"type":"frame","name":"Root","children":[]}]';
    expect(tryParseElementToolOutput(raw)).toBeNull();
  });

  it('returns null for prose-only output', () => {
    expect(tryParseElementToolOutput("I'm happy to help!")).toBeNull();
  });

  it('returns null for empty output', () => {
    expect(tryParseElementToolOutput('')).toBeNull();
    expect(tryParseElementToolOutput('   ')).toBeNull();
  });

  it('returns null for malformed <op_tool> tag (falls through to legacy)', () => {
    // Corpus parser returns garbage for these; from the sub-agent's
    // perspective, we want the legacy JSONL parser to have a chance
    // before giving up. null is the signal "not element-tool shape".
    expect(tryParseElementToolOutput('<op_tool>not json</op_tool>')).toBeNull();
  });
});
