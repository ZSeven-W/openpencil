/**
 * Orchestrator prompt — ultra-lightweight, only splits into sections.
 * No design details, no prompt rewriting. Just structure.
 */

export const ORCHESTRATOR_PROMPT = `Split a UI request into cohesive subtasks. Each subtask = a meaningful UI section or component group. Output ONLY JSON, start with {.

DESIGN TYPE DETECTION:
First determine the design type from the user's request:
- "landing page" / "website" / "官网" / "首页" → Landing Page (multi-section scrollable page, 6-10 subtasks)
- "login" / "signup" / "register" / "登录" / "注册" → App Screen (single screen, 1-4 subtasks)
- "dashboard" / "settings" / "profile" / "设置" / "个人中心" → App Screen (single screen, 2-5 subtasks)
- "form" / "表单" / "checkout" / "结算" → App Screen (single screen, 1-4 subtasks)
- Other app screens (modal, dialog, onboarding, etc.) → App Screen (1-5 subtasks)

FORMAT:
{"rootFrame":{"id":"page","name":"Page","width":1200,"height":0,"layout":"vertical","fill":[{"type":"solid","color":"#F8FAFC"}]},"styleGuide":{"palette":{"background":"#F8FAFC","surface":"#FFFFFF","text":"#0F172A","secondary":"#64748B","accent":"#2563EB","accent2":"#0EA5E9","border":"#E2E8F0"},"fonts":{"heading":"Space Grotesk","body":"Inter"},"aesthetic":"clean modern with blue accents"},"subtasks":[{"id":"nav","label":"Navigation Bar","region":{"width":1200,"height":72}},{"id":"hero","label":"Hero Section","region":{"width":1200,"height":560}},{"id":"features","label":"Feature Cards","region":{"width":1200,"height":480}}]}

RULES:
- STYLE SELECTION: Choose light or dark theme based on user intent. Dark: user mentions dark/cyber/terminal/neon/夜间/暗黑/deep/gaming/noir. Light (default): all other cases — SaaS, marketing, education, e-commerce, productivity, social. Never default to dark unless the content clearly calls for it.
- Detect the design type FIRST, then choose the appropriate structure and subtask count.
- Landing pages: include Navigation Bar as the FIRST subtask, followed by Hero, feature sections, CTA, footer, etc. (6-10 subtasks)
- App screens (login, settings, forms, etc.): do NOT include Navigation Bar, Hero, CTA, or footer. Only include the actual UI elements needed (1-5 subtasks).
- FORM INTEGRITY: Keep a form's core elements (inputs + submit button) in the same subtask. Splitting inputs into one subtask and the button into another causes duplicate buttons.
- Combine related elements: "Hero with title + image + CTA" = ONE subtask, not three.
- Each subtask generates a meaningful section (~10-30 nodes). Only split if it would exceed 40 nodes.
- Choose a visual direction (palette, fonts, aesthetic) that matches the product personality and target audience. Output it in "styleGuide".
- CJK FONT RULE: If the user's request is in Chinese/Japanese/Korean or the product targets CJK audiences, the styleGuide fonts MUST use CJK-compatible fonts: heading="Noto Sans SC" (Chinese) / "Noto Sans JP" (Japanese) / "Noto Sans KR" (Korean), body="Inter". NEVER use "Space Grotesk" or "Manrope" as heading font for CJK content — they have no CJK character support.
- Root frame fill must use the styleGuide palette background color.
- Root frame height: Mobile (width=375) → set height=812 (fixed viewport). Desktop (width=1200) → set height=0 (auto-expands as sections are generated).
- Landing page height hints: nav 64-80px, hero 500-600px, feature sections 400-600px, testimonials 300-400px, CTA 200-300px, footer 200-300px.
- App screen height hints: status bar 44px, header 56-64px, form fields 48-56px each, buttons 48px, spacing 16-24px.
- If a section is about "App截图"/"XX截图"/"screenshot"/"mockup", plan it as a phone mockup placeholder block, not a detailed mini-app reconstruction.
- For landing pages: navigation sections should preserve good horizontal balance, links evenly distributed in the center group.
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

TYPES & SCHEMA:
frame (width,height,layout,gap,padding,justifyContent,alignItems,clipContent,cornerRadius,fill,stroke,effects,children), rectangle, ellipse, text (content,fontFamily,fontSize,fontWeight,fontStyle,fill,width,height,textAlign,textGrowth,lineHeight,letterSpacing,textAlignVertical), path (d,width,height,fill,stroke), image (src,width,height)
SHARED: id, type, name, role, x, y, opacity
ROLES: Add "role" to nodes for smart defaults. System fills unset props based on role. Your props always override.
  Layout: section, row, column, centered-content, form-group, divider, spacer
  Nav: navbar, nav-links, nav-link | Interactive: button, icon-button, badge, pill, input, search-bar
  Display: card, stat-card, pricing-card, feature-card | Media: phone-mockup, avatar, icon
  Typography: heading, subheading, body-text, caption, label | Table: table, table-row, table-header
  Any string is valid — unknown roles pass through unchanged.
width/height: number (px) | "fill_container" (stretch) | "fit_content" (shrink-wrap)
textGrowth: "auto" (no wrap) | "fixed-width" (wrap, auto-height) | "fixed-width-height" (both fixed)
lineHeight: multiplier (1.1-1.2 headings, 1.4-1.6 body). letterSpacing: px (-0.5 headlines, 0.5-2 uppercase).
padding: number | [v,h] | [top,right,bottom,left]. clipContent: true on cornerRadius + image frames.
justifyContent: "start"|"center"|"end"|"space_between"|"space_around". Fill=[{"type":"solid","color":"#hex"}] or linear_gradient.
Stroke={"thickness":N,"fill":[...]}. cornerRadius=number.

LAYOUT RULES:
- Section root: width="fill_container", height="fit_content", layout="vertical". Never fixed pixel height on section root.
- Never set x/y on children inside layout frames — layout engine positions them automatically.
- All nodes must descend from the section root. No orphan nodes.
- Child width must be ≤ parent content area. Use "fill_container" when in doubt.
- Width consistency: siblings in vertical layout must use the SAME width strategy. Mixing fixed-px and fill_container causes misalignment.
- Never "fill_container" children inside "fit_content" parent — circular dependency.
- Keep hierarchy shallow: no pointless "Inner" wrappers. Only use wrappers with a visual purpose (fill, padding, border).
- clipContent: true on cards with cornerRadius + image children.
- justifyContent "space_between" for navbars (logo | links | CTA). "center" to center-pack.
- Two-column: horizontal frame, two child frames each "fill_container" width.
- Centered content: frame alignItems="center", content frame with fixed width (e.g. 1080).
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20. ONE primary action button only.
  Social login buttons: horizontal frame width="fill_container", each button width="fit_content".
  BAD: email width=350, button width=120. GOOD: email width="fill_container", button width="fill_container".

TEXT RULES:
- Body/description text in vertical layout: width="fill_container" + textGrowth="fixed-width". This wraps text and auto-sizes height.
- Short labels in horizontal rows: width="fit_content" (or omit) + textGrowth="auto" (or omit). Prevents squeezing siblings.
- NEVER fixed pixel width on text inside layout frames — causes overflow. Only allowed in layout="none" parent.
  BAD: card width=195 padding=[24,40], child text width=378 → overflows by 263px!
  GOOD: same card, text width="fill_container" → auto-constrained, wraps correctly.
- Text >15 chars MUST have textGrowth="fixed-width" — without it text won't wrap.
  BAD: {"content":"长文本...","width":"fill_container"} → one long line, overflows!
  GOOD: {"content":"长文本...","width":"fill_container","textGrowth":"fixed-width","lineHeight":1.6}
- NEVER set explicit pixel height on text. Omit height entirely — engine auto-sizes it.
  BAD: {"content":"50,000+","fontSize":36,"height":22} → text overlaps siblings!
  GOOD: {"content":"50,000+","fontSize":36,"width":"fill_container"} → auto-height ~43px.
- Headlines: 2-6 words. Subtitles: ≤15 words. Feature titles: 2-4 words. Descriptions: ≤20 words. Buttons: 1-3 words.
- Never write 3+ sentence paragraphs. Distill to core message. Design mockups are not documents.

DESIGN RULES:
- Typography scale: Display 40-56px → Heading 28-36px → Subheading 20-24px → Body 16-18px → Caption 13-14px. Set lineHeight: headings 1.1-1.2, body 1.4-1.6. letterSpacing: -0.5 for headlines, 0.5-2 for uppercase.
- CJK fonts: use "Noto Sans SC" (CN) / "Noto Sans JP" (JP) / "Noto Sans KR" (KR) for headings. Never "Space Grotesk"/"Manrope" for CJK. CJK lineHeight: 1.3-1.4 headings, 1.6-1.8 body. CJK letterSpacing: 0, never negative.
- Card rows: ALL cards use width="fill_container" + height="fill_container" for even distribution and equal height. Dense rows (5+): use short titles, max 2 text blocks per card.
- Icons: "path" nodes with descriptive names ("SearchIcon", "MenuIcon" etc.). System auto-resolves to SVG. Size 16-24px. Never use emoji as icons.
- Semantic inputs should include affordance icons when appropriate:
  - search bars: leading SearchIcon
  - password fields: trailing EyeIcon or EyeOffIcon (use justifyContent="space_between")
  - email/account fields: leading MailIcon or UserIcon
- Phone mockup: ONE frame, width 260-300, height 520-580, cornerRadius 32, solid fill + 1px stroke. No ellipse for mockups. At most ONE centered text child inside.
- Never ellipse for decorative shapes — use frame/rectangle with cornerRadius.
- Use style guide colors/fonts consistently. No random colors.
- Buttons: height 44-52px, padding [12,24] min. Icon+text: layout="horizontal", gap=8, alignItems="center".
- CJK buttons: width ≥ charCount × fontSize + horizontalPadding.
- Icon-only buttons: square ≥44×44, justifyContent/alignItems="center", path icon 20-24px.
- Badges/tags: only for short labels (CJK ≤8 / Latin ≤16 chars). Longer text → normal text row.
- Hero + phone (desktop): two-column horizontal layout (left text, right phone). Not stacked unless mobile.
- Landing pages: hero 40-56px headline, alternating section backgrounds, nav with space_between.
- App screens: focus on core function, inputs width="fill_container", consistent 48-56px height, 16-24px gap.

FORMAT: Each line has "_parent" (null=root, else parent-id). Parent before children.
${BLOCK}json
{"_parent":null,"id":"root","type":"frame","name":"Hero","role":"hero","width":"fill_container","height":"fit_content","fill":[{"type":"solid","color":"#F8FAFC"}]}
{"_parent":"root","id":"content","type":"frame","name":"Content","role":"row","width":1080,"height":400,"gap":48}
{"_parent":"content","id":"left","type":"frame","name":"Text Column","role":"column","width":520,"height":360,"gap":20}
{"_parent":"left","id":"title","type":"text","name":"Headline","role":"heading","content":"Learn Smarter","fontSize":48,"fontWeight":700,"fontFamily":"Space Grotesk","fill":[{"type":"solid","color":"#0F172A"}]}
{"_parent":"left","id":"desc","type":"text","name":"Description","role":"body-text","content":"AI-powered vocabulary learning","fontSize":18,"fill":[{"type":"solid","color":"#64748B"}]}
{"_parent":"left","id":"cta","type":"frame","name":"CTA Button","role":"button","width":180,"cornerRadius":10,"fill":[{"type":"solid","color":"#2563EB"}]}
{"_parent":"cta","id":"cta-text","type":"text","name":"CTA Label","role":"label","content":"Get Started","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFFFFF"}]}
{"_parent":"content","id":"phone","type":"frame","name":"Phone Mockup","role":"phone-mockup","fill":[{"type":"solid","color":"#F1F5F9"}],"stroke":{"thickness":1,"fill":[{"type":"solid","color":"#E2E8F0"}]}}
${BLOCK}

Start with ${BLOCK}json immediately. No preamble, no <step> tags.`
