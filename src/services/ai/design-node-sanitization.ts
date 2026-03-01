import type { PenNode } from '@/types/pen'
import { clamp } from './generation-utils'

// ---------------------------------------------------------------------------
// Deep clone
// ---------------------------------------------------------------------------

export function deepCloneNode(node: PenNode): PenNode {
  return JSON.parse(JSON.stringify(node)) as PenNode
}

// ---------------------------------------------------------------------------
// Children helpers
// ---------------------------------------------------------------------------

export function setNodeChildren(node: PenNode, children: PenNode[]): void {
  ;(node as PenNode & { children?: PenNode[] }).children = children
}

export function mergeNodeForProgressiveUpsert(
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

// ---------------------------------------------------------------------------
// Layout sanitization
// ---------------------------------------------------------------------------

export function hasActiveLayout(node: PenNode): boolean {
  if (!('layout' in node)) return false
  return node.layout === 'vertical' || node.layout === 'horizontal'
}

export function sanitizeLayoutChildPositions(
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

// ---------------------------------------------------------------------------
// Screen frame bounds sanitization
// ---------------------------------------------------------------------------

export function isScreenFrame(node: PenNode): boolean {
  if (node.type !== 'frame') return false
  if (!('width' in node) || typeof node.width !== 'number') return false
  if (!('height' in node) || typeof node.height !== 'number') return false
  const w = node.width
  const h = node.height
  const isMobileLike = w >= 320 && w <= 480 && h >= 640
  const isDesktopLike = w >= 900 && h >= 600
  return isMobileLike || isDesktopLike
}

export function clampChildrenIntoScreen(frame: PenNode): void {
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

export function sanitizeScreenFrameBounds(node: PenNode): void {
  if ('children' in node && Array.isArray(node.children)) {
    if (isScreenFrame(node)) {
      clampChildrenIntoScreen(node)
    }
    for (const child of node.children) {
      sanitizeScreenFrameBounds(child)
    }
  }
}

// ---------------------------------------------------------------------------
// Node ID uniqueness
// ---------------------------------------------------------------------------

export function ensureUniqueNodeIds(
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
