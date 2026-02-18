import type { PenNode } from '@/types/pen'
import type { AIDesignRequest } from './ai-types'
import { streamChat } from './ai-service'
import { DESIGN_GENERATOR_PROMPT } from './ai-prompts'
import { useDocumentStore } from '@/stores/document-store'

function extractJsonFromResponse(text: string): PenNode[] | null {
  // Try to extract JSON from markdown code blocks
  const jsonBlockMatch = text.match(/```json\s*\n?([\s\S]*?)\n?\s*```/)
  const rawJson = jsonBlockMatch ? jsonBlockMatch[1] : text

  try {
    const parsed = JSON.parse(rawJson.trim())
    const nodes: PenNode[] = Array.isArray(parsed) ? parsed : [parsed]
    return validateNodes(nodes) ? nodes : null
  } catch {
    return null
  }
}

function validateNodes(nodes: unknown[]): nodes is PenNode[] {
  return nodes.every(
    (node) =>
      typeof node === 'object' &&
      node !== null &&
      'id' in node &&
      'type' in node &&
      typeof (node as PenNode).id === 'string' &&
      typeof (node as PenNode).type === 'string',
  )
}

function buildContextMessage(request: AIDesignRequest): string {
  let message = request.prompt

  if (request.context?.canvasSize) {
    const { width, height } = request.context.canvasSize
    message += `\n\nCanvas size: ${width}x${height}px`
  }

  if (request.context?.documentSummary) {
    message += `\n\nCurrent document: ${request.context.documentSummary}`
  }

  return message
}

export async function generateDesign(
  request: AIDesignRequest,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  const userMessage = buildContextMessage(request)
  let fullResponse = ''

  for await (const chunk of streamChat(DESIGN_GENERATOR_PROMPT, [
    { role: 'user', content: userMessage },
  ])) {
    if (chunk.type === 'text') {
      fullResponse += chunk.content
    } else if (chunk.type === 'error') {
      throw new Error(chunk.content)
    }
  }

  const nodes = extractJsonFromResponse(fullResponse)
  if (!nodes || nodes.length === 0) {
    throw new Error('Failed to parse design nodes from AI response')
  }

  return { nodes, rawResponse: fullResponse }
}

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { addNode } = useDocumentStore.getState()
  for (const node of nodes) {
    addNode(null, node)
  }
}

/**
 * Extract PenNode JSON from AI response text and apply to canvas.
 * Returns the number of top-level elements added (0 if nothing found/applied).
 */
export function extractAndApplyDesign(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText)
  if (!nodes || nodes.length === 0) return 0

  applyNodesToCanvas(nodes)
  return nodes.length
}
