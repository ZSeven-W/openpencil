import type { PenNode, FrameNode, SizingBehavior } from '@/types/pen'
import {
  toSizeNumber,
  toGapNumber,
  parsePaddingValues,
  estimateAutoHeight,
  estimateNodeIntrinsicHeight,
  getTextContentForNode,
  hasCjkText,
} from './generation-utils'

// ---------------------------------------------------------------------------
// Context passed to each role rule function
// ---------------------------------------------------------------------------

export interface RoleContext {
  /** Role of the parent node, if any */
  parentRole?: string
  /** Layout of the parent node */
  parentLayout?: 'none' | 'vertical' | 'horizontal'
  /** Width of the parent node's content area (px) */
  parentContentWidth?: number
  /** Root canvas width (1200 for desktop, 375 for mobile) */
  canvasWidth: number
  /** Whether CJK text is detected in the design context */
  hasCjk?: boolean
  /** Whether this node is inside a table-like structure */
  isTableContext?: boolean
}

// ---------------------------------------------------------------------------
// Role defaults — partial properties that fill unset values on a node
// ---------------------------------------------------------------------------

export type RoleDefaults = Partial<{
  layout: 'none' | 'vertical' | 'horizontal'
  gap: number
  padding: number | [number, number] | [number, number, number, number]
  justifyContent: 'start' | 'center' | 'end' | 'space_between' | 'space_around'
  alignItems: 'start' | 'center' | 'end'
  width: SizingBehavior
  height: SizingBehavior
  clipContent: boolean
  cornerRadius: number
  textGrowth: 'auto' | 'fixed-width' | 'fixed-width-height'
  textAlign: 'left' | 'center' | 'right'
  textAlignVertical: 'top' | 'middle' | 'bottom'
  lineHeight: number
  letterSpacing: number
}>

/** A role rule function computes defaults based on context. */
export type RoleRuleFn = (node: PenNode, ctx: RoleContext) => RoleDefaults

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

const roleRegistry = new Map<string, RoleRuleFn>()

/**
 * Register a role rule. Any string is a valid role name.
 * If the same role is registered twice, the later one wins.
 */
export function registerRole(role: string, ruleFn: RoleRuleFn): void {
  roleRegistry.set(role, ruleFn)
}

// ---------------------------------------------------------------------------
// Per-node resolution
// ---------------------------------------------------------------------------

/**
 * Apply role-based defaults to a single node.
 * Only fills in properties that are NOT already set by the AI.
 * The AI's explicit properties always win.
 */
export function resolveNodeRole(node: PenNode, ctx: RoleContext): void {
  const role = node.role
  if (!role) return

  const ruleFn = roleRegistry.get(role)
  if (!ruleFn) return // unknown role — pass through unchanged

  const defaults = ruleFn(node, ctx)
  if (!defaults) return

  applyDefaults(node, defaults)
}

/**
 * Apply defaults to a node, only setting properties that are undefined/missing.
 */
function applyDefaults(node: PenNode, defaults: RoleDefaults): void {
  const record = node as unknown as Record<string, unknown>

  for (const [key, value] of Object.entries(defaults)) {
    if (value === undefined) continue

    // Only set if the property is not already present on the node
    if (record[key] === undefined) {
      record[key] = value
    }
  }
}

// ---------------------------------------------------------------------------
// Tree-level resolution
// ---------------------------------------------------------------------------

/**
 * Walk the tree depth-first, resolving roles for each node.
 * This replaces the old applyGenerationHeuristics tree walk.
 */
export function resolveTreeRoles(
  root: PenNode,
  canvasWidth: number,
  parentRole?: string,
  parentLayout?: 'none' | 'vertical' | 'horizontal',
  parentContentWidth?: number,
  isTableContext = false,
): void {
  const ctx: RoleContext = {
    parentRole,
    parentLayout,
    parentContentWidth,
    canvasWidth,
    isTableContext,
  }

  // Detect CJK in text nodes
  if (root.type === 'text') {
    const text = getTextContentForNode(root)
    ctx.hasCjk = hasCjkText(text)
  }

  resolveNodeRole(root, ctx)

  // Recurse into children
  if (!('children' in root) || !Array.isArray(root.children)) return

  const nodeW = toSizeNumber(
    ('width' in root ? root.width : undefined) as number | string | undefined,
    0,
  )
  const pad = parsePaddingValues('padding' in root ? root.padding : undefined)
  const contentW = nodeW > 0 ? nodeW - pad.left - pad.right : 0

  const childTableContext =
    isTableContext || root.role === 'table' || root.role === 'table-row'

  for (const child of root.children) {
    resolveTreeRoles(
      child,
      canvasWidth,
      root.role,
      'layout' in root ? root.layout : undefined,
      contentW || parentContentWidth,
      childTableContext,
    )
  }
}

// ---------------------------------------------------------------------------
// Post-pass: cross-node fixes that need the full tree
// ---------------------------------------------------------------------------

/**
 * Apply cross-node fixes after the full tree has been role-resolved.
 * These fixes need sibling/parent context that per-node rules can't see.
 */
export function resolveTreePostPass(
  root: PenNode,
  canvasWidth: number,
  getNodeById?: (id: string) => PenNode | undefined,
  updateNode?: (id: string, updates: Partial<PenNode>) => void,
): void {
  if (root.type !== 'frame') return
  if (!('children' in root) || !Array.isArray(root.children)) return

  const children = root.children

  // --- Card row equalization ---
  if (root.layout === 'horizontal' && children.length >= 2) {
    equalizeCardRow(root, children)
  }

  // --- Horizontal overflow fix ---
  if (
    root.layout === 'horizontal' &&
    typeof root.width === 'number' &&
    children.length >= 2
  ) {
    fixHorizontalOverflow(root, children, canvasWidth)
  }

  // --- Form input consistency ---
  if (
    root.layout === 'vertical' &&
    root.width !== 'fit_content' &&
    children.length >= 2
  ) {
    normalizeFormInputWidths(root, children)
  }

  // --- Input trailing icon alignment ---
  if (root.layout === 'horizontal' && children.length >= 2) {
    normalizeInputTrailingIconAlignment(root, children)
  }

  // --- Text height estimation ---
  if (root.layout && root.layout !== 'none') {
    fixTextHeights(root, children, canvasWidth)
  }

  // --- Frame height expansion ---
  if (typeof root.height === 'number' && root.layout && root.layout !== 'none') {
    const intrinsic = estimateNodeIntrinsicHeight(root, undefined, canvasWidth)
    const maxExpansion = root.height * 1.3
    if (intrinsic > root.height && intrinsic <= maxExpansion) {
      if (updateNode) {
        updateNode(root.id, { height: Math.round(intrinsic) })
      } else {
        ;(root as unknown as Record<string, unknown>).height =
          Math.round(intrinsic)
      }
    }
  }

  // --- clipContent for frames with cornerRadius + image children ---
  if (!root.clipContent) {
    const cr =
      typeof root.cornerRadius === 'number'
        ? root.cornerRadius
        : Array.isArray(root.cornerRadius) && root.cornerRadius.length > 0
          ? root.cornerRadius[0]
          : 0
    if (cr > 0 && children.some((c) => c.type === 'image')) {
      if (updateNode) {
        updateNode(root.id, { clipContent: true } as Partial<PenNode>)
      } else {
        root.clipContent = true
      }
    }
  }

  // Recurse
  for (const child of children) {
    resolveTreePostPass(child, canvasWidth, getNodeById, updateNode)
  }
}

// ---------------------------------------------------------------------------
// Post-pass helpers
// ---------------------------------------------------------------------------

function equalizeCardRow(parent: FrameNode, children: PenNode[]): void {
  if (parent.width === 'fit_content') return

  const cardCandidates = children.filter(
    (c) =>
      c.type === 'frame' &&
      c.role !== 'divider' &&
      c.role !== 'phone-mockup' &&
      toSizeNumber('height' in c ? c.height : undefined, 0) > 88,
  )
  if (cardCandidates.some((c) => 'width' in c && c.width === 'fill_container'))
    return

  const fixedFrames = cardCandidates.filter(
    (c) => 'width' in c && typeof c.width === 'number' && (c.width as number) > 0,
  )
  if (fixedFrames.length < 2) return

  const widths = fixedFrames.map((c) =>
    toSizeNumber('width' in c ? c.width : undefined, 0),
  )
  const maxW = Math.max(...widths)
  const minW = Math.min(...widths)
  if (maxW <= 0 || minW / maxW >= 0.6) return

  const heights = fixedFrames.map((c) =>
    toSizeNumber('height' in c ? c.height : undefined, 0),
  )
  const maxH = Math.max(...heights)
  const minH = Math.min(...heights)
  if (maxH <= 0 || minH / maxH <= 0.5) return

  for (const child of fixedFrames) {
    ;(child as unknown as Record<string, unknown>).width = 'fill_container'
    ;(child as unknown as Record<string, unknown>).height = 'fill_container'
  }
}

function fixHorizontalOverflow(
  parent: FrameNode,
  children: PenNode[],
  canvasWidth: number,
): void {
  const parentW = toSizeNumber(parent.width, 0)
  if (parentW <= 0) return

  const pad = parsePaddingValues(parent.padding)
  const gap = toGapNumber(parent.gap)
  const availW = parentW - pad.left - pad.right

  let childrenTotalW = 0
  for (const child of children) {
    const cw = toSizeNumber(
      'width' in child ? (child as { width?: number | string }).width : undefined,
      0,
    )
    if (typeof (child as { width?: unknown }).width === 'number' && cw > 0) {
      childrenTotalW += cw
    } else {
      childrenTotalW += 80
    }
  }
  const gapTotal = gap * (children.length - 1)
  childrenTotalW += gapTotal

  if (childrenTotalW <= availW) return

  // Strategy 1: Reduce gap
  for (const tryGap of [8, 4]) {
    if (gap > tryGap) {
      const reduced =
        childrenTotalW - gapTotal + tryGap * (children.length - 1)
      if (reduced <= availW) {
        ;(parent as unknown as Record<string, unknown>).gap = tryGap
        childrenTotalW = reduced
        break
      }
    }
  }

  // Strategy 2: Expand parent
  if (childrenTotalW > availW) {
    const neededW = Math.round(childrenTotalW + pad.left + pad.right)
    if (neededW > parentW && neededW <= canvasWidth) {
      ;(parent as unknown as Record<string, unknown>).width = neededW
    } else if (neededW > canvasWidth * 0.8) {
      ;(parent as unknown as Record<string, unknown>).width = 'fill_container'
    }
  }
}

function normalizeFormInputWidths(
  _parent: FrameNode,
  children: PenNode[],
): void {
  const hasFillSibling = children.some(
    (c) =>
      c.type === 'frame' &&
      c.width === 'fill_container' &&
      c.role !== 'divider',
  )
  if (!hasFillSibling) return

  for (const child of children) {
    if (child.type !== 'frame') continue
    if (child.role === 'divider') continue
    if (child.role !== 'input' && child.role !== 'form-input') continue
    if (typeof child.width !== 'number') continue
    ;(child as unknown as Record<string, unknown>).width = 'fill_container'
  }
}

function normalizeInputTrailingIconAlignment(
  parent: FrameNode,
  children: PenNode[],
): void {
  if (parent.role !== 'input' && parent.role !== 'form-input') return
  if (parent.justifyContent && parent.justifyContent !== 'start') return

  const visibleChildren = children.filter((c) => c.visible !== false)
  if (visibleChildren.length < 2) return

  const trailing = visibleChildren[visibleChildren.length - 1]
  if (!isIconLikeNode(trailing)) return

  const hasTextBeforeTrailing = visibleChildren
    .slice(0, -1)
    .some((child) => child.type === 'text')
  if (!hasTextBeforeTrailing) return

  ;(parent as unknown as Record<string, unknown>).justifyContent = 'space_between'
}

function isIconLikeNode(node: PenNode): boolean {
  if (node.type === 'path' || node.type === 'image') return true

  if (node.type === 'frame') {
    if (node.role === 'icon' || node.role === 'icon-button') return true
    const w = toSizeNumber(node.width, 0)
    const h = toSizeNumber(node.height, 0)
    if (w > 0 && h > 0 && Math.max(w, h) <= 32) return true
  }

  return false
}

function fixTextHeights(
  parent: FrameNode,
  children: PenNode[],
  canvasWidth: number,
): void {
  const nodeW = toSizeNumber(parent.width, 0)
  const nodePad = parsePaddingValues(parent.padding)
  const contentW = nodeW > 0 ? nodeW - nodePad.left - nodePad.right : 0

  for (const child of children) {
    if (child.type !== 'text') continue
    const fs = child.fontSize ?? 16
    const lh = child.lineHeight ?? (fs >= 28 ? 1.2 : 1.5)
    const currentH = toSizeNumber(child.height, 0)
    const singleLineMin = Math.round(fs * Math.max(lh, 1.2) * 1.15)

    if (currentH > 0 && currentH < singleLineMin) {
      const text = getTextContentForNode(child)
      if (
        text &&
        (child.textGrowth === 'fixed-width' ||
          child.textGrowth === 'fixed-width-height')
      ) {
        child.height = Math.max(
          estimateAutoHeight(text, fs, lh, contentW || undefined, canvasWidth),
          singleLineMin,
        )
      } else {
        child.height = singleLineMin
      }
    }
  }
}
