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
    (c) => c.type === 'frame' && typeof c.width === 'number' && (c.width as number) > 0,
  )
  if (fixedFrames.length < 2) return

  // Check if they look like a card row (similar heights)
  const heights = fixedFrames.map((c) => toSizeNumber('height' in c ? c.height : undefined, 0))
  const maxH = Math.max(...heights)
  const minH = Math.min(...heights)
  if (maxH <= 0 || minH / maxH <= 0.5) return

  // Convert to fill_container for even distribution
  for (const child of fixedFrames) {
    updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
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
  applyNoEmojiIconHeuristic(node)
  applyImagePlaceholderHeuristic(node)
  applyScreenshotFramePlaceholderHeuristic(node)
  applyNavbarHeuristic(node)
  applyHorizontalAlignCenterHeuristic(node)
  applyButtonWidthHeuristic(node)
  applyTextWrappingHeuristic(node)
  applyClipContentHeuristic(node)

  if (!('children' in node) || !Array.isArray(node.children)) return
  // Ensure section-level frames have minimum horizontal padding
  applySectionPaddingHeuristic(node)
  // Tree-aware: fix text widths relative to parent layout
  applyTextFillContainerInLayout(node)
  // Card row equalization: horizontal rows of cards → fill_container
  applyCardRowEqualization(node)
  for (const child of node.children) {
    applyGenerationHeuristics(child)
  }
  // Remove decorative glow/shadow frames next to phone placeholders
  applyRemoveDecorativeGlowSiblings(node)
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
  if (!Array.isArray(node.children) || node.children.length === 0) return

  // Keep a mutable reference to the current children list.
  // This gets refreshed after removals so subsequent fixes see the up-to-date tree.
  let children: PenNode[] = node.children

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
          const colors = getPlaceholderColors('fill' in child ? child.fill : undefined)
          updateNode(child.id, {
            name: 'Phone Placeholder',
            layout: 'none',
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

  // --- Fix 6: Section padding for fill_container frames ---
  if (node.width === 'fill_container' && node.layout && node.layout !== 'none') {
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
        const minH = isMobile ? 16 : 40
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
  if (node.layout && node.layout !== 'none') {
    for (const child of children) {
      if (child.type !== 'text') continue
      const needsWidthFix = typeof child.width === 'number'
      const needsGrowthFix = !child.textGrowth
      if (needsWidthFix || needsGrowthFix) {
        const updates: Record<string, unknown> = {}
        if (needsWidthFix) updates.width = 'fill_container'
        if (needsGrowthFix) updates.textGrowth = 'fixed-width'
        // Estimate auto-height based on content
        const text = typeof child.content === 'string'
          ? child.content : Array.isArray(child.content)
            ? child.content.map((s: { text: string }) => s.text).join('') : ''
        if (text) {
          const fs = child.fontSize ?? 16
          const lh = child.lineHeight ?? 1.2
          updates.height = estimateAutoHeight(text, fs, lh)
        }
        updateNode(child.id, updates as Partial<PenNode>)
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
  // This prevents cards from being too narrow for their icon+text content.
  if (node.layout === 'horizontal' && children.length >= 2) {
    const fixedFrames = children.filter((c: PenNode) =>
      c.type === 'frame' && typeof c.width === 'number' && (c.width as number) > 0,
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
          updateNode(child.id, { width: 'fill_container' } as Partial<PenNode>)
        }
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

  // Recurse into child frames
  for (const child of children) {
    applyTreeFixesRecursive(child, getNodeById, updateNode, removeNode)
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
  if (!Array.isArray(parent.children) || parent.children.length < 2) return

  const fixedFrames = parent.children.filter(
    (c) => c.type === 'frame' && typeof c.width === 'number' && (c.width as number) > 0,
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

function applyTextFillContainerInLayout(parent: PenNode): void {
  if (parent.type !== 'frame') return
  const layout = parent.layout
  if (!layout || layout === 'none') return
  if (!Array.isArray(parent.children)) return

  for (const child of parent.children) {
    if (child.type === 'text') {
      // ALL text inside layout frames: Fill Width + Auto Height.
      if (typeof child.width === 'number') child.width = 'fill_container'
      if (!child.textGrowth) child.textGrowth = 'fixed-width'
      if (!child.lineHeight) {
        const fs = child.fontSize ?? 16
        child.lineHeight = fs >= 28 ? 1.2 : 1.5
      }
    }
    // Also fix image children in vertical layout — images should fill parent width
    if (child.type === 'image' && typeof child.width === 'number' && layout === 'vertical') {
      const parentW = toSizeNumber(parent.width, 0)
      const pad = parsePaddingValues('padding' in parent ? parent.padding : undefined)
      const contentW = parentW - pad.left - pad.right
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

/** Estimate pixel width of text on a single line, with per-character width factors.
 *  CJK characters use 1.15× fontSize (not 1.0×) because actual font metrics,
 *  letter-spacing, and rendering variations make CJK glyphs slightly wider
 *  than the nominal 1em. English at 0.6× is accurate — this CJK boost
 *  prevents the under-estimation that causes text clipping in buttons/cards. */
/**
 * Estimate auto-height for text with fill_container width.
 * Uses generationCanvasWidth as a rough approximation of available width.
 */
function estimateAutoHeight(
  text: string,
  fontSize: number,
  lineHeight: number,
): number {
  const totalTextWidth = estimateSingleLineTextWidth(text, fontSize)
  // Approximate available text width: ~60% of canvas for nested layouts
  const availW = Math.max(200, generationCanvasWidth * 0.6)
  const lines = Math.max(1, Math.ceil(totalTextWidth / availW))
  return Math.round(lines * fontSize * lineHeight)
}

function estimateSingleLineTextWidth(text: string, fontSize: number): number {
  let width = 0
  for (const char of text) {
    if (/[\u4E00-\u9FFF\u3400-\u4DBF\u3000-\u303F\uFF00-\uFFEF\uFE30-\uFE4F]/.test(char)) {
      width += fontSize * 1.2 // CJK full-width + font metrics margin
    } else if (char === ' ') {
      width += fontSize * 0.3
    } else {
      width += fontSize * 0.6 // Latin
    }
  }
  return width
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
  if (node.width !== 'fill_container') return
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
  const minHorizontal = isMobile ? 16 : 40

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
      if (!node.lineHeight) node.lineHeight = 1.45
    } else if (len >= (hasCjk ? 8 : 16) && fontSize > 24) {
      node.textGrowth = 'fixed-width'
      if (!node.lineHeight) node.lineHeight = 1.2
    }
    // Auto Height: estimate a real height value (like Pencil's panel shows).
    const lh = node.lineHeight ?? 1.2
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

  // Long text → Fill Width + Auto Height.
  if (willWrap) {
    node.textGrowth = 'fixed-width'
    if (!node.lineHeight) node.lineHeight = looksBody ? 1.45 : 1.2
    node.width = 'fill_container'
    node.height = estimateAutoHeight(text, fontSize, node.lineHeight!)
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

function estimateNodeIntrinsicHeight(node: PenNode): number {
  const explicitHeight = toSizeNumber(('height' in node ? node.height : undefined) as number | string | undefined, 0)
  const textHeight = node.type === 'text'
    ? Math.max(20, Math.round((node.fontSize ?? 16) * 1.4))
    : 0

  if (!('children' in node) || !Array.isArray(node.children) || node.children.length === 0) {
    return explicitHeight || textHeight || 80
  }

  const padding = parsePaddingValues('padding' in node ? node.padding : undefined)
  const gap = toGapNumber('gap' in node ? node.gap : undefined)
  const layout = 'layout' in node ? node.layout : undefined
  const children = node.children

  if (layout === 'vertical') {
    let total = padding.top + padding.bottom
    for (const child of children) {
      total += estimateNodeIntrinsicHeight(child)
    }
    if (children.length > 1) {
      total += gap * (children.length - 1)
    }
    return Math.max(explicitHeight, total)
  }

  if (layout === 'horizontal') {
    let maxChild = 0
    for (const child of children) {
      maxChild = Math.max(maxChild, estimateNodeIntrinsicHeight(child))
    }
    const total = padding.top + padding.bottom + maxChild
    return Math.max(explicitHeight, total)
  }

  let boundsBottom = 0
  for (const child of children) {
    const childY = typeof child.y === 'number' ? child.y : 0
    const childBottom = childY + estimateNodeIntrinsicHeight(child)
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
