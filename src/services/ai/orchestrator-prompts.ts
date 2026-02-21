/**
 * Orchestrator prompt — ultra-lightweight, only splits into sections.
 * No design details, no prompt rewriting. Just structure.
 */

export const ORCHESTRATOR_PROMPT = `Split a UI request into MANY small atomic sections (4-15). Each subtask = ONE simple row/bar/card group. Output ONLY JSON, start with {.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":800,"layout":"vertical","fill":[{"type":"solid","color":"#16171B"}]},"subtasks":[{"id":"nav","label":"Navigation Bar","region":{"width":1200,"height":56}},{"id":"hero-text","label":"Hero Headline","region":{"width":1200,"height":120}},{"id":"hero-img","label":"Hero Image","region":{"width":1200,"height":280}}]}

RULES:
- Split aggressively: "Hero with title + image + CTA" → 3 subtasks (title, image, CTA).
- Each subtask generates <15 nodes. If it would need more, split further.
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812. Desktop: 1200x800.
- NO explanation. NO markdown. JUST the JSON object.`

export const ORCHESTRATOR_TIMEOUTS = {
  hardTimeoutMs: 60_000,
  noTextTimeoutMs: 45_000,
}

// Safe code block delimiter
const BLOCK = "```"

/**
 * Sub-agent prompt — lean version of DESIGN_GENERATOR_PROMPT.
 * Only essential schema + JSONL output format. Includes one example for format clarity.
 */
export const SUB_AGENT_PROMPT = `PenNode flat JSONL engine. Output a ${BLOCK}json block with ONE node per line.

TYPES: frame (width,height,layout,gap,padding,justifyContent,alignItems,cornerRadius,fill,stroke,effects,children), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fill,width,height,textAlign), path (d,width,height,fill,stroke), image (src,width,height)
SHARED: id, type, name, x, y, opacity
Fill=[{"type":"solid","color":"#hex"}] Stroke={"thickness":N,"fill":[...]}
cornerRadius=number. fill=array. No x/y on children in layout frames. Use "fill_container" to stretch.

FORMAT: Each line has "_parent" (null=root, else parent-id). Parent before children.
${BLOCK}json
{"_parent":null,"id":"root","type":"frame","name":"Section","width":"fill_container","height":300,"layout":"vertical","gap":16,"padding":24}
{"_parent":"root","id":"title","type":"text","name":"Title","content":"Hello","fontSize":24,"fontWeight":700,"width":"fill_container","height":32,"fill":[{"type":"solid","color":"#F4F4F5"}]}
${BLOCK}

STYLE: Dark. Bg #16171B, Card #1E2026, Text #F4F4F5, Secondary #52525B, Accent #22C55E, Border #2A2B30. Headlines "Space Grotesk" 700, Body "Inter". cornerRadius 4. No shadows, 1px borders.
Icons: path+SVG d 16-24px. Images: src "https://picsum.photos/{w}/{h}".

Start with ${BLOCK}json immediately. No preamble, no <step> tags.`
