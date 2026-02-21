/**
 * Heuristic complexity classifier for design prompts.
 * Determines whether a prompt should be routed through
 * the orchestrator (parallel sub-agents) or handled directly.
 *
 * No API calls — runs in <1ms. False positives are cheap
 * (one extra planning call), false negatives are safe
 * (fall back to existing single-call path).
 */

// Structural keywords indicating distinct spatial sections
const SECTION_KEYWORDS = [
  'sidebar', 'header', 'footer', 'nav', 'navigation',
  'hero', 'stats', 'statistics', 'chart', 'table',
  'form', 'modal', 'dialog', 'card section', 'cards section',
  'profile', 'settings', 'feed', 'content area',
  'grid', 'gallery', 'carousel', 'tabs',
  'search bar', 'filter', 'toolbar', 'breadcrumb',
  'notification', 'calendar', 'timeline',
  // Chinese equivalents
  '侧边栏', '头部', '底部', '导航', '英雄区',
  '统计', '图表', '表格', '表单', '弹窗',
  '卡片区', '个人资料', '设置', '时间线',
]

// Full-page keywords that almost always produce complex output
const COMPLEX_PAGE_KEYWORDS = [
  'dashboard', 'landing page', 'homepage', 'e-commerce',
  'admin panel', 'social media', 'email client',
  'settings page', 'analytics', 'crm', 'erp',
  'portfolio', 'blog', 'marketplace', 'checkout',
  // Chinese
  '仪表盘', '着陆页', '首页', '电商', '管理后台',
  '社交', '邮件', '分析页',
]

// Simple single-component keywords (suppress orchestration)
const SIMPLE_KEYWORDS = [
  'button', 'input', 'avatar', 'badge', 'tooltip',
  'toggle', 'switch', 'checkbox', 'radio',
  'tag', 'chip', 'divider', 'spinner', 'icon',
  '按钮', '输入框', '头像', '标签', '开关',
]

/** Minimum sections required to trigger orchestrator */
const COMPLEXITY_THRESHOLD = 3

export interface ComplexityAssessment {
  isComplex: boolean
  estimatedSections: number
  reason: string
}

export function assessComplexity(prompt: string): ComplexityAssessment {
  const lower = prompt.toLowerCase()

  // Count distinct section keywords mentioned
  const mentionedSections = SECTION_KEYWORDS.filter((kw) => lower.includes(kw))
  const isFullPage = COMPLEX_PAGE_KEYWORDS.some((kw) => lower.includes(kw))
  const isSimpleComponent =
    SIMPLE_KEYWORDS.some((kw) => lower.includes(kw)) && mentionedSections.length <= 1

  // Count conjunctions as section indicators
  const conjunctions = (lower.match(/\band\b/g) || []).length
  + (lower.match(/[,，、]/g) || []).length

  const estimatedSections = isFullPage
    ? Math.max(mentionedSections.length, 4)
    : mentionedSections.length + Math.min(Math.floor(conjunctions / 2), 2)

  if (isSimpleComponent) {
    return {
      isComplex: false,
      estimatedSections: 1,
      reason: 'Simple single component request',
    }
  }

  const isComplex =
    estimatedSections >= COMPLEXITY_THRESHOLD ||
    (isFullPage && estimatedSections >= 2)

  return {
    isComplex,
    estimatedSections,
    reason: isComplex
      ? `Detected ${mentionedSections.length} sections (${mentionedSections.join(', ')})`
      : `Below threshold: ${estimatedSections} sections`,
  }
}
