import type { FigmaNodeChange } from './figma-types'
import type { ContainerProps, SizingBehavior } from '@/types/pen'

/**
 * Map Figma stack (auto-layout) properties to PenNode ContainerProps.
 */
export function mapFigmaLayout(
  node: FigmaNodeChange
): Pick<
  ContainerProps,
  'layout' | 'gap' | 'padding' | 'justifyContent' | 'alignItems' | 'clipContent'
> {
  const result: Pick<
    ContainerProps,
    'layout' | 'gap' | 'padding' | 'justifyContent' | 'alignItems' | 'clipContent'
  > = {}

  if (node.stackMode && node.stackMode !== 'NONE') {
    result.layout = node.stackMode === 'HORIZONTAL' ? 'horizontal' : 'vertical'
  }

  if (node.stackSpacing !== undefined && node.stackSpacing !== 0) {
    result.gap = node.stackSpacing
  }

  const padding = mapPadding(node)
  if (padding !== undefined) {
    result.padding = padding
  }

  if (node.stackPrimaryAlignItems) {
    result.justifyContent = mapJustifyContent(node.stackPrimaryAlignItems)
  }

  if (node.stackCounterAlignItems) {
    result.alignItems = mapAlignItems(node.stackCounterAlignItems)
  }

  // frameMaskDisabled=false means clipping is enabled
  if (node.frameMaskDisabled === false) {
    result.clipContent = true
  }

  return result
}

function mapPadding(
  node: FigmaNodeChange
): number | [number, number] | [number, number, number, number] | undefined {
  // Check individual padding values first
  const hasHorizontal = node.stackHorizontalPadding !== undefined
  const hasVertical = node.stackVerticalPadding !== undefined
  const hasRight = node.stackPaddingRight !== undefined
  const hasBottom = node.stackPaddingBottom !== undefined

  if (!hasHorizontal && !hasVertical && !hasRight && !hasBottom) {
    // Uniform padding
    if (node.stackPadding && node.stackPadding > 0) return node.stackPadding
    return undefined
  }

  const vPad = node.stackVerticalPadding ?? node.stackPadding ?? 0
  const hPad = node.stackHorizontalPadding ?? node.stackPadding ?? 0
  const top = vPad
  const bottom = node.stackPaddingBottom ?? vPad
  const left = hPad
  const right = node.stackPaddingRight ?? hPad

  if (top === 0 && right === 0 && bottom === 0 && left === 0) return undefined
  if (top === right && right === bottom && bottom === left) return top
  if (top === bottom && left === right) return [top, right]
  return [top, right, bottom, left]
}

function mapJustifyContent(
  align: string
): ContainerProps['justifyContent'] {
  switch (align) {
    case 'MIN': return 'start'
    case 'CENTER': return 'center'
    case 'MAX': return 'end'
    case 'SPACE_EVENLY': return 'space_between'
    default: return undefined
  }
}

function mapAlignItems(
  align: string
): ContainerProps['alignItems'] {
  switch (align) {
    case 'MIN': return 'start'
    case 'CENTER': return 'center'
    case 'MAX': return 'end'
    default: return undefined
  }
}

/**
 * Determine width sizing behavior from Figma internal format.
 */
export function mapWidthSizing(node: FigmaNodeChange, parentStackMode?: string): SizingBehavior {
  // Check stack sizing for containers
  if (node.stackPrimarySizing === 'RESIZE_TO_FIT' && node.stackMode === 'HORIZONTAL') {
    return 'fit_content'
  }
  if (node.stackCounterSizing === 'RESIZE_TO_FIT' && node.stackMode === 'VERTICAL') {
    return 'fit_content'
  }

  // Check child sizing within parent
  if (node.stackChildPrimaryGrow === 1 && parentStackMode === 'HORIZONTAL') {
    return 'fill_container'
  }
  if (node.stackChildAlignSelf === 'STRETCH' && parentStackMode === 'VERTICAL') {
    return 'fill_container'
  }

  return node.size?.x ?? 100
}

/**
 * Determine height sizing behavior from Figma internal format.
 */
export function mapHeightSizing(node: FigmaNodeChange, parentStackMode?: string): SizingBehavior {
  if (node.stackPrimarySizing === 'RESIZE_TO_FIT' && node.stackMode === 'VERTICAL') {
    return 'fit_content'
  }
  if (node.stackCounterSizing === 'RESIZE_TO_FIT' && node.stackMode === 'HORIZONTAL') {
    return 'fit_content'
  }

  if (node.stackChildPrimaryGrow === 1 && parentStackMode === 'VERTICAL') {
    return 'fill_container'
  }
  if (node.stackChildAlignSelf === 'STRETCH' && parentStackMode === 'HORIZONTAL') {
    return 'fill_container'
  }

  return node.size?.y ?? 100
}
