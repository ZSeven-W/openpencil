import type { FigmaNodeChange } from './figma-types';
import type { ContainerProps, SizingBehavior } from '@zseven-w/pen-types';

/**
 * Map Figma 堆栈（自动布局）属性到 PenNode ContainerProps。
 */
export function mapFigmaLayout(
  node: FigmaNodeChange,
): Pick<
  ContainerProps,
  'layout' | 'gap' | 'padding' | 'justifyContent' | 'alignItems' | 'clipContent'
> {
  const result: Pick<
    ContainerProps,
    'layout' | 'gap' | 'padding' | 'justifyContent' | 'alignItems' | 'clipContent'
  > = {};

  if (node.stackMode && node.stackMode !== 'NONE') {
    result.layout = node.stackMode === 'HORIZONTAL' ? 'horizontal' : 'vertical';
  }

  if (node.stackPrimaryAlignItems) {
    result.justifyContent = mapJustifyContent(node.stackPrimaryAlignItems);
  }

  // Set 与 stackSpacing 之间有差距，但当 justifyContent 为 space_between 时跳过。 Figma 将
  // COMPUTED 项目间间距存储在 stackSpacing 中，用于 SPACE_EVENLY 模式 - 将其用作显式间隙会与
  // space_between 已提供的动态间距发生冲突。
  if (
    node.stackSpacing !== undefined &&
    node.stackSpacing !== 0 &&
    result.justifyContent !== 'space_between'
  ) {
    result.gap = node.stackSpacing;
  }

  const padding = mapPadding(node);
  if (padding !== undefined) {
    result.padding = padding;
  }

  if (node.stackCounterAlignItems) {
    result.alignItems = mapAlignItems(node.stackCounterAlignItems);
  }

  // Frames 默认剪辑在 Figma 中（frameMaskDisabled 默认为 false）。明确禁用时，Only 跳过
  // clipContent。
  if (node.frameMaskDisabled !== true) {
    result.clipContent = true;
  }

  return result;
}

function mapPadding(
  node: FigmaNodeChange,
): number | [number, number] | [number, number, number, number] | undefined {
  // Check 首先是单独的填充值
  const hasHorizontal = node.stackHorizontalPadding !== undefined;
  const hasVertical = node.stackVerticalPadding !== undefined;
  const hasRight = node.stackPaddingRight !== undefined;
  const hasBottom = node.stackPaddingBottom !== undefined;

  if (!hasHorizontal && !hasVertical && !hasRight && !hasBottom) {
    // Uniform 填充
    if (node.stackPadding && node.stackPadding > 0) return node.stackPadding;
    return undefined;
  }

  const vPad = node.stackVerticalPadding ?? node.stackPadding ?? 0;
  const hPad = node.stackHorizontalPadding ?? node.stackPadding ?? 0;
  const top = vPad;
  const bottom = node.stackPaddingBottom ?? vPad;
  const left = hPad;
  const right = node.stackPaddingRight ?? hPad;

  if (top === 0 && right === 0 && bottom === 0 && left === 0) return undefined;
  if (top === right && right === bottom && bottom === left) return top;
  if (top === bottom && left === right) return [top, right];
  return [top, right, bottom, left];
}

function mapJustifyContent(align: string): ContainerProps['justifyContent'] {
  switch (align) {
    case 'MIN':
      return 'start';
    case 'CENTER':
      return 'center';
    case 'MAX':
      return 'end';
    case 'SPACE_EVENLY':
      return 'space_between';
    default:
      return undefined;
  }
}

function mapAlignItems(align: string): ContainerProps['alignItems'] {
  switch (align) {
    case 'MIN':
      return 'start';
    case 'CENTER':
      return 'center';
    case 'MAX':
      return 'end';
    default:
      return undefined;
  }
}

/**
 * Determine Figma 内部格式的宽度大小调整行为。
 */
export function mapWidthSizing(node: FigmaNodeChange, parentStackMode?: string): SizingBehavior {
  // Check 容器的堆栈大小调整
  if (node.stackPrimarySizing === 'RESIZE_TO_FIT' && node.stackMode === 'HORIZONTAL') {
    return 'fit_content';
  }
  if (node.stackCounterSizing === 'RESIZE_TO_FIT' && node.stackMode === 'VERTICAL') {
    return 'fit_content';
  }

  // Check 父级中子级的大小调整
  if (node.stackChildPrimaryGrow === 1 && parentStackMode === 'HORIZONTAL') {
    return 'fill_container';
  }
  if (node.stackChildAlignSelf === 'STRETCH' && parentStackMode === 'VERTICAL') {
    return 'fill_container';
  }

  return node.size?.x ?? 100;
}

/**
 * Determine 内部格式的 Figma 高度调整行为。
 */
export function mapHeightSizing(node: FigmaNodeChange, parentStackMode?: string): SizingBehavior {
  if (node.stackPrimarySizing === 'RESIZE_TO_FIT' && node.stackMode === 'VERTICAL') {
    return 'fit_content';
  }
  if (node.stackCounterSizing === 'RESIZE_TO_FIT' && node.stackMode === 'HORIZONTAL') {
    return 'fit_content';
  }

  if (node.stackChildPrimaryGrow === 1 && parentStackMode === 'VERTICAL') {
    return 'fill_container';
  }
  if (node.stackChildAlignSelf === 'STRETCH' && parentStackMode === 'HORIZONTAL') {
    return 'fill_container';
  }

  return node.size?.y ?? 100;
}
