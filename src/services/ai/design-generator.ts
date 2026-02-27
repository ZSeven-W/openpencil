import type { PenNode } from '@/types/pen'
import type { VariableDefinition, ThemedValue } from '@/types/variables'
import type { AIDesignRequest } from './ai-types'
import { streamChat } from './ai-service'
import { DESIGN_MODIFIER_PROMPT } from './ai-prompts'
import { executeOrchestration } from './orchestrator'
import { DESIGN_STREAM_TIMEOUTS } from './ai-runtime-config'
import { extractJsonFromResponse } from './design-parser'

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
// Design generation (orchestrated)
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
  return executeOrchestration(request, callbacks)
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
