import type { OrchestratorPlan } from './ai-types'
import {
  ORCHESTRATOR_TIMEOUT_PROFILES,
  PROMPT_TIMEOUT_BUCKETS,
  PROMPT_OPTIMIZER_LIMITS,
  SUB_AGENT_TIMEOUT_PROFILES,
} from './ai-runtime-config'

export interface PreparedDesignPrompt {
  original: string
  orchestratorPrompt: string
  subAgentPrompt: string
  wasCompressed: boolean
  originalLength: number
}

export function getSubAgentTimeouts(promptLength: number): {
  hardTimeoutMs: number
  noTextTimeoutMs: number
  thinkingResetsTimeout: boolean
  pingResetsTimeout: boolean
  firstTextTimeoutMs: number
  thinkingMode: 'adaptive' | 'disabled' | 'enabled'
  effort: 'low' | 'medium' | 'high' | 'max'
} {
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    return { ...SUB_AGENT_TIMEOUT_PROFILES.short }
  }
  if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    return { ...SUB_AGENT_TIMEOUT_PROFILES.medium }
  }
  return { ...SUB_AGENT_TIMEOUT_PROFILES.long }
}

export function getOrchestratorTimeouts(promptLength: number): {
  hardTimeoutMs: number
  noTextTimeoutMs: number
  thinkingResetsTimeout: boolean
  pingResetsTimeout: boolean
  firstTextTimeoutMs: number
  thinkingMode: 'adaptive' | 'disabled' | 'enabled'
  effort: 'low' | 'medium' | 'high' | 'max'
} {
  if (promptLength < PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    return { ...ORCHESTRATOR_TIMEOUT_PROFILES.short }
  }
  if (promptLength < PROMPT_TIMEOUT_BUCKETS.mediumPromptMaxChars) {
    return { ...ORCHESTRATOR_TIMEOUT_PROFILES.medium }
  }
  return { ...ORCHESTRATOR_TIMEOUT_PROFILES.long }
}

export function prepareDesignPrompt(prompt: string): PreparedDesignPrompt {
  const normalized = normalizePromptText(prompt)
  if (normalized.length <= PROMPT_OPTIMIZER_LIMITS.longPromptCharThreshold) {
    return {
      original: prompt,
      orchestratorPrompt: normalized,
      subAgentPrompt: normalized,
      wasCompressed: false,
      originalLength: normalized.length,
    }
  }

  const sections = parseTopLevelSections(prompt)
  const overview = extractOverviewLine(normalized)
  const featureLines = extractFeatureHeadings(prompt).slice(0, PROMPT_OPTIMIZER_LIMITS.maxFeatureLines)
  const sectionLines = extractWebsiteSectionLines(sections).slice(0, PROMPT_OPTIMIZER_LIMITS.maxSectionLines)
  const highlightLines = extractCoreHighlightLines(sections).slice(0, 6)
  const needsScreenshotPlaceholder = hasScreenshotRequirements(normalized)

  const sloganLines = extractSloganCandidates(sections).slice(0, 4)
  const personalityHint = extractProductPersonality(normalized)

  const conciseParts: string[] = [
    'Design brief (auto-condensed from a long product document):',
  ]

  if (overview) {
    conciseParts.push(`Product: ${overview}`)
  }
  if (personalityHint) {
    conciseParts.push(`Product personality: ${personalityHint}`)
  }
  if (sloganLines.length > 0) {
    conciseParts.push(`Slogan candidates:\n${sloganLines.join('\n')}`)
  }
  if (sectionLines.length > 0) {
    conciseParts.push(`Preferred landing page sections:\n${sectionLines.join('\n')}`)
  }
  if (featureLines.length > 0) {
    conciseParts.push(`Core capabilities to reflect visually:\n${featureLines.join('\n')}`)
  }
  if (highlightLines.length > 0) {
    conciseParts.push(`Key product highlights:\n${highlightLines.join('\n')}`)
  }
  if (needsScreenshotPlaceholder) {
    conciseParts.push(
      'Screenshot rule: for sections like "App截图"/"XX截图"/"screenshot", use a minimal phone placeholder (outline + short label), avoid random real screenshots, and do not recreate detailed mini-app internals.',
    )
  }
  conciseParts.push(
    'Icon rule: never use emoji glyphs as icons; always use path nodes with descriptive icon names (e.g. "SearchIcon", "MenuIcon"). System auto-resolves to verified SVG paths.',
  )

  conciseParts.push(
    'Scope guardrails: clear hierarchy, avoid over-detailed micro-content.',
  )
  conciseParts.push(
    'Layout guardrails: keep a stable centered content width.',
  )

  // Only add landing-page-specific guardrails if the prompt looks like a landing page
  const isLandingPage = /(?:landing\s*page|website|官网|首页|marketing)/i.test(normalized)
  if (isLandingPage) {
    conciseParts.push(
      'CTA guardrails: avoid inserting extra full-width CTA stripes unless explicitly requested.',
    )
    conciseParts.push(
      'Navbar guardrails: keep logo/links/CTA horizontally aligned; links should be evenly distributed in the center group.',
    )
  }

  conciseParts.push(
    'Typography guardrails: long subtitle/body text should use constrained width so lines wrap naturally.',
  )
  conciseParts.push(
    'Overflow prevention: ALL text inside layout frames must use width="fill_container" (never fixed pixel widths). Buttons/badges with CJK text must be wide enough for character count × fontSize + padding.',
  )

  const concise = conciseParts.join('\n\n')

  return {
    original: prompt,
    orchestratorPrompt: truncateByCharCount(concise, PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForOrchestrator),
    subAgentPrompt: truncateByCharCount(concise, PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForSubAgent),
    wasCompressed: true,
    originalLength: normalized.length,
  }
}

export function buildFallbackPlanFromPrompt(prompt: string): OrchestratorPlan {
  const labels = extractFallbackSectionLabels(prompt)
  const sectionCount = Math.max(1, labels.length)

  const isMobile = /(?:mobile|手机|phone|app\s*screen|登录|注册|login|signup)/i.test(prompt)
  const isAppScreen = /(?:login|signup|register|登录|注册|settings|设置|profile|个人|form|表单|dashboard|modal|dialog)/i.test(prompt)

  const width = isMobile ? 375 : 1200
  // Mobile app screens: fixed 812px viewport. Desktop landing pages: auto-expand. Desktop app screens: fixed height.
  const totalHeight = isMobile
    ? 812
    : isAppScreen
      ? 800
      : sectionCount >= 4 ? 4000 : 800
  const heights = allocateSectionHeights(totalHeight, sectionCount)

  return {
    rootFrame: {
      id: 'page',
      name: 'Page',
      width,
      height: isMobile ? 812 : (isAppScreen ? totalHeight : 0),
      layout: 'vertical',
      fill: [{ type: 'solid', color: '#F8FAFC' }],
    },
    subtasks: labels.map((label, index) => ({
      id: makeSafeSectionId(label, index),
      label,
      region: { width, height: heights[index] ?? 120 },
      idPrefix: '',
      parentFrameId: null,
    })),
  }
}

function normalizePromptText(text: string): string {
  return text
    .replace(/\r/g, '')
    .replace(/[ \t]+\n/g, '\n')
    .replace(/\n{3,}/g, '\n\n')
    .trim()
}

function truncateByCharCount(text: string, maxChars: number): string {
  if (text.length <= maxChars) return text
  const truncated = text.slice(0, maxChars)
  const lastBoundary = Math.max(
    truncated.lastIndexOf('\n'),
    truncated.lastIndexOf('。'),
    truncated.lastIndexOf('.'),
  )
  if (lastBoundary > Math.floor(maxChars * 0.7)) {
    return `${truncated.slice(0, lastBoundary).trim()}\n\n[truncated]`
  }
  return `${truncated.trim()}\n\n[truncated]`
}

function parseTopLevelSections(markdown: string): Map<string, string> {
  const lines = markdown.replace(/\r/g, '').split('\n')
  const sections = new Map<string, string>()
  let currentTitle: string | null = null
  let currentLines: string[] = []

  const flush = () => {
    if (!currentTitle) return
    const content = currentLines.join('\n').trim()
    if (content) sections.set(currentTitle, content)
  }

  for (const line of lines) {
    const headingMatch = line.match(/^##\s+(.+)$/)
    if (headingMatch) {
      flush()
      currentTitle = headingMatch[1].trim()
      currentLines = []
      continue
    }
    if (currentTitle) {
      currentLines.push(line)
    }
  }
  flush()

  return sections
}

function extractOverviewLine(text: string): string | null {
  const lines = text.split('\n')
  for (const raw of lines) {
    const line = raw.trim()
    if (!line) continue
    if (line.startsWith('#')) continue
    if (line.startsWith('|')) continue
    if (line === '---') continue
    return line.replace(/\*\*/g, '')
  }
  return null
}

function extractFeatureHeadings(text: string): string[] {
  return text
    .split('\n')
    .map((line) => line.match(/^###\s+\d+\.\s+(.+)$/)?.[1]?.trim())
    .filter((line): line is string => Boolean(line))
    .map((line) => `- ${line}`)
}

function extractWebsiteSectionLines(sections: Map<string, string>): string[] {
  const content = sections.get('推荐的官网结构')
  if (!content) return []

  return content
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^\d+\./.test(line))
    .map((line) => `- ${line.replace(/^\d+\.\s*/, '')}`)
}

function extractCoreHighlightLines(sections: Map<string, string>): string[] {
  const content = sections.get('核心亮点')
  if (!content) return []

  const lines = content
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.startsWith('|') && !line.includes('|------|'))

  const result: string[] = []
  for (const line of lines) {
    const columns = line.split('|').map((cell) => cell.trim()).filter(Boolean)
    if (columns.length >= 2) {
      result.push(`- ${columns[0]}: ${columns[1]}`)
    }
  }
  return result
}

function hasScreenshotRequirements(text: string): boolean {
  return /(截图|screenshot|mockup)/i.test(text)
}

function extractFallbackSectionLabels(prompt: string): string[] {
  const lines = prompt.replace(/\r/g, '').split('\n')
  const labels: string[] = []
  const seen = new Set<string>()

  for (const raw of lines) {
    const line = raw.trim()
    const bulletMatch = line.match(/^- (.+)$/)
    if (!bulletMatch) continue
    const cleaned = sanitizePlanSectionLabel(bulletMatch[1])
    if (!cleaned) continue
    const key = cleaned.toLowerCase()
    if (seen.has(key)) continue
    seen.add(key)
    labels.push(cleaned)
    if (labels.length >= PROMPT_OPTIMIZER_LIMITS.maxFallbackSections) break
  }

  if (labels.length > 0) return labels

  // Detect design type to provide appropriate fallback labels
  const isAppScreen = /(?:login|signup|register|登录|注册|settings|设置|profile|个人|form|表单|dashboard|modal|dialog)/i.test(prompt)
  if (isAppScreen) {
    return [
      'Header',
      'Main Content',
      'Actions',
    ]
  }

  return [
    'Navigation',
    'Hero',
    'Core Highlights',
    'Feature Showcase',
    'CTA',
    'Footer',
  ]
}

function sanitizePlanSectionLabel(label: string): string {
  const cleaned = label
    .replace(/^["'`]+|["'`]+$/g, '')
    .replace(/\s*\([^)]*\)\s*/g, ' ')
    .replace(/[_*#]/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
  if (!cleaned) return ''
  return cleaned.slice(0, 48)
}

function makeSafeSectionId(label: string, index: number): string {
  const ascii = label
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
  if (ascii.length > 0) return ascii
  return `section-${index + 1}`
}

function allocateSectionHeights(totalHeight: number, count: number): number[] {
  if (count <= 0) return []

  const minHeight = 80
  const base = Math.max(minHeight, Math.floor(totalHeight / count))
  const heights = Array.from({ length: count }, () => base)
  let allocated = base * count

  let idx = 0
  while (allocated < totalHeight) {
    heights[idx] += 1
    allocated += 1
    idx = (idx + 1) % count
  }

  idx = count - 1
  while (allocated > totalHeight) {
    if (heights[idx] > minHeight) {
      heights[idx] -= 1
      allocated -= 1
    }
    idx = idx - 1
    if (idx < 0) idx = count - 1
  }

  return heights
}

function extractSloganCandidates(sections: Map<string, string>): string[] {
  const content = sections.get('Slogan 候选') ?? sections.get('slogan') ?? sections.get('标语')
  if (!content) return []

  return content
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => /^\d+\./.test(line) || line.startsWith('-') || line.startsWith('"'))
    .map((line) => `- ${line.replace(/^\d+\.\s*/, '').replace(/^-\s*/, '')}`)
}

function extractProductPersonality(text: string): string | null {
  // Look for personality/tone hints in the document
  const patterns = [
    /(?:风格|tone|style|personality)[：:]\s*(.+)/i,
    /(?:品牌调性|brand\s*tone)[：:]\s*(.+)/i,
    /(?:设计风格|design\s*style)[：:]\s*(.+)/i,
  ]
  for (const pattern of patterns) {
    const match = text.match(pattern)
    if (match) return match[1].trim().slice(0, 100)
  }

  // Infer from keywords in the text
  const hasKids = /(?:kids|children|儿童|少儿|幼儿)/i.test(text)
  const hasTech = /(?:AI|developer|API|tech|code|engineering)/i.test(text)
  const hasLuxury = /(?:premium|luxury|奢华|高端)/i.test(text)
  const hasFun = /(?:game|fun|play|趣味|游戏|娱乐)/i.test(text)
  const hasEducation = /(?:learn|study|education|学习|教育|vocabulary)/i.test(text)

  const traits: string[] = []
  if (hasKids) traits.push('playful, colorful')
  if (hasTech) traits.push('modern, technical')
  if (hasLuxury) traits.push('premium, elegant')
  if (hasFun) traits.push('energetic, vibrant')
  if (hasEducation) traits.push('trustworthy, encouraging')

  return traits.length > 0 ? traits.join(', ') : null
}
