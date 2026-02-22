import type { FrameNode, PenNode } from '@/types/pen'
import type { VariableDefinition, ThemedValue } from '@/types/variables'
import type { AIDesignRequest } from './ai-types'
import { streamChat } from './ai-service'
import { DESIGN_GENERATOR_PROMPT, DESIGN_MODIFIER_PROMPT } from './ai-prompts'
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import {
  pendingAnimationNodes,
  markNodesForAnimation,
  startNewAnimationBatch,
  resetAnimationState,
} from './design-animation'
import { executeOrchestration } from './orchestrator'
import { DESIGN_STREAM_TIMEOUTS } from './ai-runtime-config'

// ---------------------------------------------------------------------------
// Cross-phase ID remapping — tracks replaceEmptyFrame mappings so that
// later phases recognise the root frame ID has been remapped to DEFAULT_FRAME_ID.
// ---------------------------------------------------------------------------

const generationRemappedIds = new Map<string, string>()
let generationContextHint = ''
/** Root frame width for the current generation (1200 desktop, 375 mobile) */
let generationCanvasWidth = 1200

export function resetGenerationRemapping(): void {
  generationRemappedIds.clear()
}

export function setGenerationContextHint(hint?: string): void {
  generationContextHint = hint?.trim() ?? ''
}

export function setGenerationCanvasWidth(width: number): void {
  generationCanvasWidth = width > 0 ? width : 1200
}

// Helper to find all complete JSON blocks in text

export function extractJsonFromResponse(text: string): PenNode[] | null {
  const parsedBlocks = extractAllJsonBlocks(text)
    .map((block) => tryParseNodes(block))
    .filter(Boolean) as PenNode[][]

  if (parsedBlocks.length > 0) {
    return selectBestNodeSet(parsedBlocks)
  }

  // Try JSONL format (flat nodes with _parent field)
  const jsonlTree = parseJsonlToTree(text)
  if (jsonlTree) return jsonlTree

  // Fallback: try to find a single JSON array if no blocks found
  const arrayMatch = text.match(/\[\s*\{[\s\S]*\}\s*\]/)
  if (arrayMatch) {
     const nodes = tryParseNodes(arrayMatch[0])
     return nodes
  }

  // Fallback: try parsing raw text after removing <step> tags.
  const stripped = text.replace(/<step[\s\S]*?<\/step>/g, '').trim()
  const directNodes = tryParseNodes(stripped)
  if (directNodes) {
    return directNodes
  }

  return null
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
      // Themed variable — show default value
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

// ---------------------------------------------------------------------------
// Streaming JSONL parser — extracts completed JSON objects from within
// a ```json block as they stream in, enabling element-by-element rendering.
// ---------------------------------------------------------------------------

interface StreamingNodeResult {
  node: PenNode
  parentId: string | null
}

/**
 * Extract completed JSON objects from streaming text (within a ```json block).
 * Uses brace-counting to detect complete objects before the block closes.
 * Each object is expected to have a `_parent` field for tree insertion.
 */
export function extractStreamingNodes(
  text: string,
  processedOffset: number,
): { results: StreamingNodeResult[]; newOffset: number } {
  // Primary mode: parse inside a ```json fenced block.
  // Fallback mode: parse raw JSONL/object text when the model omits fences.
  const jsonBlockStart = text.indexOf('```json')

  let contentStart = -1
  let searchEnd = text.length
  if (jsonBlockStart !== -1) {
    const firstNewline = text.indexOf('\n', jsonBlockStart)
    if (firstNewline === -1) return { results: [], newOffset: processedOffset }
    contentStart = firstNewline + 1
    const blockEnd = text.indexOf('\n```', contentStart)
    searchEnd = blockEnd > 0 ? blockEnd : text.length
  } else {
    const firstBrace = text.indexOf('{')
    if (firstBrace === -1) return { results: [], newOffset: processedOffset }
    contentStart = firstBrace
  }

  const startPos = Math.max(processedOffset, contentStart)

  const results: StreamingNodeResult[] = []
  let i = startPos

  while (i < searchEnd) {
    // Skip to next '{' character
    while (i < searchEnd && text[i] !== '{') i++
    if (i >= searchEnd) break

    // Brace-counting to find matching '}'
    const objStart = i
    let depth = 0
    let inString = false
    let escaped = false
    let j = i

    while (j < searchEnd) {
      const ch = text[j]
      if (escaped) { escaped = false; j++; continue }
      if (ch === '\\' && inString) { escaped = true; j++; continue }
      if (ch === '"') { inString = !inString; j++; continue }
      if (inString) { j++; continue }
      if (ch === '{') depth++
      else if (ch === '}') {
        depth--
        if (depth === 0) {
          // Complete object found
          const objStr = text.slice(objStart, j + 1)
          try {
            const obj = JSON.parse(objStr) as Record<string, unknown>
            if (obj.id && obj.type) {
              const parentId = (obj._parent as string | null) ?? null
              delete obj._parent
              results.push({ node: obj as unknown as PenNode, parentId })
            }
          } catch { /* malformed JSON, skip */ }
          i = j + 1
          break
        }
      }
      j++
    }

    if (depth > 0) break // Incomplete object, wait for more data
  }

  return { results, newOffset: i }
}

/**
 * Parse JSONL-format response (flat nodes with _parent field) into a tree.
 * Used by extractAndApplyDesign for batch apply of JSONL content.
 */
function parseJsonlToTree(text: string): PenNode[] | null {
  const { results } = extractStreamingNodes(text, 0)
  if (results.length === 0) return null

  const nodeMap = new Map<string, PenNode>()
  const roots: PenNode[] = []

  for (const { node, parentId } of results) {
    nodeMap.set(node.id, node)

    if (parentId === null) {
      roots.push(node)
    } else {
      const parent = nodeMap.get(parentId)
      if (parent) {
        if (!('children' in parent) || !Array.isArray((parent as PenNode & { children?: PenNode[] }).children)) {
          ;(parent as PenNode & { children?: PenNode[] }).children = []
        }
        ;(parent as PenNode & { children: PenNode[] }).children.push(node)
      } else {
        roots.push(node) // Parent not found, treat as root
      }
    }
  }

  return roots.length > 0 ? roots : null
}

/**
 * Insert a single streaming node into the canvas instantly.
 * Handles root frame replacement and parent ID remapping.
 * Note: tree-aware heuristics (button width, frame height, clipContent)
 * cannot run here because the node has no children yet during streaming.
 * Use applyPostStreamingTreeHeuristics() after all subtask nodes are inserted.
 */
export function insertStreamingNode(
  node: PenNode,
  parentId: string | null,
): void {
  const { addNode, getNodeById } = useDocumentStore.getState()

  // Ensure container nodes have children array for later child insertions
  if ((node.type === 'frame' || node.type === 'group') && !('children' in node)) {
    ;(node as PenNode & { children: PenNode[] }).children = []
  }

  // Resolve remapped parent IDs (e.g., root frame → DEFAULT_FRAME_ID)
  const resolvedParent = parentId
    ? (generationRemappedIds.get(parentId) ?? parentId)
    : null

  const parentNode = resolvedParent
    ? getNodeById(resolvedParent)
    : null

  if (parentNode && hasActiveLayout(parentNode)) {
    if ('x' in node) delete (node as { x?: number }).x
    if ('y' in node) delete (node as { y?: number }).y
    // ALL text inside layout frames: Fill Width + Auto Height.
    // Set width/textGrowth here; height will be estimated by applyTextWrappingHeuristic.
    if (node.type === 'text') {
      if (typeof node.width === 'number') node.width = 'fill_container'
      if (!node.textGrowth) node.textGrowth = 'fixed-width'
      // Default lineHeight based on text role (heading vs body)
      if (!node.lineHeight) {
        const fs = node.fontSize ?? 16
        node.lineHeight = fs >= 28 ? 1.2 : 1.5
      }
    }
  }

  applyGenerationHeuristics(node)

  // Skip ALL children being streamed into a phone placeholder — it must stay empty.
  // Also skip if the parent node doesn't exist on canvas (was itself blocked as a phone child).
  if (resolvedParent !== null && !parentNode) {
    return
  }
  if (parentNode && isInsidePhonePlaceholder(resolvedParent!, getNodeById)) {
    return
  }

  if (resolvedParent === null && isCanvasOnlyEmptyFrame() && node.type === 'frame') {
    // Root frame replaces the default empty frame — no animation needed
    replaceEmptyFrame(node)
  } else {
    const effectiveParent = resolvedParent ?? DEFAULT_FRAME_ID
    // Verify parent exists, fall back to root frame
    const parent = getNodeById(effectiveParent)
    const insertParent = parent ? effectiveParent : DEFAULT_FRAME_ID

    // Frames with fills appear instantly (background context for children).
    // All other nodes fade in with staggered animation.
    const nodeFill = 'fill' in node ? node.fill : undefined
    const hasFill = Array.isArray(nodeFill)
      ? nodeFill.length > 0
      : (nodeFill != null && typeof nodeFill === 'object')
    const isBackgroundFrame = node.type === 'frame' && hasFill
    if (!isBackgroundFrame) {
      pendingAnimationNodes.add(node.id)
      startNewAnimationBatch()
    }

    addNode(insertParent, node)

    // When a frame is inserted into a horizontal layout, equalize sibling card widths
    // to prevent overflow when multiple cards are placed in the same row.
    if (node.type === 'frame') {
      equalizeHorizontalSiblings(insertParent)
    }

    // When a top-level section is added directly under the root frame,
    // progressively expand root height to fit the new content.
    if (insertParent === DEFAULT_FRAME_ID) {
      expandRootFrameHeight()
    }
  }
}

function selectBestNodeSet(candidates: PenNode[][]): PenNode[] {
  let best = candidates[candidates.length - 1]
  let bestScore = scoreNodeSet(best)

  for (const candidate of candidates) {
    const score = scoreNodeSet(candidate)
    // Favor later blocks on ties to keep the most recent complete output.
    if (score >= bestScore) {
      best = candidate
      bestScore = score
    }
  }

  return best
}

function scoreNodeSet(nodes: PenNode[]): number {
  let score = nodes.length

  if (nodes.length === 1 && nodes[0].type === 'frame') {
    score += 1000
    const root = nodes[0]
    if ((root.x ?? 0) === 0 && (root.y ?? 0) === 0) score += 50
    if ('children' in root && Array.isArray(root.children)) {
      score += root.children.length * 10
    }
  }

  if (nodes.length > 1) {
    score -= 200
  }

  for (const node of nodes) {
    if ('children' in node && Array.isArray(node.children)) {
      score += node.children.length * 2
    }
  }

  return score
}

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
      // Ignore thinking chunks for modification — caller already shows progress
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

/**
 * Check if the canvas only has the default empty frame (no children).
 */
function isCanvasOnlyEmptyFrame(): boolean {
  const { document, getNodeById } = useDocumentStore.getState()
  if (document.children.length !== 1) return false
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  if (!rootFrame) return false
  return !('children' in rootFrame) || !rootFrame.children || rootFrame.children.length === 0
}

/**
 * Replace the default empty frame with the generated frame node,
 * preserving the root frame ID so canvas sync continues to work.
 */
function replaceEmptyFrame(generatedFrame: PenNode): void {
  const { updateNode } = useDocumentStore.getState()
  // Record the remapping so subsequent phases can find this node by its original ID
  generationRemappedIds.set(generatedFrame.id, DEFAULT_FRAME_ID)
  // Keep root frame ID and position (x=0, y=0), take everything else from generated frame
  const { id: _id, x: _x, y: _y, ...rest } = generatedFrame
  updateNode(DEFAULT_FRAME_ID, rest)
}

export function adjustRootFrameHeightToContent(): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const root = getNodeById(DEFAULT_FRAME_ID)
  if (!root || root.type !== 'frame') return
  if (!Array.isArray(root.children) || root.children.length === 0) return

  const requiredHeight = estimateNodeIntrinsicHeight(root)
  const targetHeight = Math.max(320, Math.round(requiredHeight))
  const currentHeight = toSizeNumber(root.height, 0)
  if (currentHeight <= 0) return
  if (Math.abs(currentHeight - targetHeight) < 8) return

  updateNode(DEFAULT_FRAME_ID, { height: targetHeight })
}

/**
 * Expand-only version of adjustRootFrameHeightToContent.
 * Used during streaming: only grows the root frame, never shrinks it.
 * This prevents visual jitter while sections are being progressively added.
 */
/**
 * When a frame is inserted into a horizontal layout parent, check if sibling
 * frame children should be equalized to fill_container to prevent overflow.
 * This runs DURING streaming so cards distribute evenly as they arrive.
 */
function equalizeHorizontalSiblings(parentId: string): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const parent = getNodeById(parentId)
  if (!parent || parent.type !== 'frame') return
  if (parent.layout !== 'horizontal') return
  if (!Array.isArray(parent.children) || parent.children.length < 2) return

  // Count frame children with fixed pixel widths
  const fixedFrames = parent.children.filter(
    (c) => c.type === 'frame'
      && !isPhonePlaceholderFrame(c)
      && !isDividerLikeFrame(c)
      && !isCompactControlFrame(c)
      && toSizeNumber('height' in c ? c.height : undefined, 0) > 88
      && typeof c.width === 'number'
      && (c.width as number) > 0,
  )
  if (fixedFrames.length < 2) return

  // Check if they look like a card row (similar heights)
  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0))
  const maxH = Math.max(...heights)
  const minH = Math.min(...heights)
  if (maxH <= 0 || minH / maxH <= 0.5) return

  // Convert to fill_container for even distribution and equal height
  for (const child of fixedFrames) {
    updateNode(child.id, { width: 'fill_container', height: 'fill_container' } as Partial<PenNode>)
  }
}

export function expandRootFrameHeight(): void {
  const { getNodeById, updateNode } = useDocumentStore.getState()
  const root = getNodeById(DEFAULT_FRAME_ID)
  if (!root || root.type !== 'frame') return
  if (!Array.isArray(root.children) || root.children.length === 0) return

  // Mobile screens have fixed viewport dimensions — don't auto-expand height.
  const rootWidth = toSizeNumber(root.width, 0)
  if (rootWidth > 0 && rootWidth <= 480) return

  const requiredHeight = estimateNodeIntrinsicHeight(root)
  const targetHeight = Math.max(320, Math.round(requiredHeight))
  const currentHeight = toSizeNumber(root.height, 0)
  // Only grow — never shrink during progressive generation
  if (currentHeight > 0 && targetHeight <= currentHeight) return

  updateNode(DEFAULT_FRAME_ID, { height: targetHeight })
}

export function applyNodesToCanvas(nodes: PenNode[]): void {
  const { getFlatNodes } = useDocumentStore.getState()
  const existingIds = new Set(getFlatNodes().map((n) => n.id))
  const preparedNodes = sanitizeNodesForInsert(nodes, existingIds)

  // If canvas only has one empty frame, replace it with the generated content
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    return
  }

  const { addNode, getNodeById } = useDocumentStore.getState()
  // Insert into the root frame if it exists, otherwise at document root
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  for (const node of preparedNodes) {
    addNode(parentId, node)
  }
  adjustRootFrameHeightToContent()
}

export function upsertNodesToCanvas(nodes: PenNode[]): number {
  const preparedNodes = sanitizeNodesForUpsert(nodes)

  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    return 1
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  let count = 0

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id
    const existing = getNodeById(resolvedId)
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode)
      updateNode(resolvedId, merged)
    } else {
      addNode(parentId, node)
    }
    count++
  }

  adjustRootFrameHeightToContent()
  return count
}

/** Same as upsertNodesToCanvas but skips sanitization (caller already did it). */
function upsertPreparedNodes(preparedNodes: PenNode[]): number {
  if (isCanvasOnlyEmptyFrame() && preparedNodes.length === 1 && preparedNodes[0].type === 'frame') {
    replaceEmptyFrame(preparedNodes[0])
    return 1
  }

  const { addNode, updateNode, getNodeById } = useDocumentStore.getState()
  const rootFrame = getNodeById(DEFAULT_FRAME_ID)
  const parentId = rootFrame ? DEFAULT_FRAME_ID : null
  let count = 0

  for (const node of preparedNodes) {
    // Resolve remapped IDs (e.g., root frame that was mapped to DEFAULT_FRAME_ID in Phase 1)
    const resolvedId = generationRemappedIds.get(node.id) ?? node.id
    const existing = getNodeById(resolvedId)
    if (existing) {
      const remappedNode = resolvedId !== node.id ? { ...node, id: resolvedId } : node
      const merged = mergeNodeForProgressiveUpsert(existing, remappedNode)
      updateNode(resolvedId, merged)
    } else {
      addNode(parentId, node)
    }
    count++
  }

  adjustRootFrameHeightToContent()
  return count
}

/**
 * Animate nodes onto the canvas with a staggered fade-in effect.
 * Synchronous — nodes are inserted immediately, and canvas-sync
 * schedules fire-and-forget staggered opacity animations.
 */
export function animateNodesToCanvas(nodes: PenNode[]): void {
  resetGenerationRemapping()
  resetAnimationState()
  const prepared = sanitizeNodesForUpsert(nodes)
  startNewAnimationBatch()
  markNodesForAnimation(prepared)

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  upsertPreparedNodes(prepared)
  useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
}

function applyGenerationHeuristics(node: PenNode): void {
  applyIconPathResolution(node)
  applyNoEmojiIconHeuristic(node)
  applyImagePlaceholderHeuristic(node)
  applyScreenshotFramePlaceholderHeuristic(node)
  applyPhonePlaceholderSizingHeuristic(node)
  applyDividerSizingHeuristic(node)
  applyNavbarHeuristic(node)
  applyHorizontalAlignCenterHeuristic(node)
  applyIconButtonSizing(node)
  applyBadgeSizing(node)
  applyButtonSpacingHeuristic(node)
  applyButtonWidthHeuristic(node)
  applyTextWrappingHeuristic(node)
  applyClipContentHeuristic(node)

  if (!('children' in node) || !Array.isArray(node.children)) return
  // Flatten redundant single "Inner" wrapper layers to keep hierarchy shallow.
  const flattenUpdates = getSingleInnerWrapperFlattenUpdates(node)
  if (flattenUpdates) {
    Object.assign(node as unknown as Record<string, unknown>, flattenUpdates as unknown as Record<string, unknown>)
  }
  // Ensure section-level frames have minimum horizontal padding
  applySectionPaddingHeuristic(node)
  // Tree-aware: fix text widths relative to parent layout
  applyTextFillContainerInLayout(node)
  // Card row equalization: horizontal rows of cards → fill_container
  applyCardRowEqualization(node)
  // Dense card rows (>=5 cards): compact internal content for layout stability
  applyDenseCardRowCompaction(node)
  // Form/card children: convert fixed-width buttons/inputs to fill_container
  applyFormChildFillContainer(node)
  for (const child of node.children) {
    applyGenerationHeuristics(child)
  }
  // Remove decorative glow/shadow frames next to phone placeholders
  applyRemoveDecorativeGlowSiblings(node)
  // After children are processed, fix horizontal overflow (children wider than parent)
  applyHorizontalOverflowFix(node)
  // After children are processed (text heights fixed), expand frame to fit content
  applyFrameHeightExpansion(node)
}

/**
 * Post-streaming tree heuristics — applies tree-aware fixes after all nodes
 * of a subtask have been inserted into the store.
 *
 * During streaming, nodes are inserted individually (no children), so tree-aware
 * heuristics like button width expansion, frame height expansion, and clipContent
 * detection fail silently. This function re-runs them on the completed subtree.
 */
export function applyPostStreamingTreeHeuristics(rootNodeId: string): void {
  const { getNodeById, updateNode, removeNode } = useDocumentStore.getState()
  const rootNode = getNodeById(rootNodeId)
  if (!rootNode || rootNode.type !== 'frame') return
  if (!Array.isArray(rootNode.children) || rootNode.children.length === 0) return

  // Walk the subtree depth-first, applying tree-aware fixes
  applyTreeFixesRecursive(rootNode, getNodeById, updateNode, removeNode)
}

function applyTreeFixesRecursive(
  node: PenNode,
  getNodeById: (id: string) => PenNode | undefined,
  updateNode: (id: string, updates: Partial<PenNode>) => void,
  removeNode: (id: string) => void,
): void {
  if (node.type !== 'frame') return

  // Normalize phone placeholder frames even when they have no children.
  const phoneUpdates = getPhonePlaceholderNormalizationUpdates(node)
  if (phoneUpdates) {
    updateNode(node.id, phoneUpdates as Partial<PenNode>)
    const refreshed = getNodeById(node.id)
    if (!refreshed || refreshed.type !== 'frame') return
    node = refreshed
  }

  const dividerUpdates = getDividerNormalizationUpdates(node)
  if (dividerUpdates) {
    updateNode(node.id, dividerUpdates as Partial<PenNode>)
    const refreshed = getNodeById(node.id)
    if (!refreshed || refreshed.type !== 'frame') return
    node = refreshed
  }
  if (!Array.isArray(node.children) || node.children.length === 0) return

  // Keep a mutable reference to the current children list.
  // This gets refreshed after removals so subsequent fixes see the up-to-date tree.
  let children: PenNode[] = node.children

  // --- Fix 14: Flatten redundant single "Inner" wrappers ---
  {
    const flattenUpdates = getSingleInnerWrapperFlattenUpdates(node)
    if (flattenUpdates) {
      updateNode(node.id, flattenUpdates as Partial<PenNode>)
      const refreshed = getNodeById(node.id)
      if (!refreshed || refreshed.type !== 'frame' || !Array.isArray(refreshed.children) || refreshed.children.length === 0) {
        return
      }
      node = refreshed
      children = refreshed.children
    }
  }

  // --- Fix 5: Remove decorative glow/shadow siblings of phone placeholders ---
  const hasPhone = children.some(
    (c: PenNode) => c.name === 'Phone Placeholder'
      || (c.type === 'frame' && isPhoneShaped(c)),
  )
  if (hasPhone) {
    const toRemove: string[] = []
    for (const child of children) {
      if (child.name === 'Phone Placeholder') continue
      if (child.type === 'frame' && isPhoneShaped(child)) continue
      const marker = `${child.name ?? ''} ${child.id}`.toLowerCase()
      if (!/(glow|shadow|backdrop|blur|bg\b|background|overlay|光|阴影)/.test(marker)) continue
      // Decorative name — keep only if it has meaningful content (text or image)
      const childKids = 'children' in child ? child.children : undefined
      if (Array.isArray(childKids) && childKids.some((c: PenNode) => c.type === 'text' || c.type === 'image')) continue
      toRemove.push(child.id)
    }
    for (const id of toRemove) {
      removeNode(id)
    }
    if (toRemove.length > 0) {
      // Re-read node after removal since children changed
      const refreshed = getNodeById(node.id)
      if (!refreshed || refreshed.type !== 'frame' || !Array.isArray(refreshed.children) || refreshed.children.length === 0) return
      children = refreshed.children
    }
  }

  // --- Fix 8: Flatten nested phone-shaped frames ---
  // AI sometimes generates wrapper frame > inner phone frame (double border/glow).
  // If a phone-shaped frame contains another phone-shaped frame, keep only the inner
  // one and restyle the parent to be non-phone (just a container).
  if (isPhoneShaped(node) || node.name === 'Phone Placeholder') {
    const phoneChildren = children.filter(
      (c: PenNode) => c.type === 'frame' && isPhoneShaped(c),
    )
    if (phoneChildren.length > 0) {
      // Parent is a wrapper — remove its phone styling, make it a plain container
      const updates: Record<string, unknown> = {
        cornerRadius: 0,
        stroke: undefined,
        effects: undefined,
        fill: undefined,
        name: node.name?.replace(/Phone|phone|Placeholder|placeholder|Mockup|mockup/g, '').trim() || 'Container',
      }
      updateNode(node.id, updates as Partial<PenNode>)
      // Ensure inner phone children are styled as placeholders
      for (const child of phoneChildren) {
        if (child.name !== 'Phone Placeholder') {
          const normalizedSize = resolvePhonePlaceholderSize(child)
          const colors = getPlaceholderColors('fill' in child ? child.fill : undefined)
          updateNode(child.id, {
            name: 'Phone Placeholder',
            layout: 'none',
            width: normalizedSize?.width ?? 260,
            height: normalizedSize?.height ?? 520,
            cornerRadius: Math.max(24, toCornerRadiusNumber(
              'cornerRadius' in child ? child.cornerRadius : undefined, 32)),
            fill: [{ type: 'solid', color: colors.fillColor }],
            stroke: { thickness: 1, fill: [{ type: 'solid', color: colors.strokeColor }] },
            effects: [{ type: 'shadow', offsetX: 0, offsetY: 4, blur: 24, spread: 0, color: 'rgba(0,0,0,0.12)' }],
          } as Partial<PenNode>)
        }
        // Remove all children of inner phone
        if ('children' in child && Array.isArray(child.children)) {
          for (const gc of child.children) {
            removeNode(gc.id)
          }
        }
      }
      // Refresh children after modifications
      const refreshed2 = getNodeById(node.id)
      if (refreshed2 && refreshed2.type === 'frame' && Array.isArray(refreshed2.children)) {
        children = refreshed2.children
      }
    }
  }

  // --- Fix 6: Section padding for wide fill_container frames ---
  if (isLikelyWideSectionFrame(node) && node.layout && node.layout !== 'none') {
    const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
    if (!/(nav|navbar|navigation|header|footer|导航|顶部|底部)/.test(marker)) {
      const hasContent = children.some((c: PenNode) =>
        c.type === 'text'
        || (c.type === 'frame' && 'children' in c && Array.isArray(c.children)
            && c.children.some((gc: PenNode) => gc.type === 'text')),
      )
      if (hasContent) {
        const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
        const isMobile = generationCanvasWidth <= 480
        const minH = isMobile ? 16 : (generationCanvasWidth <= 1024 ? 20 : 24)
        if (pad.left < minH || pad.right < minH) {
          const newL = Math.max(pad.left, minH)
          const newR = Math.max(pad.right, minH)
          const newPad: [number, number, number, number] = [pad.top, newR, pad.bottom, newL]
          updateNode(node.id, { padding: newPad } as Partial<PenNode>)
        }
      }
    }
  }

  // --- Fix 1: Text in layout frames → Fill Width + Auto Height ---
  // Skip if parent is fit_content (hug) — fill_container child breaks hug parent layout
  if (node.layout && node.layout !== 'none' && !isBadgeLikeFrame(node)) {
    // Compute parent content width for accurate text height estimation
    const nodeW = toSizeNumber(node.width, 0)
    let nodePad = parsePaddingValues('padding' in node ? node.padding : undefined)
    const adjustedPad = getLongTextPaddingAdjustment(node, nodePad, nodeW)
    if (adjustedPad) {
      updateNode(node.id, { padding: adjustedPad } as Partial<PenNode>)
      nodePad = parsePaddingValues(adjustedPad)
    }
    const nodeContentW = estimateParentContentWidthForText(node, nodePad, nodeW)
    for (const child of children) {
      if (child.type !== 'text') continue
      const needsWidthFix = shouldPromoteTextWidthToFillInLayout(node, child)
      const needsGrowthFix = shouldUseFixedWidthTextGrowthInLayout(node, child)
      if (needsWidthFix || needsGrowthFix) {
        const updates: Record<string, unknown> = {}
        if (needsWidthFix) updates.width = 'fill_container'
        if (needsGrowthFix) updates.textGrowth = 'fixed-width'
        // Estimate auto-height based on content and parent width
        const text = getTextContentForNode(child)
        const nextGrowth = needsGrowthFix ? 'fixed-width' : child.textGrowth
        if (text) {
          const fs = child.fontSize ?? 16
          const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF]/.test(text)
          const lh = child.lineHeight ?? (hasCjk ? 1.5 : 1.4)
          if (nextGrowth === 'fixed-width' || nextGrowth === 'fixed-width-height') {
            updates.height = estimateAutoHeight(text, fs, lh, nodeContentW || undefined)
          }
        }
        updateNode(child.id, updates as Partial<PenNode>)
      }
    }
  }

  // --- Fix 8: Enforce minimum text height in layout frames ---
  // AI often generates text with height:22 on a 48px font → text gets clipped and overlaps siblings.
  // For ALL text children in layout frames, ensure height >= fontSize * lineHeight (single line minimum).
  // For multi-line text (textGrowth="fixed-width"), re-estimate if current height is too small.
  if (node.layout && node.layout !== 'none') {
    const nodeW8 = toSizeNumber(node.width, 0)
    const nodePad8 = parsePaddingValues('padding' in node ? node.padding : undefined)
    const nodeContentW8 = estimateParentContentWidthForText(node, nodePad8, nodeW8)
    for (const child of children) {
      if (child.type !== 'text') continue
      const fs = child.fontSize ?? 16
      const lh = child.lineHeight ?? (fs >= 28 ? 1.2 : 1.5)
      const currentH = toSizeNumber(child.height, 0)
      const singleLineMin = Math.round(fs * Math.max(lh, 1.2) * 1.15)

      if (currentH > 0 && currentH < singleLineMin) {
        // Height is too small for even a single line — re-estimate
        const text = typeof child.content === 'string'
          ? child.content : Array.isArray(child.content)
            ? child.content.map((s: { text: string }) => s.text).join('') : ''
        if (text && (child.textGrowth === 'fixed-width' || child.textGrowth === 'fixed-width-height')) {
          const estimated = estimateAutoHeight(text, fs, lh, nodeContentW8 || undefined)
          updateNode(child.id, { height: Math.max(estimated, singleLineMin) } as Partial<PenNode>)
        } else {
          updateNode(child.id, { height: singleLineMin } as Partial<PenNode>)
        }
      }
    }
  }

  // --- Fix 2: Button/badge width expansion for CJK text + icons ---
  const w = toSizeNumber(node.width, 0)
  const h = toSizeNumber(node.height, 0)
  if (typeof node.width === 'number' && w > 0 && h > 0 && h <= 72) {
    const isCentered = node.alignItems === 'center' || node.justifyContent === 'center'
    const isHorizontal = node.layout === 'horizontal'
    const hasText = children.some(
      (c: PenNode) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
    )
    if ((isCentered || isHorizontal) && hasText) {
      const gap = toGapNumber('gap' in node ? node.gap : undefined)
      let contentWidth = 0
      for (const child of children) {
        if (child.type === 'text' && typeof child.content === 'string') {
          contentWidth += estimateSingleLineTextWidth(child.content.trim(), child.fontSize ?? 16)
        } else {
          // Icons (path/frame/rectangle) — use their explicit width or 20px fallback
          contentWidth += toSizeNumber('width' in child ? child.width : undefined, 20)
        }
      }
      if (children.length > 1) {
        contentWidth += gap * (children.length - 1)
      }
      if (contentWidth > 0) {
        const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
        const minWidth = Math.round(contentWidth + pad.left + pad.right + 32)
        if (w < minWidth) {
          updateNode(node.id, { width: minWidth })
        }
      }
    }
  }

  // --- Fix 7: Card row equalization in horizontal layouts ---
  // When a horizontal layout has ≥2 fixed-width frame children of similar height
  // (a card row), convert their widths to fill_container for even distribution.
  // Skip if parent is fit_content (hug) — fill_container children break hug layout.
  if (node.layout === 'horizontal' && node.width !== 'fit_content' && children.length >= 2) {
    const fixedFrames = children.filter((c: PenNode) =>
      c.type === 'frame'
      && !isPhonePlaceholderFrame(c)
      && !isDividerLikeFrame(c)
      && !isCompactControlFrame(c)
      && toSizeNumber('height' in c ? c.height : undefined, 0) > 88
      && typeof c.width === 'number'
      && (c.width as number) > 0,
    )
    if (fixedFrames.length >= 2) {
      const heights = fixedFrames.map((c: PenNode) =>
        toSizeNumber('height' in c ? c.height : undefined, 0),
      )
      const maxH = Math.max(...heights)
      const minH = Math.min(...heights)
      // Similar heights (within 60% ratio) → likely a card row, not sidebar+content
      if (maxH > 0 && minH / maxH > 0.5) {
        for (const child of fixedFrames) {
          updateNode(child.id, { width: 'fill_container', height: 'fill_container' } as Partial<PenNode>)
        }
      }
    }
  }

  // --- Fix 7.5: Dense card rows (>=5) need compact content to avoid overflow ---
  {
    const denseChanged = compactDenseCardRowInPlace(node)
    if (denseChanged) {
      updateNode(node.id, node as Partial<PenNode>)
      const refreshedDense = getNodeById(node.id)
      if (!refreshedDense || refreshedDense.type !== 'frame' || !Array.isArray(refreshedDense.children) || refreshedDense.children.length === 0) {
        return
      }
      node = refreshedDense
      children = refreshedDense.children
    }
  }

  // --- Fix 12: Icon-only button sizing ---
  {
    const nodeW12 = toSizeNumber(node.width, 0)
    const nodeH12 = toSizeNumber(node.height, 0)
    if (nodeW12 <= 80 && nodeH12 <= 80 && nodeW12 > 0 && nodeH12 > 0) {
      const hasText12 = children.some((c: PenNode) => c.type === 'text')
      const hasIcon12 = children.some((c: PenNode) =>
        c.type === 'path' || c.type === 'rectangle' || c.type === 'ellipse',
      )
      if (!hasText12 && hasIcon12) {
        const updates12: Record<string, unknown> = {}
        if (typeof node.width === 'number' && nodeW12 < 40) updates12.width = 40
        if (typeof node.height === 'number' && nodeH12 < 40) updates12.height = 40
        if (!node.justifyContent) updates12.justifyContent = 'center'
        if (!node.alignItems) updates12.alignItems = 'center'
        if (Object.keys(updates12).length > 0) {
          updateNode(node.id, updates12 as Partial<PenNode>)
        }
      }
    }
  }

  // --- Fix 13: Badge/tag sizing ---
  {
    const textKids13 = children.filter(
      (c: PenNode) => c.type === 'text' && typeof c.content === 'string',
    )
    if (textKids13.length === 1) {
      const t13 = textKids13[0]
      if (t13.type === 'text' && typeof t13.content === 'string') {
        const txt13 = t13.content.trim()
        const fs13 = t13.fontSize ?? 14
        if (txt13.length > 0 && txt13.length <= 28 && fs13 <= 18 && isBadgeLikeFrame(node)) {
          const frameUpdates13: Record<string, unknown> = {}
          if (typeof node.width === 'string' && node.width.startsWith('fill_container')) {
            frameUpdates13.width = 'fit_content'
          }
          if (!node.layout || node.layout === 'none') frameUpdates13.layout = 'horizontal'
          if (node.alignItems !== 'center') frameUpdates13.alignItems = 'center'
          if (!node.justifyContent) frameUpdates13.justifyContent = 'center'

          const iconKids13 = children.filter((c: PenNode) =>
            c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle',
          )
          const minGap13 = iconKids13.length > 0 ? 6 : 4
          const currentGap13 = toGapNumber('gap' in node ? node.gap : undefined)
          if (currentGap13 < minGap13) frameUpdates13.gap = minGap13

          const pad13 = parsePaddingValues('padding' in node ? node.padding : undefined)
          const minPadH13 = iconKids13.length > 0 ? 14 : 12
          if (pad13.top < 6 || pad13.bottom < 6 || pad13.left < minPadH13 || pad13.right < minPadH13) {
            frameUpdates13.padding = [
              Math.max(pad13.top, 6), Math.max(pad13.right, minPadH13),
              Math.max(pad13.bottom, 6), Math.max(pad13.left, minPadH13),
            ] as [number, number, number, number]
          }

          const hasCjk13 = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF]/.test(txt13)
          const targetLh13 = hasCjk13 ? 1.25 : 1.2
          const targetPad13 = parsePaddingValues(
            (frameUpdates13.padding as number | [number, number] | [number, number, number, number] | string | undefined)
              ?? ('padding' in node ? node.padding : undefined),
          )
          const h13 = toSizeNumber(node.height, 0)
          const effectiveH13 = h13 > 0
            ? h13
            : Math.round(fs13 * targetLh13 + targetPad13.top + targetPad13.bottom)
          frameUpdates13.cornerRadius = Math.max(1, Math.round(effectiveH13 / 2))

          if (Object.keys(frameUpdates13).length > 0) {
            updateNode(node.id, frameUpdates13 as Partial<PenNode>)
          }

          const textUpdates13: Record<string, unknown> = {}
          if (!t13.lineHeight || t13.lineHeight > 1.35 || t13.lineHeight < 1.05) {
            textUpdates13.lineHeight = targetLh13
          }
          if (t13.textAlignVertical !== 'middle') textUpdates13.textAlignVertical = 'middle'
          if (!t13.textGrowth || t13.textGrowth === 'fixed-width') textUpdates13.textGrowth = 'auto'
          if (Object.keys(textUpdates13).length > 0) {
            updateNode(t13.id, textUpdates13 as Partial<PenNode>)
          }

          const iconColor13 = extractPrimaryColor(t13.fill) ?? '#4C72DF'
          for (const icon13 of iconKids13) {
            if (icon13.type !== 'path') continue
            const iconUpdates13: Record<string, unknown> = {}
            const iw13 = toSizeNumber(icon13.width, 0)
            const ih13 = toSizeNumber(icon13.height, 0)
            const targetIcon13 = clamp(toSizeNumber(icon13.width, toSizeNumber(icon13.height, Math.round(fs13 * 0.95))), 10, 16)
            if (iw13 <= 0 || iw13 > 20) iconUpdates13.width = targetIcon13
            if (ih13 <= 0 || ih13 > 20) iconUpdates13.height = targetIcon13

            const strokeW13 = toStrokeThicknessNumber(icon13.stroke, 0)
            const hasFill13 = Array.isArray(icon13.fill) && icon13.fill.length > 0
            if (!hasFill13 && strokeW13 <= 0) {
              iconUpdates13.fill = [{ type: 'solid', color: iconColor13 }]
            }
            if (icon13.stroke && strokeW13 <= 0) {
              iconUpdates13.stroke = { thickness: 1.8, fill: [{ type: 'solid', color: iconColor13 }] }
            }

            if (Object.keys(iconUpdates13).length > 0) {
              updateNode(icon13.id, iconUpdates13 as Partial<PenNode>)
            }
          }
        }
      }
    }
  }

  // --- Fix 9: Button spacing (padding/gap/height) ---
  {
    const nodeH = toSizeNumber(node.height, 0)
    const isHorizontal = node.layout === 'horizontal'
    const isCentered = node.alignItems === 'center' || node.justifyContent === 'center'
    const hasTextChild = children.some(
      (c: PenNode) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
    )
    if (nodeH > 0 && nodeH <= 72 && (isHorizontal || isCentered) && hasTextChild && !isBadgeLikeFrame(node)) {
      const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
      const minV = 8
      const minH2 = 16
      if (pad.top < minV || pad.bottom < minV || pad.left < minH2 || pad.right < minH2) {
        const newTop = Math.max(pad.top, minV)
        const newBottom = Math.max(pad.bottom, minV)
        const newLeft = Math.max(pad.left, minH2)
        const newRight = Math.max(pad.right, minH2)
        const newPad: [number, number, number, number] = [newTop, newRight, newBottom, newLeft]
        updateNode(node.id, { padding: newPad } as Partial<PenNode>)
      }
      if (children.length >= 2 && isHorizontal) {
        const currentGap = toGapNumber('gap' in node ? node.gap : undefined)
        if (currentGap < 8) {
          updateNode(node.id, { gap: 8 } as Partial<PenNode>)
        }
      }
      const fontSize = children.reduce((max: number, c: PenNode) => {
        if (c.type === 'text') return Math.max(max, c.fontSize ?? 16)
        return max
      }, 0)
      const minHeight = Math.max(36, Math.round(fontSize * 2.4))
      if (nodeH < minHeight) {
        updateNode(node.id, { height: minHeight })
      }
    }
  }

  // --- Fix 3: clipContent for frames with cornerRadius + image children ---
  if (!('clipContent' in node && node.clipContent)) {
    const cr = toCornerRadiusNumber(node.cornerRadius, 0)
    if (cr > 0 && children.some((c: PenNode) => c.type === 'image')) {
      updateNode(node.id, { clipContent: true } as Partial<PenNode>)
    }
  }

  // --- Fix 11: Form children alignment & button row layout ---
  // Skip if parent is fit_content (hug) — fill_container children break hug layout
  if (node.layout === 'vertical' && node.width !== 'fit_content' && children.length >= 2) {
    const parentW11 = toSizeNumber(node.width, 0)
    const pad11 = parsePaddingValues('padding' in node ? node.padding : undefined)
    const contentW11 = parentW11 > 0 ? parentW11 - pad11.left - pad11.right : 0

    // Check if any sibling already uses fill_container —
    // if so, fixed-width siblings should align by also using fill_container.
    const hasFillSibling11 = children.some((c: PenNode) =>
      c.type === 'frame'
      && c.width === 'fill_container'
      && !isPhonePlaceholderFrame(c)
      && !isDividerLikeFrame(c),
    )

    for (const child of children) {
      if (child.type !== 'frame') continue
      if (isPhonePlaceholderFrame(child)) continue
      if (isDividerLikeFrame(child)) continue
      const childWidthValue = child.width
      const childWidthIsNumber = typeof childWidthValue === 'number'
      const childWidthIsFit = typeof childWidthValue === 'string' && childWidthValue.startsWith('fit_content')
      if (!childWidthIsNumber && !childWidthIsFit) continue
      const childW = childWidthIsNumber ? toSizeNumber(childWidthValue, 0) : 0
      const childH = toSizeNumber('height' in child ? child.height : undefined, 0)

      // Text wrappers should not stay fit_content in wide vertical containers;
      // it creates narrow columns even when parent has enough room.
      if (
        childWidthIsFit
        && !isBadgeLikeFrame(child)
        && !isCompactButtonLikeFrame(child)
        && frameNeedsFillWidthForTextContent(child)
      ) {
        updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
        continue
      }

      // Overflow: child wider than parent content area
      if (contentW11 > 0 && childW > contentW11) {
        updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
        continue
      }

      // Narrow fixed-width text wrappers in wide vertical containers cause
      // severe over-wrapping and "squeezed" cards. Promote them to fill width.
      if (
        contentW11 > 0
        && childWidthIsNumber
        && childW > 0
        && childW < contentW11 * 0.72
        && !isBadgeLikeFrame(child)
        && !isCompactButtonLikeFrame(child)
        && frameNeedsFillWidthForTextContent(child)
      ) {
        updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
        continue
      }

      // Consistency: if a sibling uses fill_container, match it
      if (hasFillSibling11 && childH > 0 && childH <= 72 && !isCompactControlFrame(child)) {
        const hasContent = 'children' in child && Array.isArray(child.children)
          && child.children.some((gc: PenNode) => gc.type === 'text' || gc.type === 'path')
        if (hasContent) {
          updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
          continue
        }
      }

      // Horizontal button row → row fills parent
      if (child.layout === 'horizontal'
        && 'children' in child && Array.isArray(child.children)
        && child.children.length >= 2) {
        const allBtnLike = child.children.every((gc: PenNode) => isCompactButtonLikeFrame(gc))
        if (allBtnLike) {
          updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
          const childGap = toGapNumber('gap' in child ? child.gap : undefined)
          const updates11: Record<string, unknown> = {}
          if (!child.justifyContent || child.justifyContent === 'start') {
            updates11.justifyContent = 'center'
          }
          if (childGap < 8) updates11.gap = 12
          if (Object.keys(updates11).length > 0) {
            updateNode(child.id, updates11 as Partial<PenNode>)
          }
        }
      }
    }
  }

  // Recurse into child frames
  for (const child of children) {
    applyTreeFixesRecursive(child, getNodeById, updateNode, removeNode)
  }

  // --- Fix 10: Horizontal overflow — reduce gap, then expand parent ---
  if (node.layout === 'horizontal' && typeof node.width === 'number' && children.length >= 2) {
    const parentW2 = toSizeNumber(node.width, 0)
    const pad2 = parsePaddingValues('padding' in node ? node.padding : undefined)
    const gap2 = toGapNumber('gap' in node ? node.gap : undefined)
    const availW2 = parentW2 - pad2.left - pad2.right

    let childrenTotalW = 0
    for (const child of children) {
      const cw = toSizeNumber('width' in child ? (child as { width?: number | string }).width : undefined, 0)
      if ('width' in child && typeof (child as { width?: number | string }).width === 'number' && cw > 0) {
        childrenTotalW += cw
      } else {
        childrenTotalW += 80
      }
    }
    const gapTotal2 = gap2 * (children.length - 1)
    childrenTotalW += gapTotal2

    if (childrenTotalW > availW2) {
      // Strategy 1: Reduce gap
      for (const tryGap of [8, 4]) {
        if (gap2 > tryGap) {
          const reduced = childrenTotalW - gapTotal2 + tryGap * (children.length - 1)
          if (reduced <= availW2) {
            updateNode(node.id, { gap: tryGap } as Partial<PenNode>)
            childrenTotalW = reduced
            break
          }
        }
      }

      // Strategy 2: Expand parent to fit
      if (childrenTotalW > availW2) {
        const neededW2 = Math.round(childrenTotalW + pad2.left + pad2.right)
        if (neededW2 > parentW2 && neededW2 <= generationCanvasWidth) {
          updateNode(node.id, { width: neededW2 } as Partial<PenNode>)
        } else if (neededW2 > generationCanvasWidth * 0.8) {
          updateNode(node.id, { width: 'fill_container' } as Partial<PenNode>)
        }
      }
    }
  }

  // --- Fix 4: Frame height expansion (after children are processed) ---
  if (typeof node.height === 'number' && node.layout && node.layout !== 'none') {
    const intrinsic = estimateNodeIntrinsicHeight(node)
    if (intrinsic > node.height) {
      updateNode(node.id, { height: Math.round(intrinsic) })
    }
  }
}

/**
 * For text children inside layout frames, convert fixed pixel widths to "fill_container"
 * so text wraps properly within the parent's content area instead of overflowing or being clipped.
 */
/**
 * In horizontal layouts with ≥2 fixed-width frame children of similar height (card rows),
 * convert child widths to fill_container for even space distribution.
 * This prevents cards from being too narrow for their icon+text content.
 */
function applyCardRowEqualization(parent: PenNode): void {
  if (parent.type !== 'frame') return
  if (parent.layout !== 'horizontal') return
  // Never convert children to fill_container when parent is fit_content (hug)
  if (parent.width === 'fit_content') return
  if (!Array.isArray(parent.children) || parent.children.length < 2) return

  const fixedFrames = parent.children.filter(
    (c) => c.type === 'frame'
      && !isPhonePlaceholderFrame(c)
      && !isDividerLikeFrame(c)
      && !isCompactControlFrame(c)
      && toSizeNumber('height' in c ? c.height : undefined, 0) > 88
      && typeof c.width === 'number'
      && (c.width as number) > 0,
  )
  if (fixedFrames.length < 2) return

  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0))
  const maxH = Math.max(...heights)
  const minH = Math.min(...heights)
  // Similar heights → likely a card row, not sidebar+content
  if (maxH > 0 && minH / maxH > 0.5) {
    for (const child of fixedFrames) {
      ;(child as unknown as Record<string, unknown>).width = 'fill_container'
    }
  }
}

type DenseCardTextRole = 'title' | 'meta' | 'desc'

function isDenseCardRowFrame(node: PenNode): node is FrameNode {
  if (node.type !== 'frame') return false
  if (node.layout !== 'horizontal') return false
  if (!Array.isArray(node.children) || node.children.length < 5) return false
  return true
}

function isDenseRowCardFrame(node: PenNode): node is FrameNode {
  if (node.type !== 'frame') return false
  if (isPhonePlaceholderFrame(node) || isDividerLikeFrame(node)) return false
  if (isBadgeLikeFrame(node) || isCompactButtonLikeFrame(node)) return false
  if (!Array.isArray(node.children) || node.children.length === 0) return false

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(button|btn|cta|badge|tag|chip|pill|nav|navbar|menu|tab|divider|separator|按钮|标签|导航|分隔)/.test(marker)) {
    return false
  }

  const h = toSizeNumber(node.height, 0)
  if (h > 0 && h < 88) return false
  if (h > 0 && h > 460) return false

  const directTextCount = node.children.filter(
    (c) => c.type === 'text' && getTextContentForNode(c).trim().length > 0,
  ).length
  return directTextCount >= 1
}

function getDenseCardSortKey(node: PenNode, fallback: number): number {
  const y = typeof node.y === 'number' ? node.y : fallback * 1000
  const x = typeof node.x === 'number' ? node.x : 0
  return y * 10000 + x
}

function getDenseCardMaxChars(
  role: DenseCardTextRole,
  aggressive: boolean,
  hasCjk: boolean,
): number {
  if (role === 'title') return hasCjk ? (aggressive ? 6 : 8) : (aggressive ? 12 : 16)
  if (role === 'meta') return hasCjk ? (aggressive ? 8 : 10) : (aggressive ? 14 : 18)
  return hasCjk ? (aggressive ? 12 : 16) : (aggressive ? 24 : 34)
}

function denseCardTextLength(value: string): number {
  return [...value].length
}

function cleanDenseCardPhrase(value: string): string {
  let next = value
    .replace(/[()（）[\]【】]/g, ' ')
    .replace(/[“”"']/g, '')
    .replace(/\s{2,}/g, ' ')
    .trim()
  next = next.replace(
    /^(?:支持|自动识别|自动|帮助|让你|为你|通过|结合|进行|用于|提供|实现|打造|助你|能够|可以|全面|深度|专注|聚焦|系统化|针对|面向|快速|高效|轻松)\s*/u,
    '',
  ).trim()
  next = next.replace(/[，。；;！!？?]+$/g, '').trim()
  return next
}

function extractDenseCardKeywordCandidates(text: string): string[] {
  const keywords = new Set<string>()

  const knownTerms = text.match(
    /(词根拆解|谐音联想|故事记忆|手写拼写|拼写练习|易混词对比|分级词库|智能助记|科学复习|词汇学习|错词强化|发音训练|语法解析|例句跟读|记忆方案|复习计划|母语级表达|核心词汇|词库覆盖|AI助记|AI驱动)/g,
  )
  if (knownTerms) {
    for (const term of knownTerms) keywords.add(term)
  }

  const cefr = text.match(/CEFR\s*[A-C]\d(?:\s*[-~]\s*[A-C]\d)?/i)
  if (cefr) keywords.add(cefr[0].replace(/\s+/g, ''))

  const levelRange = text.match(/[A-C]\d(?:\s*[-~]\s*[A-C]\d)/i)
  if (levelRange) keywords.add(levelRange[0].replace(/\s+/g, ''))

  const cjkMetric = text.match(/~?\d{1,3}(?:,\d{3})?\s*词/)
  if (cjkMetric) keywords.add(cjkMetric[0].replace(/\s+/g, ''))

  const latinMetric = text.match(/\d+(?:\.\d+)?\s*(?:k|K|w|W)\+?/)
  if (latinMetric) keywords.add(latinMetric[0])

  return [...keywords]
}

function extractDenseCardPhrases(text: string): string[] {
  const pieces = text
    .replace(/\r?\n+/g, '|')
    .replace(/[，。；;！!？?]/g, '|')
    .replace(/[、/＋+·•,:：]/g, '|')
    .split('|')
    .map((piece) => cleanDenseCardPhrase(piece))
    .filter((piece) => piece.length > 0)

  const deduped: string[] = []
  const seen = new Set<string>()
  for (const piece of pieces) {
    if (seen.has(piece)) continue
    seen.add(piece)
    deduped.push(piece)
  }

  for (const keyword of extractDenseCardKeywordCandidates(text)) {
    if (!seen.has(keyword)) {
      seen.add(keyword)
      deduped.push(keyword)
    }
  }

  return deduped
}

function buildDenseCardSummary(
  normalized: string,
  role: DenseCardTextRole,
  maxChars: number,
  hasCjk: boolean,
): string {
  const phrases = extractDenseCardPhrases(normalized)
  if (phrases.length === 0) return ''

  const fitting = phrases.filter((phrase) => denseCardTextLength(phrase) <= maxChars)
  if (role === 'title') {
    if (fitting.length === 0) return ''
    return [...fitting].sort((a, b) => denseCardTextLength(b) - denseCardTextLength(a))[0]
  }

  const joiner = hasCjk ? '·' : ' / '
  let summary = ''
  const source = fitting.length > 0 ? fitting : phrases
  for (const phrase of source) {
    if (denseCardTextLength(phrase) > maxChars) continue
    const next = summary ? `${summary}${joiner}${phrase}` : phrase
    if (denseCardTextLength(next) <= maxChars) {
      summary = next
    }
  }

  return summary
}

function compactDenseCardText(
  text: string,
  role: DenseCardTextRole,
  aggressive: boolean,
): string {
  const normalized = text.replace(/\r?\n+/g, ' ').replace(/\s{2,}/g, ' ').trim()
  if (!normalized) return normalized
  const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(normalized)
  const maxChars = getDenseCardMaxChars(role, aggressive, hasCjk)
  if (denseCardTextLength(normalized) <= maxChars) return normalized

  const refined = buildDenseCardSummary(normalized, role, maxChars, hasCjk)
  if (refined) return refined

  // Dense rows should prefer dropping secondary copy over hard truncation.
  if (role !== 'title') return ''

  // Title fallback: pick a whole phrase token that fits instead of slicing chars.
  const tokens = normalized
    .split(/[，。；;！!？?\s、/＋+·•,:：]/)
    .map((token) => cleanDenseCardPhrase(token))
    .filter((token) => token.length > 0)
  const firstFitting = tokens.find((token) => denseCardTextLength(token) <= maxChars)
  return firstFitting ?? normalized
}

function applyDenseCardTextCompaction(
  node: PenNode,
  role: DenseCardTextRole,
  aggressive: boolean,
): boolean {
  if (node.type !== 'text') return false
  const original = getTextContentForNode(node)
  const compacted = compactDenseCardText(original, role, aggressive)
  let changed = false

  if (compacted !== original) {
    node.content = compacted
    changed = true
  }

  const fontSize = node.fontSize ?? 16
  const maxFontSize = role === 'title'
    ? (aggressive ? 20 : 22)
    : role === 'meta'
      ? (aggressive ? 15 : 16)
      : (aggressive ? 13 : 14)
  if (fontSize > maxFontSize) {
    node.fontSize = maxFontSize
    changed = true
  }

  const targetLineHeight = role === 'title' ? 1.25 : 1.35
  if (!node.lineHeight || node.lineHeight > (role === 'title' ? 1.4 : 1.6)) {
    node.lineHeight = targetLineHeight
    changed = true
  }

  if (typeof node.width === 'number' && node.width > 0) {
    node.width = 'fill_container'
    changed = true
  }

  if (role === 'desc') {
    if (node.textGrowth !== 'fixed-width') {
      node.textGrowth = 'fixed-width'
      changed = true
    }
  } else if (node.textGrowth === 'fixed-width-height') {
    node.textGrowth = 'auto'
    changed = true
  }

  return changed
}

function nodeHasTextDescendant(node: PenNode): boolean {
  if (node.type === 'text') return true
  if (!('children' in node) || !Array.isArray(node.children)) return false
  return node.children.some((child) => nodeHasTextDescendant(child))
}

function isDenseCardRemovableDecorative(node: PenNode): boolean {
  if (node.type === 'text') return false
  if (isPhonePlaceholderFrame(node) || isDividerLikeFrame(node)) return false
  if (node.type === 'frame' && nodeHasTextDescendant(node)) return false
  return node.type === 'frame'
    || node.type === 'path'
    || node.type === 'rectangle'
    || node.type === 'ellipse'
    || node.type === 'image'
    || node.type === 'line'
}

function compactDenseCardFrameInPlace(
  card: FrameNode,
  aggressive: boolean,
): boolean {
  if (!Array.isArray(card.children) || card.children.length === 0) return false
  let changed = false

  const pad = parsePaddingValues(card.padding)
  const maxHorizontalPad = aggressive ? 12 : 14
  const maxVerticalPad = aggressive ? 10 : 12
  const nextTop = Math.min(pad.top, maxVerticalPad)
  const nextRight = Math.min(pad.right, maxHorizontalPad)
  const nextBottom = Math.min(pad.bottom, maxVerticalPad)
  const nextLeft = Math.min(pad.left, maxHorizontalPad)
  if (nextTop !== pad.top || nextRight !== pad.right || nextBottom !== pad.bottom || nextLeft !== pad.left) {
    card.padding = [nextTop, nextRight, nextBottom, nextLeft]
    changed = true
  }

  const gap = toGapNumber(card.gap)
  const targetGap = aggressive ? 6 : 8
  if (gap > targetGap) {
    card.gap = targetGap
    changed = true
  }

  const textEntries = card.children
    .map((child, index) => ({ child, index, sortKey: getDenseCardSortKey(child, index) }))
    .filter((entry) => entry.child.type === 'text' && getTextContentForNode(entry.child).trim().length > 0)
    .sort((a, b) => a.sortKey - b.sortKey)

  const keepTextCount = 2
  const removeIndexes = new Set<number>()
  for (let i = 0; i < textEntries.length; i += 1) {
    const entry = textEntries[i]
    if (i >= keepTextCount) {
      removeIndexes.add(entry.index)
      changed = true
      continue
    }
    const role: DenseCardTextRole = i === 0 ? 'title' : i === 1 ? 'meta' : 'desc'
    if (applyDenseCardTextCompaction(entry.child, role, aggressive)) {
      changed = true
    }
    if (getTextContentForNode(entry.child).trim().length === 0) {
      removeIndexes.add(entry.index)
      changed = true
    }
  }

  const decorativeEntries = card.children
    .map((child, index) => ({ child, index, sortKey: getDenseCardSortKey(child, index) }))
    .filter((entry) => isDenseCardRemovableDecorative(entry.child))
    .sort((a, b) => a.sortKey - b.sortKey)
  const keepDecorCount = aggressive ? 1 : 2
  for (let i = keepDecorCount; i < decorativeEntries.length; i += 1) {
    removeIndexes.add(decorativeEntries[i].index)
    changed = true
  }

  if (removeIndexes.size > 0) {
    card.children = card.children.filter((_, index) => !removeIndexes.has(index))
  }

  return changed
}

function compactDenseCardRowInPlace(row: PenNode): boolean {
  if (!isDenseCardRowFrame(row)) return false

  const rowChildren = row.children ?? []
  const cards = rowChildren.filter((child): child is FrameNode => isDenseRowCardFrame(child))
  if (cards.length < 5) return false

  const pad = parsePaddingValues(row.padding)
  const rowGap = toGapNumber(row.gap)
  const rowW = toSizeNumber(row.width, 0)
  const perCardW = rowW > 0
    ? (rowW - pad.left - pad.right - rowGap * Math.max(0, cards.length - 1)) / cards.length
    : 0
  const aggressive = cards.length >= 6 || (perCardW > 0 && perCardW < 170)

  let changed = false
  const targetGap = aggressive ? 8 : 10
  if (rowGap > targetGap) {
    row.gap = targetGap
    changed = true
  }

  for (const card of cards) {
    if (compactDenseCardFrameInPlace(card, aggressive)) {
      changed = true
    }
  }

  return changed
}

function applyDenseCardRowCompaction(parent: PenNode): void {
  compactDenseCardRowInPlace(parent)
}

function applyTextFillContainerInLayout(parent: PenNode): void {
  if (parent.type !== 'frame') return
  const layout = parent.layout
  if (!layout || layout === 'none') return
  if (!Array.isArray(parent.children)) return

  // NEVER convert children to fill_container when parent is fit_content (hug width).
  // fill_container child + fit_content parent = circular dependency → layout breaks.
  const parentIsHug = parent.width === 'fit_content'
  const badgeLikeParent = isBadgeLikeFrame(parent)

  // Compute parent's actual content width for accurate text height estimation
  const parentW = toSizeNumber(parent.width, 0)
  let pad = parsePaddingValues('padding' in parent ? parent.padding : undefined)
  const adjustedPad = getLongTextPaddingAdjustment(parent, pad, parentW)
  if (adjustedPad) {
    parent.padding = adjustedPad
    pad = parsePaddingValues(adjustedPad)
  }
  const contentW = estimateParentContentWidthForText(parent, pad, parentW)

  for (const child of parent.children) {
    if (child.type === 'text') {
      if (badgeLikeParent) {
        const fs = child.fontSize ?? 14
        const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF]/.test(
          typeof child.content === 'string' ? child.content : '',
        )
        if (!child.textGrowth || child.textGrowth === 'fixed-width') child.textGrowth = 'auto'
        if (!child.lineHeight || child.lineHeight > 1.35 || child.lineHeight < 1.05) {
          child.lineHeight = hasCjk ? 1.25 : 1.2
        }
        child.textAlignVertical = 'middle'
        if (typeof child.height === 'number') {
          const targetTextH = Math.round(fs * (child.lineHeight ?? 1.2) * 1.08)
          if (child.height > targetTextH * 1.6 || child.height < targetTextH * 0.85) {
            child.height = targetTextH
          }
        }
        continue
      }

      const shouldFillWidth = shouldPromoteTextWidthToFillInLayout(parent, child)
      if (shouldFillWidth) {
        child.width = 'fill_container'
      }
      const shouldFixGrowth = shouldUseFixedWidthTextGrowthInLayout(parent, child)
      if (shouldFixGrowth) child.textGrowth = 'fixed-width'
      if (!child.lineHeight) {
        const fs = child.fontSize ?? 16
        const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF]/.test(
          typeof child.content === 'string' ? child.content : '',
        )
        child.lineHeight = hasCjk
          ? (fs >= 28 ? 1.35 : 1.55)
          : (fs >= 28 ? 1.2 : 1.5)
      }
      // Re-estimate height based on parent's actual content width (not canvas width)
      const textContent = getTextContentForNode(child).trim()
      if (
        contentW > 0
        && (child.textGrowth === 'fixed-width' || child.textGrowth === 'fixed-width-height')
        && textContent
      ) {
        const fs = child.fontSize ?? 16
        const lh = child.lineHeight ?? 1.5
        child.height = estimateAutoHeight(textContent, fs, lh, contentW)
      }
    }
    // Also fix image children in vertical layout — images should fill parent width
    if (child.type === 'image' && typeof child.width === 'number' && layout === 'vertical' && !parentIsHug) {
      if (contentW > 0 && child.width >= contentW * 0.9) {
        child.width = 'fill_container'
      }
    }
  }
}

function applyImagePlaceholderHeuristic(node: PenNode): void {
  if (node.type !== 'image') return

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  const contextMarker = generationContextHint.toLowerCase()
  const contextualScreenshotHint = /(截图|screenshot|mockup|手机|app[-_\s]*screen|应用截图)/.test(contextMarker)
  const screenshotLike = isScreenshotLikeMarker(marker)
    || (contextualScreenshotHint && /(preview|hero|showcase|phone|screen|展示|预览)/.test(marker))
  if (!screenshotLike) return

  const width = toSizeNumber(node.width, 360)
  const height = toSizeNumber(node.height, 720)
  // Detect dark/light from context hint (dark if mentions dark/terminal/cyber/night)
  const dark = !/(light|白|浅色|bright)/.test(generationContextHint.toLowerCase())
  node.src = createPhonePlaceholderDataUri(width, height, dark)
  if (node.cornerRadius === undefined) {
    node.cornerRadius = 24
  }
}

function isScreenshotLikeMarker(text: string): boolean {
  return /app[-_\s]*screen|screenshot|mockup|phone|mobile|device|截图|界面|手机|应用截图/.test(text)
}

function createPhonePlaceholderDataUri(width: number, height: number, dark = true): string {
  const w = Math.max(140, Math.round(width))
  const h = Math.max(240, Math.round(height))
  const pad = Math.max(8, Math.round(Math.min(w, h) * 0.05))
  const innerW = w - pad * 2
  const innerH = h - pad * 2
  const outerR = Math.round(Math.min(w, h) * 0.12)
  const innerR = Math.max(outerR - pad, 8)

  const bgColor = dark ? '#111627' : '#F1F5F9'
  const strokeColor = dark ? '#1E2440' : '#D1D5DB'

  // Clean phone shape: outer body + inner screen outline, no text
  const svg =
    `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">` +
    `<rect width="${w}" height="${h}" rx="${outerR}" fill="${bgColor}"/>` +
    `<rect x="${pad}" y="${pad}" width="${innerW}" height="${innerH}" rx="${innerR}" fill="none" stroke="${strokeColor}" stroke-width="1"/>` +
    `</svg>`
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

/** Determine if a solid fill color is dark-themed (luminance < 128). */
function isFillDark(fill: unknown): boolean {
  if (!Array.isArray(fill) || fill.length === 0) return true
  const first = fill[0] as { type?: string; color?: string }
  if (first.type !== 'solid' || !first.color) return true
  const c = first.color
  if (c.startsWith('rgba')) {
    const parts = c.replace(/rgba?\(|\)/g, '').split(',').map(Number)
    if (parts.length >= 3) return (parts[0] * 299 + parts[1] * 587 + parts[2] * 114) / 1000 < 128
  }
  const hex = c.replace('#', '')
  if (hex.length < 6) return true
  const r = parseInt(hex.substring(0, 2), 16)
  const g = parseInt(hex.substring(2, 4), 16)
  const b = parseInt(hex.substring(4, 6), 16)
  return (r * 299 + g * 587 + b * 114) / 1000 < 128
}

/** Derive consistent placeholder colors that match the current design theme. */
function getPlaceholderColors(fill: unknown): {
  fillColor: string; strokeColor: string; textColor: string
} {
  if (isFillDark(fill)) {
    return { fillColor: '#111627', strokeColor: '#1E2440', textColor: '#2A3050' }
  }
  return { fillColor: '#F1F5F9', strokeColor: '#D1D5DB', textColor: '#C0C4CC' }
}

function isDividerMarker(text: string): boolean {
  return /(divider|separator|splitter|rule|(^|[\s_-])hr([\s_-]|$)|分隔|分割|分界)/.test(text)
}

function isDividerLikeFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  const markerMatch = isDividerMarker(marker)

  const w = toSizeNumber('width' in node ? node.width : undefined, 0)
  const h = toSizeNumber('height' in node ? node.height : undefined, 0)
  const thinVertical = w > 0 && w <= 6 && h >= 18
  const thinHorizontal = h > 0 && h <= 6 && w >= 18
  if (!markerMatch && !thinVertical && !thinHorizontal) return false

  if (Array.isArray(node.children) && node.children.some((c) => c.type === 'text')) return false
  if (!markerMatch && w > 0 && h > 0 && w <= 20 && h <= 20) return false
  return true
}

function getDividerNormalizationUpdates(node: PenNode): Record<string, unknown> | null {
  if (!isDividerLikeFrame(node)) return null
  const frame = node as FrameNode

  const marker = `${frame.name ?? ''} ${frame.id}`.toLowerCase()
  const w = toSizeNumber(frame.width, 0)
  const h = toSizeNumber(frame.height, 0)
  const strokeW = toStrokeThicknessNumber(frame.stroke, 1)
  const thickness = clamp(Math.round(strokeW > 0 ? strokeW : 1), 1, 4)

  const explicitVertical = /(vertical|竖)/.test(marker)
  const explicitHorizontal = /(horizontal|横)/.test(marker)
  const isVertical = explicitVertical
    || (!explicitHorizontal && ((h > 0 && w > 0 && h >= w) || (h > 0 && w <= 0)))

  const updates: Record<string, unknown> = {}
  if (isVertical) {
    if (typeof frame.width !== 'number' || w <= 0 || w > 6) {
      updates.width = thickness
    }
  } else if (typeof frame.height !== 'number' || h <= 0 || h > 6) {
    updates.height = thickness
  }

  if (frame.layout && frame.layout !== 'none') updates.layout = 'none'
  if (Array.isArray(frame.children) && frame.children.length > 0) updates.children = []

  return Object.keys(updates).length > 0 ? updates : null
}

function applyDividerSizingHeuristic(node: PenNode): void {
  const updates = getDividerNormalizationUpdates(node)
  if (!updates) return
  Object.assign(node as unknown as Record<string, unknown>, updates)
}

function isPhonePlaceholderFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (node.name === 'Phone Placeholder') return true

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(phone[-_\s]*(placeholder|mockup)|手机占位|手机示意|手机模型)/.test(marker)) return true
  if (!isExplicitPlaceholderMarker(marker)) return false

  const w = toSizeNumber('width' in node ? node.width : undefined, 0)
  const h = toSizeNumber('height' in node ? node.height : undefined, 0)
  if (w <= 0 || h <= 0) return false
  return h / Math.max(1, w) > 1.2 && h >= 260
}

function resolvePhonePlaceholderSize(node: PenNode): { width: number; height: number } | null {
  if (node.type !== 'frame') return null
  if (!isPhonePlaceholderFrame(node) && !isPhoneShaped(node)) return null

  const rawWidth = 'width' in node ? node.width : undefined
  const rawHeight = 'height' in node ? node.height : undefined
  const parsedWidth = toSizeNumber(rawWidth, 0)
  const parsedHeight = toSizeNumber(rawHeight, 0)

  const widthValid = parsedWidth >= 220 && parsedWidth <= 360
  const heightValid = parsedHeight >= 440 && parsedHeight <= 760
  const ratio = parsedHeight > 0 && parsedWidth > 0 ? parsedHeight / parsedWidth : 0
  const ratioValid = ratio >= 1.45 && ratio <= 2.7

  let width = 260
  let height = 520
  if (widthValid && heightValid && ratioValid) {
    width = parsedWidth
    height = parsedHeight
  } else if (parsedWidth > 0 && (parsedHeight <= 0 || !heightValid || !ratioValid)) {
    width = clamp(parsedWidth, 240, 320)
    height = clamp(Math.round(width * 2), 500, 620)
  } else if (parsedHeight > 0 && (parsedWidth <= 0 || !widthValid || !ratioValid)) {
    height = clamp(parsedHeight, 500, 620)
    width = clamp(Math.round(height / 2), 240, 320)
  }

  return { width: Math.round(width), height: Math.round(height) }
}

function getPhonePlaceholderNormalizationUpdates(node: PenNode): Record<string, unknown> | null {
  if (node.type !== 'frame') return null
  const normalizedSize = resolvePhonePlaceholderSize(node)
  if (!normalizedSize) return null

  const updates: Record<string, unknown> = {}
  if (node.name !== 'Phone Placeholder') updates.name = 'Phone Placeholder'
  if (node.layout !== 'none') updates.layout = 'none'

  const currentWidth = toSizeNumber('width' in node ? node.width : undefined, 0)
  const currentHeight = toSizeNumber('height' in node ? node.height : undefined, 0)
  if (typeof node.width !== 'number' || Math.abs(currentWidth - normalizedSize.width) > 0.5) {
    updates.width = normalizedSize.width
  }
  if (typeof node.height !== 'number' || Math.abs(currentHeight - normalizedSize.height) > 0.5) {
    updates.height = normalizedSize.height
  }

  if (Array.isArray(node.children) && node.children.length > 0) {
    updates.children = []
  }

  return Object.keys(updates).length > 0 ? updates : null
}

function applyPhonePlaceholderSizingHeuristic(node: PenNode): void {
  const updates = getPhonePlaceholderNormalizationUpdates(node)
  if (!updates) return
  Object.assign(node as unknown as Record<string, unknown>, updates)
}

function applyScreenshotFramePlaceholderHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  const contextMarker = generationContextHint.toLowerCase()
  const contextualScreenshotHint = /(截图|screenshot|mockup|app[-_\s]*screen)/.test(contextMarker)
  const explicitMarker = isExplicitPlaceholderMarker(marker)
  const childScreenshotHint = hasScreenshotHintInChildren(node)
  const ratio = toSizeNumber(node.height, 0) / Math.max(1, toSizeNumber(node.width, 1))
  const tallPhoneLike = ratio > 1.35
  const contextualMarker = contextualScreenshotHint
    && /(preview|showcase|phone|screen|展示|预览|示意)/.test(marker)
    && tallPhoneLike
  const frameWidth = toSizeNumber(node.width, 320)
  const frameHeight = toSizeNumber(node.height, 560)
  const aspectRatio = frameWidth / Math.max(1, frameHeight)
  const wideContainerLike = aspectRatio > 1.35
  const phoneLikeRatio = frameHeight / Math.max(1, frameWidth) > 1.2

  // Shape-based detection: any tall frame with phone-like dimensions is a placeholder
  const shapeMatch = isPhoneShaped(node)

  if (!explicitMarker && !contextualMarker && !childScreenshotHint && !shapeMatch) return

  // Wide containers: strip loose text/image children (AI-generated labels)
  // and keep only frame children (phone frames). The phone frames will be
  // styled by recursion in applyGenerationHeuristics.
  if (wideContainerLike) {
    node.layout = node.layout || 'horizontal'
    node.justifyContent = 'center'
    node.alignItems = 'center'
    if (!node.gap) node.gap = 24
    if (Array.isArray(node.children)) {
      node.children = node.children.filter(child => child.type === 'frame')
    }
    return
  }

  // Skip frames that aren't phone-shaped (roughly square or landscape)
  if (!phoneLikeRatio) return

  // Tall phone-like frame: style as a clean phone placeholder (fill + stroke, no text)
  // Determine theme from current fill BEFORE overriding
  const colors = getPlaceholderColors(node.fill)

  node.name = 'Phone Placeholder'
  node.layout = 'none'
  node.cornerRadius = Math.max(24, toCornerRadiusNumber(node.cornerRadius, 32))
  // Always override fill for theme-adaptive consistency
  node.fill = [{ type: 'solid', color: colors.fillColor }]
  node.stroke = {
    thickness: 1,
    fill: [{ type: 'solid', color: colors.strokeColor }],
  }
  node.effects = [{ type: 'shadow', offsetX: 0, offsetY: 4, blur: 24, spread: 0, color: 'rgba(0,0,0,0.12)' }]

  // Remove all children — clean phone shape only, no text labels
  setNodeChildren(node, [])
}

function isExplicitPlaceholderMarker(text: string): boolean {
  return /app[-_\s]*screen|screenshot|mockup|screen[-_\s]*placeholder|phone|mobile|device|手机|截图/.test(text)
}

function hasScreenshotHintInChildren(node: FrameNode): boolean {
  if (!Array.isArray(node.children)) return false
  for (const child of node.children) {
    if (isExplicitPlaceholderMarker(getNodeMarker(child))) return true
    if (child.type === 'text' && typeof child.content === 'string' && isExplicitPlaceholderMarker(child.content.toLowerCase())) {
      return true
    }
  }
  return false
}

/** Light-touch navbar heuristic — only fills in missing layout defaults.
 *  Does NOT restructure children or force widths. The AI already handles
 *  navbar layout well; heavy-handed overrides degrade quality. */
function applyNavbarHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (!/(^|[\s_-])(nav|navbar|navigation|header)([\s_-]|$)|导航|顶部/.test(marker)) return

  if (!node.layout || node.layout === 'none') node.layout = 'horizontal'
  if (node.layout !== 'horizontal') return
  if (!node.alignItems) node.alignItems = 'center'
  if (!node.justifyContent) node.justifyContent = 'space_between'
}

function isInnerWrapperMarker(node: PenNode): boolean {
  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  return /(^|[\s_-])inner([\s_-]|$)|内层/.test(marker)
}

function hasMeaningfulFrameVisualStyle(node: PenNode): boolean {
  if (node.type !== 'frame') return false

  const hasFill = Array.isArray(node.fill) && node.fill.length > 0
  if (hasFill) return true

  const strokeW = toStrokeThicknessNumber(node.stroke, 0)
  if (strokeW > 0) return true

  const cr = toCornerRadiusNumber(node.cornerRadius, 0)
  if (cr > 0) return true

  if (Array.isArray(node.effects) && node.effects.length > 0) return true
  if (node.clipContent === true) return true
  if (typeof node.opacity === 'number' && node.opacity < 0.999) return true
  if (typeof node.rotation === 'number' && Math.abs(node.rotation) > 0.01) return true

  return false
}

interface InnerWrapperFlattenUpdates {
  children: PenNode[]
  layout?: FrameNode['layout']
  alignItems?: FrameNode['alignItems']
  justifyContent?: FrameNode['justifyContent']
  gap?: FrameNode['gap']
  padding?: FrameNode['padding']
}

function getSingleInnerWrapperFlattenUpdates(
  parent: PenNode,
): InnerWrapperFlattenUpdates | null {
  if (parent.type !== 'frame') return null
  if (!Array.isArray(parent.children) || parent.children.length !== 1) return null
  if (isBadgeLikeFrame(parent) || isCompactButtonLikeFrame(parent)) return null

  const inner = parent.children[0]
  if (inner.type !== 'frame') return null
  if (!Array.isArray(inner.children) || inner.children.length === 0) return null
  if (!isInnerWrapperMarker(inner)) return null
  if (isPhoneShaped(inner) || inner.name === 'Phone Placeholder') return null
  if (hasMeaningfulFrameVisualStyle(inner)) return null

  // Keep explicit fixed-width wrappers: they often act as deliberate content max-width containers.
  if (typeof inner.width === 'number' && inner.width > 0) return null

  const parentLayout = parent.layout
  const innerLayout = inner.layout
  if (parentLayout && parentLayout !== 'none' && innerLayout && innerLayout !== 'none' && parentLayout !== innerLayout) {
    return null
  }

  const updates: InnerWrapperFlattenUpdates = {
    children: inner.children,
  }

  if ((!parentLayout || parentLayout === 'none') && innerLayout && innerLayout !== 'none') {
    updates.layout = innerLayout
  }
  if (!parent.alignItems && inner.alignItems) updates.alignItems = inner.alignItems
  if (!parent.justifyContent && inner.justifyContent) updates.justifyContent = inner.justifyContent

  const parentGap = toGapNumber('gap' in parent ? parent.gap : undefined)
  const innerGap = toGapNumber('gap' in inner ? inner.gap : undefined)
  if (parentGap <= 0 && innerGap > 0) updates.gap = inner.gap

  const parentPad = parsePaddingValues('padding' in parent ? parent.padding : undefined)
  const innerPad = parsePaddingValues('padding' in inner ? inner.padding : undefined)
  const parentPadEmpty = parentPad.top === 0 && parentPad.right === 0 && parentPad.bottom === 0 && parentPad.left === 0
  const innerPadNonEmpty = innerPad.top !== 0 || innerPad.right !== 0 || innerPad.bottom !== 0 || innerPad.left !== 0
  if (parentPadEmpty && innerPadNonEmpty) updates.padding = inner.padding

  // For non-layout parents, preserve absolute positioning of promoted children.
  const parentIsLayout = !!(updates.layout ?? parent.layout) && (updates.layout ?? parent.layout) !== 'none'
  if (!parentIsLayout) {
    const ox = typeof inner.x === 'number' ? inner.x : 0
    const oy = typeof inner.y === 'number' ? inner.y : 0
    if (ox !== 0 || oy !== 0) {
      updates.children = inner.children.map((child) => {
        const lifted: PenNode = { ...child }
        if (typeof child.x === 'number') lifted.x = child.x + ox
        else if (ox !== 0) lifted.x = ox
        if (typeof child.y === 'number') lifted.y = child.y + oy
        else if (oy !== 0) lifted.y = oy
        return lifted
      })
    }
  }

  return updates
}

function isBadgeLikeFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (!Array.isArray(node.children) || node.children.length === 0 || node.children.length > 3) return false

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  const explicitBadgeMarker = /(badge|tag|chip|pill|label|徽章|标签|状态|模式)/.test(marker)
  const explicitButtonMarker = /(^|[\s_-])(button|btn|cta|submit|download|install|signup|sign[-_\s]*in|login|register)([\s_-]|$)|按钮|下载|立即|开始|购买|了解|进入|登录|注册|提交|继续|安装|试用/.test(marker)
  if (explicitButtonMarker && !explicitBadgeMarker) return false

  const h = toSizeNumber(node.height, 0)
  if (h > 0 && h > 40) return false

  const textChildren = node.children.filter(
    (c) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
  )
  if (textChildren.length !== 1) return false
  const textNode = textChildren[0]
  if (textNode.type !== 'text' || typeof textNode.content !== 'string') return false
  const label = textNode.content.trim()
  const lowerLabel = label.toLowerCase()
  const ctaLikeLabel = /(download|get\s*it\s*on|learn\s*more|start|continue|open|install|try|buy|sign\s*in|log\s*in|register|submit|app\s*store|google\s*play|免费下载|立即下载|下载|了解|查看|开始|继续|进入|安装|试用|购买|登录|注册|提交|前往|打开)/.test(lowerLabel)
  if (ctaLikeLabel && !explicitBadgeMarker) return false
  const charCount = [...label].length
  const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(label)
  const fontSize = textNode.fontSize ?? 14
  const maxLabelChars = hasCjk ? 10 : 18
  if (charCount > maxLabelChars || fontSize > 16) return false

  const allowedChildren = node.children.every((c) =>
    c.type === 'text' || c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle',
  )
  if (!allowedChildren) return false

  const iconCount = node.children.filter((c) =>
    c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle',
  ).length
  if (iconCount > 1) return false
  const hasLargeIcon = node.children.some((c) => {
    if (!(c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle')) return false
    const iw = toSizeNumber('width' in c ? c.width : undefined, 0)
    const ih = toSizeNumber('height' in c ? c.height : undefined, 0)
    return Math.max(iw, ih) >= 16
  })
  if (hasLargeIcon && !explicitBadgeMarker) return false

  const w = toSizeNumber(node.width, 0)
  if (w > 0 && w > 260) return false
  const looksButtonByWidth = w >= 140 && (
    node.layout === 'horizontal' || node.alignItems === 'center' || node.justifyContent === 'center'
  )
  if (looksButtonByWidth && ctaLikeLabel && !explicitBadgeMarker) return false

  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const inferredH = h > 0 ? h : Math.round(fontSize * 1.2 + pad.top + pad.bottom)
  if (inferredH > 40) return false

  return node.layout === 'horizontal'
    || node.alignItems === 'center'
    || node.justifyContent === 'center'
}

function isCompactButtonLikeFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (!Array.isArray(node.children) || node.children.length === 0 || node.children.length > 4) return false
  if (isBadgeLikeFrame(node)) return false

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(card|panel|section|container|feature|service|list|item|卡片|面板|容器|功能)/.test(marker)) return false

  const h = toSizeNumber(node.height, 0)
  if (h <= 0 || h > 72) return false
  if (node.layout === 'vertical') return false

  const textChildren = node.children.filter(
    (c) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
  )
  if (textChildren.length === 0 || textChildren.length > 2) return false
  for (const textNode of textChildren) {
    if (textNode.type !== 'text') return false
    const content = getTextContentForNode(textNode).trim()
    const fs = textNode.fontSize ?? 16
    const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(content)
    const compactLen = content.replace(/\s+/g, '').length
    if (fs > 24) return false
    if (/[\n\r]/.test(content)) return false
    if (compactLen > (hasCjk ? 14 : 26)) return false
  }

  const hasNestedFrames = node.children.some((c) => c.type === 'frame')
  if (hasNestedFrames) return false

  const centeredLike = node.layout === 'horizontal'
    || node.alignItems === 'center'
    || node.justifyContent === 'center'
  if (!centeredLike) return false

  const hasIcon = node.children.some((c) =>
    c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle' || c.type === 'image',
  )
  const explicitButtonMarker = /(^|[\s_-])(button|btn|cta|submit|download|install|signup|sign[-_\s]*in|login|register)([\s_-]|$)|按钮|下载|立即|开始|购买|了解|进入|登录|注册|提交|继续|安装|试用|app\s*store|google\s*play/.test(marker)
  return explicitButtonMarker || hasIcon || textChildren.length === 1
}

function isCompactControlFrame(node: PenNode): boolean {
  return isBadgeLikeFrame(node) || isCompactButtonLikeFrame(node)
}

function isLongTextLikeForLayout(node: PenNode): boolean {
  if (node.type !== 'text') return false
  const text = getTextContentForNode(node).trim()
  if (!text) return false
  if (/[\n\r]/.test(text)) return true
  if (node.textGrowth === 'fixed-width' || node.textGrowth === 'fixed-width-height') return true
  const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(text)
  const compactLen = text.replace(/\s+/g, '').length
  return compactLen >= (hasCjk ? 9 : 18)
}

function shouldPromoteTextWidthToFillInLayout(parent: PenNode, child: PenNode): boolean {
  if (parent.type !== 'frame' || child.type !== 'text') return false
  if (parent.layout !== 'vertical') return false
  if (parent.width === 'fit_content') return false
  if (isBadgeLikeFrame(parent) || isCompactButtonLikeFrame(parent)) return false

  const widthValue = child.width
  if (typeof widthValue === 'string' && widthValue.startsWith('fill_container')) return false

  // fixed-width text must have a usable width constraint in vertical layouts
  if (child.textGrowth === 'fixed-width' || child.textGrowth === 'fixed-width-height') {
    return true
  }

  if (isLongTextLikeForLayout(child)) {
    return true
  }

  // Narrow explicit widths in vertical cards frequently cause over-wrapping.
  if (typeof widthValue === 'number' && widthValue > 0) {
    const content = getTextContentForNode(child).trim()
    if (!content) return false
    const hasCjk = /[\u3400-\u4DBF\u4E00-\u9FFF\u3040-\u30FF\uAC00-\uD7AF]/.test(content)
    const compactLen = content.replace(/\s+/g, '').length
    return compactLen >= (hasCjk ? 7 : 14)
  }

  return false
}

function shouldUseFixedWidthTextGrowthInLayout(parent: PenNode, child: PenNode): boolean {
  if (child.type !== 'text') return false
  if (child.textGrowth === 'fixed-width' || child.textGrowth === 'fixed-width-height') return false
  return shouldPromoteTextWidthToFillInLayout(parent, child) || isLongTextLikeForLayout(child)
}

function frameNeedsFillWidthForTextContent(node: PenNode, depth = 0): boolean {
  if (node.type !== 'frame') return false
  if (isBadgeLikeFrame(node) || isCompactButtonLikeFrame(node)) return false
  if (!Array.isArray(node.children) || node.children.length === 0) return false

  for (const child of node.children) {
    if (child.type === 'text' && isLongTextLikeForLayout(child)) {
      return true
    }
    if (depth < 2 && child.type === 'frame' && frameNeedsFillWidthForTextContent(child, depth + 1)) {
      return true
    }
  }
  return false
}

/**
 * Horizontal inline rows (icon + text, badges, tag rows) should default to
 * alignItems="center" for vertical centering. Without this, AI sometimes
 * generates "start" or "end" alignment, causing icon/text vertical misalignment.
 * Only targets small-height horizontal frames — large containers (multi-column
 * layouts) are left unchanged.
 */
function applyHorizontalAlignCenterHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  if (node.layout !== 'horizontal') return
  // Skip large containers (likely multi-column layouts, not inline rows)
  const h = toSizeNumber(node.height, 0)
  if (h > 100) return
  // Skip if already centered
  if (node.alignItems === 'center') return
  // Skip navbar — handled separately with its own alignment rules
  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(^|[\s_-])(nav|navbar|navigation|header)([\s_-]|$)|导航|顶部/.test(marker)) return

  node.alignItems = 'center'
}

/**
 * Icon-only buttons (heart, bookmark, share, etc.) — ensure minimum square sizing.
 * AI often makes these too small (e.g. 24x24) making them untappable/invisible.
 * Detects: frame with only path/icon children, no text, small size → enforce 40x40 min.
 */
function applyIconButtonSizing(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!Array.isArray(node.children) || node.children.length === 0) return
  const w = toSizeNumber(node.width, 0)
  const h = toSizeNumber(node.height, 0)
  // Must be a small frame (likely button-sized or too small)
  if (w > 80 || h > 80) return
  // Must have only icon-like children (path, rectangle, ellipse) — NO text
  const hasText = node.children.some((c) => c.type === 'text')
  if (hasText) return
  const hasIcon = node.children.some((c) =>
    c.type === 'path' || c.type === 'rectangle' || c.type === 'ellipse',
  )
  if (!hasIcon) return

  // This is an icon-only button — enforce minimum 40x40
  const minSize = 40
  if (typeof node.width === 'number' && w < minSize) node.width = minSize
  if (typeof node.height === 'number' && h < minSize) node.height = minSize
  // Ensure centered
  if (!node.justifyContent) node.justifyContent = 'center'
  if (!node.alignItems) node.alignItems = 'center'
}

/**
 * Badge/tag frames (e.g. "NEW", "SALE", "PRO") — ensure minimum padding
 * and prevent text clipping. Detects: small frame with very short text child.
 */
function applyBadgeSizing(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!isBadgeLikeFrame(node)) return
  if (!Array.isArray(node.children)) return

  if (typeof node.width === 'string' && node.width.startsWith('fill_container')) {
    node.width = 'fit_content'
  }

  const textNode = node.children.find(
    (c: PenNode) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
  )
  if (!textNode || textNode.type !== 'text' || typeof textNode.content !== 'string') return

  const text = textNode.content.trim()
  const fontSize = textNode.fontSize ?? 14
  const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF]/.test(text)
  const textColor = extractPrimaryColor(textNode.fill) ?? '#4C72DF'
  const iconChildren = node.children.filter((c: PenNode) =>
    c.type === 'path' || c.type === 'ellipse' || c.type === 'rectangle',
  )

  if (!node.layout || node.layout === 'none') node.layout = 'horizontal'
  node.alignItems = 'center'
  if (!node.justifyContent) node.justifyContent = 'center'

  const currentGap = toGapNumber('gap' in node ? node.gap : undefined)
  const minGap = iconChildren.length > 0 ? 6 : 4
  if (currentGap < minGap) node.gap = minGap
  else if (currentGap > 14) node.gap = 8

  // Badge text should remain single-line and optically centered.
  const targetLineHeight = hasCjk ? 1.25 : 1.2
  if (!textNode.lineHeight || textNode.lineHeight < 1.05 || textNode.lineHeight > 1.35) {
    textNode.lineHeight = targetLineHeight
  }
  textNode.textAlignVertical = 'middle'
  if (!textNode.textGrowth || textNode.textGrowth === 'fixed-width') {
    textNode.textGrowth = 'auto'
  }

  // Ensure minimum padding so text/icon never clips in capsules.
  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const minV = 6
  const minH = iconChildren.length > 0 ? 14 : 12
  if (pad.top < minV || pad.bottom < minV || pad.left < minH || pad.right < minH) {
    node.padding = [
      Math.max(pad.top, minV),
      Math.max(pad.right, minH),
      Math.max(pad.bottom, minV),
      Math.max(pad.left, minH),
    ]
  }

  // Ensure text has proper sizing — don't clip.
  const estimatedW = estimateSingleLineTextWidth(text, fontSize)
  const frameW = toSizeNumber(node.width, 0)
  const newPad = parsePaddingValues(node.padding)
  const minFrameW = Math.round(estimatedW + newPad.left + newPad.right + 8)
  if (typeof node.width === 'number' && frameW > 0 && frameW < minFrameW) {
    node.width = minFrameW
  }

  // Keep badge height compact and stable.
  const h = toSizeNumber(node.height, 0)
  const minFrameH = Math.round(fontSize * (textNode.lineHeight ?? targetLineHeight) + newPad.top + newPad.bottom)
  if (typeof node.height === 'number' && h > 0 && h < minFrameH) {
    node.height = minFrameH
  }

  // Capsule radius: half of badge height.
  const effectiveH = toSizeNumber(node.height, h > 0 ? h : minFrameH)
  if (effectiveH > 0) {
    node.cornerRadius = Math.max(1, Math.round(effectiveH / 2))
  }

  // Normalize icon children so they always render and stay visually balanced.
  for (const child of iconChildren) {
    if (child.type === 'path') {
      const target = clamp(
        toSizeNumber(child.width, toSizeNumber(child.height, Math.round(fontSize * 0.95))),
        10,
        16,
      )
      const cw = toSizeNumber(child.width, 0)
      const ch = toSizeNumber(child.height, 0)
      if (cw <= 0 || cw > 20) child.width = target
      if (ch <= 0 || ch > 20) child.height = target

      const strokeWidth = toStrokeThicknessNumber(child.stroke, 0)
      const hasFill = Array.isArray(child.fill) && child.fill.length > 0
      if (!hasFill && strokeWidth <= 0) {
        child.fill = [{ type: 'solid', color: textColor }]
      }
      if (child.stroke && strokeWidth <= 0) {
        child.stroke = { thickness: 1.8, fill: [{ type: 'solid', color: textColor }] }
      } else if (child.stroke && !extractPrimaryColor(child.stroke.fill)) {
        child.stroke.fill = [{ type: 'solid', color: textColor }]
      }
    } else {
      const cw = toSizeNumber('width' in child ? child.width : undefined, 0)
      const ch = toSizeNumber('height' in child ? child.height : undefined, 0)
      const dotSize = clamp(Math.round(fontSize * 0.72), 8, 12)
      if ('width' in child && (cw <= 0 || cw > 16)) child.width = dotSize
      if ('height' in child && (ch <= 0 || ch > 16)) child.height = dotSize
      if (!('fill' in child) || !Array.isArray(child.fill) || child.fill.length === 0) {
        ;(child as unknown as { fill?: Array<{ type: 'solid'; color: string }> }).fill = [{ type: 'solid', color: textColor }]
      }
    }
  }
}

/**
 * Ensure button-like frames have minimum internal padding and gap.
 * AI often generates buttons with padding: 0 or tiny padding, causing cramped text.
 * Also ensures a minimum gap when icon + text coexist in a horizontal button.
 */
function applyButtonSpacingHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!Array.isArray(node.children) || node.children.length === 0) return
  // Badge chips use tighter spacing; button defaults make them look oversized.
  if (isBadgeLikeFrame(node)) return
  const h = toSizeNumber(node.height, 0)
  // Only target small frames that look like buttons/badges (height ≤ 72px)
  if (h <= 0 || h > 72) return
  const isHorizontal = node.layout === 'horizontal'
  const isCentered = node.alignItems === 'center' || node.justifyContent === 'center'
  if (!isHorizontal && !isCentered) return

  const hasText = node.children.some(
    (c) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
  )
  if (!hasText) return

  // Ensure minimum padding
  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const minV = 8
  const minH = 16
  if (pad.top < minV || pad.bottom < minV || pad.left < minH || pad.right < minH) {
    const newTop = Math.max(pad.top, minV)
    const newBottom = Math.max(pad.bottom, minV)
    const newLeft = Math.max(pad.left, minH)
    const newRight = Math.max(pad.right, minH)
    if (newTop === newBottom && newLeft === newRight) {
      node.padding = [newTop, newLeft]
    } else {
      node.padding = [newTop, newRight, newBottom, newLeft]
    }
  }

  // Ensure minimum gap when there are multiple children (e.g. icon + text)
  if (node.children.length >= 2 && isHorizontal) {
    const currentGap = toGapNumber('gap' in node ? node.gap : undefined)
    if (currentGap < 8) {
      node.gap = 8
    }
  }

  // Ensure minimum height for buttons
  const fontSize = node.children.reduce((max, c) => {
    if (c.type === 'text') return Math.max(max, c.fontSize ?? 16)
    return max
  }, 0)
  const minHeight = Math.max(36, Math.round(fontSize * 2.4))
  if (h < minHeight) {
    node.height = minHeight
  }
}

/**
 * Ensure button/badge-like frames are wide enough for their text content.
 * AI often generates fixed widths based on Latin text estimates, but CJK characters
 * are ~1.7× wider per char, causing wrapping or clipping inside buttons and badges.
 */
function applyButtonWidthHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!Array.isArray(node.children) || node.children.length === 0) return
  // Only target fixed-width frames (buttons/badges/tags/feature cards)
  if (typeof node.width !== 'number') return
  const w = node.width
  const h = toSizeNumber(node.height, 0)
  if (w <= 0 || h <= 0 || h > 160) return
  // Must look like a button: centered or has horizontal layout
  const isCentered = node.alignItems === 'center' || node.justifyContent === 'center'
  const isHorizontal = node.layout === 'horizontal'
  if (!isCentered && !isHorizontal) return

  // Gather text children
  const hasText = node.children.some(
    (c) => c.type === 'text' && typeof c.content === 'string' && c.content.trim().length > 0,
  )
  if (!hasText) return

  const padding = parsePaddingValues('padding' in node ? node.padding : undefined)
  const gap = toGapNumber('gap' in node ? node.gap : undefined)

  // Sum up width needed for all children
  let contentWidth = 0
  for (const child of node.children) {
    if (child.type === 'text' && typeof child.content === 'string') {
      contentWidth += estimateSingleLineTextWidth(child.content.trim(), child.fontSize ?? 16)
    } else {
      contentWidth += toSizeNumber('width' in child ? child.width : undefined, 20)
    }
  }
  if (node.children.length > 1) {
    contentWidth += gap * (node.children.length - 1)
  }

  // 24px safety margin — accounts for rounding and sub-pixel font rendering
  const minWidth = Math.round(contentWidth + padding.left + padding.right + 24)
  if (w < minWidth) {
    node.width = minWidth
  }
}

function getTextContentForNode(node: PenNode): string {
  if (node.type !== 'text') return ''
  return typeof node.content === 'string'
    ? node.content
    : Array.isArray(node.content)
      ? node.content.map((s: { text: string }) => s.text).join('')
      : ''
}

function isLongBodyTextNode(node: PenNode): boolean {
  if (node.type !== 'text') return false
  const text = getTextContentForNode(node).trim()
  if (!text) return false
  const fontSize = node.fontSize ?? 16
  if (fontSize > 24) return false
  const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF]/.test(text)
  const compactLen = text.replace(/\s+/g, '').length
  return compactLen >= (hasCjk ? 14 : 28)
}

function estimateParentContentWidthForText(
  parent: PenNode,
  padding: { top: number; right: number; bottom: number; left: number },
  parentWidth: number,
): number {
  if (parentWidth > 0) {
    return Math.max(0, parentWidth - padding.left - padding.right)
  }

  const parentWidthValue = 'width' in parent ? parent.width : undefined
  const isFillContainer = typeof parentWidthValue === 'string'
    && parentWidthValue.startsWith('fill_container')
  if (!isFillContainer) return 0

  const isMobile = generationCanvasWidth <= 520
  const estimatedParentW = isMobile
    ? Math.max(260, generationCanvasWidth - 32)
    : Math.max(360, generationCanvasWidth * 0.68)
  const safePadL = Math.min(padding.left, Math.round(estimatedParentW * 0.2))
  const safePadR = Math.min(padding.right, Math.round(estimatedParentW * 0.2))
  return Math.max(120, estimatedParentW - safePadL - safePadR)
}

function getLongTextPaddingAdjustment(
  parent: PenNode,
  padding: { top: number; right: number; bottom: number; left: number },
  parentWidth: number,
): [number, number, number, number] | null {
  if (parent.type !== 'frame') return null
  if (parent.layout !== 'vertical') return null
  if (!Array.isArray(parent.children) || parent.children.length === 0) return null
  if (isBadgeLikeFrame(parent)) return null

  const frameH = toSizeNumber('height' in parent ? parent.height : undefined, 0)
  if (frameH > 0 && frameH <= 120) return null
  if (!parent.children.some((child) => isLongBodyTextNode(child))) return null

  const effectiveParentW = parentWidth > 0
    ? parentWidth
    : estimateParentContentWidthForText(parent, { top: 0, right: 0, bottom: 0, left: 0 }, 0)
  if (effectiveParentW <= 0) return null

  const contentW = Math.max(0, effectiveParentW - padding.left - padding.right)
  const minContentW = Math.max(170, Math.round(effectiveParentW * 0.72))
  if (contentW >= minContentW) return null

  const maxHorizontalPad = clamp(Math.round(effectiveParentW * 0.1), 14, 24)
  const newLeft = Math.min(padding.left, maxHorizontalPad)
  const newRight = Math.min(padding.right, maxHorizontalPad)
  if (newLeft === padding.left && newRight === padding.right) return null
  return [padding.top, newRight, padding.bottom, newLeft]
}

function estimateWrappingLineWidth(text: string, fontSize: number): number {
  let width = 0
  for (const char of text) {
    if (/[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF\uFE30-\uFE4F]/.test(char)) {
      width += fontSize * 1.12
    } else if (char === ' ') {
      width += fontSize * 0.3
    } else {
      width += fontSize * 0.58
    }
  }
  return width
}

/** Estimate pixel width of text on a single line, with per-character width factors.
 *  CJK characters use 1.15× fontSize (not 1.0×) because actual font metrics,
 *  letter-spacing, and rendering variations make CJK glyphs slightly wider
 *  than the nominal 1em. English at 0.6× is accurate — this CJK boost
 *  prevents the under-estimation that causes text clipping in buttons/cards. */
/**
 * Estimate auto-height for text with fill_container width.
 * When parentContentWidth is provided, uses it directly for accurate wrapping.
 * Otherwise uses a canvas-aware fallback width (mobile/desktop tuned) to avoid
 * over-estimating line count when parent width is still unresolved.
 */
function estimateAutoHeight(
  text: string,
  fontSize: number,
  lineHeight: number,
  parentContentWidth?: number,
): number {
  // CJK text needs larger minimum lineHeight — CJK characters are taller
  const hasCjk = /[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F]/.test(text)
  const minLh = hasCjk
    ? (fontSize >= 28 ? 1.28 : 1.5)
    : (fontSize >= 28 ? 1.15 : 1.35)
  const effectiveLh = Math.max(lineHeight, minLh)

  const availW = parentContentWidth && parentContentWidth > 0
    ? Math.max(120, parentContentWidth)
    : (
        generationCanvasWidth <= 520
          ? Math.max(220, generationCanvasWidth * 0.74)
          : Math.max(260, generationCanvasWidth * 0.5)
      )

  const logicalLines = text.split(/\r?\n/)
  const wrappedLineCount = logicalLines.reduce((sum, line) => {
    const lineWidth = estimateWrappingLineWidth(line, fontSize)
    return sum + Math.max(1, Math.ceil(lineWidth / availW))
  }, 0)
  // Add 10% safety margin to prevent tight clipping without making boxes too tall.
  return Math.round(Math.max(1, wrappedLineCount) * fontSize * effectiveLh * 1.1)
}

function estimateSingleLineTextWidth(text: string, fontSize: number): number {
  let width = 0
  for (const char of text) {
    if (/[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF\uFE30-\uFE4F]/.test(char)) {
      width += fontSize * 1.35 // CJK full-width + font metrics + kerning margin
    } else if (char === ' ') {
      width += fontSize * 0.3
    } else {
      width += fontSize * 0.6 // Latin
    }
  }
  return width
}

/**
 * In vertical layout containers (forms, cards, panels):
 * 1. Primary action buttons that clearly overflow → fill_container
 * 2. Horizontal button rows (social login etc.) → row gets fill_container
 * 3. Only touch children that actually overflow — respect the AI's sizing choices
 */
function applyFormChildFillContainer(node: PenNode): void {
  if (node.type !== 'frame') return
  if (node.layout !== 'vertical') return
  // Never convert children when parent is fit_content (hug) — breaks layout
  if (node.width === 'fit_content') return
  if (!Array.isArray(node.children) || node.children.length < 2) return

  const parentW = toSizeNumber(node.width, 0)
  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const contentW = parentW > 0 ? parentW - pad.left - pad.right : 0

  // Check if any sibling frame already uses fill_container —
  // if so, fixed-width siblings should align by also using fill_container.
  const hasFillSibling = node.children.some((c) =>
    c.type === 'frame'
    && c.width === 'fill_container'
    && !isPhonePlaceholderFrame(c)
    && !isDividerLikeFrame(c),
  )

  for (const child of node.children) {
    if (child.type !== 'frame') continue
    if (isPhonePlaceholderFrame(child)) continue
    if (isDividerLikeFrame(child)) continue
    if (!('width' in child)) continue
    const childWidthValue = (child as { width?: number | string }).width
    const childWidthIsNumber = typeof childWidthValue === 'number'
    const childWidthIsFit = typeof childWidthValue === 'string' && childWidthValue.startsWith('fit_content')
    if (!childWidthIsNumber && !childWidthIsFit) continue

    const childW = childWidthIsNumber ? toSizeNumber(childWidthValue, 0) : 0
    const childH = toSizeNumber('height' in child ? child.height : undefined, 0)

    // Long-text wrappers should use fill_container, otherwise they collapse to content width.
    if (
      childWidthIsFit
      && !isBadgeLikeFrame(child)
      && !isCompactButtonLikeFrame(child)
      && frameNeedsFillWidthForTextContent(child)
    ) {
      ;(child as unknown as Record<string, unknown>).width = 'fill_container'
      continue
    }

    // Overflow: child wider than parent content area
    if (contentW > 0 && childW > contentW) {
      ;(child as unknown as Record<string, unknown>).width = 'fill_container'
      continue
    }

    // Narrow fixed-width text wrappers in wide vertical containers cause
    // severe over-wrapping and "squeezed" cards. Promote them to fill width.
    if (
      contentW > 0
      && childWidthIsNumber
      && childW > 0
      && childW < contentW * 0.72
      && !isBadgeLikeFrame(child)
      && !isCompactButtonLikeFrame(child)
      && frameNeedsFillWidthForTextContent(child)
    ) {
      ;(child as unknown as Record<string, unknown>).width = 'fill_container'
      continue
    }

    // Consistency: if a sibling already uses fill_container,
    // convert input/button-like children (short height, has content) too
    if (hasFillSibling && childH > 0 && childH <= 72 && !isCompactControlFrame(child)) {
      const hasContent = 'children' in child && Array.isArray(child.children)
        && child.children.some((gc) => gc.type === 'text' || gc.type === 'path')
      if (hasContent) {
        ;(child as unknown as Record<string, unknown>).width = 'fill_container'
        continue
      }
    }

    // Horizontal button row (e.g. social login: Google | Apple | GitHub)
    if (child.layout === 'horizontal'
      && 'children' in child && Array.isArray(child.children)
      && child.children.length >= 2) {
      const allButtonLike = child.children.every((gc) => isCompactButtonLikeFrame(gc))
      if (allButtonLike) {
        // Row should fill parent width
        ;(child as unknown as Record<string, unknown>).width = 'fill_container'
        // Ensure the row has proper distribution
        if (!child.justifyContent || child.justifyContent === 'start') {
          child.justifyContent = 'center'
        }
        if (!child.gap || toGapNumber(child.gap) < 8) {
          child.gap = 12
        }
      }
    }
  }
}

/**
 * When children of a horizontal layout overflow the parent's width:
 * 1. Try reducing gap first (minimal visual change)
 * 2. Expand parent if still fixable
 * 3. Switch parent to fill_container as last resort
 */
function applyHorizontalOverflowFix(node: PenNode): void {
  if (node.type !== 'frame') return
  if (node.layout !== 'horizontal') return
  if (!Array.isArray(node.children) || node.children.length < 2) return

  const parentW = toSizeNumber(node.width, 0)
  // Skip if parent uses flex sizing — layout engine handles it
  if (typeof node.width !== 'number' || parentW <= 0) return

  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const gap = toGapNumber('gap' in node ? node.gap : undefined)
  const availW = parentW - pad.left - pad.right

  // Sum up children's widths (estimate intrinsic width for each)
  let childrenTotalW = 0
  for (const child of node.children) {
    const cw = toSizeNumber('width' in child ? (child as { width?: number | string }).width : undefined, 0)
    if ('width' in child && typeof (child as { width?: number | string }).width === 'number' && cw > 0) {
      childrenTotalW += cw
    } else {
      childrenTotalW += 80
    }
  }
  const gapTotal = gap * (node.children.length - 1)
  childrenTotalW += gapTotal

  if (childrenTotalW <= availW) return // No overflow

  // Strategy 1: Reduce gap to fit (try gap=8, then gap=4)
  for (const tryGap of [8, 4]) {
    if (gap > tryGap) {
      const withReducedGap = childrenTotalW - gapTotal + tryGap * (node.children.length - 1)
      if (withReducedGap <= availW) {
        node.gap = tryGap
        return
      }
    }
  }

  // Strategy 2: Expand parent width to fit children
  const neededW = Math.round(childrenTotalW + pad.left + pad.right)
  if (neededW > parentW && neededW <= generationCanvasWidth) {
    node.width = neededW
    return
  }

  // Strategy 3: Parent is too narrow for expansion → fill_container
  if (neededW > generationCanvasWidth * 0.8) {
    node.width = 'fill_container' as unknown as number
  }
}

/**
 * After children are processed, expand frame height to fit content.
 * Prevents card/container content clipping when AI-generated height is too small
 * for the actual text content (especially CJK text which wraps more aggressively).
 */
function applyFrameHeightExpansion(node: PenNode): void {
  if (node.type !== 'frame') return
  // Only expand frames with explicit pixel height (not flex sizing)
  if (typeof node.height !== 'number') return
  if (!node.layout || node.layout === 'none') return
  if (!Array.isArray(node.children) || node.children.length === 0) return

  const intrinsic = estimateNodeIntrinsicHeight(node)
  if (intrinsic > node.height) {
    node.height = Math.round(intrinsic)
  }
}

/**
 * Ensure section-level frames have minimum horizontal padding so content
 * doesn't flush against the edge. A "section frame" is detected by:
 * - fill_container width (stretches to page width)
 * - Has a layout (vertical/horizontal)
 * - Has direct children with visible content (text, buttons, etc.)
 */
function applySectionPaddingHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!isLikelyWideSectionFrame(node)) return
  if (!node.layout || node.layout === 'none') return
  if (!Array.isArray(node.children) || node.children.length === 0) return

  // Skip navbars — they typically manage their own padding
  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(nav|navbar|navigation|header|footer|导航|顶部|底部)/.test(marker)) return

  // Check if this frame has text or button-like content as direct or shallow children
  const hasContent = node.children.some((c) =>
    c.type === 'text'
    || (c.type === 'frame' && 'children' in c && Array.isArray(c.children)
        && c.children.some((gc: PenNode) => gc.type === 'text')),
  )
  if (!hasContent) return

  const pad = parsePaddingValues('padding' in node ? node.padding : undefined)
  const isMobile = generationCanvasWidth <= 480
  const minHorizontal = isMobile ? 16 : (generationCanvasWidth <= 1024 ? 20 : 24)

  if (pad.left < minHorizontal || pad.right < minHorizontal) {
    const newLeft = Math.max(pad.left, minHorizontal)
    const newRight = Math.max(pad.right, minHorizontal)
    // Preserve vertical padding, upgrade horizontal
    if (pad.top === pad.bottom && newLeft === newRight) {
      node.padding = [pad.top, newLeft]
    } else {
      node.padding = [pad.top, newRight, pad.bottom, newLeft]
    }
  }
}

function isLikelyWideSectionFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (node.width !== 'fill_container') return false

  const marker = `${node.name ?? ''} ${node.id}`.toLowerCase()
  if (/(card|panel|tile|feature|service|item|badge|chip|pill|tag|button|cta|modal|dialog|toast|tooltip|tab|卡片|面板|徽章|标签|按钮|弹窗)/.test(marker)) {
    return false
  }

  const h = toSizeNumber(node.height, 0)
  if (h > 0 && h < 220 && Array.isArray(node.children) && node.children.length <= 2) {
    return false
  }

  return true
}

/**
 * Remove decorative glow/shadow/backdrop frames that are siblings of a phone
 * placeholder. AI often generates empty "Glow BG" or "Shadow" frames alongside
 * phone mockups — they add visual noise and serve no purpose.
 */
function applyRemoveDecorativeGlowSiblings(node: PenNode): void {
  if (node.type !== 'frame') return
  if (!Array.isArray(node.children) || node.children.length < 2) return

  const hasPhone = node.children.some(
    (c: PenNode) => c.name === 'Phone Placeholder'
      || (c.type === 'frame' && isPhoneShaped(c)),
  )
  if (!hasPhone) return

  node.children = node.children.filter((child: PenNode) => {
    if (child.name === 'Phone Placeholder') return true
    if (child.type === 'frame' && isPhoneShaped(child)) return true
    const marker = `${child.name ?? ''} ${child.id}`.toLowerCase()
    if (!/(glow|shadow|backdrop|blur|bg\b|background|overlay|光|阴影)/.test(marker)) return true
    // Decorative name — keep only if it has meaningful content (text or image)
    const kids = 'children' in child ? child.children : undefined
    if (!Array.isArray(kids) || kids.length === 0) return false
    return kids.some((c: PenNode) => c.type === 'text' || c.type === 'image')
  })
}

function isPhoneShaped(node: PenNode): boolean {
  const w = toSizeNumber('width' in node ? node.width : 0, 0)
  const h = toSizeNumber('height' in node ? node.height : 0, 0)
  if (w <= 0 || h <= 0) return false
  const ratio = h / w
  const cr = toCornerRadiusNumber('cornerRadius' in node ? node.cornerRadius : 0, 0)
  return ratio > 1.4 && w >= 200 && w <= 400 && cr >= 20
}

/** Auto-apply clipContent on frames with cornerRadius + image/overflow children. */
function applyClipContentHeuristic(node: PenNode): void {
  if (node.type !== 'frame') return
  if ('clipContent' in node && node.clipContent) return // already set
  const cr = toCornerRadiusNumber(node.cornerRadius, 0)
  if (cr <= 0) return
  if (!Array.isArray(node.children) || node.children.length === 0) return
  const hasImageChild = node.children.some((c) => c.type === 'image')
  if (hasImageChild) {
    node.clipContent = true
  }
}

function getNodeMarker(node: PenNode): string {
  const base = `${node.id} ${node.name ?? ''}`
  if (node.type === 'text') return `${base} ${node.content}`.toLowerCase()
  return base.toLowerCase()
}

// ---------------------------------------------------------------------------
// Verified icon SVG paths (Lucide-style, 24×24 viewBox, stroke-based)
// These are auto-resolved by node name to avoid LLM hallucinating SVG paths.
// ---------------------------------------------------------------------------
const ICON_PATH_MAP: Record<string, { d: string; style: 'stroke' | 'fill' }> = {
  menu:           { d: 'M4 6h16M4 12h16M4 18h16', style: 'stroke' },
  x:              { d: 'M18 6L6 18M6 6l12 12', style: 'stroke' },
  close:          { d: 'M18 6L6 18M6 6l12 12', style: 'stroke' },
  check:          { d: 'M20 6L9 17l-5-5', style: 'stroke' },
  plus:           { d: 'M12 5v14M5 12h14', style: 'stroke' },
  minus:          { d: 'M5 12h14', style: 'stroke' },
  search:         { d: 'M11 19a8 8 0 100-16 8 8 0 000 16zM21 21l-4.35-4.35', style: 'stroke' },
  arrowright:     { d: 'M5 12h14M12 5l7 7-7 7', style: 'stroke' },
  arrowleft:      { d: 'M19 12H5M12 19l-7-7 7-7', style: 'stroke' },
  arrowup:        { d: 'M12 19V5M5 12l7-7 7 7', style: 'stroke' },
  arrowdown:      { d: 'M12 5v14M19 12l-7 7-7-7', style: 'stroke' },
  chevronright:   { d: 'M9 18l6-6-6-6', style: 'stroke' },
  chevronleft:    { d: 'M15 18l-6-6 6-6', style: 'stroke' },
  chevrondown:    { d: 'M6 9l6 6 6-6', style: 'stroke' },
  chevronup:      { d: 'M18 15l-6-6-6 6', style: 'stroke' },
  star:           { d: 'M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z', style: 'fill' },
  heart:          { d: 'M20.84 4.61a5.5 5.5 0 00-7.78 0L12 5.67l-1.06-1.06a5.5 5.5 0 00-7.78 7.78l1.06 1.06L12 21.23l7.78-7.78 1.06-1.06a5.5 5.5 0 000-7.78z', style: 'stroke' },
  home:           { d: 'M3 9l9-7 9 7v11a2 2 0 01-2 2H5a2 2 0 01-2-2V9zM9 22V12h6v10', style: 'stroke' },
  user:           { d: 'M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2M16 7a4 4 0 11-8 0 4 4 0 018 0z', style: 'stroke' },
  settings:       { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  gear:           { d: 'M12.22 2h-.44a2 2 0 00-2 2v.18a2 2 0 01-1 1.73l-.43.25a2 2 0 01-2 0l-.15-.08a2 2 0 00-2.73.73l-.22.38a2 2 0 00.73 2.73l.15.1a2 2 0 011 1.72v.51a2 2 0 01-1 1.74l-.15.09a2 2 0 00-.73 2.73l.22.38a2 2 0 002.73.73l.15-.08a2 2 0 012 0l.43.25a2 2 0 011 1.73V20a2 2 0 002 2h.44a2 2 0 002-2v-.18a2 2 0 011-1.73l.43-.25a2 2 0 012 0l.15.08a2 2 0 002.73-.73l.22-.39a2 2 0 00-.73-2.73l-.15-.08a2 2 0 01-1-1.74v-.5a2 2 0 011-1.74l.15-.09a2 2 0 00.73-2.73l-.22-.38a2 2 0 00-2.73-.73l-.15.08a2 2 0 01-2 0l-.43-.25a2 2 0 01-1-1.73V4a2 2 0 00-2-2zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  mail:           { d: 'M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2zm16 2l-10 7L2 6', style: 'stroke' },
  phone:          { d: 'M22 16.92v3a2 2 0 01-2.18 2 19.79 19.79 0 01-8.63-3.07 19.5 19.5 0 01-6-6A19.79 19.79 0 012.12 4.18 2 2 0 014.11 2h3a2 2 0 012 1.72c.127.96.362 1.903.7 2.81a2 2 0 01-.45 2.11L8.09 9.91a16 16 0 006 6l1.27-1.27a2 2 0 012.11-.45c.907.339 1.85.573 2.81.7A2 2 0 0122 16.92z', style: 'stroke' },
  download:       { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M7 10l5 5 5-5M12 15V3', style: 'stroke' },
  upload:         { d: 'M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4M17 8l-5-5-5 5M12 3v12', style: 'stroke' },
  play:           { d: 'M5 3l14 9-14 9V3z', style: 'fill' },
  pause:          { d: 'M6 4h4v16H6zM14 4h4v16h-4z', style: 'fill' },
  eye:            { d: 'M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8zM15 12a3 3 0 11-6 0 3 3 0 016 0z', style: 'stroke' },
  lock:           { d: 'M19 11H5a2 2 0 00-2 2v7a2 2 0 002 2h14a2 2 0 002-2v-7a2 2 0 00-2-2zM7 11V7a5 5 0 0110 0v4', style: 'stroke' },
  shield:         { d: 'M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z', style: 'stroke' },
  zap:            { d: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z', style: 'stroke' },
  lightning:      { d: 'M13 2L3 14h9l-1 8 10-12h-9l1-8z', style: 'stroke' },
  bell:           { d: 'M18 8A6 6 0 006 8c0 7-3 9-3 9h18s-3-2-3-9M13.73 21a2 2 0 01-3.46 0', style: 'stroke' },
  sparkles:       { d: 'M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3zM5 19l1 3 1-3 3-1-3-1-1-3-1 3-3 1 3 1z', style: 'stroke' },
  ai:             { d: 'M12 3l1.5 4.5L18 9l-4.5 1.5L12 15l-1.5-4.5L6 9l4.5-1.5L12 3zM5 19l1 3 1-3 3-1-3-1-1-3-1 3-3 1 3 1z', style: 'stroke' },
  globe:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM2 12h20M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z', style: 'stroke' },
  externallink:   { d: 'M18 13v6a2 2 0 01-2 2H5a2 2 0 01-2-2V8a2 2 0 012-2h6M15 3h6v6M10 14L21 3', style: 'stroke' },
  copy:           { d: 'M20 9h-9a2 2 0 00-2 2v9a2 2 0 002 2h9a2 2 0 002-2v-9a2 2 0 00-2-2zM5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1', style: 'stroke' },
  trash:          { d: 'M3 6h18M19 6v14a2 2 0 01-2 2H7a2 2 0 01-2-2V6m3 0V4a2 2 0 012-2h4a2 2 0 012 2v2', style: 'stroke' },
  edit:           { d: 'M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z', style: 'stroke' },
  filter:         { d: 'M22 3H2l8 9.46V19l4 2v-8.54L22 3z', style: 'stroke' },
  bookmark:       { d: 'M19 21l-7-5-7 5V5a2 2 0 012-2h10a2 2 0 012 2z', style: 'stroke' },
  clock:          { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 6v6l4 2', style: 'stroke' },
  calendar:       { d: 'M19 4H5a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2V6a2 2 0 00-2-2zM16 2v4M8 2v4M3 10h18', style: 'stroke' },
  image:          { d: 'M19 3H5a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2V5a2 2 0 00-2-2zM8.5 10a1.5 1.5 0 100-3 1.5 1.5 0 000 3zM21 15l-5-5L5 21', style: 'stroke' },
  camera:         { d: 'M23 19a2 2 0 01-2 2H3a2 2 0 01-2-2V8a2 2 0 012-2h4l2-3h6l2 3h4a2 2 0 012 2zM15.5 13a3.5 3.5 0 11-7 0 3.5 3.5 0 017 0z', style: 'stroke' },
  chart:          { d: 'M18 20V10M12 20V4M6 20v-6', style: 'stroke' },
  barchart:       { d: 'M18 20V10M12 20V4M6 20v-6', style: 'stroke' },
  layers:         { d: 'M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5', style: 'stroke' },
  code:           { d: 'M16 18l6-6-6-6M8 6l-6 6 6 6', style: 'stroke' },
  terminal:       { d: 'M4 17l6-6-6-6M12 19h8', style: 'stroke' },
  share:          { d: 'M18 8a3 3 0 100-6 3 3 0 000 6zM6 15a3 3 0 100-6 3 3 0 000 6zM18 22a3 3 0 100-6 3 3 0 000 6zM8.59 13.51l6.83 3.98M15.41 6.51l-6.82 3.98', style: 'stroke' },
  send:           { d: 'M22 2L11 13M22 2l-7 20-4-9-9-4 20-7z', style: 'stroke' },
  messageCircle:  { d: 'M21 11.5a8.38 8.38 0 01-.9 3.8 8.5 8.5 0 01-7.6 4.7 8.38 8.38 0 01-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 01-.9-3.8 8.5 8.5 0 014.7-7.6 8.38 8.38 0 013.8-.9h.5a8.48 8.48 0 018 8v.5z', style: 'stroke' },
  info:           { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 16v-4M12 8h.01', style: 'stroke' },
  alertCircle:    { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM12 8v4M12 16h.01', style: 'stroke' },
  helpCircle:     { d: 'M12 22a10 10 0 100-20 10 10 0 000 20zM9.09 9a3 3 0 015.83 1c0 2-3 3-3 3M12 17h.01', style: 'stroke' },
  apple:          { d: 'M18.71 19.5c-.83 1.24-1.71 2.45-3.05 2.47-1.34.03-1.77-.79-3.29-.79-1.53 0-2 .77-3.27.81-1.31.05-2.3-1.32-3.14-2.53C4.25 17 2.94 12.45 4.7 9.39c.87-1.52 2.43-2.48 4.12-2.51 1.28-.02 2.5.87 3.29.87.78 0 2.26-1.07 3.8-.91.65.03 2.47.26 3.64 1.98-.09.06-2.17 1.28-2.15 3.81.03 3.02 2.65 4.03 2.68 4.04-.03.07-.42 1.44-1.38 2.83M13 3.5c.73-.83 1.94-1.46 2.94-1.5.13 1.17-.34 2.35-1.04 3.19-.69.85-1.83 1.51-2.95 1.42-.15-1.15.41-2.35 1.05-3.11z', style: 'fill' },
  googleplay:     { d: 'M3 20.5V3.5c0-.85.54-1.23 1.09-.81L20 12 4.09 21.31c-.55.42-1.09.04-1.09-.81zM14.5 12L4 3.5M14.5 12L4 20.5M14.5 12l6-3.5M14.5 12l6 3.5', style: 'stroke' },
  dot:            { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  bullet:         { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  point:          { d: 'M12 12m-3 0a3 3 0 1 0 6 0a3 3 0 1 0 -6 0', style: 'fill' },
  circlefill:     { d: 'M12 12m-4 0a4 4 0 1 0 8 0a4 4 0 1 0 -8 0', style: 'fill' },
}

/**
 * Resolve icon path nodes by their name. When the AI generates a path node
 * with a name like "SearchIcon" or "MenuIcon", look up the verified SVG path
 * from ICON_PATH_MAP and replace the d attribute.
 */
function applyIconPathResolution(node: PenNode): void {
  if (node.type !== 'path') return
  const name = (node.name ?? node.id ?? '').toLowerCase()
    .replace(/[-_\s]+/g, '')       // normalize separators
    .replace(/icon$/, '')          // strip trailing "icon"

  const match = ICON_PATH_MAP[name]
  if (!match) return

  // Replace with verified path data
  node.d = match.d

  // Apply correct styling (stroke-based vs fill-based)
  if (match.style === 'stroke') {
    const existingColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    const strokeWidth = toStrokeThicknessNumber(node.stroke, 0)
    const strokeColor = extractPrimaryColor(node.stroke?.fill)
    // Ensure stroke is renderable for line icons
    if (!node.stroke || strokeWidth <= 0 || !strokeColor) {
      node.stroke = {
        thickness: strokeWidth > 0 ? strokeWidth : 2,
        fill: [{ type: 'solid', color: existingColor }],
      }
    }
    // Line icons should NOT have opaque fill (transparent to show stroke only)
    if (node.fill && node.fill.length > 0) {
      // Move fill color to stroke if stroke has no color
      const fillColor = extractPrimaryColor(node.fill)
      if (fillColor && node.stroke) {
        node.stroke.fill = [{ type: 'solid', color: fillColor }]
      }
      node.fill = []
    }
  } else {
    // Fill icons must always keep a visible fill.
    const fillColor = extractPrimaryColor('fill' in node ? node.fill : undefined)
      ?? extractPrimaryColor(node.stroke?.fill)
      ?? '#64748B'
    node.fill = [{ type: 'solid', color: fillColor }]
    // Remove non-renderable stroke definitions to avoid transparent-only paths.
    if (node.stroke && toStrokeThicknessNumber(node.stroke, 0) <= 0) {
      node.stroke = undefined
    }
  }
}

const EMOJI_REGEX = /[\p{Extended_Pictographic}\p{Emoji_Presentation}\uFE0F]/gu
const GENERIC_ICON_PATH = 'M12 3l2.6 5.27 5.82.84-4.2 4.09.99 5.8L12 16.9l-5.21 2.73.99-5.8-4.2-4.09 5.82-.84L12 3z'

function applyNoEmojiIconHeuristic(node: PenNode): void {
  if (node.type !== 'text') return
  if (typeof node.content !== 'string' || !node.content) return

  EMOJI_REGEX.lastIndex = 0
  if (!EMOJI_REGEX.test(node.content)) return
  EMOJI_REGEX.lastIndex = 0
  const cleaned = node.content.replace(EMOJI_REGEX, '').replace(/\s{2,}/g, ' ').trim()
  if (cleaned.length > 0) {
    node.content = cleaned
    return
  }

  const iconSize = clamp(toSizeNumber(node.height, toSizeNumber(node.width, node.fontSize ?? 20)), 14, 24)
  const iconFill = extractPrimaryColor('fill' in node ? node.fill : undefined) ?? '#64748B'
  const replacement: PenNode = {
    id: node.id,
    type: 'path',
    name: `${node.name ?? 'Icon'} Path`,
    d: GENERIC_ICON_PATH,
    width: iconSize,
    height: iconSize,
    fill: [{ type: 'solid', color: iconFill }],
  } as PenNode

  if (typeof node.x === 'number') replacement.x = node.x
  if (typeof node.y === 'number') replacement.y = node.y
  if (typeof node.opacity === 'number') replacement.opacity = node.opacity
  if (typeof node.rotation === 'number') replacement.rotation = node.rotation
  replaceNode(node, replacement)
}

function applyTextWrappingHeuristic(node: PenNode): void {
  if (node.type !== 'text') return
  if (typeof node.content !== 'string') return

  const text = node.content.trim()
  if (!text) return

  const fontSize = node.fontSize ?? 16
  const len = text.length
  const widthNum = toSizeNumber(node.width, 0)
  const currentHeight = toSizeNumber(node.height, 0)
  const hasBreak = /[\n\r]/.test(text)
  const looksBody = fontSize <= 24
  const looksHeading = fontSize > 24
  const hasCjk = /[\u4E00-\u9FFF]/.test(text)

  // NEVER overwrite flex sizing values (fill_container / fit_content)
  const isFlexWidth = typeof node.width === 'string'
    && (node.width === 'fill_container' || node.width === 'fit_content' || node.width.startsWith('fill_container'))

  // Flex-width text (fill_container / fit_content): set textGrowth and estimate height.
  if (isFlexWidth) {
    // lineHeight is a MULTIPLIER (e.g., 1.45 means 145% of fontSize), NOT absolute pixels.
    if (len >= (hasCjk ? 12 : 26) && fontSize <= 24) {
      node.textGrowth = 'fixed-width'
      if (!node.lineHeight) node.lineHeight = hasCjk ? 1.55 : 1.45
    } else if (len >= (hasCjk ? 8 : 16) && fontSize > 24) {
      node.textGrowth = 'fixed-width'
      if (!node.lineHeight) node.lineHeight = hasCjk ? 1.4 : 1.2
    }
    // Auto Height: estimate a real height value (like Pencil's panel shows).
    const lh = node.lineHeight ?? (hasCjk ? (fontSize >= 28 ? 1.3 : 1.5) : 1.2)
    node.height = estimateAutoHeight(text, fontSize, lh)
    return
  }

  // --- Below: only fixed-pixel-width text ---

  // CJK chars are ~1.7× wider per char, so fewer chars need wrapping treatment
  const bodyThreshold = hasCjk ? 12 : 26
  const headingThreshold = hasCjk ? 8 : 16
  const willWrap = !hasBreak && (
    (looksBody && len >= bodyThreshold) || (looksHeading && len >= headingThreshold)
  )

  // Long text → wrap with auto height.
  // Use fill_container only for long body text (paragraphs). For medium-length text
  // (labels, subtitles), keep the existing width — the tree-aware heuristic
  // applyTextFillContainerInLayout will convert to fill_container later only when
  // the parent is NOT fit_content (hug). This prevents breaking hug layouts.
  if (willWrap) {
    node.textGrowth = 'fixed-width'
    if (!node.lineHeight) node.lineHeight = hasCjk
      ? (looksBody ? 1.55 : 1.4)
      : (looksBody ? 1.45 : 1.2)
    // Only force fill_container for clearly long text (>60 chars body or >30 chars heading)
    const longBodyThreshold = hasCjk ? 30 : 60
    const longHeadingThreshold = hasCjk ? 15 : 30
    if ((looksBody && len >= longBodyThreshold) || (looksHeading && len >= longHeadingThreshold)) {
      node.width = 'fill_container'
    }
    node.height = estimateAutoHeight(text, fontSize, node.lineHeight!, widthNum > 0 ? widthNum : undefined)
    return
  }

  // Short text: enforce minimum height and width
  const minHeight = Math.round(fontSize * 1.4)
  if (currentHeight > 0 && currentHeight < minHeight) {
    node.height = minHeight
  }
  if (widthNum > 0) {
    const singleLineWidth = Math.ceil(estimateSingleLineTextWidth(text, fontSize))
    if (widthNum < singleLineWidth) {
      node.width = singleLineWidth
    }
  }
}


function extractPrimaryColor(fill: unknown): string | null {
  if (!Array.isArray(fill) || fill.length === 0) return null
  const first = fill[0]
  if (!first || typeof first !== 'object') return null
  const solid = first as { type?: string; color?: string }
  if (solid.type !== 'solid') return null
  return solid.color ?? null
}

function replaceNode(target: PenNode, replacement: PenNode): void {
  const targetRecord = target as unknown as Record<string, unknown>
  for (const key of Object.keys(target)) {
    delete targetRecord[key]
  }
  Object.assign(targetRecord, replacement as unknown as Record<string, unknown>)
}

function sanitizeNodesForInsert(
  nodes: PenNode[],
  existingIds: Set<string>,
): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n))

  for (const node of cloned) {
    applyGenerationHeuristics(node)
    sanitizeLayoutChildPositions(node, false)
    sanitizeScreenFrameBounds(node)
  }

  const counters = new Map<string, number>()
  const used = new Set(existingIds)
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters)
  }

  return cloned
}

function sanitizeNodesForUpsert(nodes: PenNode[]): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n))

  for (const node of cloned) {
    applyGenerationHeuristics(node)
    sanitizeLayoutChildPositions(node, false)
    sanitizeScreenFrameBounds(node)
  }

  const counters = new Map<string, number>()
  const used = new Set<string>()
  for (const node of cloned) {
    ensureUniqueNodeIds(node, used, counters)
  }

  return cloned
}

function mergeNodeForProgressiveUpsert(
  existing: PenNode,
  incoming: PenNode,
): PenNode {
  const merged: PenNode = { ...existing, ...incoming } as PenNode
  const existingChildren = 'children' in existing && Array.isArray(existing.children)
    ? existing.children
    : undefined
  const incomingChildren = 'children' in incoming && Array.isArray(incoming.children)
    ? incoming.children
    : undefined

  if (!existingChildren && !incomingChildren) return merged
  if (!incomingChildren) {
    if ('children' in merged && Array.isArray(existingChildren)) {
      setNodeChildren(merged, existingChildren)
    }
    return merged
  }
  if (!existingChildren) {
    setNodeChildren(merged, incomingChildren)
    return merged
  }

  const existingById = new Map(existingChildren.map((c) => [c.id, c] as const))
  const incomingById = new Map(incomingChildren.map((c) => [c.id, c] as const))
  const mergedChildren: PenNode[] = []

  // 1. Existing children first (preserves already-built order)
  for (const ex of existingChildren) {
    const inc = incomingById.get(ex.id)
    mergedChildren.push(inc ? mergeNodeForProgressiveUpsert(ex, inc) : ex)
  }

  // 2. Append new incoming children (progressive sections added at end)
  for (const child of incomingChildren) {
    if (!existingById.has(child.id)) mergedChildren.push(child)
  }

  setNodeChildren(merged, mergedChildren)
  return merged
}

function setNodeChildren(node: PenNode, children: PenNode[]): void {
  ;(node as PenNode & { children?: PenNode[] }).children = children
}

function deepCloneNode(node: PenNode): PenNode {
  return JSON.parse(JSON.stringify(node)) as PenNode
}

/** Check if a node (by ID) is inside a Phone Placeholder frame (any ancestor). */
function isInsidePhonePlaceholder(
  nodeId: string,
  getNodeById: (id: string) => PenNode | undefined,
): boolean {
  let current = getNodeById(nodeId)
  while (current) {
    if (current.name === 'Phone Placeholder') return true
    const parent = useDocumentStore.getState().getParentOf(current.id)
    if (!parent) break
    current = parent
  }
  return false
}

function hasActiveLayout(node: PenNode): boolean {
  if (!('layout' in node)) return false
  return node.layout === 'vertical' || node.layout === 'horizontal'
}

function sanitizeLayoutChildPositions(
  node: PenNode,
  parentHasLayout: boolean,
): void {
  if (parentHasLayout) {
    if ('x' in node) delete (node as { x?: number }).x
    if ('y' in node) delete (node as { y?: number }).y
  }

  if (!('children' in node) || !Array.isArray(node.children)) return

  const currentHasLayout = hasActiveLayout(node)
  for (const child of node.children) {
    sanitizeLayoutChildPositions(child, currentHasLayout)
  }
}

function sanitizeScreenFrameBounds(node: PenNode): void {
  if ('children' in node && Array.isArray(node.children)) {
    if (isScreenFrame(node)) {
      clampChildrenIntoScreen(node)
    }
    for (const child of node.children) {
      sanitizeScreenFrameBounds(child)
    }
  }
}

function isScreenFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (!('width' in node) || typeof node.width !== 'number') return false
  if (!('height' in node) || typeof node.height !== 'number') return false
  const w = node.width
  const h = node.height
  const isMobileLike = w >= 320 && w <= 480 && h >= 640
  const isDesktopLike = w >= 900 && h >= 600
  return isMobileLike || isDesktopLike
}

function clampChildrenIntoScreen(frame: PenNode): void {
  if (!('children' in frame) || !Array.isArray(frame.children)) return
  if ('layout' in frame && frame.layout && frame.layout !== 'none') return
  if (!('width' in frame) || typeof frame.width !== 'number') return
  if (!('height' in frame) || typeof frame.height !== 'number') return

  const frameW = frame.width
  const frameH = frame.height
  const maxBleedX = frameW * 0.1
  const maxBleedY = frameH * 0.1

  for (const child of frame.children) {
    const childWidth = 'width' in child && typeof child.width === 'number' ? child.width : null
    const childHeight = 'height' in child && typeof child.height === 'number' ? child.height : null
    if (
      typeof child.x !== 'number' ||
      typeof child.y !== 'number' ||
      childWidth === null ||
      childHeight === null
    ) {
      continue
    }

    const minX = -maxBleedX
    const maxX = frameW - childWidth + maxBleedX
    const minY = -maxBleedY
    const maxY = frameH - childHeight + maxBleedY

    child.x = clamp(child.x, minX, maxX)
    child.y = clamp(child.y, minY, maxY)
  }
}

function clamp(value: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, value))
}

function toSizeNumber(value: number | string | undefined, fallback: number): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (typeof value === 'string') {
    const wrapped = value.match(/\((\d+(?:\.\d+)?)\)/)
    if (wrapped) return Number(wrapped[1])
    const parsed = Number(value)
    if (Number.isFinite(parsed)) return parsed
  }
  return fallback
}

function toGapNumber(value: number | string | undefined): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const parsed = Number(value)
    if (Number.isFinite(parsed)) return parsed
  }
  return 0
}

function toStrokeThicknessNumber(
  stroke: { thickness?: number | [number, number, number, number] } | undefined,
  fallback: number,
): number {
  if (!stroke) return fallback
  const t = stroke.thickness
  if (typeof t === 'number' && Number.isFinite(t)) return t
  if (Array.isArray(t) && t.length > 0 && Number.isFinite(t[0])) return t[0]
  return fallback
}

function toCornerRadiusNumber(
  value: number | [number, number, number, number] | undefined,
  fallback: number,
): number {
  if (typeof value === 'number' && Number.isFinite(value)) return value
  if (Array.isArray(value) && value.length > 0 && Number.isFinite(value[0])) return value[0]
  return fallback
}

function parsePaddingValues(
  padding: number | [number, number] | [number, number, number, number] | string | undefined,
): { top: number; right: number; bottom: number; left: number } {
  if (typeof padding === 'number') {
    return { top: padding, right: padding, bottom: padding, left: padding }
  }
  if (typeof padding === 'string') {
    const parsed = Number(padding)
    if (Number.isFinite(parsed)) {
      return { top: parsed, right: parsed, bottom: parsed, left: parsed }
    }
    return { top: 0, right: 0, bottom: 0, left: 0 }
  }
  if (Array.isArray(padding) && padding.length === 2) {
    return { top: padding[0], right: padding[1], bottom: padding[0], left: padding[1] }
  }
  if (Array.isArray(padding) && padding.length === 4) {
    return { top: padding[0], right: padding[1], bottom: padding[2], left: padding[3] }
  }
  return { top: 0, right: 0, bottom: 0, left: 0 }
}

function estimateNodeIntrinsicHeight(node: PenNode, parentContentWidth?: number): number {
  const explicitHeight = toSizeNumber(('height' in node ? node.height : undefined) as number | string | undefined, 0)
  // For text nodes: use content-aware height estimation instead of single-line fallback
  let textHeight = 0
  if (node.type === 'text') {
    const fs = node.fontSize ?? 16
    const lh = node.lineHeight ?? (fs >= 28 ? 1.2 : 1.5)
    if (typeof node.content === 'string' && node.content.trim() && node.textGrowth === 'fixed-width' && parentContentWidth && parentContentWidth > 0) {
      // Re-estimate based on actual available width
      textHeight = estimateAutoHeight(node.content.trim(), fs, lh, parentContentWidth)
    } else {
      textHeight = Math.max(20, Math.round(fs * lh))
    }
  }

  if (!('children' in node) || !Array.isArray(node.children) || node.children.length === 0) {
    // For text nodes, prefer content-based height over explicit (which may be stale)
    if (node.type === 'text' && textHeight > 0) {
      return Math.max(explicitHeight, textHeight)
    }
    return explicitHeight || textHeight || 80
  }

  const padding = parsePaddingValues('padding' in node ? node.padding : undefined)
  const gap = toGapNumber('gap' in node ? node.gap : undefined)
  const layout = 'layout' in node ? node.layout : undefined
  const children = node.children

  // Compute this node's content width for passing to text children
  const nodeW = toSizeNumber(('width' in node ? node.width : undefined) as number | string | undefined, 0)
  const childContentW = nodeW > 0 ? nodeW - padding.left - padding.right : 0

  if (layout === 'vertical') {
    let total = padding.top + padding.bottom
    for (const child of children) {
      total += estimateNodeIntrinsicHeight(child, childContentW || undefined)
    }
    if (children.length > 1) {
      total += gap * (children.length - 1)
    }
    return Math.max(explicitHeight, total)
  }

  if (layout === 'horizontal') {
    // In horizontal layout, each child gets a fraction of the width
    const childCount = children.length
    const totalGap = childCount > 1 ? gap * (childCount - 1) : 0
    const perChildW = childContentW > 0 && childCount > 0
      ? (childContentW - totalGap) / childCount
      : 0
    let maxChild = 0
    for (const child of children) {
      // Use child's explicit width if available, else distribute evenly
      const childW = toSizeNumber(('width' in child ? child.width : undefined) as number | string | undefined, 0)
      const effectiveW = childW > 0 ? childW : (perChildW > 0 ? perChildW : undefined)
      maxChild = Math.max(maxChild, estimateNodeIntrinsicHeight(child, effectiveW))
    }
    const total = padding.top + padding.bottom + maxChild
    return Math.max(explicitHeight, total)
  }

  let boundsBottom = 0
  for (const child of children) {
    const childY = typeof child.y === 'number' ? child.y : 0
    const childBottom = childY + estimateNodeIntrinsicHeight(child, childContentW || undefined)
    boundsBottom = Math.max(boundsBottom, childBottom)
  }

  const contentHeight = boundsBottom + padding.bottom
  return Math.max(explicitHeight, contentHeight)
}

function ensureUniqueNodeIds(
  node: PenNode,
  used: Set<string>,
  counters: Map<string, number>,
): void {
  const base = normalizeIdBase(node.id, node.type)
  let finalId = base

  if (used.has(finalId)) {
    finalId = makeUniqueId(base, used, counters)
  }

  if (finalId !== node.id) {
    node.id = finalId
  }

  used.add(finalId)

  if (!('children' in node) || !Array.isArray(node.children)) return
  for (const child of node.children) {
    ensureUniqueNodeIds(child, used, counters)
  }
}

function normalizeIdBase(id: string | undefined, type: PenNode['type']): string {
  const trimmed = id?.trim()
  return trimmed && trimmed.length > 0 ? trimmed : `${type}-node`
}

function makeUniqueId(
  base: string,
  used: Set<string>,
  counters: Map<string, number>,
): string {
  let next = counters.get(base) ?? 2
  let candidate = `${base}-${next}`
  while (used.has(candidate)) {
    next += 1
    candidate = `${base}-${next}`
  }
  counters.set(base, next + 1)
  return candidate
}

/**
 * Extract PenNode JSON from AI response text and apply to canvas.
 * Returns the number of top-level elements added (0 if nothing found/applied).
 */
export function extractAndApplyDesign(responseText: string): number {
  const nodes = extractJsonFromResponse(responseText)
  if (!nodes || nodes.length === 0) return 0

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  try {
    applyNodesToCanvas(nodes)
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
  }
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

  useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  try {
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
  } finally {
    useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
  }
  return count
}
