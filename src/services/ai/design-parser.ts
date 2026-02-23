import type { PenNode } from '@/types/pen'

// ---------------------------------------------------------------------------
// Streaming JSONL parser result
// ---------------------------------------------------------------------------

export interface StreamingNodeResult {
  node: PenNode
  parentId: string | null
}

// ---------------------------------------------------------------------------
// JSON extraction from AI response text
// ---------------------------------------------------------------------------

/**
 * Extract PenNode JSON from AI response text.
 * Handles ```json blocks, JSONL format, raw arrays, and fallback parsing.
 */
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

// ---------------------------------------------------------------------------
// Streaming JSONL parser — extracts completed JSON objects from within
// a ```json block as they stream in, enabling element-by-element rendering.
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/**
 * Helper to find all complete JSON blocks in text (```json or ``` blocks).
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

function tryParseNodes(json: string): PenNode[] | null {
  try {
     const parsed = JSON.parse(json.trim())
     const nodes = Array.isArray(parsed) ? parsed : [parsed]
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
