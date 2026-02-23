import type { PenNode } from '@/types/pen'
import type { VariableDefinition, ThemedValue } from '@/types/variables'
import type { AIDesignRequest } from './ai-types'
import { streamChat } from './ai-service'
import { DESIGN_GENERATOR_PROMPT, DESIGN_MODIFIER_PROMPT } from './ai-prompts'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { resetAnimationState } from './design-animation'
import { executeOrchestration } from './orchestrator'
import { DESIGN_STREAM_TIMEOUTS } from './ai-runtime-config'
import { extractJsonFromResponse, extractStreamingNodes } from './design-parser'
import {
  resetGenerationRemapping,
  setGenerationContextHint,
  insertStreamingNode,
  adjustRootFrameHeightToContent,
  applyPostStreamingTreeHeuristics,
} from './design-canvas-ops'

// ---------------------------------------------------------------------------
// Re-exports for backward compatibility — consumers that import from
// './design-generator' continue to work without changes.
// ---------------------------------------------------------------------------

// Re-exports from design-parser
export { extractJsonFromResponse, extractStreamingNodes } from './design-parser'
export type { StreamingNodeResult } from './design-parser'

// Re-exports from design-canvas-ops
export {
  resetGenerationRemapping,
  setGenerationContextHint,
  setGenerationCanvasWidth,
  insertStreamingNode,
  applyNodesToCanvas,
  upsertNodesToCanvas,
  animateNodesToCanvas,
  extractAndApplyDesign,
  extractAndApplyDesignModification,
  adjustRootFrameHeightToContent,
  expandRootFrameHeight,
  applyPostStreamingTreeHeuristics,
  applyGenerationHeuristics,
} from './design-canvas-ops'

// ---------------------------------------------------------------------------
// Context building helpers
// ---------------------------------------------------------------------------

function buildContextMessage(request: AIDesignRequest): string {
  let message = request.prompt

  if (request.context?.canvasSize) {
    const { width, height } = request.context.canvasSize
    message += `\n\nCanvas size: ${width}x${height}px`
  }

  if (request.context?.documentSummary) {
    message += `\n\nCurrent document: ${request.context.documentSummary}`
  }

  // Append variable context so AI can use $variable references
  const varContext = buildVariableContext(request.context?.variables, request.context?.themes)
  if (varContext) {
    message += `\n\n${varContext}`
  }

  // FORCE override to prevent tool usage
  message += `\n\nIMPORTANT: You remain in DIRECT RESPONSE MODE. Do NOT use the "Write" tool or any other function. I cannot see tool outputs. Just write the JSON response directly.`

  return message
}

/** Build a concise summary of document variables for AI context. */
export function buildVariableContext(
  variables?: Record<string, VariableDefinition>,
  themes?: Record<string, string[]>,
): string | null {
  if (!variables || Object.keys(variables).length === 0) return null

  const lines: string[] = ['DOCUMENT VARIABLES (use "$name" to reference, e.g. fill color "$color-1"):']

  for (const [name, def] of Object.entries(variables)) {
    const val = def.value
    if (Array.isArray(val)) {
      // Themed variable -- show default value
      const defaultVal = (val as ThemedValue[])[0]?.value ?? '?'
      lines.push(`  - ${name} (${def.type}): ${defaultVal} [themed]`)
    } else {
      lines.push(`  - ${name} (${def.type}): ${val}`)
    }
  }

  if (themes && Object.keys(themes).length > 0) {
    const themeSummary = Object.entries(themes)
      .map(([axis, values]) => `${axis}: [${values.join(', ')}]`)
      .join('; ')
    lines.push(`Themes: ${themeSummary}`)
  }

  return lines.join('\n')
}

// ---------------------------------------------------------------------------
// Design generation (orchestrated with direct-stream fallback)
// ---------------------------------------------------------------------------

export async function generateDesign(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    /** When true, nodes are inserted with staggered fade-in animation. */
    animated?: boolean
  }
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  setGenerationContextHint(request.prompt)
  try {
    // Always route through orchestrator (fallback to direct generation on failure)
    try {
      return await executeOrchestration(request, callbacks)
    } catch (err) {
      console.error('[Orchestrator] Failed, falling back to direct generation:', err)
    }

    const userMessage = buildContextMessage(request)
    let fullResponse = ''
    let streamingOffset = 0 // Tracks how far we've parsed in the streaming text
    let appliedCount = 0
    let streamError: string | null = null
    let directStreamRootId: string | null = null
    const animated = callbacks?.animated ?? false

    resetGenerationRemapping()

    if (animated) {
      resetAnimationState()
      useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
    }

    let thinkingContent = ''

    try {
      for await (const chunk of streamChat(DESIGN_GENERATOR_PROMPT, [
        { role: 'user', content: userMessage },
      ], undefined, DESIGN_STREAM_TIMEOUTS)) {
        if (chunk.type === 'thinking') {
          thinkingContent += chunk.content
          // Stream actual thinking content to UI in real-time
          const thinkingStep = `<step title="Thinking">${thinkingContent}</step>`
          callbacks?.onTextUpdate?.(thinkingStep)
        } else if (chunk.type === 'text') {
          fullResponse += chunk.content
          // Prepend thinking step so it stays visible after text starts
          const thinkingPrefix = thinkingContent
            ? `<step title="Thinking">${thinkingContent}</step>\n`
            : ''
          callbacks?.onTextUpdate?.(thinkingPrefix + fullResponse)

          if (animated) {
            // Element-by-element streaming: extract completed JSON objects
            // from the JSONL block as they finish generating.
            const { results, newOffset } = extractStreamingNodes(fullResponse, streamingOffset)
            if (results.length > 0) {
              streamingOffset = newOffset
              for (const { node, parentId } of results) {
                insertStreamingNode(node, parentId)
                if (!directStreamRootId && parentId === null) directStreamRootId = node.id
                appliedCount++
              }
              callbacks?.onApplyPartial?.(appliedCount)
            }
          }
        } else if (chunk.type === 'error') {
          streamError = chunk.content
          break
        }
      }
    } finally {
      if (animated) {
        // Apply tree-aware heuristics after streaming is done
        if (directStreamRootId) {
          applyPostStreamingTreeHeuristics(directStreamRootId)
        }
        if (appliedCount > 0) {
          adjustRootFrameHeightToContent()
        }
        useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
      }
    }

    if (!animated && appliedCount > 0) {
      adjustRootFrameHeightToContent()
    }

    // Build final tree from response for return value
    const streamedNodes = extractJsonFromResponse(fullResponse)
    if (streamedNodes && streamedNodes.length > 0) {
      return { nodes: streamedNodes, rawResponse: fullResponse }
    }

    if (streamError) {
      throw new Error(streamError)
    }

    return { nodes: [], rawResponse: fullResponse }
  } finally {
    setGenerationContextHint('')
  }
}

// ---------------------------------------------------------------------------
// Design modification (selected nodes + instruction)
// ---------------------------------------------------------------------------

export async function generateDesignModification(
  nodesToModify: PenNode[],
  instruction: string,
  options?: {
    variables?: Record<string, VariableDefinition>
    themes?: Record<string, string[]>
  },
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  // Build context from selected nodes
  const contextJson = JSON.stringify(nodesToModify, (_key, value) => {
    // omit children to avoid massive context if deep tree
    return value
  })

  // We use standard string concatenation to avoid backtick issues in tool calls
  let userMessage = "CONTEXT NODES:\n" + contextJson + "\n\nINSTRUCTION:\n" + instruction

  // Append variable context so AI can use $variable references
  const varContext = buildVariableContext(options?.variables, options?.themes)
  if (varContext) {
    userMessage += "\n\n" + varContext
  }
  let fullResponse = ''
  let streamError: string | null = null

  for await (const chunk of streamChat(DESIGN_MODIFIER_PROMPT, [
    { role: 'user', content: userMessage },
  ], undefined, DESIGN_STREAM_TIMEOUTS)) {
    if (chunk.type === 'thinking') {
      // Ignore thinking chunks for modification -- caller already shows progress
    } else if (chunk.type === 'text') {
      fullResponse += chunk.content
    } else if (chunk.type === 'error') {
      streamError = chunk.content
      break
    }
  }

  const streamedNodes = extractJsonFromResponse(fullResponse)
  if (streamedNodes && streamedNodes.length > 0) {
    return { nodes: streamedNodes, rawResponse: fullResponse }
  }

  if (streamError) {
    throw new Error(streamError)
  }

  throw new Error('Failed to parse modified nodes from AI response')
}
