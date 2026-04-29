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
  'OUTPUT FORMAT — EMIT AS TOOL CALL(S):',
  '',
  'Respond with one or more `<op_tool>` tags, nothing else. The harness reads every `<op_tool>` tag in your output, so chain as many as the brief implies. Pick ONE strategy for the whole response — do NOT mix element tools with batch_design in the same output:',
  '',
  'STRATEGY A — element tools (preferred): when every component in the brief fits an `add_*_v0` element tool, emit one tag per component, in render order (top-to-bottom for vertical layouts, left-to-right for horizontal). Single-component briefs produce exactly one tag; multi-component briefs (settings panel with N rows, team list with N members, audit feed with N entries, onboarding screen with N step cards) produce N+1 or more.',
  '  <op_tool>{"name": "add_section_header_v0", "arguments": {"title": "Notifications"}}</op_tool>',
  '  <op_tool>{"name": "add_setting_row_v0", "arguments": {...}}</op_tool>',
  '  <op_tool>{"name": "add_setting_row_v0", "arguments": {...}}</op_tool>',
  '',
  'STRATEGY B — batch_design fallback: when the brief includes any component shape that NO element tool covers (heterogeneous custom layout, post-hoc styling, bespoke scaffolding), emit a SINGLE batch_design call covering the WHOLE response. Multiple `<op_tool>` tags carrying batch_design or any mix of batch_design + element tools is not supported — the harness drops the batch_design half and only the element calls run.',
  '  <op_tool>{"name": "batch_design", "arguments": {"operations": "<DSL_STRING>"}}</op_tool>',
  'The `operations` value is a single string containing the batch_design DSL covering every component in the brief.',
  '',
  'Do not add prose before, after, or between tags. Do not mix Strategy A and Strategy B in the same output — choose element tools for every component or batch_design for every component.',
  '',
].join('\n');

export interface BuiltPrompt {
  system: string;
  variant: 'B' | 'T';
}

export interface BuildPromptOpts {
  /**
   * Prompt difficulty signal. When variant=T, controls whether the
   * elements-cookbook (~18kb of arg-shape examples) is included.
   *
   * - 'composite', 'optional', or undefined → keep cookbook
   *   (multi-tool briefs need the chained recipes; safe default for
   *   free-form callers).
   * - 'obvious' → strip cookbook. Decision tree + PREFER list still
   *   teach tool selection; the per-tool minimal-usage examples are
   *   the cost we trade for ~80% smaller T prompts on single-tool
   *   prompts. Validated by ab-v4 sweep (Phase 2 of token-diet plan).
   *
   * Variant=B is unaffected — baseline always strips both skills so
   * the A/B comparison still isolates "tools vs no-tools".
   */
  difficulty?: 'obvious' | 'optional' | 'composite';
}

export function buildSystemPrompt(variant: 'B' | 'T', opts: BuildPromptOpts = {}): BuiltPrompt {
  // buildDesignPrompt() already concatenates every section the harness
  // needs — schema, style, examples, DESIGN_TYPE_DETECTION, roles,
  // layout, text rules, guidelines, variables, auto-replace,
  // post-processing, AND elements (appended last session).
  const full = buildDesignPrompt();
  if (variant === 'T') {
    if (opts.difficulty === 'obvious') {
      // Save ~18kb by stripping the cookbook on single-tool prompts.
      // Models still see the decision tree + PREFER list (which is
      // enough to route to the right tool); they just lose the
      // copy-paste arg-shape examples. Risk: small dip in arg
      // compliance on weaker models — measured in ab-v4.
      const cookbookSkill = getSkillByName('elements-cookbook');
      if (!cookbookSkill) {
        throw new Error(
          'elements-cookbook skill not found in registry — cannot apply obvious-difficulty diet without it',
        );
      }
      const stripped = full.replace(cookbookSkill.content, '');
      if (stripped === full) {
        throw new Error(
          'elements-cookbook content was not present in the full prompt — buildDesignPrompt() may have been refactored; update build-prompt.ts to match',
        );
      }
      return { system: stripped.trim() + '\n\n' + T_TOOL_CALL_INSTRUCTIONS, variant };
    }
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
