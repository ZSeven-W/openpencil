import type { PenNode } from '@/types/pen'
import type { AIDesignRequest } from './ai-types'
import { streamChat } from './ai-service'
import { DESIGN_GENERATOR_PROMPT, DESIGN_MODIFIER_PROMPT } from './ai-prompts'
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store'

const JSON_BLOCK_REGEX = /```(?:json)?\s*\n?([\s\S]*?)\n?\s*```/

function extractJsonFromResponse(text: string): PenNode[] | null {
  // Try to extract JSON from any code block
  let jsonBlockMatch = text.match(JSON_BLOCK_REGEX)
  let rawJson = jsonBlockMatch ? jsonBlockMatch[1].trim() : text.trim()

  // Use fallback if block failed to parse or was missing
  if (!jsonBlockMatch) {
    // If no block found, maybe the text contains just JSON?
    // But text might contain <step> tags.
    // Try to find the first array-like structure.
    const arrayMatch = text.match(/\[\s*\{[\s\S]*\}\s*\]/)
    if (arrayMatch) {
        rawJson = arrayMatch[0]
    } else {
        // Remove <step> tags before parsing?
        rawJson = text.replace(/<step[\s\S]*?<\/step>/g, '').trim()
    }
  }

  try {
    const parsed = JSON.parse(rawJson)
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

  // FORCE override to prevent tool usage
  message += `\n\nIMPORTANT: You remain in DIRECT RESPONSE MODE. Do NOT use the "Write" tool or any other function. I cannot see tool outputs. Just write the JSON response directly.`

  return message
}

/**
 * Helper to find all complete JSON blocks in text
 */
function extractAllJsonBlocks(text: string): string[] {
  const blocks: string[] = []
  // Matches ```json or ``` blocks
  const regex = /```(?:json)?\s*\n?([\s\S]*?)\n?\s*```/g
  let match
  while ((match = regex.exec(text)) !== null) {
    // Basic heuristic: check if it looks like JSON array/object before adding
    const content = match[1].trim()
    if (content.startsWith('[') || content.startsWith('{')) {
       blocks.push(content)
    }
  }
  return blocks
}

export async function generateDesign(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
  }
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  const userMessage = buildContextMessage(request)
  let fullResponse = ''
  let processedBlockCount = 0

  for await (const chunk of streamChat(DESIGN_GENERATOR_PROMPT, [
    { role: 'user', content: userMessage },
  ])) {
    if (chunk.type === 'text') {
      fullResponse += chunk.content
      if (callbacks?.onTextUpdate) {
        callbacks.onTextUpdate(fullResponse)
      }
      
      // Check for new complete JSON blocks
      // We do this inside the loop to support "live" updates
      if (callbacks?.onApplyPartial) {
        const allBlocks = extractAllJsonBlocks(fullResponse)
        if (allBlocks.length > processedBlockCount) {
          // New block(s) found!
          const newBlocks = allBlocks.slice(processedBlockCount)
          let newNodesApplied = 0
          
          for (const blockJson of newBlocks) {
             const nodes = tryParseNodes(blockJson)
             if (nodes) {
               // We use the "modification" logic (upsert) for all phases
               // so subsequent phases can update earlier nodes
               const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
               
               for (const node of nodes) {
                 const existing = getNodeById(node.id)
                 if (existing) {
                   updateNode(node.id, node)
                 } else {
                   const rootFrame = getNodeById(DEFAULT_FRAME_ID)
                   const parentId = rootFrame ? DEFAULT_FRAME_ID : null
                   addNode(parentId, node)
                 }
                 newNodesApplied++
               }
             }
          }

          if (newNodesApplied > 0) {
            callbacks.onApplyPartial(newNodesApplied)
          }
          processedBlockCount = allBlocks.length
        }
      }

    } else if (chunk.type === 'error') {
      throw new Error(chunk.content)
    }
  }

  const nodes = extractJsonFromResponse(fullResponse)
  // strict check only if NO partials were applied? 
  // or just return what we have. 
  // If we processed blocks incrementally, we might want to return the final state.
  
  // Check if we found ANY nodes at all (either via partials or final extraction)
  if ((!nodes || nodes.length === 0) && processedBlockCount === 0) {
     // If no JSON found, return empty nodes but valid response.
     // This allows the "chatty" response ("I'll create a plan...") to be shown to the user
     // instead of an ugly "Failed to parse" error.
     // The UI will just show the text and appliedCount will be 0.
     return { nodes: [], rawResponse: fullResponse }
  }

  return { nodes: nodes || [], rawResponse: fullResponse }
}

function tryParseNodes(json: string): PenNode[] | null {
  try {
     const parsed = JSON.parse(json.trim())
     const nodes = Array.isArray(parsed) ? parsed : [parsed]
     return validateNodes(nodes) ? nodes : null
  } catch {
     return null
  }
}

export async function generateDesignModification(
  nodesToModify: PenNode[],
  instruction: string,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  // Build context from selected nodes
  const contextJson = JSON.stringify(nodesToModify, (_key, value) => {
    // omit children to avoid massive context if deep tree
    return value
  })

  // We use standard string concatenation to avoid backtick issues in tool calls
  const userMessage = "CONTEXT NODES:\n" + contextJson + "\n\nINSTRUCTION:\n" + instruction
  let fullResponse = ''

  for await (const chunk of streamChat(DESIGN_MODIFIER_PROMPT, [
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
    throw new Error('Failed to parse modified nodes from AI response')
  }

  return { nodes, rawResponse: fullResponse }
}

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { addNode, getNodeById } = useDocumentStore.getState()
  // Insert into the root frame if it exists, otherwise at document root
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  for (const node of nodes) {
    addNode(parentId, node)
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

/**
 * Extract PenNode JSON from AI response text and apply updates/insertions to canvas.
 * Handles both new nodes and modifications (matching by ID).
 */
export function extractAndApplyDesignModification(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText)
  if (!nodes || nodes.length === 0) return 0

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  let count = 0

  for (const node of nodes) {
    const existing = getNodeById(node.id)
    if (existing) {
      // Update existing node
      updateNode(node.id, node)
      count++
    } else {
      // It's a new node implied by the modification (e.g. "add a button")
       const rootFrame = getNodeById(DEFAULT_FRAME_ID)
       const parentId = rootFrame ? DEFAULT_FRAME_ID : null
       addNode(parentId, node)
       count++
    }
  }
  return count
}
