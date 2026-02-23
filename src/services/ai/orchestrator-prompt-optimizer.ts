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

/**
 * Prepare a user prompt for the orchestrator and sub-agents.
 * Simply normalizes whitespace and truncates if too long.
 * No lossy "intelligent" extraction — the user's original intent is preserved.
 */
export function prepareDesignPrompt(prompt: string): PreparedDesignPrompt {
  const normalized = normalizePromptText(prompt)

  return {
    original: prompt,
    orchestratorPrompt: truncateByCharCount(normalized, PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForOrchestrator),
    subAgentPrompt: truncateByCharCount(normalized, PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForSubAgent),
    wasCompressed: normalized.length > PROMPT_OPTIMIZER_LIMITS.maxPromptCharsForOrchestrator,
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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

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

function extractFallbackSectionLabels(prompt: string): string[] {
  const lines = prompt.replace(/\r/g, '').split('\n')
  const labels: string[] = []
  const seen = new Set<string>()

  // Try bullet points first
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

  // Try ## headings (h2) as section labels
  for (const raw of lines) {
    const line = raw.trim()
    const headingMatch = line.match(/^##\s+(.+)$/)
    if (!headingMatch) continue
    const cleaned = sanitizePlanSectionLabel(headingMatch[1])
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
  if (count === 1) return [totalHeight]

  const minHeight = 80
  // Weighted allocation: first section (hero/header) gets 1.4×, last (footer) gets 0.6×, rest even
  const weights = Array.from({ length: count }, (_, i) => {
    if (i === 0) return 1.4 // hero/header
    if (i === count - 1 && count >= 3) return 0.6 // footer
    return 1.0
  })
  const totalWeight = weights.reduce((sum, w) => sum + w, 0)
  const heights = weights.map((w) => Math.max(minHeight, Math.round((totalHeight * w) / totalWeight)))

  // Adjust to match total exactly
  let allocated = heights.reduce((sum, h) => sum + h, 0)
  let idx = Math.floor(count / 2) // adjust middle sections first
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
