/**
 * Design principles selector — loads only relevant principles based on request context.
 *
 * Each principle module contains domain-specific design knowledge extracted from
 * professional design practices. The selector uses keyword matching to avoid
 * loading all principles into a single context window.
 */

import { TYPOGRAPHY_PRINCIPLES } from './typography'
import { COLOR_PRINCIPLES } from './color'
import { SPACING_PRINCIPLES } from './spacing'
import { COMPOSITION_PRINCIPLES } from './composition'
import { COMPONENT_PRINCIPLES } from './components'

interface PrincipleEntry {
  id: string
  content: string
  keywords: RegExp
}

const PRINCIPLE_REGISTRY: PrincipleEntry[] = [
  {
    id: 'composition',
    content: COMPOSITION_PRINCIPLES,
    // Composition is relevant for any multi-section design
    keywords: /hero|landing|page|website|官网|首页|dashboard|section|screen|界面|页面|design|设计/i,
  },
  {
    id: 'typography',
    content: TYPOGRAPHY_PRINCIPLES,
    keywords: /font|text|headline|title|typography|heading|body|字体|文字|标题|排版/i,
  },
  {
    id: 'color',
    content: COLOR_PRINCIPLES,
    keywords: /color|palette|theme|dark|light|brand|gradient|颜色|配色|主题|深色|浅色/i,
  },
  {
    id: 'spacing',
    content: SPACING_PRINCIPLES,
    keywords: /layout|spacing|grid|whitespace|padding|gap|布局|间距|留白|网格/i,
  },
  {
    id: 'components',
    content: COMPONENT_PRINCIPLES,
    keywords: /card|button|form|input|nav|dashboard|pricing|login|signup|卡片|按钮|表单|导航|登录|注册/i,
  },
]

/**
 * Select relevant design principles based on the user's prompt.
 * Returns a combined string of applicable principles, or empty string if none match.
 *
 * For comprehensive designs (landing pages, full websites), all principles are included.
 * For focused designs (login form, single card), only relevant principles are selected.
 */
export function selectPrinciples(prompt: string): string {
  const lower = prompt.toLowerCase()

  // Full designs get all principles
  const isComprehensive = /landing\s*page|website|官网|首页|full.*(page|site)|完整.*页|web\s*app|webapp/i.test(lower)

  if (isComprehensive) {
    return PRINCIPLE_REGISTRY.map((p) => p.content).join('\n\n')
  }

  // Otherwise, select based on keyword matching
  const matched = new Set<string>()
  const selected: string[] = []

  for (const entry of PRINCIPLE_REGISTRY) {
    if (entry.keywords.test(lower) && !matched.has(entry.id)) {
      matched.add(entry.id)
      selected.push(entry.content)
    }
  }

  // Always include at least composition for any design request
  if (selected.length === 0) {
    selected.push(COMPOSITION_PRINCIPLES)
  }

  return selected.join('\n\n')
}

/**
 * Get all principles combined (for use in code generation prompts
 * where the full design context is always relevant).
 */
export function getAllPrinciples(): string {
  return PRINCIPLE_REGISTRY.map((p) => p.content).join('\n\n')
}

// Re-export individual principles for direct access
export { TYPOGRAPHY_PRINCIPLES } from './typography'
export { COLOR_PRINCIPLES } from './color'
export { SPACING_PRINCIPLES } from './spacing'
export { COMPOSITION_PRINCIPLES } from './composition'
export { COMPONENT_PRINCIPLES } from './components'
