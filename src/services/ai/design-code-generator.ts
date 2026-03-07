/**
 * Design Code Generator (Stage 1 of visual reference pipeline).
 *
 * Generates self-contained HTML/CSS code using the model's strongest design
 * capability. The output is a visual reference that guides PenNode generation.
 * Design principles are included to ensure consistent visual quality.
 */

import type { DesignSystem } from './ai-types'
import type { AIProviderType } from '@/types/agent-settings'
import { generateCompletion } from './ai-service'
import {
  DESIGN_CODE_SYSTEM_PROMPT,
  buildCodeGenUserPrompt,
} from './design-code-prompts'
import { getAllPrinciples } from './design-principles'
import { designSystemToPromptContext } from './design-system-generator'

interface CodeGenOptions {
  width: number
  height: number
  model?: string
  provider?: AIProviderType
}

/**
 * Generate self-contained HTML/CSS code for a design request.
 * The code is production-grade and serves as a visual blueprint.
 */
export async function generateDesignCode(
  prompt: string,
  designSystem: DesignSystem,
  options: CodeGenOptions,
): Promise<string> {
  const principles = getAllPrinciples()

  // Build the system prompt with principles injected
  const systemPrompt = principles
    ? `${DESIGN_CODE_SYSTEM_PROMPT}\n\n${principles}`
    : DESIGN_CODE_SYSTEM_PROMPT

  // Build the user prompt with design system context
  const dsContext = designSystemToPromptContext(designSystem)
  const userPrompt = buildCodeGenUserPrompt(
    prompt,
    dsContext,
    options.width,
    options.height,
  )

  const response = await generateCompletion(
    systemPrompt,
    userPrompt,
    options.model,
    options.provider,
  )

  return extractHtmlFromResponse(response)
}

/**
 * Extract the HTML content from an AI response.
 * Handles responses with code fences, markdown, or bare HTML.
 */
function extractHtmlFromResponse(response: string): string {
  const trimmed = response.trim()

  // Check for code fence wrapped HTML
  const fenceMatch = trimmed.match(/```(?:html)?\s*\n?([\s\S]*?)\n?```/)
  if (fenceMatch) {
    const content = fenceMatch[1].trim()
    if (content.includes('<!DOCTYPE') || content.includes('<html')) {
      return content
    }
  }

  // Check if the response itself starts with HTML
  if (trimmed.startsWith('<!DOCTYPE') || trimmed.startsWith('<html')) {
    return trimmed
  }

  // Try to find HTML document in the response
  const htmlMatch = trimmed.match(/(<!DOCTYPE[\s\S]*<\/html>)/i)
  if (htmlMatch) {
    return htmlMatch[1]
  }

  // Last resort: wrap bare content
  return `<!DOCTYPE html>
<html lang="en">
<head><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><title>Design</title></head>
<body>${trimmed}</body>
</html>`
}

/**
 * Extract a structural summary from HTML for use as sub-agent reference.
 * Produces a concise text description of the HTML structure.
 */
export function extractStructureSummary(html: string): string {
  const lines: string[] = ['DESIGN REFERENCE STRUCTURE:']

  // Extract section-level elements
  const sectionPattern = /<(?:section|header|footer|nav|main|div)\s+[^>]*(?:class|id)="([^"]*)"[^>]*>/gi
  let match: RegExpExecArray | null
  while ((match = sectionPattern.exec(html)) !== null) {
    const classOrId = match[1]
    if (classOrId && !classOrId.includes('__')) {
      lines.push(`- Section: ${classOrId}`)
    }
  }

  // Extract heading content for structure hints
  const headingPattern = /<h([1-6])[^>]*>([\s\S]*?)<\/h\1>/gi
  while ((match = headingPattern.exec(html)) !== null) {
    const level = match[1]
    const content = match[2].replace(/<[^>]+>/g, '').trim().slice(0, 60)
    if (content) {
      lines.push(`- H${level}: "${content}"`)
    }
  }

  // Extract button/CTA text
  const buttonPattern = /<(?:button|a)\s+[^>]*class="[^"]*(?:btn|button|cta)[^"]*"[^>]*>([\s\S]*?)<\/(?:button|a)>/gi
  while ((match = buttonPattern.exec(html)) !== null) {
    const text = match[1].replace(/<[^>]+>/g, '').trim().slice(0, 30)
    if (text) {
      lines.push(`- CTA: "${text}"`)
    }
  }

  // If we couldn't extract structure, provide a generic summary
  if (lines.length <= 1) {
    lines.push('(HTML structure extracted — use as visual layout reference)')
  }

  return lines.join('\n')
}

/**
 * Extract the HTML section relevant to a specific subtask label.
 * Uses heuristic matching on section/div IDs, classes, and heading content.
 */
export function extractHtmlSection(html: string, subtaskLabel: string): string | null {
  const labelLower = subtaskLabel.toLowerCase()

  // Try to find a matching section by common keywords
  const keywords = labelLower
    .replace(/[（(].+[)）]/g, '')
    .split(/[\s,/]+/)
    .filter((w) => w.length > 2)

  if (keywords.length === 0) return null

  // Build a regex to match section containers
  const keywordPattern = keywords.map((k) => k.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')).join('|')
  const sectionRegex = new RegExp(
    `<(?:section|div|header|footer|nav)[^>]*(?:class|id)="[^"]*(?:${keywordPattern})[^"]*"[^>]*>[\\s\\S]*?(?=<(?:section|div|header|footer|nav)[^>]*(?:class|id)="|$)`,
    'i',
  )

  const match = sectionRegex.exec(html)
  if (match) {
    // Truncate to reasonable length for context
    const section = match[0].slice(0, 1500)
    return `HTML reference for "${subtaskLabel}":\n${section}`
  }

  return null
}
