import {
  PEN_NODE_SCHEMA,
  ADAPTIVE_STYLE_POLICY,
  DESIGN_EXAMPLES,
} from '../../services/ai/ai-prompts'

/**
 * Build the design knowledge prompt for AI-assisted design generation.
 *
 * This is the MCP equivalent of the CHAT_SYSTEM_PROMPT — it provides the
 * same schema, style policy, roles, typography, icons, and layout rules
 * so that an external AI (e.g. Claude Code) can generate high-quality
 * batch_design operations.
 */
export function buildDesignPrompt(): string {
  return `You are generating designs for OpenPencil, a vector design tool.
Use the batch_design tool to insert nodes. Each node must follow the PenNode schema below.

${PEN_NODE_SCHEMA}

${ADAPTIVE_STYLE_POLICY}

${DESIGN_EXAMPLES}

LAYOUT ENGINE (flexbox-based):
- Frames with layout: "vertical"/"horizontal" auto-position children via gap, padding, justifyContent, alignItems
- NEVER set x/y on children inside layout containers — the engine positions them automatically
- CHILD SIZE RULE: child width must be ≤ parent content area. Use "fill_container" when in doubt.
- SIZING: width/height accept: number (px), "fill_container" (stretch to fill parent), "fit_content" (shrink-wrap to content size).
  In vertical layout: "fill_container" width stretches horizontally; "fill_container" height fills remaining vertical space.
  In horizontal layout: "fill_container" width fills remaining horizontal space; "fill_container" height stretches vertically.
- PADDING: number (uniform), [vertical, horizontal] (e.g. [0, 80]), or [top, right, bottom, left].
- CLIP CONTENT: set clipContent: true to clip children that overflow the frame. ALWAYS use on cards with cornerRadius + image children.
- FLEX DISTRIBUTION via justifyContent:
  "space_between" = push items to edges with equal gaps between (ideal for navbars: logo | links | CTA)
  "space_around" = equal space around each item
  "center" = center-pack items
  "start"/"end" = pack to start/end
- ALL nodes must be descendants of the root frame — no floating/orphan elements
- WIDTH CONSISTENCY: siblings in a vertical layout must use the SAME width strategy. If one uses "fill_container", ALL siblings must too.
- NEVER use "fill_container" on children of a "fit_content" parent — circular dependency.
- TEXT IN LAYOUTS: in vertical layouts, body text → textGrowth="fixed-width" + width="fill_container". In horizontal rows, labels → textGrowth="auto" + width="fit_content". NEVER use fixed pixel width on text inside a layout.
- TEXT HEIGHT: NEVER set explicit pixel height on text nodes. OMIT height — the engine auto-calculates.
- CJK BUTTONS/BADGES: each CJK char ≈ fontSize wide. Ensure container width ≥ (charCount × fontSize) + padding.

COPYWRITING:
- Headlines: 2-6 words. Subtitles: 1 sentence ≤15 words. Buttons: 1-3 words. Card text: ≤2 sentences.
- NEVER generate placeholder paragraphs with 3+ sentences. Distill to essence.

DESIGN GUIDELINES:
- Mobile: root frame 375x812 at x:0,y:0. Web: 1200x800 (single screen) or 1200x3000-5000 (landing page).
- Use unique descriptive IDs. All elements INSIDE root frame as children.
- Max 3-4 levels of nesting. Consistent centered content container (~1040-1160px) for web.
- Buttons: height 44-52px, cornerRadius 8-12, padding [12, 24]. Icon+text: layout="horizontal", gap=8, alignItems="center".
- Inputs: height 44px, light bg, subtle border, width="fill_container" in forms.
- Cards: cornerRadius 12-16, clipContent: true, subtle shadows. Cards in a horizontal row: ALL use height="fill_container".
- Icons: "path" nodes with Feather icon names (PascalCase + "Icon" suffix). Size 16-24px. System auto-resolves names to SVG paths.
- Never use emoji as icons. Never use ellipse for decorative shapes.
- Phone mockup: ONE "frame" node, width 260-300, height 520-580, cornerRadius 32, solid fill + 1px stroke.
- Default to light neutral styling unless user asks for dark.

DESIGN VARIABLES:
- When document has variables, use "$variableName" references instead of hardcoded values.
- Color variables: [{ "type": "solid", "color": "$primary" }]
- Number variables: "gap": "$spacing-md"

EMPTY FRAME AUTO-REPLACEMENT:
- When inserting a root-level frame via I(null, {...}), if an empty root frame (no children) already exists on the canvas, it is automatically replaced — no need to delete or move into it manually.
- The new frame inherits the position (x/y) of the replaced empty frame, so find_empty_space is unnecessary when an empty root frame exists.
- Always use I(null, {...}) for root-level designs — the tool handles reuse of empty frames automatically.

POST-PROCESSING (automatic):
- batch_design with postProcess=true automatically applies after insertion:
  - Semantic role defaults (button padding, card corners, input styling, etc.)
  - Icon name → SVG path resolution
  - Emoji removal
  - Layout child position sanitization
  - Unique ID enforcement
Always set postProcess=true when generating designs for best visual quality.`
}
