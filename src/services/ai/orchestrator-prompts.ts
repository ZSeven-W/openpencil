/**
 * Orchestrator prompt — ultra-lightweight, only splits into sections.
 * No design details, no prompt rewriting. Just structure.
 */

export const ORCHESTRATOR_PROMPT = `Split a UI request into 6-10 cohesive sections. Each subtask = a meaningful page section (e.g. hero with headline+CTA+image = ONE subtask). Output ONLY JSON, start with {.

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","fill":[{"type":"solid","color":"#0B1120"}]},"styleGuide":{"palette":{"background":"#0B1120","surface":"#141D33","text":"#F1F5F9","secondary":"#94A3B8","accent":"#3B82F6","accent2":"#06B6D4","border":"#1E293B"},"fonts":{"heading":"Space Grotesk","body":"Inter"},"aesthetic":"dark navy with blue gradient accents"},"subtasks":[{"id":"nav","label":"Navigation Bar","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","region":{"width":1200,"height":480}}]}

RULES:
- ALWAYS include a Navigation Bar as the FIRST subtask.
- Combine related elements: "Hero with title + image + CTA" = ONE subtask, not three.
- Each subtask generates a meaningful section (~10-30 nodes). Only split if it would exceed 40 nodes.
- Choose a visual direction (palette, fonts, aesthetic) that matches the product personality and target audience. Output it in "styleGuide".
- Root frame fill must use the styleGuide palette background color.
- Root frame height: Mobile (width=375) → set height=812 (fixed viewport). Desktop (width=1200) → set height=0 (auto-expands as sections are generated).
- Subtask height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, testimonials 300-400px, CTA 200-300px, footer 200-300px.
- If a section is about "App截图"/"XX截图"/"screenshot"/"mockup", plan it as a phone mockup placeholder block, not a detailed mini-app reconstruction.
- Navigation sections should preserve good horizontal balance.
- Ensure navigation links are planned as an evenly distributed middle group.
- Desktop landing pages MUST always include a Navigation Bar as the FIRST subtask.
- Regions tile to fill rootFrame. vertical = top-to-bottom.
- Mobile: 375x812 (both width AND height are fixed). Desktop: 1200x0 (width fixed, height auto-expands).
- NO explanation. NO markdown. JUST the JSON object.`

// Safe code block delimiter
const BLOCK = "```"

/**
 * Sub-agent prompt — lean version of DESIGN_GENERATOR_PROMPT.
 * Only essential schema + JSONL output format. Includes one example for format clarity.
 */
export const SUB_AGENT_PROMPT = `PenNode flat JSONL engine. Output a ${BLOCK}json block with ONE node per line.

TYPES: frame (width,height,layout,gap,padding,justifyContent,alignItems,clipContent,cornerRadius,fill,stroke,effects,children), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fontStyle,fill,width,height,textAlign,textGrowth,lineHeight,letterSpacing,textAlignVertical), path (d,width,height,fill,stroke), image (src,width,height)
textGrowth: "auto" (expand horizontally, no wrap), "fixed-width" (fixed width, height auto-sizes to wrapped content), "fixed-width-height" (both fixed). Default for text in layouts: "fixed-width" + width="fill_container".
lineHeight: multiplier (1.1-1.2 for headings, 1.4-1.6 for body). letterSpacing: px (-0.5 for headlines, 0.5-2 for uppercase labels). textAlignVertical: "top"|"middle"|"bottom".
SHARED: id, type, name, x, y, opacity
width/height: number (px), "fill_container" (stretch), "fit_content" (shrink-wrap)
  In vertical layout: "fill_container" width = stretch horizontally; height = fill remaining vertical space.
  In horizontal layout: "fill_container" width = fill remaining horizontal space; height = stretch vertically.
padding: number or [vertical,horizontal] (e.g. [0,80]) or [top,right,bottom,left]
clipContent: boolean — clips overflowing children. ALWAYS use on frames with cornerRadius + image children.
justifyContent: "start"|"center"|"end"|"space_between"|"space_around". Use "space_between" for navbars/footers.
Fill=[{"type":"solid","color":"#hex"}] or [{"type":"linear_gradient","angle":N,"stops":[{"offset":0,"color":"#hex"},{"offset":1,"color":"#hex"}]}]
Stroke={"thickness":N,"fill":[...]}
cornerRadius=number. fill=array.

CRITICAL LAYOUT RULES (violations cause rendering bugs):
- Section root frame: width="fill_container", height="fit_content", layout="vertical". NEVER use fixed pixel height on section root — it causes blank gaps. Let content determine height.
- NEVER set x or y on ANY child inside a layout frame. The layout engine positions them automatically.
- ALL nodes must be descendants of the section root. No orphan/floating nodes.
- CHILD SIZE RULE: every child's width must be ≤ parent's content area. Use "fill_container" when in doubt.
- CLIP CONTENT: set clipContent: true on cards with cornerRadius + image children. Prevents overflow past rounded corners.
- FLEX LAYOUT: use justifyContent to distribute children:
  "space_between" = push first/last to edges, even space between (BEST for navbars: logo | links | CTA).
  "space_around" = equal space around each child.
  "center" = center-pack children.
- SIZING: width/height accept: number (px), "fill_container" (stretch to parent), "fit_content" (shrink to content).
- PADDING: number (uniform), [vertical, horizontal] (e.g. [0, 80] for side padding), or [top, right, bottom, left].
- For two-column layouts: horizontal frame with two child frames, each "fill_container" width.
- For centered content: frame with alignItems="center", then content frame with fixed width (e.g. 1080).
- TEXT IN LAYOUTS: text inside layout frames MUST use textGrowth="fixed-width" + width="fill_container". This makes text wrap within the parent and height auto-sizes. NEVER use fixed pixel widths/heights for text.
- SHORT TEXT: buttons/labels can omit textGrowth (defaults to "auto" = expands horizontally).

FORMAT: Each line has "_parent" (null=root, else parent-id). Parent before children.
${BLOCK}json
{"_parent":null,"id":"root","type":"frame","name":"Hero","width":"fill_container","height":"fit_content","layout":"vertical","padding":80,"gap":24,"alignItems":"center","fill":[{"type":"solid","color":"#0B1120"}]}
{"_parent":"root","id":"content","type":"frame","name":"Content","width":1080,"height":400,"layout":"horizontal","gap":48,"alignItems":"center"}
{"_parent":"content","id":"left","type":"frame","name":"Text Column","width":520,"height":360,"layout":"vertical","gap":20}
{"_parent":"left","id":"title","type":"text","name":"Headline","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","lineHeight":1.1,"letterSpacing":-0.5,"textGrowth":"fixed-width","width":"fill_container","fill":[{"type":"solid","color":"#F1F5F9"}]}
{"_parent":"left","id":"desc","type":"text","name":"Description","content":"AI-powered vocabulary learning","fontSize":18,"lineHeight":1.5,"textGrowth":"fixed-width","width":"fill_container","fill":[{"type":"solid","color":"#94A3B8"}]}
{"_parent":"left","id":"cta","type":"frame","name":"CTA Button","width":180,"height":48,"cornerRadius":10,"layout":"horizontal","gap":8,"justifyContent":"center","alignItems":"center","fill":[{"type":"solid","color":"#3B82F6"}]}
{"_parent":"cta","id":"cta-text","type":"text","name":"CTA Label","content":"Get Started","fontSize":16,"fontWeight":600,"width":"fill_container","fill":[{"type":"solid","color":"#FFFFFF"}]}
{"_parent":"content","id":"phone","type":"frame","name":"Phone Mockup","width":280,"height":560,"cornerRadius":32,"fill":[{"type":"solid","color":"#141D33"}],"stroke":{"thickness":1,"fill":[{"type":"solid","color":"#1E293B"}]}}
${BLOCK}

DESIGN RULES:
- Hero: large headline (40-56px), gradient or bold backgrounds, clear CTA, generous whitespace
- Visual rhythm: alternate section backgrounds for separation
- Typography: Display 40-56px → Heading 28-36px → Subheading 20-24px → Body 16-18px → Caption 13-14px. Always set lineHeight: headings 1.1-1.2, body 1.4-1.6, captions 1.3. Use letterSpacing: -0.5 for large headlines, 0.5-2 for uppercase labels.
- Cards: give cards enough height for their content. A card with icon+title+description needs at least 160-200px height. Use "fill_container" height when cards are in a row. ALWAYS set clipContent: true on cards with cornerRadius + image children.
- CTAs: bold accent color, padding 16-20px v / 32-48px h
- Icons: SVG path nodes 16-24px; NEVER use emoji
- PHONE MOCKUP: exactly ONE "frame" node, width 260-300, height 520-580, cornerRadius 32, solid fill + 1px stroke. NEVER use ellipse or circle for mockups. NEVER add any children inside (no text, no frames, no images). Every phone mockup must look identical.
- NEVER use ellipse nodes for decorative shapes. Use frame or rectangle with cornerRadius instead.
- Use STYLE GUIDE colors/fonts from user prompt consistently. Do not introduce random colors.
- Nav bar: use justifyContent="space_between" with 3 child groups (logo-group | links-group | cta-button). padding=[0,80]. This auto-distributes them perfectly.
- TEXT: text inside layout frames MUST use textGrowth="fixed-width" + width="fill_container". NEVER set fixed pixel widths/heights on text — textGrowth handles auto-sizing. Short labels/buttons can omit textGrowth.

Start with ${BLOCK}json immediately. No preamble, no <step> tags.`
