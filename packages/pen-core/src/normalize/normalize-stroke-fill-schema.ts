import type { PenNode, PenFill, PenStroke, SolidFill } from '@zseven-w/pen-types';

/**
 * Normalize stroke/fill schema violations commonly emitted by AI sub-agents
 * (MiniMax M2.7, GLM, Kimi) that don't strictly follow the PenNode types.
 *
 * Three classes of schema violation are repaired in place, recursively
 * across the tree:
 *
 * 1. `stroke` as an array of one entry — AI wraps the stroke object in an
 *    array as if it were `fill` (which IS an array). We unwrap the first
 *    element and continue normalizing it.
 *
 * 2. Stroke value shaped like a `SolidFill` ({ type, color }) instead of
 *    a `PenStroke` ({ thickness, fill }). We migrate the inner `color`
 *    into a proper `stroke.fill[0]`, pull the `strokeWidth` top-level
 *    field (the CSS/SVG-style spelling that many models emit) into
 *    `stroke.thickness`, and delete the stray `strokeWidth`. If neither
 *    a thickness nor a strokeWidth is present, default to 2 so the
 *    stroke actually draws something.
 *
 * 3. Fill entries with illegal CSS-keyword colors (`"none"`, `"transparent"`)
 *    are dropped. The 8-digit transparent hex (`"#00000000"`) is valid
 *    and kept. The same rule applies to any `stroke.fill[]` entries.
 *
 * Returns nothing — the tree is mutated in place, matching the other
 * pen-core normalize passes. Callers that rely on Zustand publish
 * semantics should route the result through `forcePageResync()` the same
 * way they already do for other mutating post-streaming passes.
 */
export function normalizeStrokeFillSchema(node: PenNode): void {
  normalizeNodeStroke(node);
  normalizeNodeFill(node);

  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      normalizeStrokeFillSchema(child);
    }
  }
}

// ---------------------------------------------------------------------------
// Stroke normalization
// ---------------------------------------------------------------------------

interface MaybeStrokeHolder {
  stroke?: unknown;
  strokeWidth?: unknown;
}

function normalizeNodeStroke(node: PenNode): void {
  const rec = node as unknown as MaybeStrokeHolder;
  const rawStroke = rec.stroke;
  if (rawStroke === undefined || rawStroke === null) return;

  // (1) Unwrap `stroke: [ ... ]` by taking the first element.
  let stroke: unknown = rawStroke;
  if (Array.isArray(stroke)) {
    stroke = stroke.length > 0 ? stroke[0] : undefined;
  }
  if (!stroke || typeof stroke !== 'object') {
    delete rec.stroke;
    return;
  }

  // (2) Detect the fill-shape-as-stroke pattern and migrate it.
  const maybeFillShape = stroke as {
    type?: unknown;
    color?: unknown;
    thickness?: unknown;
    fill?: unknown;
  };
  const looksLikeFillShape =
    typeof maybeFillShape.type === 'string' &&
    typeof maybeFillShape.color === 'string' &&
    maybeFillShape.thickness === undefined &&
    maybeFillShape.fill === undefined;

  if (looksLikeFillShape) {
    const thickness = readThickness(rec);
    rec.stroke = {
      thickness,
      fill: [
        {
          type: 'solid',
          color: maybeFillShape.color as string,
        } as SolidFill,
      ],
    } as PenStroke;
    delete rec.strokeWidth;
    // Now clean illegal color inside the migrated stroke.fill
    stripIllegalColorsFromStrokeFill(node);
    return;
  }

  // Otherwise we have something that looks like a real PenStroke — fix
  // missing thickness, clean up illegal colors, and persist any
  // strokeWidth field that survived as a top-level property.
  const strokeObj = stroke as Partial<PenStroke> & { [k: string]: unknown };
  if (strokeObj.thickness === undefined || strokeObj.thickness === null) {
    const width = readThickness(rec);
    (strokeObj as { thickness?: number }).thickness = width;
  }
  rec.stroke = strokeObj as PenStroke;
  delete rec.strokeWidth;
  stripIllegalColorsFromStrokeFill(node);

  // If after cleanup the stroke has no fill at all, drop the whole stroke.
  const cleaned = rec.stroke as PenStroke | undefined;
  if (cleaned && (!cleaned.fill || cleaned.fill.length === 0)) {
    delete rec.stroke;
  }
}

function readThickness(rec: MaybeStrokeHolder): number {
  const raw = rec.strokeWidth;
  if (typeof raw === 'number' && raw > 0) return raw;
  if (typeof raw === 'string') {
    const n = parseFloat(raw);
    if (Number.isFinite(n) && n > 0) return n;
  }
  return 2;
}

function stripIllegalColorsFromStrokeFill(node: PenNode): void {
  const rec = node as unknown as { stroke?: { fill?: unknown } };
  const stroke = rec.stroke;
  if (!stroke || typeof stroke !== 'object') return;
  const fillArr = stroke.fill;
  if (!Array.isArray(fillArr)) return;
  (stroke as { fill?: PenFill[] }).fill = fillArr.filter(
    (f) => isLegalFillEntry(f),
  ) as PenFill[];
}

// ---------------------------------------------------------------------------
// Fill normalization
// ---------------------------------------------------------------------------

/**
 * Explicit transparent hex. Used when we need to preserve a node's
 * "no fill" intent but cannot leave the fill field absent (which would
 * make canvas-object-factory fall back to an opaque default gray fill).
 */
const EXPLICIT_TRANSPARENT_FILL: SolidFill = {
  type: 'solid',
  color: '#00000000',
};

function normalizeNodeFill(node: PenNode): void {
  const rec = node as unknown as { fill?: unknown };
  const raw = rec.fill;
  if (!raw) return;
  if (!Array.isArray(raw)) return;
  // Separate legal entries from CSS-keyword illegal entries.
  const cleaned = raw.filter((f) => isLegalFillEntry(f));
  if (cleaned.length > 0) {
    rec.fill = cleaned as PenFill[];
    return;
  }
  // Every original entry was a CSS keyword ("none" / "transparent").
  // The AI's intent was "no fill" — but DELETING the field would let
  // canvas-object-factory fall back to its default opaque gray fill,
  // which is the opposite of no-fill. Replace with an explicit
  // transparent hex so the renderer honours the intent.
  if (raw.length > 0) {
    rec.fill = [EXPLICIT_TRANSPARENT_FILL] as PenFill[];
  } else {
    // Empty array in, empty array out — leave unchanged.
    rec.fill = [] as PenFill[];
  }
}

/** Reject fill entries whose color is an unsupported CSS keyword. */
function isLegalFillEntry(entry: unknown): boolean {
  if (!entry || typeof entry !== 'object') return false;
  const e = entry as { type?: unknown; color?: unknown };
  if (e.type === 'solid' && typeof e.color === 'string') {
    const c = e.color.trim().toLowerCase();
    if (c === 'none' || c === 'transparent') return false;
  }
  return true;
}
