import {
  anchorsToPathData,
  getPathBoundsFromAnchors,
  inferPathAnchorPointType,
  pathDataToAnchors,
  type PathBounds,
} from '@zseven-w/pen-core';

import type { PathNode, PenPathAnchor, PenPathPointType } from '@/types/pen';

export type PathControlKind = 'anchor' | 'handleIn' | 'handleOut';

export interface PathFrame {
  x: number;
  y: number;
  width: number;
  height: number;
}

export interface EditablePathState {
  anchors: PenPathAnchor[];
  sceneAnchors: PenPathAnchor[];
  closed: boolean;
  localBounds: PathBounds;
}

const DEFAULT_HANDLE_RATIO = 1 / 3;
const HANDLE_EPSILON = 1e-6;

export function getEditablePathState(node: PathNode, frame: PathFrame): EditablePathState | null {
  const parsed = node.anchors
    ? {
        anchors: cloneAnchors(node.anchors),
        closed: node.closed ?? /[Zz]\s*$/.test(node.d),
      }
    : pathDataToAnchors(node.d);

  if (!parsed) return null;

  const localBounds = getPathBoundsFromAnchors(parsed.anchors, parsed.closed);
  return {
    anchors: cloneAnchors(parsed.anchors),
    sceneAnchors: mapAnchorsToScene(parsed.anchors, localBounds, frame),
    closed: parsed.closed,
    localBounds,
  };
}

export function mapAnchorsToScene(
  anchors: PenPathAnchor[],
  localBounds: PathBounds,
  frame: PathFrame,
): PenPathAnchor[] {
  const sx = localBounds.width > 0 ? frame.width / localBounds.width : 1;
  const sy = localBounds.height > 0 ? frame.height / localBounds.height : 1;

  return anchors.map((anchor) => ({
    x: frame.x + (anchor.x - localBounds.x) * sx,
    y: frame.y + (anchor.y - localBounds.y) * sy,
    handleIn: anchor.handleIn ? { x: anchor.handleIn.x * sx, y: anchor.handleIn.y * sy } : null,
    handleOut: anchor.handleOut ? { x: anchor.handleOut.x * sx, y: anchor.handleOut.y * sy } : null,
    ...(anchor.pointType ? { pointType: anchor.pointType } : {}),
  }));
}

export function movePathControl(
  anchors: PenPathAnchor[],
  anchorIndex: number,
  control: PathControlKind,
  dx: number,
  dy: number,
): PenPathAnchor[] {
  return anchors.map((anchor, index) => {
    if (index !== anchorIndex) return cloneAnchor(anchor);

    if (control === 'anchor') {
      return {
        ...cloneAnchor(anchor),
        x: anchor.x + dx,
        y: anchor.y + dy,
      };
    }

    const nextHandle = anchor[control]
      ? {
          x: anchor[control]!.x + dx,
          y: anchor[control]!.y + dy,
        }
      : { x: dx, y: dy };

    const resolvedPointType = getPathPointType(anchor);
    const nextAnchor: PenPathAnchor = {
      ...cloneAnchor(anchor),
      [control]: nextHandle,
    };

    if (resolvedPointType === 'mirrored') {
      const oppositeControl = control === 'handleIn' ? 'handleOut' : 'handleIn';
      if (nextAnchor[oppositeControl]) {
        nextAnchor[oppositeControl] = {
          x: -nextHandle.x,
          y: -nextHandle.y,
        };
      }
      nextAnchor.pointType = 'mirrored';
      return nextAnchor;
    }

    if (anchor.pointType === 'independent') {
      nextAnchor.pointType = 'independent';
    }
    return nextAnchor;
  });
}

export function setPathPointType(
  anchors: PenPathAnchor[],
  anchorIndex: number,
  pointType: PenPathPointType,
  closed: boolean,
): PenPathAnchor[] {
  return anchors.map((anchor, index) => {
    if (index !== anchorIndex) return cloneAnchor(anchor);

    if (pointType === 'corner') {
      return {
        ...cloneAnchor(anchor),
        handleIn: null,
        handleOut: null,
        pointType: 'corner',
      };
    }

    const defaults = buildDefaultHandles(anchors, anchorIndex, closed);
    if (pointType === 'mirrored') {
      return applyMirroredPointType(anchor, defaults);
    }

    return {
      ...cloneAnchor(anchor),
      handleIn: anchor.handleIn ?? defaults.handleIn,
      handleOut: anchor.handleOut ?? defaults.handleOut,
      pointType: 'independent',
    };
  });
}

export function resetPathPointHandles(
  anchors: PenPathAnchor[],
  anchorIndex: number,
  closed: boolean,
): PenPathAnchor[] {
  return anchors.map((anchor, index) => {
    if (index !== anchorIndex) return cloneAnchor(anchor);

    const defaults = buildDefaultHandles(anchors, anchorIndex, closed);
    const pointType = inferPathAnchorPointType({
      ...anchor,
      handleIn: defaults.handleIn,
      handleOut: defaults.handleOut,
    });

    return {
      ...cloneAnchor(anchor),
      handleIn: defaults.handleIn,
      handleOut: defaults.handleOut,
      pointType,
    };
  });
}

export function bakeSceneAnchorsToPathNode(
  sceneAnchors: PenPathAnchor[],
  closed: boolean,
  parentSceneOrigin: { x: number; y: number },
): Pick<PathNode, 'x' | 'y' | 'width' | 'height' | 'd' | 'anchors' | 'closed'> | null {
  if (sceneAnchors.length < 2) return null;

  const sceneBounds = getPathBoundsFromAnchors(sceneAnchors, closed);
  if (sceneBounds.width < 0.001 && sceneBounds.height < 0.001) return null;

  const anchors = sceneAnchors.map((anchor) => ({
    ...cloneAnchor(anchor),
    x: anchor.x - sceneBounds.x,
    y: anchor.y - sceneBounds.y,
  }));

  return {
    x: sceneBounds.x - parentSceneOrigin.x,
    y: sceneBounds.y - parentSceneOrigin.y,
    width: sceneBounds.width,
    height: sceneBounds.height,
    closed,
    d: anchorsToPathData(anchors, closed),
    anchors,
  };
}

function buildDefaultHandles(
  anchors: PenPathAnchor[],
  anchorIndex: number,
  closed: boolean,
): Pick<PenPathAnchor, 'handleIn' | 'handleOut'> {
  const current = anchors[anchorIndex];
  const prev = getAdjacentAnchor(anchors, anchorIndex, -1, closed);
  const next = getAdjacentAnchor(anchors, anchorIndex, 1, closed);

  if (!prev && !next) {
    return { handleIn: null, handleOut: null };
  }

  if (prev && next) {
    const tangent = normalizeVector({
      x: next.x - prev.x,
      y: next.y - prev.y,
    });
    if (tangent) {
      const inLen = distance(current, prev) * DEFAULT_HANDLE_RATIO;
      const outLen = distance(current, next) * DEFAULT_HANDLE_RATIO;
      return {
        handleIn: { x: -tangent.x * inLen, y: -tangent.y * inLen },
        handleOut: { x: tangent.x * outLen, y: tangent.y * outLen },
      };
    }
  }

  if (prev) {
    return {
      handleIn: {
        x: (prev.x - current.x) * DEFAULT_HANDLE_RATIO,
        y: (prev.y - current.y) * DEFAULT_HANDLE_RATIO,
      },
      handleOut: null,
    };
  }

  return {
    handleIn: null,
    handleOut: {
      x: (next!.x - current.x) * DEFAULT_HANDLE_RATIO,
      y: (next!.y - current.y) * DEFAULT_HANDLE_RATIO,
    },
  };
}

function applyMirroredPointType(
  anchor: PenPathAnchor,
  defaults: Pick<PenPathAnchor, 'handleIn' | 'handleOut'>,
): PenPathAnchor {
  const preferredDirection = normalizeVector(anchor.handleOut) ??
    invertVector(normalizeVector(anchor.handleIn)) ??
    normalizeVector(defaults.handleOut) ??
    invertVector(normalizeVector(defaults.handleIn)) ?? { x: 1, y: 0 };

  const mirrorLength =
    average(
      [
        handleLength(anchor.handleIn),
        handleLength(anchor.handleOut),
        handleLength(defaults.handleIn),
        handleLength(defaults.handleOut),
      ].filter((value) => value > HANDLE_EPSILON),
    ) || 40;

  const hasIn = !!anchor.handleIn || !!defaults.handleIn;
  const hasOut = !!anchor.handleOut || !!defaults.handleOut;

  return {
    ...cloneAnchor(anchor),
    handleIn: hasIn
      ? { x: -preferredDirection.x * mirrorLength, y: -preferredDirection.y * mirrorLength }
      : null,
    handleOut: hasOut
      ? { x: preferredDirection.x * mirrorLength, y: preferredDirection.y * mirrorLength }
      : null,
    pointType: 'mirrored',
  };
}

function getAdjacentAnchor(
  anchors: PenPathAnchor[],
  anchorIndex: number,
  delta: -1 | 1,
  closed: boolean,
): PenPathAnchor | null {
  const nextIndex = anchorIndex + delta;
  if (nextIndex >= 0 && nextIndex < anchors.length) {
    return anchors[nextIndex];
  }
  if (!closed || anchors.length === 0) return null;
  return anchors[(nextIndex + anchors.length) % anchors.length];
}

function getPathPointType(anchor: PenPathAnchor): PenPathPointType {
  return anchor.pointType ?? inferPathAnchorPointType(anchor);
}

function cloneAnchors(anchors: PenPathAnchor[]): PenPathAnchor[] {
  return anchors.map((anchor) => cloneAnchor(anchor));
}

function cloneAnchor(anchor: PenPathAnchor): PenPathAnchor {
  return {
    x: anchor.x,
    y: anchor.y,
    handleIn: anchor.handleIn ? { ...anchor.handleIn } : null,
    handleOut: anchor.handleOut ? { ...anchor.handleOut } : null,
    ...(anchor.pointType ? { pointType: anchor.pointType } : {}),
  };
}

function normalizeVector(
  handle: { x: number; y: number } | null | undefined,
): { x: number; y: number } | null {
  if (!handle) return null;
  const length = Math.hypot(handle.x, handle.y);
  if (length <= HANDLE_EPSILON) return null;
  return { x: handle.x / length, y: handle.y / length };
}

function invertVector(handle: { x: number; y: number } | null): { x: number; y: number } | null {
  if (!handle) return null;
  return { x: -handle.x, y: -handle.y };
}

function handleLength(handle: { x: number; y: number } | null | undefined): number {
  return handle ? Math.hypot(handle.x, handle.y) : 0;
}

function average(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((sum, value) => sum + value, 0) / values.length;
}

function distance(a: { x: number; y: number }, b: { x: number; y: number }): number {
  return Math.hypot(a.x - b.x, a.y - b.y);
}
