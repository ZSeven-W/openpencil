import type { ParsedOutput } from './types';

/**
 * The in-prompt tool-call convention used by the treatment variant:
 *
 *   <op_tool>{"name": "add_X_v0", "arguments": {...}}</op_tool>
 *
 * Matches the A/B plan §2 "Option Y — in-prompt simulation". The tag
 * `op_tool` (not `tool_call`) avoids confusion with Claude's native
 * tool_use blocks so reference Claude runs stay parseable.
 *
 * Convention intentionally permits one call per output — the plan's
 * "element tool OR batch_design, not both" rule. Multi-call support
 * can come later if the corpus grows composite prompts.
 */
const OP_TOOL_RE_G = /<op_tool>\s*([\s\S]*?)\s*<\/op_tool>/g;

const ELEMENT_TOOL_NAME_RE = /^add_[a-z_]+_v0$/;

/**
 * Reasoning-model chain-of-thought wrapper. MiniMax M-series,
 * DeepSeek-R1, and some others emit `<think>...</think>` before the
 * actual answer. Strip it defensively so the parser sees the real
 * output; no legitimate design payload should contain these tags.
 * Greedy to the LAST `</think>` so nested / multiple blocks collapse
 * cleanly.
 */
const THINK_RE = /<think>[\s\S]*?<\/think>\s*/gi;

/**
 * Heuristic marker for a batch_design DSL payload.
 *
 * `DSL_AT_START` requires the string's FIRST non-whitespace content
 * to be a DSL operation — any of the three executeLine forms
 * (binding=I/C/R/M/G, bindless I/C/R/G, or standalone U/D/M). Used
 * as the cheap first-pass check before the stricter `isCleanDsl`
 * full scan.
 */
const DSL_AT_START = /^\s*(?:\w+\s*=\s*[ICRMG]\s*\(|[ICRGUDM]\s*\()/;

/**
 * Shape of a single DSL operation line after top-level splitting.
 * Mirrors the THREE regexes in
 * `packages/pen-mcp/src/tools/batch-design.ts::executeLine`:
 *
 *   assignMatch:         /^(\w+)\s*=\s*([ICRMG])\((.+)\)$/
 *   bindlessAssignMatch: /^([ICRG])\((.+)\)$/       // I/C/R/G only
 *   callMatch:           /^([UDM])\((.+)\)$/
 *
 * Combined: the line must start with either `word=[ICRMG]` or a bare
 * opcode letter `[ICRGUDM]` and end with `)`. Keeping this in sync
 * with executeLine's regexes ensures a payload `isCleanDsl` accepts
 * is also parsed cleanly by `handleBatchDesign` — the whole point
 * of the strict check is to avoid letting mixed content through that
 * later fails line-by-line in the downstream parser.
 */
const DSL_OP_LINE = /^(?:\w+\s*=\s*[ICRMG]|[ICRGUDM])\([\s\S]+\)$/;

/**
 * Verify a payload is CLEAN batch_design DSL — no prose preambles,
 * no interleaved explanation lines, no trailing commentary. Uses the
 * same string-aware brace/paren/bracket splitter as `handleBatchDesign`
 * so "one logical op" stays one chunk even when pretty-printed JSON
 * spans multiple physical lines.
 *
 * Why a full scan rather than just `DSL_AT_START`: a model may emit
 * `root=I(null, {...})\nLet me know if that works!\nnav=I(root, {...})`
 * which passes `DSL_AT_START` but the middle "Let me know..." line
 * splits into its own chunk that isn't a valid DSL op. Feeding the
 * whole thing to handleBatchDesign produces a noisy per-line error
 * array instead of a clean apply result — and the scorer ends up
 * classifying the run as "fallback" (model chose batch_design) when
 * it should really be "garbage" (model couldn't emit valid DSL).
 */
export function isCleanDsl(payload: string): boolean {
  if (!DSL_AT_START.test(payload)) return false;
  const chunks = splitDslTopLevel(payload);
  if (chunks.length === 0) return false;
  for (const chunk of chunks) {
    if (!DSL_OP_LINE.test(chunk)) return false;
  }
  return true;
}

/**
 * Mirror of `splitOperations` in pen-mcp's batch-design.ts — splits
 * on newlines that are OUTSIDE quoted strings and balanced brackets.
 * Kept as a local copy so the corpus package doesn't depend on
 * pen-mcp internals (that would create the same cycle the scorer
 * itself was structured to avoid).
 */
function splitDslTopLevel(raw: string): string[] {
  const result: string[] = [];
  let buf = '';
  let depth = 0;
  let inString = false;
  let escape = false;
  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i];
    buf += ch;
    if (escape) {
      escape = false;
      continue;
    }
    if (ch === '\\' && inString) {
      escape = true;
      continue;
    }
    if (ch === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;
    if (ch === '(' || ch === '[' || ch === '{') depth += 1;
    else if (ch === ')' || ch === ']' || ch === '}') depth -= 1;
    else if (ch === '\n' && depth === 0) {
      const trimmed = buf.trim();
      if (trimmed && !trimmed.startsWith('//')) result.push(trimmed);
      buf = '';
    }
  }
  const tail = buf.trim();
  if (tail && !tail.startsWith('//')) result.push(tail);
  return result;
}

/**
 * Optional prefix / suffix the model might wrap around its DSL payload:
 *   ```
 *   ... or explanatory prose ...
 *   ```
 * strip the markdown fence and surrounding prose when extracting the
 * DSL so downstream `handleBatchDesign` gets a clean payload.
 */
const CODE_FENCE_RE = /```(?:\w+)?\s*([\s\S]*?)```/;

/**
 * Parse a raw model output into a tagged union. Order of checks:
 *
 *   1. `<op_tool>{...}</op_tool>` — treatment-arm tool call
 *   2. Code-fenced block whose inner text matches the batch_design
 *      pattern → batch_design DSL
 *   3. Free-form text that still matches the batch_design pattern →
 *      batch_design DSL (fence optional)
 *   4. Otherwise garbage
 *
 * Never throws — garbage responses get a `kind: 'garbage'` tag with a
 * human-readable `reason` so aggregation can distinguish parse failure
 * from apply failure.
 */
export function parseModelOutput(raw: string): ParsedOutput {
  if (typeof raw !== 'string' || raw.trim().length === 0) {
    return { kind: 'garbage', reason: 'empty output', raw: raw ?? '' };
  }

  // Strip reasoning-model <think> blocks before pattern-matching.
  // `raw` field in the return value keeps the full original for
  // debugging; parse logic sees the cleaned text.
  const cleaned = raw.replace(THINK_RE, '').trim();

  // Extract every `<op_tool>` tag (models frequently emit multiple when
  // composing). We prefer the FIRST element-tool call over any
  // `batch_design` scaffold tags, because intent matching is what M5
  // measures — scaffolding doesn't reflect routing quality.
  //
  // When a tag body fails to parse as JSON we ALSO inspect whether
  // the body happens to contain raw DSL (seen in the wild: some
  // models wrap DSL directly inside `<op_tool>` tags without the
  // JSON wrapper). Such a tag should be recoverable as batch_design
  // rather than wasted as garbage.
  const tags: ParsedOpTool[] = [];
  let firstParseFailureReason: string | null = null;
  let dslFromMalformedTag: string | null = null;
  for (const match of cleaned.matchAll(OP_TOOL_RE_G)) {
    const body = match[1].trim();
    const parsed = parseOpTool(body);
    if (parsed.ok) {
      tags.push(parsed.value);
      continue;
    }
    if (firstParseFailureReason === null) firstParseFailureReason = parsed.reason;
    // Only recover a malformed-tag body as DSL when the body is
    // CLEAN DSL end-to-end — otherwise we'd be feeding prose to
    // handleBatchDesign, which just produces a pile of per-line
    // parse errors and surfaces a useless "invalid DSL" result.
    if (dslFromMalformedTag === null && isCleanDsl(body)) {
      dslFromMalformedTag = body;
    }
  }

  // Preserved across tag-processing so we can emit a specific garbage
  // reason after DSL fallback also fails. `null` means we never
  // encountered a batch_design tag with bad operations.
  let batchDesignUnusableReason: string | null = null;
  if (tags.length > 0) {
    const elementCall = tags.find((t) => ELEMENT_TOOL_NAME_RE.test(t.name));
    if (elementCall) {
      return {
        kind: 'tool_call',
        name: elementCall.name,
        arguments: elementCall.arguments,
        raw,
      };
    }
    const batchCall = tags.find((t) => t.name === 'batch_design');
    if (batchCall) {
      const dsl = extractDslFromBatchDesignArgs(batchCall.arguments);
      if (dsl) return { kind: 'batch_design', dsl, raw };
      batchDesignUnusableReason = '<op_tool>{name:"batch_design"} had no usable `operations` field';
      // Fall through to DSL-outside-tag detection rather than silently
      // drop the tag.
    } else {
      // Tag present but with a name that's neither an element tool
      // nor batch_design — unknown tool call. Keep the first as-is
      // so the scorer records it as a tool_call (routing='wrong-tool'
      // for obvious prompts, since it's not the expected tool).
      return {
        kind: 'tool_call',
        name: tags[0].name,
        arguments: tags[0].arguments,
        raw,
      };
    }
  }

  // DSL detection — runs AFTER tag parsing so valid DSL in the
  // response body (or wrapped in a malformed tag) isn't lost to an
  // early "parse error" short-circuit. Every path uses `isCleanDsl`
  // so we only return payloads that are CLEAN DSL end-to-end — a
  // mixed payload with prose interleaved between ops would pass
  // splitOperations as a prose chunk and fail regex match inside
  // handleBatchDesign, producing a noisy error array. Classifying
  // those as garbage (not batch_design) gives the scorer a truer
  // routing signal.
  //
  // Order of precedence:
  //   1. Fenced block whose content is clean DSL
  //   2. Text OUTSIDE all `<op_tool>` tags is clean DSL (common case
  //      of a tag followed by raw DSL on new lines)
  //   3. Clean DSL recovered from inside a malformed `<op_tool>` body
  const fenceMatch = cleaned.match(CODE_FENCE_RE);
  if (fenceMatch) {
    const fenceBody = fenceMatch[1].trim();
    if (isCleanDsl(fenceBody)) {
      return { kind: 'batch_design', dsl: fenceBody, raw };
    }
  }
  const outsideTags = cleaned.replace(OP_TOOL_RE_G, '').trim();
  if (outsideTags.length > 0 && isCleanDsl(outsideTags)) {
    return { kind: 'batch_design', dsl: outsideTags, raw };
  }
  if (dslFromMalformedTag) {
    return { kind: 'batch_design', dsl: dslFromMalformedTag, raw };
  }

  // Nothing usable remains. Prefer specific reasons over generic
  // ones, in priority order: "batch_design tag had unusable
  // operations" → "<op_tool> tag parse failure" → generic "no DSL".
  if (batchDesignUnusableReason !== null) {
    return { kind: 'garbage', reason: batchDesignUnusableReason, raw };
  }
  if (firstParseFailureReason !== null) {
    return { kind: 'garbage', reason: firstParseFailureReason, raw };
  }
  return {
    kind: 'garbage',
    reason: 'no <op_tool> block and no batch_design DSL lines detected',
    raw,
  };
}

interface ParsedOpTool {
  name: string;
  arguments: Record<string, unknown>;
}

type ParseOpToolResult = { ok: true; value: ParsedOpTool } | { ok: false; reason: string };

function parseOpTool(payload: string): ParseOpToolResult {
  let obj: unknown;
  try {
    obj = JSON.parse(payload);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    return { ok: false, reason: `<op_tool> payload was not valid JSON: ${msg}` };
  }
  if (!obj || typeof obj !== 'object' || Array.isArray(obj)) {
    return { ok: false, reason: '<op_tool> payload must be an object' };
  }
  const o = obj as Record<string, unknown>;
  const name = o.name;
  const args = o.arguments;
  if (typeof name !== 'string' || name.length === 0) {
    return { ok: false, reason: '<op_tool> missing string "name"' };
  }
  if (!args || typeof args !== 'object' || Array.isArray(args)) {
    return { ok: false, reason: '<op_tool> missing object "arguments"' };
  }
  return { ok: true, value: { name, arguments: args as Record<string, unknown> } };
}

/**
 * Models sometimes wrap `batch_design` in an `<op_tool>` tag (e.g. when
 * the baseline prompt teaches the tool-call format uniformly). Unpack
 * the `operations` argument into a DSL string. Accepts either:
 *   - `operations: "DSL text"` — passed through
 *   - `operations: [{type, ...}, ...]` — serialized back to DSL lines
 *     (best effort; unknown op shapes are dropped).
 * Returns null if no usable form is found.
 */
function extractDslFromBatchDesignArgs(args: Record<string, unknown>): string | null {
  const ops = args.operations;
  if (typeof ops === 'string' && ops.trim().length > 0) return ops;
  if (Array.isArray(ops) && ops.length > 0) {
    const lines: string[] = [];
    for (const op of ops) {
      if (!op || typeof op !== 'object') continue;
      const line = serializeOpToDsl(op as Record<string, unknown>);
      if (line) lines.push(line);
    }
    if (lines.length > 0) return lines.join('\n');
  }
  return null;
}

function serializeOpToDsl(op: Record<string, unknown>): string | null {
  // Minimal shim — covers the shape most models invent when they treat
  // batch_design's operations as a structured array. Not a full DSL
  // reimplementation. If a model emits some other shape, apply will
  // throw and the run scores as M1=false (correct behavior — the
  // model produced unusable output).
  const type = op.type;
  if (typeof type !== 'string') return null;
  const id = op.id;
  const node = op.node;
  const parent = op.parent ?? null;
  if (type === 'I' && node && typeof node === 'object') {
    const parentRef = parent == null ? 'null' : JSON.stringify(parent);
    const binding = typeof id === 'string' && id.length > 0 ? id : 'n';
    return `${binding}=I(${parentRef}, ${JSON.stringify(node)})`;
  }
  if (type === 'U' && typeof id === 'string' && op.props && typeof op.props === 'object') {
    return `U(${JSON.stringify(id)}, ${JSON.stringify(op.props)})`;
  }
  if (type === 'D' && typeof id === 'string') {
    return `D(${JSON.stringify(id)})`;
  }
  return null;
}
