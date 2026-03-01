import { resolve } from 'node:path'
import { openDocument, saveDocument } from '../document-manager'
import {
  findNodeInTree,
  insertNodeInTree,
  updateNodeInTree,
  removeNodeFromTree,
  cloneNodeWithNewIds,
  getDocChildren,
  setDocChildren,
} from '../utils/node-operations'
import { generateId } from '../utils/id'
import type { PenDocument, PenNode } from '../../types/pen'

export interface BatchDesignParams {
  filePath: string
  operations: string
}

interface OpResult {
  binding: string
  nodeId: string
}

/**
 * Parse and execute the batch_design operations DSL.
 *
 * Supported operations (one per line):
 *   binding=I(parent, { ...nodeData })      — Insert
 *   binding=C(nodeId, parent, { ...data })   — Copy
 *   U(path, { ...updates })                 — Update
 *   binding=R(path, { ...nodeData })         — Replace
 *   M(nodeId, parent, index?)               — Move
 *   D(nodeId)                               — Delete
 */
export async function handleBatchDesign(
  params: BatchDesignParams,
): Promise<{ results: OpResult[]; nodeCount: number }> {
  const filePath = resolve(params.filePath)
  let doc = await openDocument(filePath)
  doc = structuredClone(doc)

  const bindings = new Map<string, string>()
  const results: OpResult[] = []
  const lines = params.operations
    .split('\n')
    .map((l) => l.trim())
    .filter((l) => l && !l.startsWith('//'))

  for (const line of lines) {
    try {
      executeLine(line, doc, bindings, results)
    } catch (err) {
      throw new Error(
        `Error executing "${line}": ${err instanceof Error ? err.message : String(err)}`,
      )
    }
  }

  await saveDocument(filePath, doc)

  return {
    results,
    nodeCount: countNodes(getDocChildren(doc)),
  }
}

function executeLine(
  line: string,
  doc: PenDocument,
  bindings: Map<string, string>,
  results: OpResult[],
): void {
  // Parse: binding=OP(args) or OP(args)
  const assignMatch = line.match(/^(\w+)\s*=\s*([ICRM])\((.+)\)$/)
  const callMatch = line.match(/^([UDM])\((.+)\)$/)

  if (assignMatch) {
    const [, binding, op, argsStr] = assignMatch
    switch (op) {
      case 'I': {
        const { parent, data } = parseInsertArgs(argsStr, bindings)
        const node = { ...data, id: generateId() } as PenNode
        setDocChildren(doc, insertNodeInTree(getDocChildren(doc), parent, node))
        bindings.set(binding, node.id)
        results.push({ binding, nodeId: node.id })
        break
      }
      case 'C': {
        const { sourceId, parent, data } = parseCopyArgs(argsStr, bindings)
        const source = findNodeInTree(getDocChildren(doc), sourceId)
        if (!source) throw new Error(`Copy source not found: ${sourceId}`)
        const cloned = cloneNodeWithNewIds(source, generateId)
        // Apply override properties
        if (data) {
          Object.assign(cloned, data)
          // Don't override the cloned id
          if (data.id) delete (cloned as any).id
        }
        // Apply descendant overrides
        if (data?.descendants) {
          applyDescendantOverrides(cloned, data.descendants)
        }
        setDocChildren(doc, insertNodeInTree(getDocChildren(doc), parent, cloned))
        bindings.set(binding, cloned.id)
        results.push({ binding, nodeId: cloned.id })
        break
      }
      case 'R': {
        const { path, data } = parseReplaceArgs(argsStr, bindings)
        const resolvedPath = resolveSlashPath(path, doc, bindings)
        const newNode = { ...data, id: generateId() } as PenNode
        // Find and replace the node
        const oldNode = findNodeByPath(resolvedPath, doc)
        if (!oldNode) throw new Error(`Replace target not found: ${path}`)
        // Remove old, insert new at same position
        const parent = findParentByPath(resolvedPath, doc)
        const parentId = parent ? parent.id : null
        const siblings = parent
          ? ('children' in parent ? parent.children ?? [] : [])
          : getDocChildren(doc)
        const idx = siblings.findIndex((n) => n.id === oldNode.id)
        let children = removeNodeFromTree(getDocChildren(doc), oldNode.id)
        children = insertNodeInTree(children, parentId, newNode, idx)
        setDocChildren(doc, children)
        bindings.set(binding, newNode.id)
        results.push({ binding, nodeId: newNode.id })
        break
      }
      case 'M': {
        const { nodeId, parent, index } = parseMoveArgs(argsStr, bindings)
        const node = findNodeInTree(getDocChildren(doc), nodeId)
        if (!node) throw new Error(`Move target not found: ${nodeId}`)
        let children = removeNodeFromTree(getDocChildren(doc), nodeId)
        children = insertNodeInTree(children, parent, node, index)
        setDocChildren(doc, children)
        bindings.set(binding, nodeId)
        results.push({ binding, nodeId })
        break
      }
    }
  } else if (callMatch) {
    const [, op, argsStr] = callMatch
    switch (op) {
      case 'U': {
        const { path, data } = parseUpdateArgs(argsStr, bindings)
        const resolvedPath = resolveSlashPath(path, doc, bindings)
        const targetNode = findNodeByPath(resolvedPath, doc)
        if (!targetNode)
          throw new Error(`Update target not found: ${path}`)
        // Update the node in-place
        setDocChildren(doc, updateNodeInTree(getDocChildren(doc), targetNode.id, data))
        break
      }
      case 'D': {
        const nodeId = resolveRef(argsStr.trim().replace(/^"|"$/g, ''), bindings)
        setDocChildren(doc, removeNodeFromTree(getDocChildren(doc), nodeId))
        break
      }
      case 'M': {
        const { nodeId, parent, index } = parseMoveArgs(argsStr, bindings)
        const node = findNodeInTree(getDocChildren(doc), nodeId)
        if (!node) throw new Error(`Move target not found: ${nodeId}`)
        let children = removeNodeFromTree(getDocChildren(doc), nodeId)
        children = insertNodeInTree(children, parent, node, index)
        setDocChildren(doc, children)
        break
      }
    }
  } else {
    throw new Error(`Cannot parse operation: ${line}`)
  }
}

// --- Argument parsers ---

function parseInsertArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { parent: string | null; data: Record<string, any> } {
  const firstComma = findTopLevelComma(argsStr)
  if (firstComma === -1) throw new Error('Insert requires parent and node data')
  const parentRaw = argsStr.slice(0, firstComma).trim()
  const dataStr = argsStr.slice(firstComma + 1).trim()
  const parent = parentRaw === 'null' ? null : resolveRef(parentRaw, bindings)
  const data = parseJsonArg(dataStr)
  return { parent, data }
}

function parseCopyArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { sourceId: string; parent: string | null; data: Record<string, any> } {
  const first = findTopLevelComma(argsStr)
  if (first === -1) throw new Error('Copy requires sourceId, parent, and data')
  const sourceRaw = argsStr.slice(0, first).trim()
  const rest = argsStr.slice(first + 1).trim()
  const second = findTopLevelComma(rest)

  let parentRaw: string
  let dataStr: string

  if (second === -1) {
    parentRaw = rest
    dataStr = '{}'
  } else {
    parentRaw = rest.slice(0, second).trim()
    dataStr = rest.slice(second + 1).trim()
  }

  return {
    sourceId: resolveRef(sourceRaw, bindings),
    parent: parentRaw === 'null' ? null : resolveRef(parentRaw, bindings),
    data: parseJsonArg(dataStr),
  }
}

function parseUpdateArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { path: string; data: Record<string, any> } {
  const firstComma = findTopLevelComma(argsStr)
  if (firstComma === -1)
    throw new Error('Update requires path and update data')
  const pathRaw = argsStr.slice(0, firstComma).trim()
  const dataStr = argsStr.slice(firstComma + 1).trim()
  const path = resolvePathExpr(pathRaw, bindings)
  return { path, data: parseJsonArg(dataStr) }
}

function parseReplaceArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { path: string; data: Record<string, any> } {
  const firstComma = findTopLevelComma(argsStr)
  if (firstComma === -1) throw new Error('Replace requires path and node data')
  const pathRaw = argsStr.slice(0, firstComma).trim()
  const dataStr = argsStr.slice(firstComma + 1).trim()
  const path = resolvePathExpr(pathRaw, bindings)
  return { path, data: parseJsonArg(dataStr) }
}

function parseMoveArgs(
  argsStr: string,
  bindings: Map<string, string>,
): { nodeId: string; parent: string | null; index?: number } {
  const parts = splitTopLevel(argsStr)
  if (parts.length < 2) throw new Error('Move requires nodeId and parent')
  return {
    nodeId: resolveRef(parts[0].trim(), bindings),
    parent:
      parts[1].trim() === 'null' || parts[1].trim() === 'undefined'
        ? null
        : resolveRef(parts[1].trim(), bindings),
    index: parts[2] ? parseInt(parts[2].trim(), 10) : undefined,
  }
}

// --- Helpers ---

function resolveRef(raw: string, bindings: Map<string, string>): string {
  const cleaned = raw.replace(/^"|"$/g, '')
  return bindings.get(cleaned) ?? cleaned
}

/** Resolve path expressions like `binding+"/child"` or `"id"` */
function resolvePathExpr(
  raw: string,
  bindings: Map<string, string>,
): string {
  if (raw.includes('+')) {
    return raw
      .split('+')
      .map((p) => {
        const t = p.trim()
        if (t.startsWith('"') || t.startsWith("'")) {
          return t.slice(1, -1)
        }
        return bindings.get(t) ?? t
      })
      .join('')
  }
  const cleaned = raw.replace(/^"|"$/g, '')
  return bindings.get(cleaned) ?? cleaned
}

/** Resolve slash-separated path (e.g. "instanceId/childId") to find the actual node. */
function resolveSlashPath(
  path: string,
  _doc: PenDocument,
  _bindings: Map<string, string>,
): string {
  // The path may be "parentId/childId" — we return as-is for findNodeByPath
  return path
}

function findNodeByPath(path: string, doc: PenDocument): PenNode | undefined {
  const children = getDocChildren(doc)
  const parts = path.split('/')
  if (parts.length === 1) {
    return findNodeInTree(children, parts[0])
  }
  // For paths like "instanceId/childId", resolve through the tree
  // First find the instance, then look for child
  let current = findNodeInTree(children, parts[0])
  for (let i = 1; i < parts.length && current; i++) {
    if ('children' in current && current.children) {
      current = current.children.find((c) => c.id === parts[i])
    } else {
      // For ref nodes, the child might be in the referenced component
      return undefined
    }
  }
  return current
}

function findParentByPath(
  path: string,
  doc: PenDocument,
): PenNode | undefined {
  const parts = path.split('/')
  if (parts.length <= 1) {
    // Top-level or simple ID — find parent in tree
    return findParentInTree(getDocChildren(doc), parts[0])
  }
  // Parent is the second-to-last part
  const parentPath = parts.slice(0, -1).join('/')
  return findNodeByPath(parentPath, doc)
}

function findParentInTree(
  nodes: PenNode[],
  id: string,
): PenNode | undefined {
  for (const node of nodes) {
    if ('children' in node && node.children) {
      for (const child of node.children) {
        if (child.id === id) return node
      }
      const found = findParentInTree(node.children, id)
      if (found) return found
    }
  }
  return undefined
}

function applyDescendantOverrides(
  node: PenNode,
  descendants: Record<string, any>,
): void {
  if (!('children' in node) || !node.children) return
  for (const child of node.children) {
    const override = descendants[child.id]
    if (override) {
      Object.assign(child, override)
    }
    applyDescendantOverrides(child, descendants)
  }
}

/** Parse a JSON-like argument, handling unquoted keys. */
function parseJsonArg(str: string): Record<string, any> {
  let normalized = str.trim()
  // Convert JavaScript-style object to JSON: unquoted keys → quoted
  normalized = normalized.replace(
    /(?<=\{|,)\s*(\w+)\s*:/g,
    ' "$1":',
  )
  // Handle single-quoted strings → double-quoted
  normalized = normalized.replace(/'/g, '"')
  try {
    return JSON.parse(normalized)
  } catch {
    throw new Error(`Failed to parse JSON: ${str}`)
  }
}

/** Find the index of the first comma not inside braces/brackets/quotes. */
function findTopLevelComma(str: string): number {
  let depth = 0
  let inString = false
  let quote = ''
  for (let i = 0; i < str.length; i++) {
    const ch = str[i]
    if (inString) {
      if (ch === '\\') {
        i++
        continue
      }
      if (ch === quote) inString = false
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      continue
    }
    if (ch === '{' || ch === '[' || ch === '(') depth++
    if (ch === '}' || ch === ']' || ch === ')') depth--
    if (ch === ',' && depth === 0) return i
  }
  return -1
}

function splitTopLevel(str: string): string[] {
  const result: string[] = []
  let start = 0
  let depth = 0
  let inString = false
  let quote = ''
  for (let i = 0; i < str.length; i++) {
    const ch = str[i]
    if (inString) {
      if (ch === '\\') {
        i++
        continue
      }
      if (ch === quote) inString = false
      continue
    }
    if (ch === '"' || ch === "'") {
      inString = true
      quote = ch
      continue
    }
    if (ch === '{' || ch === '[' || ch === '(') depth++
    if (ch === '}' || ch === ']' || ch === ')') depth--
    if (ch === ',' && depth === 0) {
      result.push(str.slice(start, i))
      start = i + 1
    }
  }
  result.push(str.slice(start))
  return result
}

function countNodes(nodes: PenNode[]): number {
  let count = 0
  for (const node of nodes) {
    count++
    if ('children' in node && node.children) {
      count += countNodes(node.children)
    }
  }
  return count
}

