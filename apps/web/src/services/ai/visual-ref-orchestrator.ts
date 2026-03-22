/**
 * Visual Reference Orchestrator — full A+B+C pipeline.
 *
 * Orchestrates the visual reference pipeline:
 * Stage 0: Generate design system tokens (Phase B)
 * Stage 1: Generate HTML/CSS code with skill-enhanced prompts (Phase A+C)
 * Stage 2: Render HTML to screenshot (Phase C)
 * Stage 3: Run PenNode generation with visual reference context
 * Stage 4: Validate against reference screenshot
 *
 * The key insight: separating "what looks good" (Stages 0-2) from
 * "how to encode it" (Stage 3) lets each LLM call focus on what it's best at.
 */

import type { PenNode } from '@/types/pen'
import type {
  AIDesignRequest,
  DesignSystem,
  VisualReference,
} from './ai-types'
import { generateDesignSystem, designSystemToVariables, designSystemToPromptContext } from './design-system-generator'
import { generateDesignCode, extractStructureSummary, extractHtmlSection } from './design-code-generator'
import { renderHtmlToScreenshot } from './html-renderer'
import { executeOrchestration } from './orchestrator'
import { useDocumentStore } from '@/stores/document-store'

// ---------------------------------------------------------------------------
// Module state — reference data for the current generation
// ---------------------------------------------------------------------------

let currentReference: VisualReference | null = null

export function getCurrentVisualReference(): VisualReference | null {
  return currentReference
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function executeVisualRefOrchestration(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
  abortSignal?: AbortSignal,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  currentReference = null

  const emitStatus = (title: string, detail: string) => {
    callbacks?.onTextUpdate?.(
      `<step title="${title}" status="streaming">${detail}</step>`,
    )
  }

  try {
    // -- Stage 0: Generate Design System --
    emitStatus('Crafting design system', 'Selecting colors, typography, and spacing...')

    if (abortSignal?.aborted) throw new Error('Aborted')

    let designSystem: DesignSystem
    try {
      designSystem = await generateDesignSystem(
        request.prompt,
        request.model,
        request.provider,
      )
    } catch (err) {
      console.warn('[VisualRef] Design system generation failed, using defaults:', err)
      designSystem = getDefaultDesignSystem()
    }

    // Write design tokens to document variables
    const variables = designSystemToVariables(designSystem)
    const store = useDocumentStore.getState()
    for (const [name, def] of Object.entries(variables)) {
      store.setVariable(name, def)
    }

    emitStatus('Crafting design system', `Style: ${designSystem.aesthetic}`)

    if (abortSignal?.aborted) throw new Error('Aborted')

    // -- Stage 1: Generate HTML/CSS Code --
    emitStatus('Generating design reference', 'Creating high-fidelity HTML blueprint...')

    // Determine viewport from request context or defaults
    const width = request.context?.canvasSize?.width ?? 1200
    const height = request.context?.canvasSize?.height ?? 0

    let html: string
    try {
      html = await generateDesignCode(request.prompt, designSystem, {
        width,
        height,
        model: request.model,
        provider: request.provider,
      })
    } catch (err) {
      console.warn('[VisualRef] Code generation failed, falling back to direct pipeline:', err)
      return executeOrchestration(request, callbacks, abortSignal)
    }

    emitStatus('Generating design reference', 'HTML blueprint ready')

    if (abortSignal?.aborted) throw new Error('Aborted')

    // -- Stage 2: Render to Screenshot --
    emitStatus('Rendering reference', 'Capturing visual reference...')

    let screenshot: string
    try {
      screenshot = await renderHtmlToScreenshot(html, width, height)
    } catch (err) {
      console.warn('[VisualRef] Screenshot rendering failed, continuing without visual reference:', err)
      // Continue without screenshot — sub-agents still get the HTML structure
      screenshot = ''
    }

    if (screenshot) {
      emitStatus('Rendering reference', 'Visual reference captured')
    }

    // Build structure summary for orchestrator context
    const structureSummary = extractStructureSummary(html)

    // Store the reference for validation phase
    currentReference = {
      html,
      screenshot,
      designSystem,
      structureSummary,
    }

    if (abortSignal?.aborted) throw new Error('Aborted')

    // -- Stage 3: PenNode Generation with Reference --
    // Enhance the request prompt with design reference context
    const dsContext = designSystemToPromptContext(designSystem)
    const enhancedPrompt = buildEnhancedPrompt(request.prompt, structureSummary, dsContext)

    const enhancedRequest: AIDesignRequest = {
      ...request,
      prompt: enhancedPrompt,
      // Pass through existing context, adding our variables
      context: {
        ...request.context,
        variables: {
          ...request.context?.variables,
          ...variables,
        },
      },
    }

    // Run the existing orchestration pipeline with enhanced context
    const result = await executeOrchestration(enhancedRequest, callbacks, abortSignal)

    return result
  } finally {
    // Don't clear currentReference here — validation needs it
  }
}

// ---------------------------------------------------------------------------
// Prompt enhancement
// ---------------------------------------------------------------------------

function buildEnhancedPrompt(
  originalPrompt: string,
  structureSummary: string,
  designSystemContext: string,
): string {
  return `${originalPrompt}

${structureSummary}

${designSystemContext}

IMPORTANT: Follow the design reference structure closely. The design system colors, fonts, and spacing have already been determined — use them consistently. The reference structure shows the intended layout — match its section order and composition.`
}

// ---------------------------------------------------------------------------
// HTML section extraction for sub-agents
// ---------------------------------------------------------------------------

/**
 * Enrich subtasks with HTML reference snippets from the current visual reference.
 * Called after orchestrator planning to give each sub-agent structural context.
 */
export function enrichSubtasksWithHtmlReference(
  subtasks: Array<{ id: string; label: string; htmlReference?: string }>,
): void {
  if (!currentReference) return

  for (const subtask of subtasks) {
    const section = extractHtmlSection(currentReference.html, subtask.label)
    if (section) {
      subtask.htmlReference = section
    }
  }
}

// ---------------------------------------------------------------------------
// Cleanup
// ---------------------------------------------------------------------------

export function clearVisualReference(): void {
  currentReference = null
}

// ---------------------------------------------------------------------------
// Fallback design system
// ---------------------------------------------------------------------------

function getDefaultDesignSystem(): DesignSystem {
  return {
    palette: {
      background: '#F8FAFC',
      surface: '#FFFFFF',
      text: '#0F172A',
      textSecondary: '#475569',
      primary: '#2563EB',
      primaryLight: '#DBEAFE',
      accent: '#0EA5E9',
      border: '#E2E8F0',
    },
    typography: {
      headingFont: 'Space Grotesk',
      bodyFont: 'Inter',
      scale: [14, 16, 20, 28, 40, 56],
    },
    spacing: {
      unit: 8,
      scale: [8, 16, 24, 32, 48, 64],
    },
    radius: [8, 12, 16],
    aesthetic: 'clean modern',
  }
}
