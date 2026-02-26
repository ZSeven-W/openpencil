import type { PenNode, ContainerProps } from '@/types/pen'
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store'
import {
  parseSizing,
  estimateTextWidth,
  estimateTextHeight,
  estimateLineWidth,
  getTextOpticalCenterYOffset,
} from './canvas-text-measure'

// ---------------------------------------------------------------------------
// Padding
// ---------------------------------------------------------------------------

export interface Padding {
  top: number
  right: number
  bottom: number
  left: number
}

export function resolvePadding(
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined,
): Padding {
  if (!padding || typeof padding === 'string')
    return { top: 0, right: 0, bottom: 0, left: 0 }
  if (typeof padding === 'number')
    return { top: padding, right: padding, bottom: padding, left: padding }
  if (padding.length === 2)
    return {
      top: padding[0],
      right: padding[1],
      bottom: padding[0],
      left: padding[1],
    }
  return {
    top: padding[0],
    right: padding[1],
    bottom: padding[2],
    left: padding[3],
  }
}

// ---------------------------------------------------------------------------
// Visibility check
// ---------------------------------------------------------------------------

export function isNodeVisible(node: PenNode): boolean {
  return ('visible' in node ? node.visible : undefined) !== false
}

// ---------------------------------------------------------------------------
// Root fill-width fallback
// ---------------------------------------------------------------------------

export function getRootFillWidthFallback(): number {
  const roots = useDocumentStore.getState().document.children
  const rootFrame = roots.find(
    (n) => n.type === 'frame'
      && n.id === DEFAULT_FRAME_ID
      && 'width' in n
      && typeof n.width === 'number'
      && n.width > 0,
  )
  if (rootFrame && 'width' in rootFrame && typeof rootFrame.width === 'number' && rootFrame.width > 0) {
    return rootFrame.width
  }
  const anyTopFrame = roots.find(
    (n) => n.type === 'frame' && 'width' in n && typeof n.width === 'number' && n.width > 0,
  )
  if (anyTopFrame && 'width' in anyTopFrame && typeof anyTopFrame.width === 'number' && anyTopFrame.width > 0) {
    return anyTopFrame.width
  }
  return 1200
}

// ---------------------------------------------------------------------------
// Fit-content size computation
// ---------------------------------------------------------------------------

/** Compute fit-content width from children. */
export function fitContentWidth(node: PenNode, parentAvail?: number): number {
  if (!('children' in node) || !node.children?.length) return 0
  const visibleChildren = node.children.filter((child) => isNodeVisible(child))
  if (visibleChildren.length === 0) return 0
  const layout = 'layout' in node ? (node as ContainerProps).layout : undefined
  const pad = resolvePadding('padding' in node ? (node as any).padding : undefined)
  const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
  if (layout === 'horizontal') {
    const gapTotal = gap * Math.max(0, visibleChildren.length - 1)
    const childAvail = parentAvail !== undefined
      ? Math.max(0, parentAvail - pad.left - pad.right - gapTotal)
      : undefined
    const childTotal = visibleChildren.reduce((sum, c) => sum + getNodeWidth(c, childAvail), 0)
    return childTotal + gapTotal + pad.left + pad.right
  }
  const childAvail = parentAvail !== undefined
    ? Math.max(0, parentAvail - pad.left - pad.right)
    : undefined
  const maxChildW = visibleChildren.reduce((max, c) => Math.max(max, getNodeWidth(c, childAvail)), 0)
  return maxChildW + pad.left + pad.right
}

/** Compute fit-content height from children. */
export function fitContentHeight(node: PenNode, parentAvailW?: number): number {
  if (!('children' in node) || !node.children?.length) return 0
  const visibleChildren = node.children.filter((child) => isNodeVisible(child))
  if (visibleChildren.length === 0) return 0
  const layout = 'layout' in node ? (node as ContainerProps).layout : undefined
  const pad = resolvePadding('padding' in node ? (node as any).padding : undefined)
  const gap = 'gap' in node && typeof (node as any).gap === 'number' ? (node as any).gap : 0
  // Compute available width for children (used by text height estimation)
  const nodeW = getNodeWidth(node, parentAvailW)
  const childAvailW = nodeW > 0 ? Math.max(0, nodeW - pad.left - pad.right) : parentAvailW
  if (layout === 'vertical') {
    const childTotal = visibleChildren.reduce((sum, c) => sum + getNodeHeight(c, undefined, childAvailW), 0)
    const gapTotal = gap * Math.max(0, visibleChildren.length - 1)
    return childTotal + gapTotal + pad.top + pad.bottom
  }
  const maxChildH = visibleChildren.reduce((max, c) => Math.max(max, getNodeHeight(c, undefined, childAvailW)), 0)
  return maxChildH + pad.top + pad.bottom
}

// ---------------------------------------------------------------------------
// Node dimension resolution
// ---------------------------------------------------------------------------

export function getNodeWidth(node: PenNode, parentAvail?: number): number {
  if ('width' in node) {
    const s = parseSizing(node.width)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill') {
      if (parentAvail && parentAvail > 0) return parentAvail
      // Unresolved fill width (no parent available): use root viewport width
      // to avoid collapsing frames to content width and causing squeeze.
      if (node.type !== 'text') {
        const fallbackFillW = getRootFillWidthFallback()
        if (fallbackFillW > 0) return fallbackFillW
      }
      // If fill width cannot be resolved yet, prefer intrinsic content width
      // over collapsing to 0. This prevents accidental narrowing cascades.
      if ('children' in node && node.children?.length) {
        const intrinsic = fitContentWidth(node)
        if (intrinsic > 0) return intrinsic
      }
      if (node.type === 'text') {
        const fontSize = node.fontSize ?? 16
        const letterSpacing = node.letterSpacing ?? 0
        const content =
          typeof node.content === 'string'
            ? node.content
            : node.content.map((s2) => s2.text).join('')
        return Math.max(Math.ceil(estimateTextWidth(content, fontSize, letterSpacing)), 20)
      }
    }
    if (s === 'fit') {
      const fit = fitContentWidth(node, parentAvail)
      if (fit > 0) return fit
    }
  }
  // Containers without explicit width: compute from children
  if ('children' in node && node.children?.length) {
    const fit = fitContentWidth(node, parentAvail)
    if (fit > 0) return fit
  }
  if (node.type === 'text') {
    const fontSize = node.fontSize ?? 16
    const letterSpacing = node.letterSpacing ?? 0
    const content =
      typeof node.content === 'string'
        ? node.content
        : node.content.map((s) => s.text).join('')
    return Math.max(Math.ceil(estimateTextWidth(content, fontSize, letterSpacing)), 20)
  }
  return 0
}

export function getNodeHeight(node: PenNode, parentAvail?: number, parentAvailW?: number): number {
  if ('height' in node) {
    const s = parseSizing(node.height)
    if (typeof s === 'number' && s > 0) return s
    if (s === 'fill' && parentAvail) return parentAvail
    if (s === 'fit') {
      const fit = fitContentHeight(node, parentAvailW)
      if (fit > 0) return fit
    }
  }
  // Containers without explicit height: compute from children
  if ('children' in node && node.children?.length) {
    const fit = fitContentHeight(node, parentAvailW)
    if (fit > 0) return fit
  }
  if (node.type === 'text') {
    return estimateTextHeight(node, parentAvailW)
  }
  return 0
}

// ---------------------------------------------------------------------------
// Auto-layout position computation
// ---------------------------------------------------------------------------

/** Compute child positions according to the parent's layout rules. */
export function computeLayoutPositions(
  parent: PenNode,
  children: PenNode[],
): PenNode[] {
  if (children.length === 0) return children
  const visibleChildren = children.filter((child) => isNodeVisible(child))
  if (visibleChildren.length === 0) return []
  const c = parent as PenNode & ContainerProps
  const layout = c.layout
  if (!layout || layout === 'none') return visibleChildren

  const pW = parseSizing(c.width)
  const pH = parseSizing(c.height)
  const parentW = typeof pW === 'number' ? pW : 100
  const parentH = typeof pH === 'number' ? pH : 100
  const pad = resolvePadding(c.padding)
  const gap = typeof c.gap === 'number' ? c.gap : 0
  const justify = normalizeJustifyContent(c.justifyContent)
  const align = normalizeAlignItems(c.alignItems)

  const isVertical = layout === 'vertical'
  const availW = parentW - pad.left - pad.right
  const availH = parentH - pad.top - pad.bottom
  const availMain = isVertical ? availH : availW
  const totalGapSpace = gap * Math.max(0, visibleChildren.length - 1)

  // Two-pass sizing: first compute fixed sizes, then allocate remaining space for fill children
  const mainSizing = visibleChildren.map((ch) => {
    const prop = isVertical ? 'height' : 'width'
    if (prop in ch) {
      const s = parseSizing((ch as any)[prop])
      if (s === 'fill') return 'fill' as const
    }
    return isVertical ? getNodeHeight(ch, availH, availW) : getNodeWidth(ch, availW)
  })
  const fixedTotal = mainSizing.reduce<number>(
    (sum, s) => sum + (typeof s === 'number' ? s : 0),
    0,
  )
  const fillCount = mainSizing.filter((s) => s === 'fill').length
  const remainingMain = Math.max(0, availMain - fixedTotal - totalGapSpace)
  const fillSize = fillCount > 0 ? remainingMain / fillCount : 0

  const sizes = visibleChildren.map((ch, i) => {
    const mainSize = mainSizing[i] === 'fill' ? fillSize : (mainSizing[i] as number)
    return {
      w: isVertical ? getNodeWidth(ch, availW) : mainSize,
      h: isVertical ? mainSize : getNodeHeight(ch, availH, availW),
    }
  })

  const totalMain = sizes.reduce(
    (sum, s) => sum + (isVertical ? s.h : s.w),
    0,
  )
  const freeSpace = Math.max(0, availMain - totalMain - totalGapSpace)

  let mainPos = 0
  let effectiveGap = gap

  switch (justify) {
    case 'center':
      mainPos = freeSpace / 2
      break
    case 'end':
      mainPos = freeSpace
      break
    case 'space_between':
      effectiveGap =
        visibleChildren.length > 1
          ? (availMain - totalMain) / (visibleChildren.length - 1)
          : 0
      break
    case 'space_around': {
      const spacing =
        visibleChildren.length > 0
          ? (availMain - totalMain) / visibleChildren.length
          : 0
      mainPos = spacing / 2
      effectiveGap = spacing
      break
    }
    default:
      // 'start' — mainPos stays 0
      break
  }

  return visibleChildren.map((child, i) => {
    const size = sizes[i]
    const crossAvail = isVertical ? availW : availH
    const childCross = isVertical ? size.w : size.h
    let crossPos = 0

    // For text nodes, use the actual Fabric-rendered height for cross-axis
    // centering instead of the declared height. Fabric.js text height =
    // fontSize * lineHeight, which is typically smaller than the AI-declared
    // height, causing text to appear shifted upward when centered.
    let effectiveChildCross = childCross
    if (align === 'center' && child.type === 'text') {
      const fontSize = child.fontSize ?? 16
      const lineHeight = ('lineHeight' in child ? child.lineHeight : undefined) ?? 1.2
      const visualH = fontSize * lineHeight
      if (!isVertical && visualH < childCross) {
        effectiveChildCross = visualH
      } else if (isVertical && visualH < childCross) {
        // vertical layout: cross axis is width, not applicable
      }
    }

    switch (align) {
      case 'center':
        crossPos = (crossAvail - effectiveChildCross) / 2
        // Optical correction: centered text in horizontal layouts tends to
        // look slightly too high; nudge it down a bit for visual centering.
        if (!isVertical && child.type === 'text') {
          crossPos += getTextOpticalCenterYOffset(child)
        }
        break
      case 'end':
        crossPos = crossAvail - childCross
        break
      default:
        break
    }

    // Keep child within cross-axis bounds after optical correction.
    const clampCrossSize =
      (!isVertical && align === 'center' && child.type === 'text')
        ? effectiveChildCross
        : childCross
    if (crossAvail >= clampCrossSize) {
      crossPos = Math.max(0, Math.min(crossPos, crossAvail - clampCrossSize))
    }

    const computedX = isVertical ? pad.left + crossPos : pad.left + mainPos
    const computedY = isVertical ? pad.top + mainPos : pad.top + crossPos

    mainPos += (isVertical ? size.h : size.w) + effectiveGap

    // Always use computed positions for layout children — this function
    // is only called when layout !== 'none', so all children here are
    // layout-managed and should not retain manual x/y values.
    const out: Record<string, unknown> = {
      ...child,
      x: computedX,
      y: computedY,
      width: size.w,
      height: size.h,
    }
    return out as unknown as PenNode
  })
}

function normalizeJustifyContent(
  value: unknown,
): 'start' | 'center' | 'end' | 'space_between' | 'space_around' {
  if (typeof value !== 'string') return 'start'
  const v = value.trim().toLowerCase()
  switch (v) {
    case 'start':
    case 'flex-start':
    case 'left':
    case 'top':
      return 'start'
    case 'center':
    case 'middle':
      return 'center'
    case 'end':
    case 'flex-end':
    case 'right':
    case 'bottom':
      return 'end'
    case 'space_between':
    case 'space-between':
      return 'space_between'
    case 'space_around':
    case 'space-around':
      return 'space_around'
    default:
      return 'start'
  }
}

function normalizeAlignItems(value: unknown): 'start' | 'center' | 'end' {
  if (typeof value !== 'string') return 'start'
  const v = value.trim().toLowerCase()
  switch (v) {
    case 'start':
    case 'flex-start':
    case 'left':
    case 'top':
      return 'start'
    case 'center':
    case 'middle':
      return 'center'
    case 'end':
    case 'flex-end':
    case 'right':
    case 'bottom':
      return 'end'
    default:
      return 'start'
  }
}

// Re-export estimateLineWidth for convenience (used by drag-into-layout etc.)
export { estimateLineWidth }
