import type { PenPathAnchor, PenPathHandle, PenPathPointType } from '@zseven-w/pen-types';

export interface PathAnchorParseResult {
  anchors: PenPathAnchor[];
  closed: boolean;
}

export interface PathBounds {
  x: number;
  y: number;
  width: number;
  height: number;
}

const COMMAND_RE = /^[MmLlHhVvCcSsZz]$/;
const UNSUPPORTED_COMMAND_RE = /[QqTtAa]/;
const TOKEN_RE = /[MmLlHhVvCcSsZz]|[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?/g;
const EPSILON = 1e-6;

export function pathDataToAnchors(d: string): PathAnchorParseResult | null {
  if (!d.trim()) return null;
  if (UNSUPPORTED_COMMAND_RE.test(d)) return null;

  const tokens = d.match(TOKEN_RE);
  if (!tokens || tokens.length === 0) return null;

  const anchors: PenPathAnchor[] = [];
  let closed = false;
  let index = 0;
  let currentX = 0;
  let currentY = 0;
  let lastCommand = '';
  let lastCubicControlX: number | null = null;
  let lastCubicControlY: number | null = null;

  const readNumbers = (count: number): number[] | null => {
    const values: number[] = [];
    for (let i = 0; i < count; i++) {
      const token = tokens[index];
      if (!token || COMMAND_RE.test(token)) return null;
      const value = Number.parseFloat(token);
      if (!Number.isFinite(value)) return null;
      values.push(value);
      index += 1;
    }
    return values;
  };

  const pushAnchor = (x: number, y: number, handleIn: PenPathAnchor['handleIn']) => {
    anchors.push({
      x,
      y,
      handleIn,
      handleOut: null,
    });
    currentX = x;
    currentY = y;
  };

  while (index < tokens.length) {
    let command = tokens[index];
    if (COMMAND_RE.test(command)) {
      lastCommand = command;
      index += 1;
    } else if (lastCommand) {
      command = lastCommand;
    } else {
      return null;
    }

    switch (command) {
      case 'M':
      case 'm': {
        const pair = readNumbers(2);
        if (!pair) return null;
        const x = command === 'm' ? currentX + pair[0] : pair[0];
        const y = command === 'm' ? currentY + pair[1] : pair[1];
        if (anchors.length > 0) return null;
        pushAnchor(x, y, null);
        lastCommand = command === 'm' ? 'l' : 'L';
        lastCubicControlX = null;
        lastCubicControlY = null;
        break;
      }

      case 'L':
      case 'l': {
        const pair = readNumbers(2);
        if (!pair || anchors.length === 0) return null;
        const x = command === 'l' ? currentX + pair[0] : pair[0];
        const y = command === 'l' ? currentY + pair[1] : pair[1];
        pushAnchor(x, y, null);
        lastCubicControlX = null;
        lastCubicControlY = null;
        break;
      }

      case 'H':
      case 'h': {
        const pair = readNumbers(1);
        if (!pair || anchors.length === 0) return null;
        const x = command === 'h' ? currentX + pair[0] : pair[0];
        pushAnchor(x, currentY, null);
        lastCubicControlX = null;
        lastCubicControlY = null;
        break;
      }

      case 'V':
      case 'v': {
        const pair = readNumbers(1);
        if (!pair || anchors.length === 0) return null;
        const y = command === 'v' ? currentY + pair[0] : pair[0];
        pushAnchor(currentX, y, null);
        lastCubicControlX = null;
        lastCubicControlY = null;
        break;
      }

      case 'C':
      case 'c': {
        const values = readNumbers(6);
        if (!values || anchors.length === 0) return null;
        const prev = anchors[anchors.length - 1];
        const cx1 = command === 'c' ? currentX + values[0] : values[0];
        const cy1 = command === 'c' ? currentY + values[1] : values[1];
        const cx2 = command === 'c' ? currentX + values[2] : values[2];
        const cy2 = command === 'c' ? currentY + values[3] : values[3];
        const x = command === 'c' ? currentX + values[4] : values[4];
        const y = command === 'c' ? currentY + values[5] : values[5];
        prev.handleOut = { x: cx1 - prev.x, y: cy1 - prev.y };
        pushAnchor(x, y, { x: cx2 - x, y: cy2 - y });
        lastCubicControlX = cx2;
        lastCubicControlY = cy2;
        break;
      }

      case 'S':
      case 's': {
        const values = readNumbers(4);
        if (!values || anchors.length === 0) return null;
        const prev = anchors[anchors.length - 1];
        const cx1 =
          lastCommand.toLowerCase() === 'c' || lastCommand.toLowerCase() === 's'
            ? 2 * currentX - (lastCubicControlX ?? currentX)
            : currentX;
        const cy1 =
          lastCommand.toLowerCase() === 'c' || lastCommand.toLowerCase() === 's'
            ? 2 * currentY - (lastCubicControlY ?? currentY)
            : currentY;
        const cx2 = command === 's' ? currentX + values[0] : values[0];
        const cy2 = command === 's' ? currentY + values[1] : values[1];
        const x = command === 's' ? currentX + values[2] : values[2];
        const y = command === 's' ? currentY + values[3] : values[3];
        prev.handleOut = { x: cx1 - prev.x, y: cy1 - prev.y };
        pushAnchor(x, y, { x: cx2 - x, y: cy2 - y });
        lastCubicControlX = cx2;
        lastCubicControlY = cy2;
        break;
      }

      case 'Z':
      case 'z':
        if (anchors.length < 2) return null;
        closed = true;
        lastCubicControlX = null;
        lastCubicControlY = null;
        break;

      default:
        return null;
    }
  }

  if (anchors.length < 2) return null;
  return { anchors, closed };
}

export function anchorsToPathData(anchors: PenPathAnchor[], closed: boolean): string {
  if (anchors.length === 0) return '';

  const parts: string[] = [`M ${anchors[0].x} ${anchors[0].y}`];
  for (let i = 1; i < anchors.length; i++) {
    appendSegment(parts, anchors[i - 1], anchors[i]);
  }

  if (closed && anchors.length > 1) {
    appendSegment(parts, anchors[anchors.length - 1], anchors[0]);
    parts.push('Z');
  }

  return parts.join(' ');
}

function appendSegment(parts: string[], from: PenPathAnchor, to: PenPathAnchor) {
  if (!from.handleOut && !to.handleIn) {
    parts.push(`L ${to.x} ${to.y}`);
    return;
  }

  const cx1 = from.x + (from.handleOut?.x ?? 0);
  const cy1 = from.y + (from.handleOut?.y ?? 0);
  const cx2 = to.x + (to.handleIn?.x ?? 0);
  const cy2 = to.y + (to.handleIn?.y ?? 0);
  parts.push(`C ${cx1} ${cy1} ${cx2} ${cy2} ${to.x} ${to.y}`);
}

export function inferPathAnchorPointType(anchor: PenPathAnchor): PenPathPointType {
  const hasIn = hasMeaningfulHandle(anchor.handleIn);
  const hasOut = hasMeaningfulHandle(anchor.handleOut);

  if (!hasIn && !hasOut) return 'corner';
  if (hasIn && hasOut && areMirroredHandles(anchor.handleIn!, anchor.handleOut!)) {
    return 'mirrored';
  }
  return 'independent';
}

export function getPathBoundsFromAnchors(anchors: PenPathAnchor[], closed: boolean): PathBounds {
  if (anchors.length === 0) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }

  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;

  const includePoint = (x: number, y: number) => {
    minX = Math.min(minX, x);
    minY = Math.min(minY, y);
    maxX = Math.max(maxX, x);
    maxY = Math.max(maxY, y);
  };

  const includeSegment = (from: PenPathAnchor, to: PenPathAnchor) => {
    includePoint(from.x, from.y);
    includePoint(to.x, to.y);

    const p0 = { x: from.x, y: from.y };
    const p1 = {
      x: from.x + (from.handleOut?.x ?? 0),
      y: from.y + (from.handleOut?.y ?? 0),
    };
    const p2 = {
      x: to.x + (to.handleIn?.x ?? 0),
      y: to.y + (to.handleIn?.y ?? 0),
    };
    const p3 = { x: to.x, y: to.y };

    const roots = new Set<number>([
      ...solveCubicDerivativeRoots(p0.x, p1.x, p2.x, p3.x),
      ...solveCubicDerivativeRoots(p0.y, p1.y, p2.y, p3.y),
    ]);

    for (const t of roots) {
      includePoint(
        evaluateCubic(p0.x, p1.x, p2.x, p3.x, t),
        evaluateCubic(p0.y, p1.y, p2.y, p3.y, t),
      );
    }
  };

  for (let i = 1; i < anchors.length; i++) {
    includeSegment(anchors[i - 1], anchors[i]);
  }

  if (closed && anchors.length > 1) {
    includeSegment(anchors[anchors.length - 1], anchors[0]);
  } else if (anchors.length === 1) {
    includePoint(anchors[0].x, anchors[0].y);
  }

  if (!Number.isFinite(minX) || !Number.isFinite(minY)) {
    return { x: 0, y: 0, width: 0, height: 0 };
  }

  return {
    x: minX,
    y: minY,
    width: maxX - minX,
    height: maxY - minY,
  };
}

function hasMeaningfulHandle(handle: PenPathHandle | null): boolean {
  if (!handle) return false;
  return Math.abs(handle.x) > EPSILON || Math.abs(handle.y) > EPSILON;
}

function areMirroredHandles(a: PenPathHandle, b: PenPathHandle): boolean {
  const lenA = Math.hypot(a.x, a.y);
  const lenB = Math.hypot(b.x, b.y);
  if (lenA <= EPSILON || lenB <= EPSILON) return false;

  const tol = Math.max(EPSILON, Math.max(lenA, lenB) * 1e-3);
  return Math.abs(a.x + b.x) <= tol && Math.abs(a.y + b.y) <= tol;
}

function solveCubicDerivativeRoots(p0: number, p1: number, p2: number, p3: number): number[] {
  const a = -p0 + 3 * p1 - 3 * p2 + p3;
  const b = 2 * (p0 - 2 * p1 + p2);
  const c = -p0 + p1;

  if (Math.abs(a) <= EPSILON) {
    if (Math.abs(b) <= EPSILON) return [];
    const t = -c / b;
    return isUnitIntervalInterior(t) ? [t] : [];
  }

  const disc = b * b - 4 * a * c;
  if (disc < -EPSILON) return [];
  if (Math.abs(disc) <= EPSILON) {
    const t = -b / (2 * a);
    return isUnitIntervalInterior(t) ? [t] : [];
  }

  const sqrtDisc = Math.sqrt(Math.max(disc, 0));
  const t1 = (-b + sqrtDisc) / (2 * a);
  const t2 = (-b - sqrtDisc) / (2 * a);
  return [t1, t2].filter(isUnitIntervalInterior);
}

function isUnitIntervalInterior(t: number): boolean {
  return t > EPSILON && t < 1 - EPSILON;
}

function evaluateCubic(p0: number, p1: number, p2: number, p3: number, t: number): number {
  const mt = 1 - t;
  return mt * mt * mt * p0 + 3 * mt * mt * t * p1 + 3 * mt * t * t * p2 + t * t * t * p3;
}
