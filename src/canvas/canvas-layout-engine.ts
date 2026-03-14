import type { PenNode, ContainerProps, SizingBehavior } from '@/types/pen'
import { isBadgeOverlayNode } from '@/services/ai/design-node-sanitization'
import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store'
import {
  parseSizing,
  estimateTextWidth,
  estimateTextWidthPrecise,
  estimateTextHeight,
  estimateLineWidth,
  resolveTextContent,
  countExplicitTextLines,
  defaultLineHeight,
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
// Layout inference — shared logic for detecting implicit layout
// ---------------------------------------------------------------------------

/**
 * Infer layout direction for a frame that has no explicit `layout` property.
 * Pencil treats frames as horizontal layout (CSS flexbox default = row) when:
 * - gap, justifyContent, or alignItems are set, OR
 * - padding is set (CSS flexbox respects padding for child positioning), OR
 * - any child uses `fill_container` sizing (only meaningful in a layout context)
 */
export function inferLayout(node: PenNode): 'horizontal' | undefined {
  if (node.type !== 'frame') return undefined
  const c = node as PenNode & ContainerProps
  if (c.gap != null || c.justifyContent || c.alignItems) return 'horizontal'
  // Padding implies layout context: in Pencil (CSS flexbox), padding offsets
  // child content. Without layout inference, children sit at (0,0) ignoring padding.
  if (c.padding != null) return 'horizontal'
  // Check if any child uses fill_container, implying layout context
  if ('children' in node && node.children?.length) {
    for (const child of node.children) {
      if ('width' in child && child.width === 'fill_container') return 'horizontal'
      if ('height' in child && child.height === 'fill_container') return 'horizontal'
    }
  }
  return undefined
}

// ---------------------------------------------------------------------------
// Fit-content size computation
// ---------------------------------------------------------------------------

/** Compute fit-content width from children. */
export function fitContentWidth(node: PenNode, parentAvail?: number): number {
  if (!('children' in node) || !node.children?.length) return 0
  // Exclude badge/overlay nodes — they use absolute positioning and
  // should not inflate the container's fit-content dimensions.
  const visibleChildren = node.children.filter(
    (child) => isNodeVisible(child) && !isBadgeOverlayNode(child),
  )
  if (visibleChildren.length === 0) return 0
  const c = node as PenNode & ContainerProps
  const layout = c.layout || inferLayout(node)
  const pad = resolvePadding(c.padding)
  const nodeGap = typeof c.gap === 'number' ? c.gap : 0
  if (layout === 'horizontal') {
    const gapTotal = nodeGap * Math.max(0, visibleChildren.length - 1)
    const childAvail = parentAvail !== undefined
      ? Math.max(0, parentAvail - pad.left - pad.right - gapTotal)
      : undefined
    const childTotal = visibleChildren.reduce((sum, ch) => sum + getNodeWidth(ch, childAvail), 0)
    return childTotal + gapTotal + pad.left + pad.right
  }
  const childAvail = parentAvail !== undefined
    ? Math.max(0, parentAvail - pad.left - pad.right)
    : undefined
  const maxChildW = visibleChildren.reduce((max, ch) => Math.max(max, getNodeWidth(ch, childAvail)), 0)
  return maxChildW + pad.left + pad.right
}

/** Compute fit-content height from children. */
export function fitContentHeight(node: PenNode, parentAvailW?: number): number {
  if (!('children' in node) || !node.children?.length) return 0
  // Exclude badge/overlay nodes — they use absolute positioning and
  // should not inflate the container's fit-content dimensions.
  const visibleChildren = node.children.filter(
    (child) => isNodeVisible(child) && !isBadgeOverlayNode(child),
  )
  if (visibleChildren.length === 0) return 0
  const c = node as PenNode & ContainerProps
  const layout = c.layout || inferLayout(node)
  const pad = resolvePadding(c.padding)
  const nodeGap = typeof c.gap === 'number' ? c.gap : 0
  // Compute available width for children (used by text height estimation)
  const nodeW = getNodeWidth(node, parentAvailW)
  const childAvailW = nodeW > 0 ? Math.max(0, nodeW - pad.left - pad.right) : parentAvailW
  if (layout === 'vertical') {
    const childTotal = visibleChildren.reduce((sum, ch) => sum + getNodeHeight(ch, undefined, childAvailW), 0)
    const gapTotal = nodeGap * Math.max(0, visibleChildren.length - 1)
    return childTotal + gapTotal + pad.top + pad.bottom
  }
  const maxChildH = visibleChildren.reduce((max, ch) => Math.max(max, getNodeHeight(ch, undefined, childAvailW)), 0)
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
        const fontWeight = node.fontWeight
        const content =
          typeof node.content === 'string'
            ? node.content
            : node.content.map((s2) => s2.text).join('')
        return Math.max(Math.ceil(estimateTextWidth(content, fontSize, letterSpacing, fontWeight)), 1)
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
    const fontWeight = node.fontWeight
    const content =
      typeof node.content === 'string'
        ? node.content
        : node.content.map((s) => s.text).join('')
    // Use precise estimation (no safety factor) for fit-content / natural-width
    // text.  Fabric IText auto-computes its own width, so overestimating only
    // inflates the layout allocation and creates visible gaps.  The space_between
    // overflow issue is handled by correct layout inference in inferLayout().
    return Math.max(Math.ceil(estimateTextWidthPrecise(content, fontSize, letterSpacing, fontWeight)), 1)
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
  // Infer layout when gap/justifyContent/alignItems are set but layout is not.
  // Pencil treats these frames as horizontal layout (CSS flexbox default = row).
  const layout = c.layout || inferLayout(parent)
  if (!layout || layout === 'none') return visibleChildren

  // Separate badge/overlay nodes from layout children — badges use absolute
  // positioning and should not participate in the layout flow.
  const badgeNodes = visibleChildren.filter(isBadgeOverlayNode)
  const layoutChildren = visibleChildren.filter((ch) => !isBadgeOverlayNode(ch))
  if (layoutChildren.length === 0) return visibleChildren

  const pW = parseSizing(c.width)
  const pH = parseSizing(c.height)
  // When parent has no explicit dimensions (fit_content), resolve actual size
  // from children. parseSizing(undefined) returns 0 which would make available
  // space negative after subtracting padding, breaking all child positioning.
  const parentW = (typeof pW === 'number' && pW > 0) ? pW : (getNodeWidth(parent) || 100)
  const parentH = (typeof pH === 'number' && pH > 0) ? pH : (getNodeHeight(parent) || 100)
  const pad = resolvePadding(c.padding)
  const gap = typeof c.gap === 'number' ? c.gap : 0
  const justify = normalizeJustifyContent(c.justifyContent)
  const align = normalizeAlignItems(c.alignItems)

  const isVertical = layout === 'vertical'
  const availW = parentW - pad.left - pad.right
  const availH = parentH - pad.top - pad.bottom
  const availMain = isVertical ? availH : availW
  const totalGapSpace = gap * Math.max(0, layoutChildren.length - 1)

  // Two-pass sizing: first compute fixed sizes, then allocate remaining space for fill children
  const mainSizing = layoutChildren.map((ch) => {
    const prop = isVertical ? 'height' : 'width'
    if (prop in ch) {
      const s = parseSizing((ch as PenNode & { width?: SizingBehavior; height?: SizingBehavior })[prop])
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

  const sizes = layoutChildren.map((ch, i) => {
    let mainSize = mainSizing[i] === 'fill' ? fillSize : (mainSizing[i] as number)
    // For single-line text in vertical layouts, use the actual CanvasKit
    // Paragraph height (fontSize * lineHeight) for correct main-axis positioning.
    if (isVertical && ch.type === 'text' && mainSizing[i] !== 'fill') {
      const content = resolveTextContent(ch)
      if (countExplicitTextLines(content) <= 1) {
        const fontSize = ch.fontSize ?? 16
        const lineHeight = ch.lineHeight ?? defaultLineHeight(fontSize)
        const singleLineH = fontSize * lineHeight
        const estH = estimateTextHeight(ch, availW)
        if (estH <= singleLineH + 1) {
          mainSize = singleLineH
        }
      }
    }
    return {
      w: isVertical ? getNodeWidth(ch, availW) : mainSize,
      // For horizontal layouts, use the child's resolved width (mainSize) for
      // height estimation. This ensures text wrapping is calculated at the
      // actual width the child will occupy, not the parent's full available width.
      h: isVertical ? mainSize : getNodeHeight(ch, availH, isVertical ? availW : mainSize),
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
        layoutChildren.length > 1
          ? (availMain - totalMain) / (layoutChildren.length - 1)
          : 0
      break
    case 'space_around': {
      const spacing =
        layoutChildren.length > 0
          ? (availMain - totalMain) / layoutChildren.length
          : 0
      mainPos = spacing / 2
      effectiveGap = spacing
      break
    }
    default:
      // 'start' — mainPos stays 0
      break
  }

  const positioned = layoutChildren.map((child, i) => {
    const size = sizes[i]
    const crossAvail = isVertical ? availW : availH
    const childCross = isVertical ? size.w : size.h
    let crossPos = 0

    // For single-line text centered in horizontal layouts, use the actual
    // CanvasKit Paragraph height (fontSize * lineHeight) for centering.
    // CanvasKit renders with halfLeading:true, distributing leading evenly
    // above and below, so no optical correction is needed.
    let effectiveChildCross = childCross
    if (align === 'center' && !isVertical && child.type === 'text') {
      const fontSize = child.fontSize ?? 16
      const lineHeight = child.lineHeight ?? defaultLineHeight(fontSize)
      const content = resolveTextContent(child)
      const isSingleLine = countExplicitTextLines(content) <= 1
      if (isSingleLine) {
        effectiveChildCross = fontSize * lineHeight
      }
    }

    switch (align) {
      case 'center':
        crossPos = (crossAvail - effectiveChildCross) / 2
        break
      case 'end':
        crossPos = crossAvail - childCross
        break
      default:
        break
    }

    // Keep child within cross-axis bounds.
    const clampCrossSize =
      (!isVertical && align === 'center' && child.type === 'text')
        ? effectiveChildCross
        : childCross
    if (crossAvail >= clampCrossSize) {
      crossPos = Math.max(0, Math.min(crossPos, crossAvail - clampCrossSize))
    }

    const computedX = Math.round(isVertical ? pad.left + crossPos : pad.left + mainPos)
    const computedY = Math.round(isVertical ? pad.top + mainPos : pad.top + crossPos)

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

    // For text nodes centered in a vertical layout, expand to full available
    // width and set textAlign:'center'. This avoids width estimation inaccuracy:
    // IText ignores our width and computes its own, so textAlign has no effect.
    // By using full width (which triggers Textbox in the factory) + center align,
    // the text is precisely centered regardless of glyph estimation error.
    if (isVertical && align === 'center' && child.type === 'text') {
      const hasExplicitAlign = 'textAlign' in child && child.textAlign && child.textAlign !== 'left'
      if (!hasExplicitAlign) {
        out.width = availW
        out.x = Math.round(pad.left)
        out.textAlign = 'center'
      }
    }


    return out as unknown as PenNode
  })

  // Prepend badge/overlay nodes (they keep original x/y for absolute positioning).
  // flattenNodes iterates in REVERSE, so index 0 = frontmost z-order.
  // Badges at the beginning render on top of layout children.
  if (badgeNodes.length > 0) {
    return [...badgeNodes, ...positioned]
  }
  return positioned
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
