import type { PenNode } from '@/types/pen';
import { parseModelOutput } from '@zseven-w/pen-ai-skills';

// ---------------------------------------------------------------------------
// Streaming JSONL parser result
// ---------------------------------------------------------------------------

export interface StreamingNodeResult {
  node: PenNode;
  parentId: string | null;
}

// ---------------------------------------------------------------------------
// Element-tool output shape detection
// ---------------------------------------------------------------------------
//
// When `needsElementTools(modelProfile)` is true the sub-agent prompt
// teaches the model to wrap its response in `<op_tool>{...}</op_tool>`
// (see orchestrator-sub-agent.ts::ELEMENT_TOOL_OUTPUT_FORMAT). This
// helper detects that wrapper BEFORE the caller hands the raw output
// to the legacy `extractJsonFromResponse`, so the dispatch layer can
// route element-tool calls through pen-mcp's handler family instead
// of the legacy JSONL path.
//
// Returns `null` when the output isn't `<op_tool>`-wrapped — caller
// should then try `extractJsonFromResponse` as a fallback. Never
// throws; malformed or partial tags degrade to the legacy path.

export type DesignOutputShape =
  | {
      kind: 'element-tool';
      /** Element-tool name, e.g. `add_card_row_v0`. Matches pen-mcp's ELEMENT_TOOL_NAMES. */
      name: string;
      /** Raw tool arguments from the model. Pen-mcp handler validates shape. */
      arguments: Record<string, unknown>;
      raw: string;
    }
  | {
      kind: 'batch-design-dsl';
      /** DSL string (single operation per line), ready for handleBatchDesign. */
      dsl: string;
      raw: string;
    };

/**
 * Try to detect an `<op_tool>`-wrapped output. Returns a tagged-union
 * route hint when detected; null otherwise.
 *
 * Precedence mirrors the corpus A/B parser:
 *   1. If ANY `<op_tool>` tag names an `add_*_v0` element tool, that
 *      wins (multi-tag outputs where batch_design is used as
 *      scaffolding still route through the element tool — intent
 *      matching is the metric of interest)
 *   2. Otherwise if an `<op_tool>` names `batch_design`, return the
 *      extracted `operations` as DSL
 *   3. Otherwise the output uses the legacy format — return null so
 *      the caller falls through to `extractJsonFromResponse`
 */
export function tryParseElementToolOutput(raw: string): DesignOutputShape | null {
  const parsed = parseModelOutput(raw);
  if (parsed.kind === 'tool_call') {
    return {
      kind: 'element-tool',
      name: parsed.name,
      arguments: parsed.arguments,
      raw: parsed.raw,
    };
  }
  if (parsed.kind === 'batch_design') {
    return { kind: 'batch-design-dsl', dsl: parsed.dsl, raw: parsed.raw };
  }
  return null;
}

/**
 * Parse ALL `<op_tool>` tags in a response into a list of
 * DesignOutputShape. Current prompt in ELEMENT_TOOL_OUTPUT_FORMAT
 * instructs the model to emit exactly one tag, so at runtime this
 * usually returns a 0- or 1-length array. But the plumbing is here
 * for future prompts that allow multi-tool composition (e.g. "emit
 * one add_* per section" — each wrapped separately).
 *
 * Order preserved: tags appear in the array in emit order. Element
 * tool names come through as `kind: 'element-tool'`; `batch_design`
 * tags come through as `kind: 'batch-design-dsl'`. Unknown tool
 * names are dropped (same as single-tag path's "wrong-tool"
 * classification — we only surface shapes the dispatcher can
 * route).
 *
 * Does NOT cross-check against the covered shim registry — that's
 * the dispatcher's job (short-circuit with canonical list on miss).
 */
export function tryParseAllElementToolOutputs(raw: string): DesignOutputShape[] {
  if (typeof raw !== 'string' || raw.trim().length === 0) return [];
  const shapes: DesignOutputShape[] = [];
  const tagRe = /<op_tool>\s*([\s\S]*?)\s*<\/op_tool>/g;
  for (const match of raw.matchAll(tagRe)) {
    const body = match[1].trim();
    try {
      const parsed = JSON.parse(body) as { name?: unknown; arguments?: unknown };
      if (typeof parsed.name !== 'string') continue;
      const args =
        parsed.arguments && typeof parsed.arguments === 'object'
          ? (parsed.arguments as Record<string, unknown>)
          : {};
      if (/^add_[a-z_]+_v\d+$/.test(parsed.name)) {
        shapes.push({ kind: 'element-tool', name: parsed.name, arguments: args, raw: match[0] });
      } else if (parsed.name === 'batch_design') {
        const dsl = typeof args.operations === 'string' ? args.operations : '';
        if (dsl.length > 0) {
          shapes.push({ kind: 'batch-design-dsl', dsl, raw: match[0] });
        }
      }
      // Unknown name → silently skipped; single-tag path routes as
      // "wrong-tool" but the multi-tag path keeps only dispatchable
      // shapes so the batch loop doesn't need special-casing.
    } catch {
      // Malformed tag body → skip. The single-tag path has DSL-
      // recovery for this; add if needed here, but multi-tag flows
      // are expected to be well-formed (whole prompt effort).
    }
  }
  return shapes;
}

// ---------------------------------------------------------------------------
// JSON extraction from AI response text
// ---------------------------------------------------------------------------

/**
 * Extract PenNode JSON from AI response text.
 * Handles ```json blocks, JSONL format, raw arrays, and fallback parsing.
 */
/**
 * Strip non-standard XML-like tags that third-party models may inject
 * (e.g. `<minimax:tool_call>`, `<tool_call>`, `<|im_start|>`).
 * Preserves content between tags so any embedded JSON can still be parsed.
 */
function stripNonStandardTags(text: string): string {
  return text
    .replace(/<\/?[\w:.]+:[\w]+[^>]*>/g, '') // namespaced tags: <minimax:tool_call>
    .replace(/<\/?tool_call[^>]*>/g, '') // <tool_call>, </tool_call>
    .replace(/<\|[\w_]+\|>/g, '') // chat template markers: <|im_start|>
    .replace(/\[TOOL_CALL\]/gi, ''); // bracket-style tool call markers
}

/**
 * Strip fake tool call blocks that basic-tier models (MiniMax, etc.) may emit.
 * These look like `{tool => "Write", args => { ... }}` and are not valid JSON.
 * Must run BEFORE JSON extraction so brace-scanning doesn't pick them up.
 */
function stripToolCallBlocks(text: string): string {
  // Remove `{tool => "...", args => { ... }}` blocks (arrow-syntax pseudo-calls)
  // These use `=>` instead of `:` for key-value pairs
  return text.replace(/\{tool\s*=>\s*"[^"]*"\s*,\s*args\s*=>[\s\S]*$/gi, '');
}

export function extractJsonFromResponse(text: string): PenNode[] | null {
  // Clean non-standard model artifacts before parsing
  const cleaned = stripToolCallBlocks(stripNonStandardTags(text));

  const parsedBlocks = extractAllJsonBlocks(cleaned)
    .map((block) => tryParseNodes(block))
    .filter(Boolean) as PenNode[][];

  if (parsedBlocks.length > 0) {
    return selectBestNodeSet(parsedBlocks);
  }

  // Try JSONL format (flat nodes with _parent field)
  const jsonlTree = parseJsonlToTree(cleaned);
  if (jsonlTree) return jsonlTree;

  // Fallback: try to find a single JSON array if no blocks found
  const arrayMatch = cleaned.match(/\[\s*\{[\s\S]*\}\s*\]/);
  if (arrayMatch) {
    const nodes = tryParseNodes(arrayMatch[0]);
    return nodes;
  }

  // Fallback: try parsing a single root node with nested children
  // (weaker models may output one root object instead of an array)
  const singleRoot = tryParseSingleRootNode(cleaned);
  if (singleRoot) return singleRoot;

  // Fallback: try parsing raw text after removing <step> tags.
  const stripped = cleaned.replace(/<step[\s\S]*?<\/step>/g, '').trim();
  const directNodes = tryParseNodes(stripped);
  if (directNodes) {
    return directNodes;
  }

  return null;
}

// ---------------------------------------------------------------------------
// Streaming JSONL parser — extracts completed JSON objects from within
// a ```json block as they stream in, enabling element-by-element rendering.
// ---------------------------------------------------------------------------

/**
 * Extract completed JSON objects from streaming text (within a ```json block).
 * Uses brace-counting to detect complete objects before the block closes.
 * Each object is expected to have a `_parent` field for tree insertion.
 */
export function extractStreamingNodes(
  text: string,
  processedOffset: number,
): { results: StreamingNodeResult[]; newOffset: number } {
  // Primary mode: parse inside a ```json fenced block.
  // Fallback mode: parse raw JSONL/object text when the model omits fences.
  const jsonBlockStart = text.indexOf('```json');

  let contentStart = -1;
  let searchEnd = text.length;
  if (jsonBlockStart !== -1) {
    const firstNewline = text.indexOf('\n', jsonBlockStart);
    if (firstNewline === -1) return { results: [], newOffset: processedOffset };
    contentStart = firstNewline + 1;
    const blockEnd = text.indexOf('\n```', contentStart);
    searchEnd = blockEnd > 0 ? blockEnd : text.length;
  } else {
    const firstBrace = text.indexOf('{');
    if (firstBrace === -1) return { results: [], newOffset: processedOffset };
    contentStart = firstBrace;
  }

  const startPos = Math.max(processedOffset, contentStart);

  const results: StreamingNodeResult[] = [];
  let i = startPos;

  while (i < searchEnd) {
    // Skip to next '{' character
    while (i < searchEnd && text[i] !== '{') i++;
    if (i >= searchEnd) break;

    // Brace-counting to find matching '}'
    const objStart = i;
    let depth = 0;
    let inString = false;
    let escaped = false;
    let j = i;

    while (j < searchEnd) {
      const ch = text[j];
      if (escaped) {
        escaped = false;
        j++;
        continue;
      }
      if (ch === '\\' && inString) {
        escaped = true;
        j++;
        continue;
      }
      if (ch === '"') {
        inString = !inString;
        j++;
        continue;
      }
      if (inString) {
        j++;
        continue;
      }
      if (ch === '{') depth++;
      else if (ch === '}') {
        depth--;
        if (depth === 0) {
          // Complete object found
          const objStr = text.slice(objStart, j + 1);
          try {
            const obj = JSON.parse(objStr) as Record<string, unknown>;
            if (obj.id && obj.type) {
              const parentId = (obj._parent as string | null) ?? null;
              delete obj._parent;
              results.push({ node: obj as unknown as PenNode, parentId });
            }
          } catch {
            /* malformed JSON, skip */
          }
          i = j + 1;
          break;
        }
      }
      j++;
    }

    if (depth > 0) break; // Incomplete object, wait for more data
  }

  return { results, newOffset: i };
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Helper to find all complete JSON blocks in text (```json or ``` blocks).
 */
function extractAllJsonBlocks(text: string): string[] {
  const blocks: string[] = [];
  // Matches ```json or ``` blocks
  const regex = /```(?:json)?\s*\n?([\s\S]*?)\n?\s*```/g;
  let match;
  while ((match = regex.exec(text)) !== null) {
    // Basic heuristic: check if it looks like JSON array/object before adding
    const content = match[1].trim();
    if (content.startsWith('[') || content.startsWith('{')) {
      blocks.push(content);
    }
  }
  return blocks;
}

/**
 * Parse JSONL-format response (flat nodes with _parent field) into a tree.
 * Used by extractAndApplyDesign for batch apply of JSONL content.
 */
function parseJsonlToTree(text: string): PenNode[] | null {
  const { results } = extractStreamingNodes(text, 0);
  if (results.length === 0) return null;

  const nodeMap = new Map<string, PenNode>();
  const roots: PenNode[] = [];

  for (const { node, parentId } of results) {
    nodeMap.set(node.id, node);

    if (parentId === null) {
      roots.push(node);
    } else {
      const parent = nodeMap.get(parentId);
      if (parent) {
        if (
          !('children' in parent) ||
          !Array.isArray((parent as PenNode & { children?: PenNode[] }).children)
        ) {
          (parent as PenNode & { children?: PenNode[] }).children = [];
        }
        (parent as PenNode & { children: PenNode[] }).children.push(node);
      } else {
        roots.push(node); // Parent not found, treat as root
      }
    }
  }

  return roots.length > 0 ? roots : null;
}

/**
 * Try to parse a single root PenNode with nested children from raw text.
 * Handles the case where weaker models output a single JSON object
 * instead of an array or JSONL format.
 */
function tryParseSingleRootNode(text: string): PenNode[] | null {
  const first = text.indexOf('{');
  const last = text.lastIndexOf('}');
  if (first < 0 || last <= first) return null;
  try {
    const obj = JSON.parse(text.slice(first, last + 1)) as Record<string, unknown>;
    if (typeof obj.id === 'string' && typeof obj.type === 'string' && Array.isArray(obj.children)) {
      return [obj as unknown as PenNode];
    }
  } catch {
    /* ignore parse errors */
  }
  return null;
}

function tryParseNodes(json: string): PenNode[] | null {
  try {
    const parsed = JSON.parse(json.trim());
    const nodes = Array.isArray(parsed) ? parsed : [parsed];
    return validateNodes(nodes) ? nodes : null;
  } catch {
    return null;
  }
}

function validateNodes(nodes: unknown[]): nodes is PenNode[] {
  return nodes.every(
    (node) =>
      typeof node === 'object' &&
      node !== null &&
      'id' in node &&
      'type' in node &&
      typeof (node as PenNode).id === 'string' &&
      typeof (node as PenNode).type === 'string',
  );
}

function selectBestNodeSet(candidates: PenNode[][]): PenNode[] {
  let best = candidates[candidates.length - 1];
  let bestScore = scoreNodeSet(best);

  for (const candidate of candidates) {
    const score = scoreNodeSet(candidate);
    // Favor later blocks on ties to keep the most recent complete output.
    if (score >= bestScore) {
      best = candidate;
      bestScore = score;
    }
  }

  return best;
}

function scoreNodeSet(nodes: PenNode[]): number {
  let score = nodes.length;

  if (nodes.length === 1 && nodes[0].type === 'frame') {
    score += 1000;
    const root = nodes[0];
    if ((root.x ?? 0) === 0 && (root.y ?? 0) === 0) score += 50;
    if ('children' in root && Array.isArray(root.children)) {
      score += root.children.length * 10;
    }
  }

  if (nodes.length > 1) {
    score -= 200;
  }

  for (const node of nodes) {
    if ('children' in node && Array.isArray(node.children)) {
      score += node.children.length * 2;
    }
  }

  return score;
}
