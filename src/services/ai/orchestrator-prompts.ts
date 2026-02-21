/**
 * Orchestrator prompt — ultra-lightweight, only splits into sections.
 * No design details, no prompt rewriting. Just structure.
 */

export const ORCHESTRATOR_PROMPT = `Split a UI request into 2-6 spatial sections. Output ONLY JSON, start with {.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":800,"layout":"vertical","fill":[{"type":"solid","color":"#16171B"}]},"subtasks":[{"id":"nav","label":"Navigation Bar","region":{"width":1200,"height":56}},{"id":"hero","label":"Hero Section","region":{"width":1200,"height":400}}]}

RULES:
- Each subtask = one section with id, label, region (width+height).
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812. Desktop: 1200x800.
- NO explanation. NO markdown. JUST the JSON object.`

export const ORCHESTRATOR_TIMEOUTS = {
  hardTimeoutMs: 30_000,
  noTextTimeoutMs: 20_000,
}
