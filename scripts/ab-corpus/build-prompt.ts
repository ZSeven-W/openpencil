/**
 * Build the system prompt for each A/B variant.
 *
 * Baseline (B): current OpenPencil design prompt MINUS both element-
 *   tool skills (decision tree + cookbook). The model sees schema /
 *   layout / text rules + examples and can only emit `batch_design`
 *   DSL.
 *
 * Treatment (T): same baseline PLUS both element-tool skills PLUS the
 *   `<op_tool>` output-format instruction. The model can either emit
 *   an element-tool call (preferred, schema-narrow) or fall back to
 *   `batch_design` DSL.
 *
 * Implementation: `buildDesignPrompt()` includes elements + elements-
 * cookbook by default. We derive B by string-subtracting each skill
 * body; T is base + trailing instructions. The cookbook lives in a
 * separate file so neither piece exceeds the 800-line per-file ceiling
 * — text-only LLMs (no MCP `tools/list`) need both halves, the
 * decision tree to pick the tool and the cookbook to know its arg
 * shape. String subtraction is brittle if elements.md / elements-
 * cookbook.md move, but it's O(minutes) to fix if it does — much less
 * churn than refactoring the prompt builder just for this eval.
 */

import { getSkillByName } from '@zseven-w/pen-ai-skills';
import { buildDesignPrompt } from '@zseven-w/pen-mcp';

// Both variants teach the SAME `<op_tool>` wrapper format so the only
// thing that differs between B and T is the tool set — this isolates
// "does having element tools help?" from "does teaching a tool-call
// format help?". Without this, weak models in B would invent their own
// format (observed with MiniMax M2.7 emitting `[TOOL_CALL]` pseudo-JS)
// and the variant comparison becomes noise.
const B_TOOL_CALL_INSTRUCTIONS = [
  '',
  'OUTPUT FORMAT — EMIT AS TOOL CALL:',
  '',
  'Respond with exactly one line in this form, nothing else:',
  '  <op_tool>{"name": "batch_design", "arguments": {"operations": "<DSL_STRING>"}}</op_tool>',
  'The `operations` value is a single string containing the batch_design DSL (one operation per line — use \\n to separate). Do not add prose before or after the tag.',
  '',
].join('\n');

const T_TOOL_CALL_INSTRUCTIONS = [
  '',
  'OUTPUT FORMAT — EMIT AS TOOL CALL:',
  '',
  'Respond with one `<op_tool>` tag, nothing else. Choose based on intent:',
  '',
  'PRIMARY: when your intent matches an add_*_v0 element tool above, emit:',
  '  <op_tool>{"name": "add_X_v0", "arguments": {...}}</op_tool>',
  '',
  'FALLBACK: when no element tool fits (heterogeneous layout, composite section, post-hoc styling), emit:',
  '  <op_tool>{"name": "batch_design", "arguments": {"operations": "<DSL_STRING>"}}</op_tool>',
  'The `operations` value is a single string containing the batch_design DSL.',
  '',
  'Do not combine multiple tags. Do not add prose before or after.',
  '',
].join('\n');

export interface BuiltPrompt {
  system: string;
  variant: 'B' | 'T';
}

export function buildSystemPrompt(variant: 'B' | 'T'): BuiltPrompt {
  // buildDesignPrompt() already concatenates every section the harness
  // needs — schema, style, examples, DESIGN_TYPE_DETECTION, roles,
  // layout, text rules, guidelines, variables, auto-replace,
  // post-processing, AND elements (appended last session).
  const full = buildDesignPrompt();
  if (variant === 'T') {
    return { system: full + '\n\n' + T_TOOL_CALL_INSTRUCTIONS, variant };
  }
  // Baseline: strip BOTH elements skills (decision-tree + cookbook).
  // The cookbook split landed when elements.md crossed the 800-line
  // ceiling — both halves carry element-tool guidance that B must not
  // see, otherwise the baseline leaks tool names + arg shapes and the
  // A/B delta no longer measures "tools vs no-tools". Exact-match
  // removal of each skill body keeps the rest of the prompt byte-
  // identical so any delta isn't explained by accidental shape drift.
  const elementsSkill = getSkillByName('elements');
  const cookbookSkill = getSkillByName('elements-cookbook');
  if (!elementsSkill) {
    throw new Error(
      'elements skill not found in registry — cannot build baseline prompt without it (would leak elements content into B if left in place)',
    );
  }
  if (!cookbookSkill) {
    throw new Error(
      'elements-cookbook skill not found in registry — cannot build baseline prompt without stripping it (would leak the arg-shape examples into B)',
    );
  }
  let stripped = full.replace(elementsSkill.content, '');
  if (stripped === full) {
    throw new Error(
      'elements content was not present in the full prompt — buildDesignPrompt() may have been refactored; update build-prompt.ts to match',
    );
  }
  const beforeCookbook = stripped;
  stripped = stripped.replace(cookbookSkill.content, '');
  if (stripped === beforeCookbook) {
    throw new Error(
      'elements-cookbook content was not present in the full prompt — buildDesignPrompt() may have been refactored; update build-prompt.ts to match',
    );
  }
  return { system: stripped.trim() + '\n\n' + B_TOOL_CALL_INSTRUCTIONS, variant };
}
