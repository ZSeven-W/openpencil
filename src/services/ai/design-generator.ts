import type { PenNode } from '@/types/pen'
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
import { assessComplexity } from './complexity-classifier'
import { executeOrchestration } from './orchestrator'

const DESIGN_STREAM_TIMEOUTS = {
  hardTimeoutMs: 180_000,
  noTextTimeoutMs: 60_000,
  thinkingResetsTimeout: false,
}

// ---------------------------------------------------------------------------
// Cross-phase ID remapping — tracks replaceEmptyFrame mappings so that
// later phases recognise the root frame ID has been remapped to DEFAULT_FRAME_ID.
// ---------------------------------------------------------------------------

const generationRemappedIds = new Map<string, string>()

export function resetGenerationRemapping(): void {
  generationRemappedIds.clear()
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
  // Find the start of the json block
  const jsonBlockStart = text.indexOf('```json')
  if (jsonBlockStart === -1) return { results: [], newOffset: processedOffset }

  const contentStart = text.indexOf('\n', jsonBlockStart)
  if (contentStart === -1) return { results: [], newOffset: processedOffset }

  const startPos = Math.max(processedOffset, contentStart + 1)

  // Check if the block has ended (stop before closing ```)
  const blockEnd = text.indexOf('\n```', contentStart + 1)
  const searchEnd = blockEnd > 0 ? blockEnd : text.length

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
 * Insert a single streaming node into the canvas with animation.
 * Handles root frame replacement and parent ID remapping.
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

  // Mark node for fade-in animation
  pendingAnimationNodes.add(node.id)
  startNewAnimationBatch()

  if (resolvedParent === null && isCanvasOnlyEmptyFrame() && node.type === 'frame') {
    // Root frame replaces the default empty frame
    replaceEmptyFrame(node)
  } else {
    const effectiveParent = resolvedParent ?? DEFAULT_FRAME_ID
    // Verify parent exists, fall back to root frame
    const parent = getNodeById(effectiveParent)
    addNode(parent ? effectiveParent : DEFAULT_FRAME_ID, node)
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
  // Route complex prompts through orchestrator for parallel generation
  const { isComplex } = assessComplexity(request.prompt)
  if (isComplex) {
    try {
      return await executeOrchestration(request, callbacks)
    } catch (err) {
      // Orchestrator failed — silently fall back to single-call generation
      console.warn('Orchestrator failed, falling back to direct generation:', err)
    }
  }

  const userMessage = buildContextMessage(request)
  let fullResponse = ''
  let streamingOffset = 0 // Tracks how far we've parsed in the streaming text
  let appliedCount = 0
  let streamError: string | null = null
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
      callbacks?.onTextUpdate?.(`<step title="Thinking">${thinkingContent}</step>`)
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
      useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
    }
  }

  // Build final tree from response for return value
  const streamedNodes = extractJsonFromResponse(fullResponse)
  if (streamedNodes && streamedNodes.length > 0) {
    // If nothing was applied during streaming, apply now as fallback
    if (appliedCount === 0) {
      return { nodes: streamedNodes, rawResponse: fullResponse }
    }
    return { nodes: streamedNodes, rawResponse: fullResponse }
  }

  if (streamError) {
    throw new Error(streamError)
  }

  return { nodes: [], rawResponse: fullResponse }
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

function sanitizeNodesForInsert(
  nodes: PenNode[],
  existingIds: Set<string>,
): PenNode[] {
  const cloned = nodes.map((n) => deepCloneNode(n))

  for (const node of cloned) {
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
