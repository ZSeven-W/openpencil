/**
 * Isomorphic SVG parser: uses DOMParser in browser, regex fallback in Node.js.
 * Consolidated from:
 *   - apps/web/src/utils/svg-parser.ts  (browser, DOMParser + inline style support)
 *   - packages/pen-mcp/src/utils/svg-node-parser.ts  (Node.js, regex-based)
 */

import type { PenNode, PenNodeBase, LineNode } from '@zseven-w/pen-types';
import type { PenFill, PenStroke } from '@zseven-w/pen-types';
import { generateId } from '@zseven-w/pen-core';

interface StyleCtx {
  fill: string | null;
  stroke: string | null;
  strokeWidth: number;
}

const SKIP_TAGS = new Set([
  'defs',
  'style',
  'title',
  'desc',
  'metadata',
  'clippath',
  'mask',
  'filter',
  'lineargradient',
  'radialgradient',
  'symbol',
  'marker',
  'pattern',
  'script',
  'foreignobject',
  'animate',
  'animatemotion',
  'set',
]);

/**
 * Parse an SVG string into editable PenNode array.
 * Isomorphic: uses DOMParser in browser environments, regex fallback in Node.js.
 */
export function parseSvgToNodes(svgText: string, maxDim = 400): PenNode[] {
  if (typeof DOMParser !== 'undefined') {
    return parseSvgDOM(svgText, maxDim);
  }
  return parseSvgRegex(svgText, maxDim);
}

// ---------------------------------------------------------------------------
// Shared dimension parsing
// ---------------------------------------------------------------------------

function parseSvgDimensions(
  viewBox: string | null,
  rawWidth: string | null,
  rawHeight: string | null,
  fill: string | null,
  stroke: string | null,
  strokeWidth: string | null,
  maxDim: number,
): { scale: number; outW: number; outH: number; rootCtx: StyleCtx } {
  const vb = viewBox?.split(/[\s,]+/).map(Number);
  const vbW = vb?.[2] || 100;
  const vbH = vb?.[3] || 100;
  const svgW = rawWidth && /^[\d.]+$/.test(rawWidth) ? parseFloat(rawWidth) : vbW;
  const svgH = rawHeight && /^[\d.]+$/.test(rawHeight) ? parseFloat(rawHeight) : vbH;

  let outW = svgW;
  let outH = svgH;
  if (outW > maxDim || outH > maxDim) {
    const s = maxDim / Math.max(outW, outH);
    outW *= s;
    outH *= s;
  }

  const scale = Math.min(outW / vbW, outH / vbH);
  const rootCtx: StyleCtx = {
    fill,
    stroke,
    strokeWidth: parseFloat(strokeWidth ?? '') || 1,
  };

  return { scale, outW, outH, rootCtx };
}

function wrapIfMultiple(nodes: PenNode[], outW: number, outH: number): PenNode[] {
  if (nodes.length === 0) return [];
  if (nodes.length === 1) return nodes;
  return [
    {
      id: generateId(),
      type: 'frame',
      name: 'SVG',
      width: Math.ceil(outW),
      height: Math.ceil(outH),
      layout: 'none' as const,
      children: nodes,
    } as PenNode,
  ];
}

// ---------------------------------------------------------------------------
// Browser path (DOMParser)
// ---------------------------------------------------------------------------

function parseSvgDOM(svgText: string, maxDim: number): PenNode[] {
  const parser = new DOMParser();
  const doc = parser.parseFromString(svgText, 'image/svg+xml');
  const svg = doc.querySelector('svg');
  if (!svg) return [];

  const { scale, outW, outH, rootCtx } = parseSvgDimensions(
    svg.getAttribute('viewBox'),
    svg.getAttribute('width'),
    svg.getAttribute('height'),
    svg.getAttribute('fill'),
    svg.getAttribute('stroke'),
    svg.getAttribute('stroke-width'),
    maxDim,
  );

  const nodes = parseDOMChildren(svg, scale, rootCtx);
  return wrapIfMultiple(nodes, outW, outH);
}

function parseDOMChildren(parent: Element, scale: number, ctx: StyleCtx): PenNode[] {
  const nodes: PenNode[] = [];
  for (const el of parent.children) {
    const node = parseDOMElement(el, scale, ctx);
    if (node) nodes.push(node);
  }
  return nodes;
}

function parseDOMElement(el: Element, scale: number, parentCtx: StyleCtx): PenNode | null {
  const tag = el.tagName.toLowerCase();
  if (SKIP_TAGS.has(tag)) return null;
  const ctx = mergeStyleCtxDOM(parentCtx, el);

  if (tag === 'g') {
    const children = parseDOMChildren(el, scale, ctx);
    if (children.length === 0) return null;
    if (children.length === 1) return children[0];
    const bounds = computeChildrenBounds(children);
    for (const child of children) {
      offsetChild(child, -bounds.x, -bounds.y);
    }
    return {
      id: generateId(),
      type: 'frame',
      name: el.getAttribute('id') ?? 'Group',
      x: bounds.x,
      y: bounds.y,
      width: Math.ceil(bounds.w),
      height: Math.ceil(bounds.h),
      layout: 'none' as const,
      children,
    } as PenNode;
  }

  const fill = resolveFillDOM(el, ctx);
  const stroke = resolveStrokeDOM(el, ctx, scale);
  const opacity = parseFloat(getAttrDOM(el, 'opacity') ?? '1');
  const base = { id: generateId(), opacity: opacity !== 1 ? opacity : undefined };

  return parseShapeDOM(tag, el, base, fill, stroke, scale);
}

function parseShapeDOM(
  tag: string,
  el: Element,
  base: { id: string; opacity?: number },
  fill: PenFill[] | undefined,
  stroke: PenStroke | undefined,
  scale: number,
): PenNode | null {
  const numDOM = (name: string) => parseFloat(getAttrDOM(el, name) ?? '0') || 0;

  switch (tag) {
    case 'path': {
      const d = el.getAttribute('d');
      if (!d) return null;
      const scaledD = scaleSvgPath(d, scale);
      const bbox = getPathBBoxDOM(scaledD);
      return {
        ...base,
        type: 'path' as const,
        name: el.getAttribute('id') ?? 'Path',
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill,
        stroke,
      } as PenNode;
    }
    case 'rect': {
      return {
        ...base,
        type: 'rectangle' as const,
        name: el.getAttribute('id') ?? 'Rectangle',
        x: numDOM('x') * scale,
        y: numDOM('y') * scale,
        width: Math.ceil(numDOM('width') * scale),
        height: Math.ceil(numDOM('height') * scale),
        cornerRadius: numDOM('rx') * scale || undefined,
        fill,
        stroke,
      } as PenNode;
    }
    case 'circle': {
      const cx = numDOM('cx') * scale;
      const cy = numDOM('cy') * scale;
      const r = numDOM('r') * scale;
      return {
        ...base,
        type: 'ellipse' as const,
        name: el.getAttribute('id') ?? 'Circle',
        x: cx - r,
        y: cy - r,
        width: Math.ceil(r * 2),
        height: Math.ceil(r * 2),
        fill,
        stroke,
      } as PenNode;
    }
    case 'ellipse': {
      const cx = numDOM('cx') * scale;
      const cy = numDOM('cy') * scale;
      const rx = numDOM('rx') * scale;
      const ry = numDOM('ry') * scale;
      return {
        ...base,
        type: 'ellipse' as const,
        name: el.getAttribute('id') ?? 'Ellipse',
        x: cx - rx,
        y: cy - ry,
        width: Math.ceil(rx * 2),
        height: Math.ceil(ry * 2),
        fill,
        stroke,
      } as PenNode;
    }
    case 'line': {
      return {
        ...base,
        type: 'line' as const,
        name: el.getAttribute('id') ?? 'Line',
        x: numDOM('x1') * scale,
        y: numDOM('y1') * scale,
        x2: numDOM('x2') * scale,
        y2: numDOM('y2') * scale,
        stroke: stroke ?? { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
      } as PenNode;
    }
    case 'polygon':
    case 'polyline': {
      const pts = el.getAttribute('points');
      if (!pts) return null;
      const scaledD = scaleSvgPath(pointsToD(pts, tag === 'polygon'), scale);
      const bbox = estimatePathBBox(scaledD);
      return {
        ...base,
        type: 'path' as const,
        name: el.getAttribute('id') ?? (tag === 'polygon' ? 'Polygon' : 'Polyline'),
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill: tag === 'polygon' ? fill : noFill(),
        stroke,
      } as PenNode;
    }
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------
// Node.js path (regex)
// ---------------------------------------------------------------------------

function parseSvgRegex(svgText: string, maxDim: number): PenNode[] {
  const svgMatch = svgText.match(/<svg\b([^>]*)>([\s\S]*)<\/svg>/i);
  if (!svgMatch) return [];
  const svgAttrs = svgMatch[1];
  const svgBody = svgMatch[2];

  const { scale, outW, outH, rootCtx } = parseSvgDimensions(
    extractAttr(svgAttrs, 'viewBox'),
    extractAttr(svgAttrs, 'width'),
    extractAttr(svgAttrs, 'height'),
    extractAttr(svgAttrs, 'fill'),
    extractAttr(svgAttrs, 'stroke'),
    extractAttr(svgAttrs, 'stroke-width'),
    maxDim,
  );

  const nodes = parseRegexElements(svgBody, scale, rootCtx);
  return wrapIfMultiple(nodes, outW, outH);
}

function parseRegexElements(body: string, scale: number, ctx: StyleCtx): PenNode[] {
  const nodes: PenNode[] = [];
  const tagRe = /<(\w+)\b([^>]*?)(?:\/>|>([\s\S]*?)<\/\1>)/gi;
  let match: RegExpExecArray | null;
  while ((match = tagRe.exec(body)) !== null) {
    const tag = match[1].toLowerCase();
    const attrs = match[2];
    const innerBody = match[3] ?? '';
    if (SKIP_TAGS.has(tag)) continue;
    const node = parseRegexTag(tag, attrs, innerBody, scale, ctx);
    if (node) nodes.push(node);
  }
  return nodes;
}

function parseRegexTag(
  tag: string,
  attrs: string,
  innerBody: string,
  scale: number,
  parentCtx: StyleCtx,
): PenNode | null {
  const ctx = mergeStyleCtxAttrs(parentCtx, attrs);

  if (tag === 'g') {
    const children = parseRegexElements(innerBody, scale, ctx);
    if (children.length === 0) return null;
    if (children.length === 1) return children[0];
    const bounds = computeChildrenBounds(children);
    for (const child of children) {
      offsetChild(child, -bounds.x, -bounds.y);
    }
    return {
      id: generateId(),
      type: 'frame',
      name: extractAttr(attrs, 'id') ?? 'Group',
      x: bounds.x,
      y: bounds.y,
      width: Math.ceil(bounds.w),
      height: Math.ceil(bounds.h),
      layout: 'none' as const,
      children,
    } as PenNode;
  }

  const fill = resolveFillAttrs(attrs, ctx);
  const stroke = resolveStrokeAttrs(attrs, ctx, scale);
  const opacity = parseFloat(extractStyleOrAttr(attrs, 'opacity') ?? '1');
  const base = { id: generateId(), opacity: opacity !== 1 ? opacity : undefined };
  const attrNum = (name: string) => parseFloat(extractAttr(attrs, name) ?? '0') || 0;

  switch (tag) {
    case 'path': {
      const d = extractAttr(attrs, 'd');
      if (!d) return null;
      const scaledD = scaleSvgPath(d, scale);
      const bbox = estimatePathBBox(scaledD);
      return {
        ...base,
        type: 'path' as const,
        name: extractAttr(attrs, 'id') ?? 'Path',
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill,
        stroke,
      } as PenNode;
    }
    case 'rect': {
      return {
        ...base,
        type: 'rectangle' as const,
        name: extractAttr(attrs, 'id') ?? 'Rectangle',
        x: attrNum('x') * scale,
        y: attrNum('y') * scale,
        width: Math.ceil(attrNum('width') * scale),
        height: Math.ceil(attrNum('height') * scale),
        cornerRadius: attrNum('rx') * scale || undefined,
        fill,
        stroke,
      } as PenNode;
    }
    case 'circle': {
      const cx = attrNum('cx') * scale;
      const cy = attrNum('cy') * scale;
      const r = attrNum('r') * scale;
      return {
        ...base,
        type: 'ellipse' as const,
        name: extractAttr(attrs, 'id') ?? 'Circle',
        x: cx - r,
        y: cy - r,
        width: Math.ceil(r * 2),
        height: Math.ceil(r * 2),
        fill,
        stroke,
      } as PenNode;
    }
    case 'ellipse': {
      const cx = attrNum('cx') * scale;
      const cy = attrNum('cy') * scale;
      const rx = attrNum('rx') * scale;
      const ry = attrNum('ry') * scale;
      return {
        ...base,
        type: 'ellipse' as const,
        name: extractAttr(attrs, 'id') ?? 'Ellipse',
        x: cx - rx,
        y: cy - ry,
        width: Math.ceil(rx * 2),
        height: Math.ceil(ry * 2),
        fill,
        stroke,
      } as PenNode;
    }
    case 'line': {
      return {
        ...base,
        type: 'line' as const,
        name: extractAttr(attrs, 'id') ?? 'Line',
        x: attrNum('x1') * scale,
        y: attrNum('y1') * scale,
        x2: attrNum('x2') * scale,
        y2: attrNum('y2') * scale,
        stroke: stroke ?? { thickness: 1, fill: [{ type: 'solid', color: '#000000' }] },
      } as PenNode;
    }
    case 'polygon':
    case 'polyline': {
      const pts = extractAttr(attrs, 'points');
      if (!pts) return null;
      const scaledD = scaleSvgPath(pointsToD(pts, tag === 'polygon'), scale);
      const bbox = estimatePathBBox(scaledD);
      return {
        ...base,
        type: 'path' as const,
        name: extractAttr(attrs, 'id') ?? (tag === 'polygon' ? 'Polygon' : 'Polyline'),
        d: scaledD,
        x: bbox.x,
        y: bbox.y,
        width: Math.ceil(bbox.w),
        height: Math.ceil(bbox.h),
        fill: tag === 'polygon' ? fill : noFill(),
        stroke,
      } as PenNode;
    }
    default:
      return null;
  }
}

// ---------------------------------------------------------------------------
// Style helpers — DOM path
// ---------------------------------------------------------------------------

/** Read an attribute from the element, checking inline style= first */
function getAttrDOM(el: Element, name: string): string | null {
  const style = el.getAttribute('style');
  if (style) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const m = style.match(new RegExp(`${escaped}\\s*:\\s*([^;]+)`));
    if (m) return m[1].trim();
  }
  return el.getAttribute(name);
}

function mergeStyleCtxDOM(parent: StyleCtx, el: Element): StyleCtx {
  return {
    fill: getAttrDOM(el, 'fill') ?? parent.fill,
    stroke: getAttrDOM(el, 'stroke') ?? parent.stroke,
    strokeWidth: parseFloat(getAttrDOM(el, 'stroke-width') ?? '') || parent.strokeWidth,
  };
}

function resolveFillDOM(el: Element, ctx: StyleCtx): PenFill[] | undefined {
  const raw = getAttrDOM(el, 'fill') ?? ctx.fill;
  return resolveFillValue(raw);
}

function resolveStrokeDOM(el: Element, ctx: StyleCtx, scale: number): PenStroke | undefined {
  const raw = getAttrDOM(el, 'stroke') ?? ctx.stroke;
  if (!raw || raw === 'none') return undefined;
  if (raw.startsWith('url(')) return undefined;
  const width = (parseFloat(getAttrDOM(el, 'stroke-width') ?? '') || ctx.strokeWidth) * scale;
  return { thickness: width, fill: [{ type: 'solid', color: normalizeColor(raw) }] };
}

// ---------------------------------------------------------------------------
// Style helpers — regex path
// ---------------------------------------------------------------------------

/** Extract an attribute value from an attrs string */
function extractAttr(attrs: string, name: string): string | null {
  const re = new RegExp(`\\b${name}\\s*=\\s*"([^"]*)"`, 'i');
  return attrs.match(re)?.[1] ?? null;
}

/** Extract attribute, checking inline style= first */
function extractStyleOrAttr(attrs: string, name: string): string | null {
  const styleMatch = attrs.match(/\bstyle\s*=\s*"([^"]*)"/);
  if (styleMatch) {
    const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const m = styleMatch[1].match(new RegExp(`${escaped}\\s*:\\s*([^;]+)`));
    if (m) return m[1].trim();
  }
  return extractAttr(attrs, name);
}

function mergeStyleCtxAttrs(parent: StyleCtx, attrs: string): StyleCtx {
  return {
    fill: extractStyleOrAttr(attrs, 'fill') ?? parent.fill,
    stroke: extractStyleOrAttr(attrs, 'stroke') ?? parent.stroke,
    strokeWidth: parseFloat(extractStyleOrAttr(attrs, 'stroke-width') ?? '') || parent.strokeWidth,
  };
}

function resolveFillAttrs(attrs: string, ctx: StyleCtx): PenFill[] | undefined {
  const raw = extractStyleOrAttr(attrs, 'fill') ?? ctx.fill;
  return resolveFillValue(raw);
}

function resolveStrokeAttrs(attrs: string, ctx: StyleCtx, scale: number): PenStroke | undefined {
  const raw = extractStyleOrAttr(attrs, 'stroke') ?? ctx.stroke;
  if (!raw || raw === 'none') return undefined;
  if (raw.startsWith('url(')) return undefined;
  const width =
    (parseFloat(extractStyleOrAttr(attrs, 'stroke-width') ?? '') || ctx.strokeWidth) * scale;
  return { thickness: width, fill: [{ type: 'solid', color: normalizeColor(raw) }] };
}

// ---------------------------------------------------------------------------
// Shared style helpers
// ---------------------------------------------------------------------------

function normalizeColor(raw: string): string {
  if (raw === 'currentColor' || raw === 'inherit') return '#000000';
  return raw;
}

function noFill(): PenFill[] {
  return [{ type: 'solid', color: 'transparent' }];
}

function resolveFillValue(raw: string | null): PenFill[] | undefined {
  if (raw === 'none') return noFill();
  if (raw && raw.startsWith('url(')) return [{ type: 'solid', color: '#000000' }];
  if (raw) return [{ type: 'solid', color: normalizeColor(raw) }];
  return [{ type: 'solid', color: '#000000' }];
}

// ---------------------------------------------------------------------------
// SVG path scaling (token-based, arc-flag aware)
// ---------------------------------------------------------------------------

/**
 * Scale all coordinates in an SVG path `d` string by a factor.
 * Handles M/L/C/S/Q/T/H/V/A commands (both absolute and relative).
 * For arc (A) commands, only rx/ry/x/y are scaled — flags and rotation are preserved.
 */
function scaleSvgPath(d: string, scale: number): string {
  if (scale === 1) return d;

  const tokens = d.match(/[a-zA-Z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g);
  if (!tokens) return d;

  let result = '';
  let cmd = '';
  let paramIdx = 0;

  for (const tok of tokens) {
    if (/[a-zA-Z]/.test(tok)) {
      cmd = tok;
      paramIdx = 0;
      result += tok;
      continue;
    }

    const n = parseFloat(tok);
    const upper = cmd.toUpperCase();

    if (upper === 'A') {
      // Arc: rx ry x-rotation large-arc-flag sweep-flag x y (7 params per arc)
      const pos = paramIdx % 7;
      // Scale rx(0), ry(1), x(5), y(6); keep rotation(2), flags(3,4) unchanged
      const shouldScale = pos === 0 || pos === 1 || pos === 5 || pos === 6;
      result += ' ' + (shouldScale ? n * scale : n);
    } else {
      result += ' ' + n * scale;
    }
    paramIdx++;
  }

  return result.trim();
}

// ---------------------------------------------------------------------------
// Path bounding box
// ---------------------------------------------------------------------------

/**
 * In browser environments, use the SVG DOM for accurate bounding box.
 * Falls back to coordinate estimation (used in Node.js and as catch fallback).
 */
function getPathBBoxDOM(d: string): { x: number; y: number; w: number; h: number } {
  try {
    const svgEl = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    const pathEl = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    pathEl.setAttribute('d', d);
    svgEl.appendChild(pathEl);
    svgEl.style.position = 'absolute';
    svgEl.style.width = '0';
    svgEl.style.height = '0';
    svgEl.style.overflow = 'hidden';
    document.body.appendChild(svgEl);
    const bbox = pathEl.getBBox();
    document.body.removeChild(svgEl);
    return { x: bbox.x, y: bbox.y, w: bbox.width, h: bbox.height };
  } catch {
    return estimatePathBBox(d);
  }
}

/**
 * Estimate bounding box from SVG path coordinates.
 * Command-aware (tracks M, L, H, V, C, S, Q, A).
 * Not pixel-perfect for curves but sufficient for layout.
 */
function estimatePathBBox(d: string): { x: number; y: number; w: number; h: number } {
  const tokens = d.match(/[a-zA-Z]|[-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?/g);
  if (!tokens) return { x: 0, y: 0, w: 100, h: 100 };

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  let curX = 0;
  let curY = 0;
  let cmd = '';
  let paramIdx = 0;

  function track(x: number, y: number) {
    if (x < minX) minX = x;
    if (x > maxX) maxX = x;
    if (y < minY) minY = y;
    if (y > maxY) maxY = y;
  }

  for (const tok of tokens) {
    if (/[a-zA-Z]/.test(tok)) {
      cmd = tok;
      paramIdx = 0;
      continue;
    }

    const n = parseFloat(tok);
    const upper = cmd.toUpperCase();
    const isRel = cmd !== upper;

    switch (upper) {
      case 'M':
      case 'L':
      case 'T': {
        if (paramIdx % 2 === 0) {
          curX = isRel ? curX + n : n;
        } else {
          curY = isRel ? curY + n : n;
          track(curX, curY);
        }
        break;
      }
      case 'H': {
        curX = isRel ? curX + n : n;
        track(curX, curY);
        break;
      }
      case 'V': {
        curY = isRel ? curY + n : n;
        track(curX, curY);
        break;
      }
      case 'C': {
        // 6 params: cx1 cy1 cx2 cy2 x y — track all as approximation
        const pos = paramIdx % 6;
        if (pos % 2 === 0) {
          const x = isRel ? curX + n : n;
          track(x, curY);
          if (pos === 4) curX = x;
        } else {
          const y = isRel ? curY + n : n;
          track(curX, y);
          if (pos === 5) curY = y;
        }
        break;
      }
      case 'S':
      case 'Q': {
        // S: 4 params (cx2 cy2 x y), Q: 4 params (cx cy x y)
        const pos = paramIdx % 4;
        if (pos % 2 === 0) {
          const x = isRel ? curX + n : n;
          track(x, curY);
          if (pos === 2) curX = x;
        } else {
          const y = isRel ? curY + n : n;
          track(curX, y);
          if (pos === 3) curY = y;
        }
        break;
      }
      case 'A': {
        // 7 params: rx ry rotation large-arc sweep x y
        const pos = paramIdx % 7;
        if (pos === 5) {
          curX = isRel ? curX + n : n;
        } else if (pos === 6) {
          curY = isRel ? curY + n : n;
          track(curX, curY);
        }
        break;
      }
      default:
        break;
    }
    paramIdx++;
  }

  if (!isFinite(minX)) return { x: 0, y: 0, w: 100, h: 100 };
  return {
    x: Math.floor(minX),
    y: Math.floor(minY),
    w: Math.ceil(maxX - Math.floor(minX)) || 1,
    h: Math.ceil(maxY - Math.floor(minY)) || 1,
  };
}

// ---------------------------------------------------------------------------
// Shared geometry utilities
// ---------------------------------------------------------------------------

/** Compute the bounding box that encloses all children (including stroke extent) */
function computeChildrenBounds(children: PenNode[]): {
  x: number;
  y: number;
  w: number;
  h: number;
} {
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  for (const child of children) {
    const cx = child.x ?? 0;
    const cy = child.y ?? 0;
    const stroke =
      'stroke' in child ? (child as PenNode & { stroke?: PenStroke }).stroke : undefined;
    const halfStroke = (typeof stroke?.thickness === 'number' ? stroke.thickness : 0) / 2;

    if (child.type === 'line') {
      const lineChild = child as LineNode;
      const x2 = lineChild.x2 ?? cx;
      const y2 = lineChild.y2 ?? cy;
      minX = Math.min(minX, cx - halfStroke, x2 - halfStroke);
      minY = Math.min(minY, cy - halfStroke, y2 - halfStroke);
      maxX = Math.max(maxX, cx + halfStroke, x2 + halfStroke);
      maxY = Math.max(maxY, cy + halfStroke, y2 + halfStroke);
    } else {
      const sized = child as PenNode & { width?: number; height?: number };
      const cw = sized.width ?? 0;
      const ch = sized.height ?? 0;
      minX = Math.min(minX, cx - halfStroke);
      minY = Math.min(minY, cy - halfStroke);
      maxX = Math.max(maxX, cx + cw + halfStroke);
      maxY = Math.max(maxY, cy + ch + halfStroke);
    }
  }

  if (!isFinite(minX)) return { x: 0, y: 0, w: 0, h: 0 };
  return { x: minX, y: minY, w: maxX - minX, h: maxY - minY };
}

/** Offset a child node's position to make it relative to a parent origin */
function offsetChild(node: PenNode, dx: number, dy: number) {
  const mutable = node as PenNodeBase;
  if (node.type === 'line') {
    const lineNode = node as LineNode;
    mutable.x = (mutable.x ?? 0) + dx;
    mutable.y = (mutable.y ?? 0) + dy;
    lineNode.x2 = (lineNode.x2 ?? 0) + dx;
    lineNode.y2 = (lineNode.y2 ?? 0) + dy;
  } else {
    mutable.x = (mutable.x ?? 0) + dx;
    mutable.y = (mutable.y ?? 0) + dy;
  }
}

function pointsToD(points: string, close: boolean): string {
  const nums = points
    .trim()
    .split(/[\s,]+/)
    .map(Number);
  if (nums.length < 2) return '';
  let d = `M${nums[0]} ${nums[1]}`;
  for (let i = 2; i < nums.length - 1; i += 2) {
    d += `L${nums[i]} ${nums[i + 1]}`;
  }
  if (close) d += 'Z';
  return d;
}
