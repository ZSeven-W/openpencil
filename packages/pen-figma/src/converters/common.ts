import type {
  FigmaNodeChange,
  FigmaMatrix,
  FigmaImportLayoutMode,
  FigmaSymbolOverride,
  FigmaDerivedSymbolDataEntry,
  FigmaGUID,
} from '../figma-types.js';
import type { PenNode, SizingBehavior } from '@zseven-w/pen-types';
import { mapWidthSizing, mapHeightSizing } from '../figma-layout-mapper.js';
import type { TreeNode } from '../figma-tree-builder.js';

export type {
  FigmaNodeChange,
  FigmaMatrix,
  FigmaImportLayoutMode,
  FigmaSymbolOverride,
  FigmaDerivedSymbolDataEntry,
  FigmaGUID,
};
export type { PenNode, SizingBehavior };
export type { TreeNode };

// Icon 查找是可注入的 — 通过主机应用程序中的 setIconLookup() 设置
export interface IconLookupResult {
  d: string;
  iconId?: string;
  style?: 'fill' | 'stroke';
}

let _lookupIconByName: ((name: string) => IconLookupResult | null) | null = null;

/** Set 图标查找功能（由主机应用程序的图标解析器提供）。 */
export function setIconLookup(fn: (name: string) => IconLookupResult | null): void {
  _lookupIconByName = fn;
}

export function lookupIconByName(name: string): IconLookupResult | null {
  return _lookupIconByName?.(name) ?? null;
}

export interface ConversionContext {
  componentMap: Map<string, string>;
  /** SYMBOL TreeNodes 由 Figma GUID 键入 — 包括内部画布，例如内联 */
  symbolTree: Map<string, TreeNode>;
  warnings: string[];
  generateId: () => string;
  blobs: (Uint8Array | string)[];
  layoutMode: FigmaImportLayoutMode;
}

// --- Size 分辨率 ---

export function resolveWidth(
  figma: FigmaNodeChange,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): SizingBehavior {
  if (ctx.layoutMode === 'preserve') return figma.size?.x ?? 100;
  return mapWidthSizing(figma, parentStackMode);
}

export function resolveHeight(
  figma: FigmaNodeChange,
  parentStackMode: string | undefined,
  ctx: ConversionContext,
): SizingBehavior {
  if (ctx.layoutMode === 'preserve') return figma.size?.y ?? 100;
  return mapHeightSizing(figma, parentStackMode);
}

// --- Common 属性提取 ---

export function extractPosition(figma: FigmaNodeChange): { x: number; y: number } {
  if (!figma.transform) return { x: 0, y: 0 };

  const m = figma.transform;

  // Detect 旋转或翻转：任何非恒等 2×2 子矩阵意味着 m02/m12 是 NOT 边界框的左上角。
  const hasRotation = Math.abs(m.m01) > 0.001 || Math.abs(m.m10) > 0.001;
  const hasFlip = m.m00 < -0.001 || m.m11 < -0.001;

  if ((hasRotation || hasFlip) && figma.size) {
    // Figma 的 m02/m12 给出局部原点 (0,0) 在父空间中的映射位置。 For rotated/flipped 节点，这与
    // OpenPencil 所需的预变换左上角不同。 Compute 对象中心（在 rotation/flip 下不变）并从中导出左上角的预变换。
    const w = figma.size.x;
    const h = figma.size.y;
    const cx = (m.m00 * w) / 2 + (m.m01 * h) / 2 + m.m02;
    const cy = (m.m10 * w) / 2 + (m.m11 * h) / 2 + m.m12;
    return {
      x: Math.round((cx - w / 2) * 100) / 100,
      y: Math.round((cy - h / 2) * 100) / 100,
    };
  }

  return {
    x: Math.round(m.m02 * 100) / 100,
    y: Math.round(m.m12 * 100) / 100,
  };
}

export function normalizeAngle(deg: number): number {
  let a = deg % 360;
  if (a < 0) a += 360;
  return Math.round(a * 100) / 100;
}

export function extractRotation(transform?: FigmaMatrix): number | undefined {
  if (!transform) return undefined;
  // Use abs(m00) 忽略水平翻转（作为 flipX 单独处理）
  const angle = Math.atan2(transform.m10, Math.abs(transform.m00)) * (180 / Math.PI);
  const rounded = Math.round(angle);
  return rounded !== 0 ? rounded : undefined;
}

export function extractFlip(transform?: FigmaMatrix): { flipX?: boolean; flipY?: boolean } {
  if (!transform) return {};
  const result: { flipX?: boolean; flipY?: boolean } = {};
  // Determinant 2x2 rotation/scale 子矩阵的符号检测到反射 m00*m11 - m01*m10 < 0 表示单轴翻转
  const det = transform.m00 * transform.m11 - transform.m01 * transform.m10;
  if (det < -0.001) {
    // Check 通过查看刻度符号来翻转哪个轴
    if (transform.m00 < 0) result.flipX = true;
    else result.flipY = true;
  }
  return result;
}

export function mapCornerRadius(
  figma: FigmaNodeChange,
): number | [number, number, number, number] | undefined {
  if (figma.rectangleCornerRadiiIndependent) {
    const tl = figma.rectangleTopLeftCornerRadius ?? 0;
    const tr = figma.rectangleTopRightCornerRadius ?? 0;
    const br = figma.rectangleBottomRightCornerRadius ?? 0;
    const bl = figma.rectangleBottomLeftCornerRadius ?? 0;
    if (tl === tr && tr === br && br === bl) {
      return tl > 0 ? tl : undefined;
    }
    return [tl, tr, br, bl];
  }
  if (figma.cornerRadius && figma.cornerRadius > 0) {
    return figma.cornerRadius;
  }
  return undefined;
}

export function commonProps(
  figma: FigmaNodeChange,
  id: string,
): {
  id: string;
  name?: string;
  x: number;
  y: number;
  rotation?: number;
  opacity?: number;
  locked?: boolean;
  flipX?: boolean;
  flipY?: boolean;
} {
  const { x, y } = extractPosition(figma);
  const flip = extractFlip(figma.transform);
  return {
    id,
    name: figma.name || undefined,
    x,
    y,
    rotation: extractRotation(figma.transform),
    opacity: figma.opacity !== undefined && figma.opacity < 1 ? figma.opacity : undefined,
    locked: figma.locked || undefined,
    ...flip,
  };
}

// --- Image 助手 ---

export function figmaFillColor(figma: FigmaNodeChange): string | undefined {
  const paint = figma.fillPaints?.find((f) => f.visible !== false && f.type === 'SOLID');
  if (!paint?.color) return undefined;
  const { r: cr, g: cg, b: cb } = paint.color;
  const toHex = (v: number) =>
    Math.round(v * 255)
      .toString(16)
      .padStart(2, '0');
  return `#${toHex(cr)}${toHex(cg)}${toHex(cb)}`;
}

export function collectImageBlobs(blobs: (Uint8Array | string)[]): Map<number, Uint8Array> {
  const map = new Map<number, Uint8Array>();
  for (let i = 0; i < blobs.length; i++) {
    const blob = blobs[i];
    if (blob instanceof Uint8Array && blob.length > 8) {
      // Detect image magic bytes: PNG, JPEG, GIF, WebP
      const isPng = blob[0] === 0x89 && blob[1] === 0x50;
      const isJpeg = blob[0] === 0xff && blob[1] === 0xd8;
      const isGif = blob[0] === 0x47 && blob[1] === 0x49;
      const isWebp = blob[0] === 0x52 && blob[1] === 0x49;
      if (isPng || isJpeg || isGif || isWebp) {
        map.set(i, blob);
      }
    }
  }
  return map;
}

export const SKIPPED_TYPES = new Set([
  'SLICE',
  'CONNECTOR',
  'SHAPE_WITH_TEXT',
  'STICKY',
  'STAMP',
  'HIGHLIGHT',
  'WASHI_TAPE',
  'CODE_BLOCK',
  'MEDIA',
  'WIDGET',
  'SECTION_OVERLAY',
  'NONE',
]);

/** Scale tree children's transforms and sizes to fit a different parent size.
 *  Also scales strokeWeight proportionally so strokes don't appear
 *  disproportionately thick when an instance is smaller than its symbol. */
export function scaleTreeChildren(children: TreeNode[], sx: number, sy: number): TreeNode[] {
  if (Math.abs(sx - 1) < 0.001 && Math.abs(sy - 1) < 0.001) return children;
  const strokeScale = Math.min(sx, sy);
  return children.map((child) => {
    const figma = { ...child.figma };
    if (figma.transform) {
      figma.transform = {
        ...figma.transform,
        m02: figma.transform.m02 * sx,
        m12: figma.transform.m12 * sy,
      };
    }
    if (figma.size) {
      figma.size = { x: figma.size.x * sx, y: figma.size.y * sy };
    }
    // Scale stroke weight so lines stay visually proportional
    if (figma.strokeWeight !== undefined && strokeScale < 0.99) {
      figma.strokeWeight = Math.round(figma.strokeWeight * strokeScale * 100) / 100;
    }
    return {
      figma,
      children: scaleTreeChildren(child.children, sx, sy),
    };
  });
}
