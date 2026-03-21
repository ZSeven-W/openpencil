/**
 * Pre-validation: pure code checks that don't require LLM.
 *
 * Inspired by Pencil's search_all_unique_properties + replace_all_matching_properties.
 * Runs before vision API validation to fix obvious inconsistencies:
 *   1. Invisible containers — frame with same fill as parent → add border
 *   2. Empty icons — path nodes without geometry data
 *   3. Sibling consistency — majority-rule for height, cornerRadius, fontSize
 */

import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'

interface PreValidationFix {
  nodeId: string
  property: string
  value: unknown
  reason: string
}

/**
 * Run pre-validation checks on the generated design.
 * Returns the number of fixes applied.
 */
export function runPreValidationFixes(): number {
  const store = useDocumentStore.getState()
  const root = store.getNodeById(DEFAULT_FRAME_ID)
  if (!root) return 0

  const fixes: PreValidationFix[] = []

  // Pass 1: Find invisible containers (same fill as parent → needs border)
  detectInvisibleContainers(root, null, fixes, store)

  // Pass 2: Find empty path/icon nodes (missing geometry)
  detectEmptyPaths(root, fixes)

  // Pass 3: Strip explicit pixel heights from text nodes (causes clipping/overlap)
  detectTextExplicitHeights(root, fixes)

  // Pass 4: Find sibling property inconsistencies
  detectSiblingInconsistencies(root, fixes)

  // Deduplicate (a node might get multiple fixes for same property)
  const seen = new Set<string>()
  const uniqueFixes = fixes.filter((f) => {
    const key = `${f.nodeId}:${f.property}`
    if (seen.has(key)) return false
    seen.add(key)
    return true
  })

  // Apply
  for (const fix of uniqueFixes) {
    if (fix.property === '__remove') {
      store.removeNode(fix.nodeId)
      console.log(`[Pre-validation] ${fix.nodeId}: removed (${fix.reason})`)
    } else {
      store.updateNode(fix.nodeId, { [fix.property]: fix.value })
      console.log(
        `[Pre-validation] ${fix.nodeId}: ${fix.property} → ${JSON.stringify(fix.value)} (${fix.reason})`,
      )
    }
  }

  return uniqueFixes.length
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/** Extract the first fill color from a node (raw, including variable refs) */
function getFirstFillColor(node: PenNode): string | null {
  if (!('fill' in node) || !Array.isArray(node.fill) || node.fill.length === 0) return null
  const first = node.fill[0]
  if (first && 'color' in first && first.color) return first.color
  return null
}

/** Check if a node already has a visible stroke */
function hasStroke(node: PenNode): boolean {
  if (!('stroke' in node)) return false
  const s = node.stroke as { thickness?: number } | undefined
  return s != null && (s.thickness ?? 0) > 0
}

/** Resolve border color: prefer $color-border variable if it exists, else neutral gray */
function getBorderStroke(store: ReturnType<typeof useDocumentStore.getState>): {
  thickness: number
  fill: Array<{ type: 'solid'; color: string }>
} {
  const doc = store.document
  const hasBorderVar = doc.variables && 'color-border' in doc.variables
  const color = hasBorderVar ? '$color-border' : '#E2E8F0'
  return { thickness: 1, fill: [{ type: 'solid', color }] }
}

// ---------------------------------------------------------------------------
// Pass 1: Invisible container detection
// ---------------------------------------------------------------------------

/**
 * Detect containers with same fill as parent.
 * These blend visually and need a border to be distinguishable.
 */
function detectInvisibleContainers(
  node: PenNode,
  parentFillColor: string | null,
  fixes: PreValidationFix[],
  store: ReturnType<typeof useDocumentStore.getState>,
): void {
  const nodeFill = getFirstFillColor(node)

  if (
    parentFillColor &&
    nodeFill &&
    nodeFill === parentFillColor &&
    !hasStroke(node) &&
    node.type === 'frame' &&
    'layout' in node &&
    node.layout && // only layout frames (inputs, buttons, cards)
    'children' in node &&
    node.children &&
    node.children.length > 0 // has content inside
  ) {
    fixes.push({
      nodeId: node.id,
      property: 'stroke',
      value: getBorderStroke(store),
      reason: `same fill as parent (${nodeFill})`,
    })
  }

  // Recurse
  if ('children' in node && node.children) {
    for (const child of node.children) {
      detectInvisibleContainers(child, nodeFill ?? parentFillColor, fixes, store)
    }
  }
}

// ---------------------------------------------------------------------------
// Pass 2: Empty path/icon detection
// ---------------------------------------------------------------------------

/**
 * Detect path nodes without geometry data.
 * These render as invisible empty rectangles on canvas.
 */
function detectEmptyPaths(node: PenNode, fixes: PreValidationFix[]): void {
  if (node.type === 'path') {
    const hasD = 'd' in node && (node as unknown as Record<string, unknown>).d
    if (!hasD) {
      fixes.push({
        nodeId: node.id,
        property: '__remove',
        value: true,
        reason: 'path node without geometry (renders invisible)',
      })
    }
  }

  if ('children' in node && node.children) {
    for (const child of node.children) {
      detectEmptyPaths(child, fixes)
    }
  }
}

// ---------------------------------------------------------------------------
// Pass 3: Text explicit height detection
// ---------------------------------------------------------------------------

/**
 * Detect text nodes with explicit pixel heights.
 * Explicit heights on text always cause clipping or overlap — the layout
 * engine should auto-calculate height from content + fontSize + lineHeight.
 */
function detectTextExplicitHeights(node: PenNode, fixes: PreValidationFix[]): void {
  if (node.type === 'text') {
    const textNode = node as PenNode & { height?: unknown; textGrowth?: string }
    if (typeof textNode.height === 'number' && textNode.textGrowth !== 'fixed-width-height') {
      fixes.push({
        nodeId: node.id,
        property: 'height',
        value: 'fit_content',
        reason: `text node has explicit height=${textNode.height}px — causes clipping`,
      })
    }
  }

  if ('children' in node && node.children) {
    for (const child of node.children) {
      detectTextExplicitHeights(child, fixes)
    }
  }
}

// ---------------------------------------------------------------------------
// Pass 4: Sibling property consistency
// ---------------------------------------------------------------------------

/** Properties to check for consistency among same-type siblings */
const FRAME_CONSISTENCY_PROPS = ['height', 'cornerRadius'] as const
const TEXT_CONSISTENCY_PROPS = ['fontSize'] as const

/**
 * Detect property inconsistencies among siblings.
 * Uses majority rule: if >= 2/3 of siblings agree on a value, fix the outlier.
 */
function detectSiblingInconsistencies(node: PenNode, fixes: PreValidationFix[]): void {
  if (!('children' in node) || !node.children) {
    return
  }

  // Need at least 3 siblings for meaningful majority
  if (node.children.length >= 3) {
    // Group children by type
    const groups = new Map<string, PenNode[]>()
    for (const child of node.children) {
      if (!groups.has(child.type)) groups.set(child.type, [])
      groups.get(child.type)!.push(child)
    }

    for (const [type, siblings] of groups) {
      if (siblings.length < 3) continue
      const props = type === 'text' ? TEXT_CONSISTENCY_PROPS : FRAME_CONSISTENCY_PROPS
      for (const prop of props) {
        checkConsistency(siblings, prop, fixes)
      }
    }
  }

  // Recurse
  for (const child of node.children) {
    detectSiblingInconsistencies(child, fixes)
  }
}

function checkConsistency(
  siblings: PenNode[],
  property: string,
  fixes: PreValidationFix[],
): void {
  const values = new Map<string, { value: unknown; nodes: PenNode[] }>()

  for (const node of siblings) {
    const raw = (node as unknown as Record<string, unknown>)[property]
    if (raw == null) continue
    const key = JSON.stringify(raw)
    if (!values.has(key)) values.set(key, { value: raw, nodes: [] })
    values.get(key)!.nodes.push(node)
  }

  if (values.size < 2) return // all same, no inconsistency

  // Find majority
  let majority: { value: unknown; nodes: PenNode[] } | null = null
  for (const entry of values.values()) {
    if (!majority || entry.nodes.length > majority.nodes.length) {
      majority = entry
    }
  }
  if (!majority) return

  // Only fix if clear majority (>= 2/3)
  const totalWithProp = Array.from(values.values()).reduce((s, e) => s + e.nodes.length, 0)
  if (majority.nodes.length < (totalWithProp * 2) / 3) return

  // Fix outliers
  for (const entry of values.values()) {
    if (entry === majority) continue
    for (const node of entry.nodes) {
      fixes.push({
        nodeId: node.id,
        property,
        value: majority.value,
        reason: `inconsistent with ${majority.nodes.length} siblings`,
      })
    }
  }
}
