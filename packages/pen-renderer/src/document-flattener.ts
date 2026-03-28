import type { PenNode, ContainerProps, RefNode } from '@zseven-w/pen-types';
import {
  resolvePadding,
  isNodeVisible,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
  inferLayout,
  parseSizing,
  defaultLineHeight,
  findNodeInTree,
  cssFontFamily,
} from '@zseven-w/pen-core';
import { wrapLine } from './paint-utils.js';
import type { RenderNode } from './types.js';

// ---------------------------------------------------------------------------
// Pre-measure text widths using Canvas 2D (browser fonts)
// ---------------------------------------------------------------------------

let _measureCtx: CanvasRenderingContext2D | null = null;
function getMeasureCtx(): CanvasRenderingContext2D {
  if (!_measureCtx) {
    const c = document.createElement('canvas');
    _measureCtx = c.getContext('2d')!;
  }
  return _measureCtx;
}

/**
 * Walk the node tree and fix text HEIGHTS using actual Canvas 2D wrapping.
 *
 * Only targets fixed-width text with auto height — these are the cases where
 * estimateTextHeight may underestimate because its width estimation differs
 * from Canvas 2D's actual text measurement, leading to incorrect wrap counts.
 *
 * IMPORTANT: This function never touches WIDTH or container-relative sizing
 * strings (fill_container / fit_content). Changing widths breaks layout
 * resolution in computeLayoutPositions.
 */
export function premeasureTextHeights(nodes: PenNode[]): PenNode[] {
  return nodes.map((node) => {
    let result = node;

    if (node.type === 'text') {
      const tNode = node as PenNode & {
        width?: number | string;
        height?: number | string;
        fontSize?: number;
        fontWeight?: string;
        fontFamily?: string;
        lineHeight?: number;
        textAlign?: string;
        textGrowth?: string;
        content?: string | { text?: string }[];
      };
      const hasFixedWidth = typeof tNode.width === 'number' && tNode.width > 0;
      const isContainerHeight =
        typeof tNode.height === 'string' &&
        (tNode.height === 'fill_container' || tNode.height === 'fit_content');
      const textGrowth = tNode.textGrowth;
      const content =
        typeof tNode.content === 'string'
          ? tNode.content
          : Array.isArray(tNode.content)
            ? tNode.content.map((s) => s.text ?? '').join('')
            : (((tNode as unknown as Record<string, unknown>).text as string) ?? '');

      const textAlign = tNode.textAlign;
      const isFixedWidthText =
        textGrowth === 'fixed-width' ||
        textGrowth === 'fixed-width-height' ||
        (textGrowth !== 'auto' && textAlign != null && textAlign !== 'left');
      if (content && hasFixedWidth && isFixedWidthText && !isContainerHeight) {
        const fontSize = tNode.fontSize ?? 16;
        const fontWeight = tNode.fontWeight ?? '400';
        const fontFamily =
          tNode.fontFamily ??
          'Inter, -apple-system, "Noto Sans SC", "PingFang SC", system-ui, sans-serif';
        const ctx = getMeasureCtx();
        ctx.font = `${fontWeight} ${fontSize}px ${cssFontFamily(fontFamily)}`;

        const wrapWidth = (tNode.width as number) + fontSize * 0.2;
        const rawLines = content.split('\n');
        const wrappedLines: string[] = [];
        for (const raw of rawLines) {
          if (!raw) {
            wrappedLines.push('');
            continue;
          }
          wrapLine(ctx, raw, wrapWidth, wrappedLines);
        }
        const lineHeightMul = tNode.lineHeight ?? defaultLineHeight(fontSize);
        const lineHeight = lineHeightMul * fontSize;
        const glyphH = fontSize * 1.13;
        const measuredHeight = Math.ceil(
          wrappedLines.length <= 1
            ? glyphH + 2
            : (wrappedLines.length - 1) * lineHeight + glyphH + 2,
        );
        const currentHeight = typeof tNode.height === 'number' ? tNode.height : 0;
        const explicitLineCount = rawLines.length;
        const needsHeight = currentHeight <= 0 || wrappedLines.length > explicitLineCount;
        if (needsHeight && measuredHeight > currentHeight) {
          result = { ...node, height: measuredHeight } as unknown as PenNode;
        }
      }
    }

    // Recurse into children
    if ('children' in result && result.children) {
      const children = result.children;
      const measured = premeasureTextHeights(children);
      if (measured !== children) {
        result = { ...result, children: measured } as unknown as PenNode;
      }
    }

    return result;
  });
}

// ---------------------------------------------------------------------------
// Flatten document tree -> absolute-positioned RenderNode list
// ---------------------------------------------------------------------------

interface ClipInfo {
  x: number;
  y: number;
  w: number;
  h: number;
  rx: number;
}

function sizeToNumber(val: number | string | undefined, fallback: number): number {
  if (typeof val === 'number') return val;
  if (typeof val === 'string') {
    const m = val.match(/\((\d+(?:\.\d+)?)\)/);
    if (m) return parseFloat(m[1]);
    const n = parseFloat(val);
    if (!isNaN(n)) return n;
  }
  return fallback;
}

function cornerRadiusVal(cr: number | [number, number, number, number] | undefined): number {
  if (cr === undefined) return 0;
  if (typeof cr === 'number') return cr;
  return cr[0];
}

export function flattenToRenderNodes(
  nodes: PenNode[],
  offsetX = 0,
  offsetY = 0,
  parentAvailW?: number,
  parentAvailH?: number,
  clipCtx?: ClipInfo,
  depth = 0,
): RenderNode[] {
  const result: RenderNode[] = [];

  // Reverse order: children[0] = top layer = rendered last (frontmost)
  for (let i = nodes.length - 1; i >= 0; i--) {
    const node = nodes[i];
    if (!isNodeVisible(node)) continue;

    // Resolve fill_container / fit_content
    let resolved = node;
    if (parentAvailW !== undefined || parentAvailH !== undefined) {
      let changed = false;
      const r: Record<string, unknown> = { ...node };
      if ('width' in node && typeof node.width !== 'number') {
        const s = parseSizing(node.width);
        if (s === 'fill' && parentAvailW) {
          r.width = parentAvailW;
          changed = true;
        } else if (s === 'fit') {
          r.width = getNodeWidth(node, parentAvailW);
          changed = true;
        }
      }
      if ('height' in node && typeof node.height !== 'number') {
        const s = parseSizing(node.height);
        if (s === 'fill' && parentAvailH) {
          r.height = parentAvailH;
          changed = true;
        } else if (s === 'fit') {
          r.height = getNodeHeight(node, parentAvailH, parentAvailW);
          changed = true;
        }
      }
      if (changed) resolved = r as unknown as PenNode;
    }

    // Compute height for frames without explicit numeric height
    if (
      node.type === 'frame' &&
      'children' in node &&
      node.children?.length &&
      (!('height' in resolved) || typeof resolved.height !== 'number')
    ) {
      const computedH = getNodeHeight(resolved, parentAvailH, parentAvailW);
      if (computedH > 0) resolved = { ...resolved, height: computedH } as unknown as PenNode;
    }

    const absX = (resolved.x ?? 0) + offsetX;
    const absY = (resolved.y ?? 0) + offsetY;

    // Compute authoritative dimensions once via getNodeWidth/getNodeHeight.
    // Used for: RenderNode absW/absH, child available space, and clip rect.
    // This replaces the prior split where absW/absH used sizeToNumber (raw
    // parse + 100 fallback) while child layout used getNodeWidth/getNodeHeight,
    // causing divergence when nodes lacked numeric dimensions.
    const nodeW = getNodeWidth(resolved, parentAvailW);
    const nodeH = getNodeHeight(resolved, parentAvailH, parentAvailW);
    const absW = nodeW > 0 ? nodeW : 'width' in resolved ? sizeToNumber(resolved.width, 100) : 100;
    const absH =
      nodeH > 0 ? nodeH : 'height' in resolved ? sizeToNumber(resolved.height, 100) : 100;

    result.push({
      node: { ...resolved, x: absX, y: absY } as PenNode,
      absX,
      absY,
      absW,
      absH,
      clipRect: clipCtx,
    });

    // Recurse into children
    const children = 'children' in node ? node.children : undefined;
    if (children && children.length > 0) {
      const pad = resolvePadding(
        'padding' in resolved ? (resolved as PenNode & ContainerProps).padding : undefined,
      );
      const childAvailW = Math.max(0, nodeW - pad.left - pad.right);
      const childAvailH = Math.max(0, nodeH - pad.top - pad.bottom);

      const layout =
        ('layout' in node ? (node as ContainerProps).layout : undefined) || inferLayout(node);
      const positioned =
        layout && layout !== 'none' ? computeLayoutPositions(resolved, children) : children;

      // Clipping — only clip for root frames (artboard behavior).
      let childClip = clipCtx;
      const isRootFrame = node.type === 'frame' && depth === 0;
      if (isRootFrame) {
        const crRaw = 'cornerRadius' in node ? cornerRadiusVal(node.cornerRadius) : 0;
        const cr = Math.min(crRaw, nodeH / 2);
        childClip = { x: absX, y: absY, w: nodeW, h: nodeH, rx: cr };
      }

      const childRNs = flattenToRenderNodes(
        positioned,
        absX,
        absY,
        childAvailW,
        childAvailH,
        childClip,
        depth + 1,
      );

      // Propagate parent flip to children
      const parentFlipX = node.flipX === true;
      const parentFlipY = node.flipY === true;
      if (parentFlipX || parentFlipY) {
        const pcx = absX + nodeW / 2;
        const pcy = absY + nodeH / 2;
        for (const crn of childRNs) {
          const updates: Record<string, unknown> = {};
          if (parentFlipX) {
            const ccx = crn.absX + crn.absW / 2;
            crn.absX = 2 * pcx - ccx - crn.absW / 2;
            const childFlip = crn.node.flipX === true;
            updates.flipX = !childFlip || undefined;
          }
          if (parentFlipY) {
            const ccy = crn.absY + crn.absH / 2;
            crn.absY = 2 * pcy - ccy - crn.absH / 2;
            const childFlip = crn.node.flipY === true;
            updates.flipY = !childFlip || undefined;
          }
          crn.node = { ...crn.node, x: crn.absX, y: crn.absY, ...updates } as PenNode;
        }
      }

      // Propagate parent rotation to children
      const parentRot = node.rotation ?? 0;
      if (parentRot !== 0) {
        const cx = absX + nodeW / 2;
        const cy = absY + nodeH / 2;
        const rad = (parentRot * Math.PI) / 180;
        const cosA = Math.cos(rad);
        const sinA = Math.sin(rad);

        for (const crn of childRNs) {
          const ccx = crn.absX + crn.absW / 2;
          const ccy = crn.absY + crn.absH / 2;
          const dx = ccx - cx;
          const dy = ccy - cy;
          const newCx = cx + dx * cosA - dy * sinA;
          const newCy = cy + dx * sinA + dy * cosA;
          crn.absX = newCx - crn.absW / 2;
          crn.absY = newCy - crn.absH / 2;
          const childRot = crn.node.rotation ?? 0;
          crn.node = {
            ...crn.node,
            x: crn.absX,
            y: crn.absY,
            rotation: childRot + parentRot,
          } as PenNode;
        }
      }

      result.push(...childRNs);
    }
  }

  return result;
}

// ---------------------------------------------------------------------------
// Ref resolution — resolve RefNodes to their target components
// ---------------------------------------------------------------------------

/** Resolve RefNodes inline (same logic as use-canvas-sync.ts). */
export function resolveRefs(
  nodes: PenNode[],
  rootNodes: PenNode[],
  findInTree?: (nodes: PenNode[], id: string) => PenNode | null,
  visited = new Set<string>(),
): PenNode[] {
  const finder = findInTree ?? ((ns: PenNode[], id: string) => findNodeInTree(ns, id) ?? null);
  return nodes.flatMap((node) => {
    if (node.type !== 'ref') {
      if ('children' in node && node.children) {
        return [
          { ...node, children: resolveRefs(node.children, rootNodes, finder, visited) } as PenNode,
        ];
      }
      return [node];
    }
    if (visited.has(node.ref)) return [];
    const component = finder(rootNodes, node.ref);
    if (!component) return [];
    visited.add(node.ref);
    const resolved: Record<string, unknown> = { ...component };
    for (const [key, val] of Object.entries(node)) {
      if (key === 'type' || key === 'ref' || key === 'descendants' || key === 'children') continue;
      if (val !== undefined) resolved[key] = val;
    }
    resolved.type = component.type;
    if (!resolved.name) resolved.name = component.name;
    delete resolved.reusable;
    const resolvedNode = resolved as unknown as PenNode;
    if ('children' in component && component.children) {
      const refNode = node as RefNode;
      (resolvedNode as PenNode & ContainerProps).children = remapIds(
        component.children,
        node.id,
        refNode.descendants,
      );
    }
    visited.delete(node.ref);
    return [resolvedNode];
  });
}

export function remapIds(
  children: PenNode[],
  refId: string,
  overrides?: Record<string, Partial<PenNode>>,
): PenNode[] {
  return children.map((child) => {
    const virtualId = `${refId}__${child.id}`;
    const ov = overrides?.[child.id] ?? {};
    const mapped = { ...child, ...ov, id: virtualId } as PenNode;
    if ('children' in mapped && mapped.children) {
      (mapped as PenNode & ContainerProps).children = remapIds(mapped.children, refId, overrides);
    }
    return mapped;
  });
}

// ---------------------------------------------------------------------------
// Component / instance ID collection (from raw tree, before ref resolution)
// ---------------------------------------------------------------------------

export function collectReusableIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'frame' && node.reusable === true) {
      result.add(node.id);
    }
    if ('children' in node && node.children) {
      collectReusableIds(node.children, result);
    }
  }
}

export function collectInstanceIds(nodes: PenNode[], result: Set<string>) {
  for (const node of nodes) {
    if (node.type === 'ref') {
      result.add(node.id);
    }
    if ('children' in node && node.children) {
      collectInstanceIds(node.children, result);
    }
  }
}
