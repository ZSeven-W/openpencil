/**
 * Temporary debug flags for diagnosing the cross-provider empty-response
 * bug (MiniMax + GLM 5.1 both return zero content blocks for the same
 * sub-agent prompt). Toggle these to bisect what part of the
 * generation skill stack is triggering the silent reject.
 *
 * USAGE
 *   1. Edit a flag below from `false` → `true`.
 *   2. Reload the web app (Vite HMR picks up the change instantly —
 *      no MCP recompile needed; this file is browser-side).
 *   3. Trigger a fresh design generation.
 *   4. Watch the dev console for the `[sub-agent]` log line that
 *      reports system prompt size + included skill names.
 *
 * Each generation now logs ONE line right before the streamChat call:
 *
 *   [sub-agent] systemPrompt: chars=4231 skills=schema,jsonl,layout,...
 *
 * Rough decision tree:
 *
 *   - SKILLS_MINIMAL_ONLY = true
 *       Loads ONLY `schema` and `jsonl-format`. If THIS works, the
 *       failure is in some other skill. If it still fails, the
 *       failure is in those two skills themselves, the user prompt,
 *       or the tool schema (none of which we touched recently).
 *
 *   - SKILLS_DISABLE_ANTI_SLOP = true
 *       Filters anti-slop from the resolved skill set without
 *       changing anything else. If failures stop, anti-slop's
 *       Chinese keywords or `{{recentHistory}}` template are
 *       implicated.
 *
 *   - SKILLS_DISABLE_LAYOUT = true
 *       Filters the `layout` skill (which I expanded by ~45 lines
 *       in commit 5bd2c5f). If failures stop, the recent ring/row-
 *       width additions are too long or include content the
 *       provider rejects.
 *
 *   - SKILLS_DISABLE_OVERFLOW = true
 *       Filters the `overflow` skill (priority 16, always loads).
 *
 * REMOVAL
 *   This file is intentionally tiny and isolated. Once the empty-
 *   response bug is fixed, delete it and remove the import from
 *   `orchestrator-sub-agent.ts`. No other code references it.
 */

// COMMITTED DEFAULTS: every skill flag is `false` so this file is a
// no-op for any build. Toggling a flag is a LOCAL UNCOMMITTED edit:
// the developer flips a flag, reloads the web app (Vite HMR), runs
// the failing prompt, observes the result, then reverts the local
// edit before committing anything else. Never push a flipped flag
// to a shared branch — it would silently disable a skill for every
// pulled build.
//
// LOG_PROMPT_SIZE is the one exception: it's a passive observer that
// just emits one console line per sub-agent call. It can stay true
// in committed code while we're actively debugging the empty-response
// bug, and gets flipped to false once the diagnosis is complete.
export const SUB_AGENT_DEBUG_FLAGS = {
  /** Strip every skill except `schema` and `jsonl-format`. */
  SKILLS_MINIMAL_ONLY: false,
  /** Filter `anti-slop` from the resolved skill set. */
  SKILLS_DISABLE_ANTI_SLOP: false,
  /** Filter `layout` from the resolved skill set. */
  SKILLS_DISABLE_LAYOUT: false,
  /** Filter `overflow` from the resolved skill set. */
  SKILLS_DISABLE_OVERFLOW: false,
  /**
   * Log a one-liner with system prompt size + included skill names
   * before every sub-agent streamChat call. Passive observer only —
   * safe to leave true while actively debugging.
   */
  LOG_PROMPT_SIZE: true,
} as const;
