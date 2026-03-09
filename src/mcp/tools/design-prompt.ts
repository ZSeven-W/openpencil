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
Use batch_design (for multi-node designs with DSL) or insert_node (for single node trees with JSON).
Both support postProcess=true for automatic role defaults, icon resolution, and layout sanitization.
Each node must follow the PenNode schema below.

${PEN_NODE_SCHEMA}

${ADAPTIVE_STYLE_POLICY}

${DESIGN_EXAMPLES}

DESIGN TYPE DETECTION:
Classify by the design's PURPOSE to choose the correct root frame size — reason about intent, do not keyword-match:
- Multi-section page (marketing, promotional, informational content designed to be scrolled) → Desktop: width=1200, height=0 (auto-expands), layout="vertical"
- Single-task screen (functional UI focused on one user task: authentication, forms, settings, profiles, modals, onboarding, etc.) → Mobile: width=375, height=812 (FIXED)
- Data-rich workspace (dashboards, admin panels, analytics) → Desktop: width=1200, height=0
- CRITICAL: Single-task screens MUST be 375×812. NEVER use 1200 width for focused app screens.
- Multi-section page height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, CTA 200-300px, footer 200-300px.
- Single-task screen height hints: status bar 44px, header 56-64px, form fields 48-56px each, buttons 48px, spacing 16-24px.

SEMANTIC ROLES (context-aware defaults):
Add "role" to nodes for automatic smart defaults. System fills unset props based on role. Your explicit props always override.
Available roles:
  Layout:       section, row, column, centered-content, form-group, divider, spacer
  Navigation:   navbar, nav-links, nav-link
  Interactive:  button, icon-button, badge, tag, pill, input, form-input, search-bar
  Display:      card, stat-card, pricing-card, feature-card, image-card
  Media:        phone-mockup, screenshot-frame, avatar, icon
  Typography:   heading, subheading, body-text, caption, label
  Content:      hero, feature-grid, testimonial, cta-section, footer, stats-section
  Table:        table, table-row, table-header, table-cell
Key role defaults:
  section     → width:fill_container, height:fit_content, gap:24, padding:[60,80] (desktop)/[40,16] (mobile)
  navbar      → height:72 (desktop)/56 (mobile), layout:horizontal, justifyContent:space_between, alignItems:center
  hero        → layout:vertical, padding:[80,80] (desktop)/[40,16] (mobile), gap:24, alignItems:center
  button      → padding:[12,24], height:44, cornerRadius:8, layout:horizontal, alignItems:center
  button (in navbar) → padding:[8,16], height:36
  button (in form-group) → width:fill_container, height:48, padding:[12,24]
  icon-button → 44×44, layout:horizontal, justifyContent:center, alignItems:center, cornerRadius:8
  badge/pill  → layout:horizontal, padding:[6,12], cornerRadius:999
  input       → height:48, layout:horizontal, padding:[12,16]
  form-input  → same as input + width:fill_container
  search-bar  → height:44, cornerRadius:22
  card        → gap:12, cornerRadius:12, clipContent:true
  card (in horizontal layout) → width:fill_container, height:fill_container
  feature-card (in horizontal) → width:fill_container, height:fill_container
  phone-mockup → width:280, height:560, cornerRadius:32, layout:none
  avatar      → circular (cornerRadius=width/2), clipContent:true
  heading     → lineHeight:1.2, letterSpacing:-0.5
  body-text   → lineHeight:1.5, textGrowth:fixed-width, width:fill_container
  caption     → lineHeight:1.3, textGrowth:auto
  label       → lineHeight:1.2, textGrowth:auto, textAlignVertical:middle
  divider     → width:fill_container, height:1 (or width:1 for vertical)
  spacer      → width:fill_container, height:40
  feature-grid → layout:horizontal, gap:24, alignItems:start
  table       → layout:vertical, gap:0, clipContent:true
  table-row   → layout:horizontal, padding:[12,16], alignItems:center
  table-cell  → width:fill_container
Any string is valid as a role — unknown roles pass through unchanged.

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
- Section root: width="fill_container", height="fit_content", layout="vertical". Never fixed pixel height on section root.
- Two-column: horizontal frame, two child frames each "fill_container" width.
- Centered content: frame alignItems="center", content frame with fixed width (e.g. 1080).
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20. ONE primary action button only.
  Social login buttons: horizontal frame width="fill_container", each button width="fit_content".
- Keep hierarchy shallow: no pointless "Inner" wrappers. Only use wrappers with a visual purpose (fill, padding, border).

TEXT RULES:
- Body/description text in vertical layout: width="fill_container" + textGrowth="fixed-width". This wraps text and auto-sizes height.
- Short labels in horizontal rows: width="fit_content" (or omit) + textGrowth="auto" (or omit). Prevents squeezing siblings.
- NEVER fixed pixel width on text inside layout frames — causes overflow. Only allowed in layout="none" parent.
- Text >15 chars MUST have textGrowth="fixed-width" — without it text won't wrap.
- NEVER set explicit pixel height on text nodes. OMIT height — the engine auto-calculates.
- Typography scale: Display 40-56px → Heading 28-36px → Subheading 20-24px → Body 16-18px → Caption 13-14px.
  lineHeight: headings 1.1-1.2, body 1.4-1.6. letterSpacing: -0.5 for headlines, 0.5-2 for uppercase.

CJK TYPOGRAPHY:
- CJK font selection: heading="Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean), body="Inter".
  NEVER use "Space Grotesk" or "Manrope" for CJK content — they have no CJK glyphs.
- CJK lineHeight: headings 1.3-1.4 (NOT 1.1-1.2 like Latin), body 1.6-1.8 (NOT 1.4-1.6 like Latin).
- CJK letterSpacing: 0, NEVER negative. Negative letterSpacing causes CJK character overlap.
- CJK buttons/badges: each CJK char ≈ fontSize wide. Ensure container width ≥ (charCount × fontSize) + padding.

COPYWRITING:
- Headlines: 2-6 words. Subtitles: 1 sentence ≤15 words. Buttons: 1-3 words. Card text: ≤2 sentences.
- NEVER generate placeholder paragraphs with 3+ sentences. Distill to essence.

DESIGN GUIDELINES:
- Use unique descriptive IDs. All elements INSIDE root frame as children.
- Max 3-4 levels of nesting. Consistent centered content container (~1040-1160px) for web.
- Buttons: height 44-52px, cornerRadius 8-12, padding [12, 24]. Icon+text: layout="horizontal", gap=8, alignItems="center".
- Icon-only buttons: square ≥44×44, justifyContent/alignItems="center", path icon 20-24px.
- Inputs: height 48px, light bg, subtle border, width="fill_container" in forms.
  Semantic affordance icons: search→leading SearchIcon, password→trailing EyeIcon, email→leading MailIcon.
- Cards: cornerRadius 12-16, clipContent: true, subtle shadows. Cards in a horizontal row: ALL use width="fill_container" + height="fill_container".
- Icons: "path" nodes with Feather icon names (PascalCase + "Icon" suffix, e.g. "SearchIcon", "MenuIcon"). Size 16-24px. System auto-resolves names to SVG paths.
- Never use emoji as icons. Never use ellipse for decorative shapes — use frame/rectangle with cornerRadius.
- Phone mockup: ONE "frame" node with role="phone-mockup". No ellipse for mockups. At most ONE centered text child inside.
- Hero + phone (desktop): two-column horizontal layout (left text, right phone). Not stacked unless mobile.
- Landing pages: hero 40-56px headline, alternating section backgrounds, nav with space_between.
- App screens: focus on core function, inputs width="fill_container", consistent 48-56px height, 16-24px gap.
- Default to light neutral styling unless user asks for dark.
  Dark theme only when user explicitly mentions: dark/cyber/terminal/neon/夜间/暗黑/gaming/noir.

DESIGN VARIABLES:
- When document has variables, use "$variableName" references instead of hardcoded values.
- Color variables: [{ "type": "solid", "color": "$primary" }]
- Number variables: "gap": "$spacing-md"
- Variables can have per-theme values. Use $name syntax — the engine resolves to concrete values for rendering.

EMPTY FRAME AUTO-REPLACEMENT:
- When inserting a root-level frame via I(null, {...}), if an empty root frame (no children) already exists on the canvas, it is automatically replaced — no need to delete or move into it manually.
- The new frame inherits the position (x/y) of the replaced empty frame, so find_empty_space is unnecessary when an empty root frame exists.
- Always use I(null, {...}) for root-level designs — the tool handles reuse of empty frames automatically.

POST-PROCESSING (automatic with postProcess=true):
- Semantic role defaults: fills unset props based on role (see SEMANTIC ROLES above). Context-aware — e.g. button defaults differ in navbar vs form.
- Icon name → SVG path auto-resolution: set icon "name" field, system resolves to SVG "d" path.
- Card row equalization: horizontal layout with 2+ cards auto-equalizes to fill_container width+height.
- Horizontal overflow fix: auto-reduces gap or expands parent when children exceed width.
- Form input consistency: if any input uses fill_container, all sibling inputs get normalized.
- Text height estimation: auto-calculates optimal height based on fontSize, lineHeight, and content width.
- Frame height expansion: auto-expands frames when content exceeds fixed height.
- clipContent auto-addition: frames with cornerRadius + image children get clipContent:true.
- Emoji removal and layout child position sanitization.
- Unique ID enforcement.
Always set postProcess=true when generating designs for best visual quality.`
}
