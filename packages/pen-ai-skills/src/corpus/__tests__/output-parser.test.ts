import { describe, it, expect } from 'vitest';
import { parseModelOutput } from '../output-parser';

describe('parseModelOutput — tool_call path', () => {
  it('extracts name + arguments from <op_tool> tags', () => {
    const raw = `<op_tool>{"name": "add_card_row_v0", "arguments": {"items": [{"title": "A"}]}}</op_tool>`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
    if (out.kind !== 'tool_call') throw new Error();
    expect(out.name).toBe('add_card_row_v0');
    expect(out.arguments).toEqual({ items: [{ title: 'A' }] });
  });

  it('trims whitespace inside tags', () => {
    const raw = `<op_tool>
      {"name": "add_link_v0", "arguments": {"label": "hi"}}
    </op_tool>`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
  });

  it('survives chatty prose around the tag', () => {
    const raw = `Sure, here's the tool call:\n<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>\nDone!`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
    if (out.kind !== 'tool_call') throw new Error();
    expect(out.name).toBe('add_divider_v0');
  });

  it('returns garbage when payload is not valid JSON', () => {
    const raw = `<op_tool>{not json}</op_tool>`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
    if (out.kind !== 'garbage') throw new Error();
    expect(out.reason).toMatch(/payload was not valid JSON/);
  });

  it('returns garbage when payload is missing "name"', () => {
    const raw = `<op_tool>{"arguments": {}}</op_tool>`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
    if (out.kind !== 'garbage') throw new Error();
    expect(out.reason).toMatch(/missing string "name"/);
  });

  it('returns garbage when "arguments" is not an object', () => {
    const raw = `<op_tool>{"name":"add_x_v0","arguments":"oops"}</op_tool>`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
    if (out.kind !== 'garbage') throw new Error();
    expect(out.reason).toMatch(/missing object "arguments"/);
  });
});

describe('parseModelOutput — batch_design path', () => {
  it('detects bare DSL without code fence', () => {
    const raw = `root=I(null, {"type": "frame", "children": []})
nav=I(root, {"type": "frame", "role": "navbar"})`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });

  it('strips markdown code fence around DSL', () => {
    const raw = '```\nroot=I(null, {"type": "frame"})\n```';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toBe('root=I(null, {"type": "frame"})');
  });

  it('detects U(...) / D(...) / C(...) / R(...) / M(...) forms', () => {
    for (const dsl of [
      'U(root, {"fill": []})',
      'D(some-id)',
      'copy=C(src, dst, {})',
      'repl=R(target, {"type":"frame"})',
      'M(nodeId, parent)',
    ]) {
      const out = parseModelOutput(dsl);
      expect(out.kind, `expected batch_design for: ${dsl}`).toBe('batch_design');
    }
  });

  it('detects bindless I/C/R/G forms (binding is optional per batch-design.ts)', () => {
    // Codex stop-hook 2026-04-20: earlier isCleanDsl regex required
    // `word=` prefix for I/C/R/G and rejected bindless forms that
    // handleBatchDesign's `bindlessAssignMatch` accepts.
    for (const dsl of [
      'I(null, {"type":"frame","name":"Root"})',
      'C("source-id", "parent-id", {})',
      'R("target-id", {"type":"frame"})',
      'G("pattern", {})',
    ]) {
      const out = parseModelOutput(dsl);
      expect(out.kind, `expected batch_design for bindless: ${dsl}`).toBe('batch_design');
    }
  });

  it('multi-line payload mixing bound and bindless forms stays batch_design', () => {
    const raw =
      'I(null, {"type":"frame","name":"Root"})\n' +
      'nav=I("parent-id", {"type":"frame","role":"navbar"})\n' +
      'U("nav-id", {"gap": 16})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
  });

  it('returns garbage on prose-only output', () => {
    const raw = "I'm happy to help! What kind of design would you like?";
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('returns garbage on empty output', () => {
    expect(parseModelOutput('').kind).toBe('garbage');
    expect(parseModelOutput('   ').kind).toBe('garbage');
  });
});

describe('parseModelOutput — tool_call takes precedence over DSL when both present', () => {
  it('prefers <op_tool> even if DSL-looking text follows it', () => {
    const raw = `<op_tool>{"name":"add_link_v0","arguments":{"label":"x"}}</op_tool>\nroot=I(null, {})`;
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
  });
});

describe('parseModelOutput — multi-tag preference (real-world weak-model output)', () => {
  it('prefers element-tool name over earlier batch_design scaffold tag', () => {
    // Exact shape observed from MiniMax M2.7 treatment run — model
    // emitted batch_design scaffold + add_nav_chip_row_v0 + another
    // batch_design. Parser must see through the scaffolding.
    const raw =
      '<op_tool>{"name": "batch_design", "arguments": {"operations": "root=I(null, {})"}}</op_tool>\n' +
      '<op_tool>{"name": "add_nav_chip_row_v0", "arguments": {"items": [{"label":"All"}]}}</op_tool>\n' +
      '<op_tool>{"name": "batch_design", "arguments": {"operations": "U(x,{})"}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
    if (out.kind !== 'tool_call') throw new Error();
    expect(out.name).toBe('add_nav_chip_row_v0');
  });

  it('picks first element-tool tag when multiple element tools appear', () => {
    const raw =
      '<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>\n' +
      '<op_tool>{"name":"add_link_v0","arguments":{"label":"x"}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
    if (out.kind !== 'tool_call') throw new Error();
    expect(out.name).toBe('add_divider_v0');
  });

  it('falls back to batch_design when only batch_design <op_tool> tags present', () => {
    const raw =
      '<op_tool>{"name":"batch_design","arguments":{"operations":"root=I(null, {\\"type\\":\\"frame\\"})"}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });

  it('batch_design <op_tool> with array operations serializes to DSL lines', () => {
    const raw =
      '<op_tool>{"name":"batch_design","arguments":{"operations":[{"type":"I","id":"r","node":{"type":"frame","name":"R"}}]}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('r=I(null, ');
    expect(out.dsl).toContain('"name":"R"');
  });

  it('batch_design <op_tool> with no usable operations → garbage', () => {
    const raw = '<op_tool>{"name":"batch_design","arguments":{"operations":null}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
    if (out.kind !== 'garbage') throw new Error();
    expect(out.reason).toMatch(/no usable.*operations/);
  });

  it('unknown tool name (not element, not batch_design) → tool_call with that name for scorer to mark wrong-tool', () => {
    const raw = '<op_tool>{"name":"made_up_tool","arguments":{}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
    if (out.kind !== 'tool_call') throw new Error();
    expect(out.name).toBe('made_up_tool');
  });
});

describe('parseModelOutput — malformed tag must not eclipse valid DSL', () => {
  it('DSL after a malformed <op_tool> block still classified as batch_design', () => {
    // Codex stop-hook 2026-04-20 regression: earlier parser returned
    // garbage-with-parse-reason whenever any <op_tool> tag failed,
    // even if valid batch_design DSL existed elsewhere in the output.
    const raw =
      '<op_tool>not valid json here</op_tool>\n' +
      'root=I(null, {"type":"frame","name":"Root"})\n' +
      'nav=I(root, {"type":"frame","role":"navbar"})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
    expect(out.dsl).toContain('nav=I(root');
  });

  it('DSL wrapped inside a malformed <op_tool> body still recovered', () => {
    // Model emits DSL directly inside <op_tool> without the JSON
    // wrapper. Earlier parser returned garbage; now we peek inside
    // and reclassify as batch_design.
    const raw = '<op_tool>root=I(null, {"type":"frame"})</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });

  it('valid <op_tool> with unusable batch_design operations falls through to DSL detection in body', () => {
    // <op_tool>{name:batch_design, operations:null} is unusable BUT
    // the response body also contains valid raw DSL. Prefer the DSL.
    const raw =
      '<op_tool>{"name":"batch_design","arguments":{"operations":null}}</op_tool>\n' +
      'root=I(null, {"type":"frame"})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });

  it('malformed tag + no DSL anywhere → garbage with specific parse reason (existing behavior preserved)', () => {
    const raw = '<op_tool>not valid json</op_tool>\nnothing else useful here.';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
    if (out.kind !== 'garbage') throw new Error();
    expect(out.reason).toMatch(/not valid JSON/);
  });

  it('malformed-tag body with prose + one DSL line buried → NOT recovered as batch_design', () => {
    // Codex stop-hook 2026-04-20 regression fix: earlier recovery
    // fired whenever ANY line in the body looked like DSL, so a
    // mostly-prose body with a single DSL-ish line would be
    // returned as a "batch_design" payload that handleBatchDesign
    // would then reject line-by-line. Now recovery requires the
    // body to START with DSL.
    const raw =
      '<op_tool>\nLet me think about this request carefully.\nroot=I(null, {"type":"frame"})\nMore prose that trails off.\n</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('cleaned text with prose before DSL line → NOT classified as batch_design', () => {
    // Same principle at the outer (non-tag) fallback path.
    const raw = 'Here\'s my design:\nroot=I(null, {"type":"frame"})\nLet me know!';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('fenced-block content that starts with prose (not DSL) → NOT classified as batch_design', () => {
    const raw = '```\nLet me think:\nroot=I(null, {})\n```';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('pure DSL at the very start of body still recovered', () => {
    // Sanity — must not regress the happy path that motivated the
    // recovery in the first place.
    const raw = '<op_tool>root=I(null, {"type":"frame"})\nnav=I(root, {"type":"frame"})</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('root=I(null');
  });
});

describe('parseModelOutput — mixed DSL + prose rejected as garbage', () => {
  it('DSL line + prose line + DSL line → garbage (codex stop-hook fix)', () => {
    // Earlier parser accepted this as batch_design because it started
    // with DSL; handleBatchDesign would then split on top-level
    // newlines and emit a parse error for the prose chunk. That
    // misclassifies "malformed output" as "model chose batch_design".
    const raw =
      'root=I(null, {"type":"frame"})\n' +
      "Let me explain what I'm doing here...\n" +
      'nav=I(root, {"type":"frame"})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('DSL line + trailing prose → garbage', () => {
    const raw = 'root=I(null, {"type":"frame"})\nHope this helps!';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('DSL across multiple physical lines (pretty-printed JSON inside op) → batch_design', () => {
    // Happy-path sanity — the splitter's brace/paren awareness means
    // pretty-printed JSON inside I(...) stays ONE op, not N mixed
    // chunks. Must not regress or pretty-printed model output fails.
    const raw =
      'root=I(null, {\n' +
      '  "type": "frame",\n' +
      '  "name": "Root",\n' +
      '  "children": []\n' +
      '})\n' +
      'nav=I(root, {"type":"frame","role":"navbar"})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
    if (out.kind !== 'batch_design') throw new Error();
    expect(out.dsl).toContain('"name": "Root"');
  });

  it('mixed payload inside malformed tag body → garbage', () => {
    const raw =
      '<op_tool>root=I(null, {"type":"frame"})\n' +
      'Some prose in between.\n' +
      'nav=I(root, {"type":"frame"})</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });

  it('DSL string literal containing ( or { does not confuse splitter', () => {
    const raw =
      'root=I(null, {"type":"text","content":"price: $(100)","name":"{bold}"})\nnav=I(root, {})';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('batch_design');
  });
});

describe('parseModelOutput — <think> stripping (reasoning-model robustness)', () => {
  it('strips <think>...</think> prefix before matching', () => {
    const raw =
      '<think>\nThe user wants X. Let me use Y.\n</think>\n\n' +
      '<op_tool>{"name":"add_link_v0","arguments":{"label":"ok"}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
  });

  it('strips multiple <think> blocks (MiniMax sometimes re-thinks)', () => {
    const raw =
      '<think>first</think>\n<think>second</think>\n' +
      '<op_tool>{"name":"add_divider_v0","arguments":{}}</op_tool>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('tool_call');
  });

  it('output that is entirely <think> → garbage (after stripping nothing remains)', () => {
    const raw = '<think>just thinking, never answered</think>';
    const out = parseModelOutput(raw);
    expect(out.kind).toBe('garbage');
  });
});
