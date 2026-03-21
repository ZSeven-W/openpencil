/**
 * Prompt Knowledge Sections — modular knowledge blocks for progressive loading.
 *
 * Each section is a self-contained knowledge block with:
 *   - `content`: the actual prompt text
 *   - `triggers`: keywords that activate this section (matched against user message)
 *   - `always`: if true, always included in design generation prompts
 *   - `priority`: loading order (lower = loaded first)
 *
 * To add a new section: just add an entry to SECTION_REGISTRY.
 * Detection, assembly, and MCP listing all derive from the registry automatically.
 *
 * Usage:
 *   import { detectSections, assembleSections } from './ai-prompt-sections'
 *   const needed = detectSections(userMessage)
 *   const knowledge = assembleSections(needed)
 */

import { AVAILABLE_FEATHER_ICONS } from './icon-resolver'
import type { DesignMdSpec } from '@/types/design-md'

const FEATHER_ICON_NAMES = AVAILABLE_FEATHER_ICONS.join(', ')

// ---------------------------------------------------------------------------
// Section type
// ---------------------------------------------------------------------------

export type PromptSectionKey =
  | 'schema'
  | 'layout'
  | 'text'
  | 'style'
  | 'guidelines'
  | 'examples'
  | 'roles'
  | 'copywriting'
  | 'overflow'
  | 'cjk'
  | 'variables'
  | 'icons'
  | 'design-md'

// ---------------------------------------------------------------------------
// Section registry — single source of truth
// ---------------------------------------------------------------------------

interface SectionDef {
  /** The prompt knowledge block content */
  content: string
  /** Keywords that trigger this section (matched case-insensitively against user message).
   *  Supports plain strings and regex patterns (prefix with `/`). */
  triggers?: string[]
  /** If true, always included in design generation (not modification) prompts */
  always?: boolean
  /** Loading priority — lower numbers load first (default 50) */
  priority?: number
  /** If true, only loaded when a special flag is set (e.g. hasVariables, hasDesignMd) */
  flag?: 'hasVariables' | 'hasDesignMd'
}

export const SECTION_REGISTRY: Record<PromptSectionKey, SectionDef> = {
  schema: {
    content: `PenNode types (the ONLY format you output for designs):
- frame: Container. Props: width, height, layout ('none'|'vertical'|'horizontal'), gap, padding, justifyContent ('start'|'center'|'end'|'space_between'|'space_around'), alignItems ('start'|'center'|'end'), clipContent (boolean), children[], cornerRadius, fill, stroke, effects
- rectangle: Props: width, height, cornerRadius, fill, stroke, effects
- ellipse: Props: width, height, fill, stroke, effects
- text: Props: content, fontFamily, fontSize, fontWeight, fontStyle ('normal'|'italic'), fill, width, height, textAlign, textGrowth ('auto'|'fixed-width'|'fixed-width-height'), lineHeight (multiplier), letterSpacing (px), textAlignVertical ('top'|'middle'|'bottom')
- path: SVG icon. Props: d (SVG path), width, height, fill, stroke, effects
- image: Props: width, height, cornerRadius, effects, imageSearchQuery (2-3 English keywords)

All nodes share: id, type, name, role, x, y, rotation, opacity
Fill = [{ type: "solid", color: "#hex" }] or [{ type: "linear_gradient", angle, stops: [{ offset, color }] }]
Stroke = { thickness, fill: [...] }  Effects = [{ type: "shadow", offsetX, offsetY, blur, spread, color }]
SIZING: width/height accept number (px), "fill_container", or "fit_content".
PADDING: number (uniform), [v, h], or [top, right, bottom, left].
cornerRadius is a number. fill is ALWAYS an array. Do NOT set x/y on children inside layout frames.`,
    always: true,
    priority: 0,
  },

  layout: {
    content: `LAYOUT ENGINE (flexbox-based):
- Frames with layout: "vertical"/"horizontal" auto-position children via gap, padding, justifyContent, alignItems.
- NEVER set x/y on children inside layout containers.
- CHILD SIZE RULE: child width must be ≤ parent content area. Use "fill_container" when in doubt.
- In vertical layout: "fill_container" width stretches horizontally. In horizontal: fills remaining space.
- CLIP CONTENT: clipContent: true clips overflowing children. ALWAYS use on cards with cornerRadius + image.
- justifyContent: "space_between" (navbars), "center", "start"/"end", "space_around".
- WIDTH CONSISTENCY: siblings must use same width strategy. Don't mix fixed-px and fill_container.
- NEVER use "fill_container" on children of "fit_content" parent — circular dependency.
- Two-column: horizontal frame → two child frames each "fill_container" width.
- Keep hierarchy shallow: no pointless wrappers. Only use wrappers with visual purpose (fill, padding).
- Section root: width="fill_container", height="fit_content", layout="vertical".
- FORMS: ALL inputs AND primary button MUST use width="fill_container". Vertical layout, gap=16-20.`,
    always: true,
    priority: 10,
  },

  text: {
    content: `TEXT RULES:
- Body/description in vertical layout: width="fill_container" + textGrowth="fixed-width" (wraps text, auto-sizes height).
- Short labels in horizontal rows: width="fit_content" + textGrowth="auto". Prevents squeezing siblings.
- NEVER fixed pixel width on text inside layout frames — causes overflow.
- Text >15 chars MUST have textGrowth="fixed-width". NEVER set explicit pixel height on text nodes — OMIT height.
- Typography: Display 40-56px, Heading 28-36px, Subheading 20-24px, Body 16-18px, Caption 13-14px.
- lineHeight: headings 1.1-1.2, body 1.4-1.6. letterSpacing: -0.5 for headlines, 0.5-2 for uppercase.`,
    always: true,
    priority: 15,
  },

  overflow: {
    content: `OVERFLOW PREVENTION (CRITICAL):
- Text in vertical layout: width="fill_container" + textGrowth="fixed-width". In horizontal: width="fit_content".
- NEVER set fixed pixel width on text inside layout frames (e.g. width:378 in 195px card → overflows!).
- Fixed-width children must be ≤ parent content area (parent width − padding).
- Badges: short labels only (CJK ≤8 chars / Latin ≤16 chars).`,
    always: true,
    priority: 16,
  },

  icons: {
    content: `ICONS:
- Use "path" nodes, size 16-24px. ONLY use Feather icon names — PascalCase + "Icon" suffix (e.g. "SearchIcon").
- System auto-resolves names to SVG paths. "d" is replaced automatically.
- Available: ${FEATHER_ICON_NAMES}
- NEVER use emoji as icons. Use icon_font nodes for lucide icons.`,
    always: true,
    priority: 20,
  },

  style: {
    content: `VISUAL STYLE POLICY:
- Default to clean light marketing style unless user explicitly asks for dark/cyber/terminal.
DEFAULT LIGHT PALETTE:
- Page Bg: #F8FAFC, Surface: #FFFFFF, Text: #0F172A, Secondary: #475569
- Accent: #2563EB, Accent2: #0EA5E9, Border: #E2E8F0
TYPOGRAPHY SCALE:
- Display: 40-56px — "Space Grotesk"/"Manrope" (700), lineHeight 1.1
- Heading: 28-36px — "Space Grotesk"/"Manrope" (600-700), lineHeight 1.2
- Subheading: 20-24px — "Inter" (600), lineHeight 1.3
- Body: 16-18px — "Inter" (400-500), lineHeight 1.5
- Caption: 13-14px — "Inter" (400), lineHeight 1.4
SHAPES: cornerRadius 8-14. Subtle shadows. Clear hierarchy via spacing and contrast.
LANDING PAGES: hero 80-120px padding, alternate section backgrounds, cards cornerRadius 12-16, centered content ~1040-1160px.`,
    // Loaded when no design-md is present (default style fallback)
    always: true,
    priority: 5,
  },

  'design-md': {
    content: '', // populated dynamically
    flag: 'hasDesignMd',
    priority: 5,
  },

  guidelines: {
    content: `DESIGN GUIDELINES:
- Mobile: 375×812. Web: 1200×800 (single) or 1200×3000-5000 (landing page).
- "mobile"/"移动端" + screen type = ACTUAL 375×812 screen, NOT desktop with phone mockup.
- Buttons: height 44-52px, cornerRadius 8-12, padding [12, 24]. Icon+text: layout="horizontal", gap=8.
- Icon-only buttons: 44×44, justifyContent/alignItems="center", path icon 20-24px.
- Inputs: height 44px, light bg, subtle border, width="fill_container" in forms.
- Cards: cornerRadius 12-16, clipContent: true, subtle shadows.
- CARD ROW ALIGNMENT: sibling cards in horizontal layout ALL use width/height="fill_container".
- Navigation: justifyContent="space_between", 3 groups (logo | links | CTA), padding=[0,80].
- Phone mockup: ONE "frame", width 260-300, height 520-580, cornerRadius 32. NEVER ellipse.
- NEVER use ellipse for decorative shapes. Use frame/rectangle with cornerRadius.
- NEVER use emoji as icons. Use path nodes with Feather icon names.`,
    triggers: [
      'form', 'input', 'login', 'signup', 'sign up', 'register', 'password', 'email',
      '搜索', '表单', '登录', '注册',
      'mobile', 'phone', '手机', '移动端', 'app screen', 'ios', 'android',
      'button', 'card', 'nav', 'navigation', 'mockup',
      '按钮', '卡片', '导航', '模型',
    ],
    priority: 30,
  },

  roles: {
    content: `SEMANTIC ROLES (add "role" to nodes — system fills unset props based on role):
Layout: section, row, column, centered-content, form-group, divider, spacer
Navigation: navbar, nav-links, nav-link
Interactive: button, icon-button, badge, tag, pill, input, form-input, search-bar
Display: card, stat-card, pricing-card, feature-card, image-card
Media: phone-mockup, screenshot-frame, avatar, icon
Typography: heading, subheading, body-text, caption, label
Content: hero, feature-grid, testimonial, cta-section, footer, stats-section
Table: table, table-row, table-header, table-cell
Key defaults: section→padding:[60,80], navbar→height:72/layout:horizontal/space_between, hero→padding:[80,80], button→padding:[12,24]/height:44, card→gap:12/cornerRadius:12/clipContent:true.
Your explicit props ALWAYS override role defaults.`,
    triggers: [
      'landing', 'marketing', 'hero', 'website', '官网', '首页', '产品页',
      'table', 'grid', '表格', '表头', 'dashboard', '数据', 'admin',
      'testimonial', 'pricing', 'footer', 'stats',
      '评价', '定价', '页脚', '数据统计',
    ],
    priority: 35,
  },

  copywriting: {
    content: `COPYWRITING:
- Headlines: 2-6 words. Subtitles: 1 sentence ≤15 words.
- Feature titles: 2-4 words. Descriptions: 1 sentence ≤20 words.
- Buttons: 1-3 words. Card text: ≤2 sentences. Stats: number + 1-3 word label.
- NEVER 3+ sentence paragraphs. Distill to essence. Power words > vague adjectives.`,
    triggers: [
      'landing', 'marketing', 'hero', 'website', '官网', '首页', '产品页',
      'copy', 'text', 'headline', 'content',
      '文案', '标题', '内容',
    ],
    priority: 40,
  },

  cjk: {
    content: `CJK TYPOGRAPHY (Chinese/Japanese/Korean):
- Headings: "Noto Sans SC" (Chinese) / "Noto Sans JP" / "Noto Sans KR". NEVER "Space Grotesk"/"Manrope" for CJK.
- Body: "Inter" (system CJK fallback) or "Noto Sans SC".
- CJK lineHeight: headings 1.3-1.4 (NOT 1.1), body 1.6-1.8. letterSpacing: 0, NEVER negative.
- CJK buttons: each char ≈ fontSize wide. Container width ≥ (charCount × fontSize) + padding.
- Detect CJK from user request language — use CJK fonts for ALL text nodes.`,
    // CJK is triggered by Unicode range detection, not keywords
    triggers: ['/[\\u4e00-\\u9fff\\u3040-\\u309f\\u30a0-\\u30ff\\uac00-\\ud7af]/'],
    priority: 25,
  },

  variables: {
    content: `DESIGN VARIABLES:
- When document has variables, use "$variableName" references instead of hardcoded values.
- Color: [{ "type": "solid", "color": "$primary" }]. Number: "gap": "$spacing-md".
- Only reference listed variables — do NOT invent names.`,
    flag: 'hasVariables',
    priority: 45,
  },

  examples: {
    content: `EXAMPLES:
Button: { "id":"btn-1","type":"frame","role":"button","width":180,"cornerRadius":8,"fill":[{"type":"solid","color":"#3B82F6"}],"children":[{"id":"btn-icon","type":"path","name":"ArrowRightIcon","role":"icon","d":"M5 12h14m-7-7 7 7-7 7","width":20,"height":20,"stroke":{"thickness":2,"fill":[{"type":"solid","color":"#FFF"}]}},{"id":"btn-text","type":"text","role":"label","content":"Continue","fontSize":16,"fontWeight":600,"fill":[{"type":"solid","color":"#FFF"}]}] }
Card: { "id":"card-1","type":"frame","role":"card","width":320,"height":340,"fill":[{"type":"solid","color":"#FFF"}],"effects":[{"type":"shadow","offsetX":0,"offsetY":4,"blur":12,"spread":0,"color":"rgba(0,0,0,0.1)"}],"children":[{"id":"card-img","type":"image","width":"fill_container","height":180},{"id":"card-body","type":"frame","width":"fill_container","height":"fit_content","layout":"vertical","padding":20,"gap":8,"children":[{"id":"card-title","type":"text","role":"heading","content":"Title","fontSize":20,"fontWeight":700,"fill":[{"type":"solid","color":"#111827"}]},{"id":"card-desc","type":"text","role":"body-text","content":"Description","fontSize":14,"fill":[{"type":"solid","color":"#6B7280"}]}]}] }`,
    triggers: [
      'example', 'sample', 'show me', 'how to',
      '示例', '样例', '怎么',
    ],
    priority: 50,
  },
}

// ---------------------------------------------------------------------------
// Backward-compatible flat content map (used by MCP design-prompt.ts)
// ---------------------------------------------------------------------------

export const PROMPT_SECTIONS: Record<PromptSectionKey, string> = Object.fromEntries(
  Object.entries(SECTION_REGISTRY).map(([key, def]) => [key, def.content]),
) as Record<PromptSectionKey, string>

// ---------------------------------------------------------------------------
// Intent detection — derive from registry, no hardcoded regexes
// ---------------------------------------------------------------------------

/** Compile trigger patterns from registry definitions. */
function buildTriggerMatchers(): Array<{ key: PromptSectionKey; test: (msg: string) => boolean }> {
  const matchers: Array<{ key: PromptSectionKey; test: (msg: string) => boolean }> = []

  for (const [key, def] of Object.entries(SECTION_REGISTRY) as [PromptSectionKey, SectionDef][]) {
    if (!def.triggers?.length) continue

    const regexes: RegExp[] = []
    const keywords: string[] = []

    for (const t of def.triggers) {
      if (t.startsWith('/') && t.endsWith('/')) {
        // Regex pattern
        regexes.push(new RegExp(t.slice(1, -1)))
      } else if (t.startsWith('/')) {
        // Regex without trailing slash (e.g. /[unicode]/)
        regexes.push(new RegExp(t.slice(1)))
      } else {
        keywords.push(t.toLowerCase())
      }
    }

    // Build a single test function combining keywords and regexes
    matchers.push({
      key,
      test: (msg: string) => {
        const lower = msg.toLowerCase()
        if (keywords.some((kw) => lower.includes(kw))) return true
        if (regexes.some((re) => re.test(msg))) return true
        return false
      },
    })
  }

  return matchers
}

const triggerMatchers = buildTriggerMatchers()

/** Detect which sections are needed based on user message content. */
export function detectSections(
  userMessage: string,
  options?: {
    hasDesignMd?: boolean
    hasVariables?: boolean
    isModification?: boolean
  },
): PromptSectionKey[] {
  const selected = new Set<PromptSectionKey>()

  // Modification mode: minimal context
  if (options?.isModification) {
    selected.add('schema')
    if (options.hasVariables) selected.add('variables')
    if (options.hasDesignMd) selected.add('design-md')
    return [...selected]
  }

  // Always-on sections
  for (const [key, def] of Object.entries(SECTION_REGISTRY) as [PromptSectionKey, SectionDef][]) {
    if (def.always) {
      // design-md replaces style when present
      if (key === 'style' && options?.hasDesignMd) continue
      selected.add(key)
    }
  }

  // Flag-based sections
  if (options?.hasDesignMd) selected.add('design-md')
  if (options?.hasVariables) selected.add('variables')

  // Trigger-based sections: match keywords/regex against user message
  for (const matcher of triggerMatchers) {
    if (matcher.test(userMessage)) {
      selected.add(matcher.key)
    }
  }

  // Sort by priority
  return [...selected].sort((a, b) =>
    (SECTION_REGISTRY[a].priority ?? 50) - (SECTION_REGISTRY[b].priority ?? 50),
  )
}

/** Assemble selected sections into a single knowledge block string. */
export function assembleSections(
  keys: PromptSectionKey[],
  designMdContent?: string,
): string {
  const parts: string[] = []
  for (const key of keys) {
    if (key === 'design-md' && designMdContent) {
      parts.push(`DESIGN SYSTEM (design.md — follow these rules for visual consistency):\n${designMdContent}`)
    } else {
      const content = SECTION_REGISTRY[key].content
      if (content) parts.push(content)
    }
  }
  return parts.join('\n\n')
}

/** Build a condensed design.md style policy string for AI prompt injection. */
export function buildDesignMdStylePolicy(spec: DesignMdSpec): string {
  const parts: string[] = []

  if (spec.visualTheme) {
    const theme = spec.visualTheme.length > 200
      ? spec.visualTheme.substring(0, 200) + '...'
      : spec.visualTheme
    parts.push(`VISUAL THEME: ${theme}`)
  }

  if (spec.colorPalette?.length) {
    const colors = spec.colorPalette
      .slice(0, 10)
      .map(c => `${c.name} (${c.hex}) — ${c.role}`)
      .join('\n- ')
    parts.push(`COLOR PALETTE:\n- ${colors}`)
  }

  if (spec.typography?.fontFamily) {
    parts.push(`FONT: ${spec.typography.fontFamily}`)
  }
  if (spec.typography?.headings) {
    parts.push(`Headings: ${spec.typography.headings}`)
  }
  if (spec.typography?.body) {
    parts.push(`Body: ${spec.typography.body}`)
  }

  if (spec.componentStyles) {
    const styles = spec.componentStyles.length > 300
      ? spec.componentStyles.substring(0, 300) + '...'
      : spec.componentStyles
    parts.push(`COMPONENT STYLES:\n${styles}`)
  }

  return parts.join('\n\n')
}
