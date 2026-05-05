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
  'stroke-width'?: unknown;
  'stroke-dasharray'?: unknown;
  'stroke-dashoffset'?: unknown;
  'stroke-linecap'?: unknown;
  'stroke-linejoin'?: unknown;
}

function normalizeNodeStroke(node: PenNode): void {
  const rec = node as unknown as MaybeStrokeHolder;
  const rawStroke = rec.stroke;
  const hasSvgStrokeAttrs =
    rec['stroke-width'] !== undefined ||
    rec['stroke-dasharray'] !== undefined ||
    rec['stroke-dashoffset'] !== undefined ||
    rec['stroke-linecap'] !== undefined ||
    rec['stroke-linejoin'] !== undefined;
  if ((rawStroke === undefined || rawStroke === null) && !hasSvgStrokeAttrs) return;

  // (1) Unwrap `stroke: [ ... ]` by taking the first element.
  let stroke: unknown = rawStroke;
  if (typeof stroke === 'string') {
    stroke = { type: 'solid', color: stroke };
  }
  if (Array.isArray(stroke)) {
    stroke = stroke.length > 0 ? stroke[0] : undefined;
  }
  if ((!stroke || typeof stroke !== 'object') && !hasSvgStrokeAttrs) {
    delete rec.stroke;
    delete rec.strokeWidth;
    delete rec['stroke-width'];
    delete rec['stroke-dasharray'];
    delete rec['stroke-dashoffset'];
    delete rec['stroke-linecap'];
    delete rec['stroke-linejoin'];
    return;
  }

  // (2) Detect the fill-shape-as-stroke pattern and migrate it.
  const maybeFillShape = (stroke ?? {}) as {
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
  const strokeObj = (stroke ?? {}) as Partial<PenStroke> & { [k: string]: unknown };
  if (strokeObj.thickness === undefined || strokeObj.thickness === null) {
    const width = readThickness(rec);
    (strokeObj as { thickness?: number }).thickness = width;
  }
  const dashPattern = readDashPattern(rec['stroke-dasharray']);
  if (dashPattern && dashPattern.length > 0 && strokeObj.dashPattern === undefined) {
    strokeObj.dashPattern = dashPattern;
  }
  const dashOffset = readDashOffset(rec['stroke-dashoffset']);
  if (dashOffset !== null && strokeObj.dashOffset === undefined) {
    strokeObj.dashOffset = dashOffset;
  }
  const cap = readCap(rec['stroke-linecap']);
  if (cap && strokeObj.cap === undefined) {
    strokeObj.cap = cap;
  }
  const join = readJoin(rec['stroke-linejoin']);
  if (join && strokeObj.join === undefined) {
    strokeObj.join = join;
  }
  if (
    (!Array.isArray(strokeObj.fill) || strokeObj.fill.length === 0) &&
    typeof maybeFillShape.color !== 'string'
  ) {
    const inferredColor = inferStrokeColor(node);
    if (inferredColor) {
      strokeObj.fill = [{ type: 'solid', color: inferredColor }] as PenFill[];
    }
  }
  rec.stroke = strokeObj as PenStroke;
  delete rec.strokeWidth;
  delete rec['stroke-width'];
  delete rec['stroke-dasharray'];
  delete rec['stroke-dashoffset'];
  delete rec['stroke-linecap'];
  delete rec['stroke-linejoin'];
  stripIllegalColorsFromStrokeFill(node);

  // If after cleanup the stroke has no fill at all, drop the whole stroke.
  const cleaned = rec.stroke as PenStroke | undefined;
  if (cleaned && (!cleaned.fill || cleaned.fill.length === 0)) {
    delete rec.stroke;
  }
}

function readThickness(rec: MaybeStrokeHolder): number {
  const raw = rec.strokeWidth ?? rec['stroke-width'];
  if (typeof raw === 'number' && raw > 0) return raw;
  if (typeof raw === 'string') {
    const n = parseFloat(raw);
    if (Number.isFinite(n) && n > 0) return n;
  }
  return 2;
}

function readDashPattern(raw: unknown): number[] | null {
  if (Array.isArray(raw)) {
    const nums = raw.filter((value): value is number => typeof value === 'number' && value > 0);
    if (nums.length === 1) return [nums[0], nums[0]];
    return nums.length > 0 ? nums : null;
  }
  if (typeof raw === 'string') {
    const nums = raw
      .split(/[,\s]+/)
      .map((part) => parseFloat(part))
      .filter((value) => Number.isFinite(value) && value > 0);
    if (nums.length === 1) return [nums[0], nums[0]];
    return nums.length > 0 ? nums : null;
  }
  return null;
}

function readDashOffset(raw: unknown): number | null {
  if (typeof raw === 'number' && Number.isFinite(raw)) return raw;
  if (typeof raw === 'string') {
    const parsed = parseFloat(raw);
    if (Number.isFinite(parsed)) return parsed;
  }
  return null;
}

function readCap(raw: unknown): PenStroke['cap'] | null {
  return raw === 'round' || raw === 'square' || raw === 'none' ? raw : null;
}

function readJoin(raw: unknown): PenStroke['join'] | null {
  return raw === 'round' || raw === 'bevel' || raw === 'miter' ? raw : null;
}

function inferStrokeColor(node: PenNode): string | null {
  const name = typeof node.name === 'string' ? node.name.toLowerCase() : '';
  if (/track/.test(name)) return '#2A2A2A';
  if (/(progress|chart line|line|curve|wave)/.test(name)) return '#22C55E';
  return null;
}

function stripIllegalColorsFromStrokeFill(node: PenNode): void {
  const rec = node as unknown as { stroke?: { fill?: unknown } };
  const stroke = rec.stroke;
  if (!stroke || typeof stroke !== 'object') return;
  const fillArr = stroke.fill;
  if (!Array.isArray(fillArr)) return;
  (stroke as { fill?: PenFill[] }).fill = fillArr.filter((f) => isLegalFillEntry(f)) as PenFill[];
}

// ---------------------------------------------------------------------------
// Fill normalization
// ---------------------------------------------------------------------------

/**
 * Explicit transparent hex. Used for SHAPE fills (frame, rectangle,
 * ellipse, path, group…) where "no fill" really means a hollow shape
 * that should let the background show through. We write the 8-digit
 * transparent hex so canvas-object-factory doesn't fall back to an
 * opaque default gray fill.
 */
const EXPLICIT_TRANSPARENT_FILL: SolidFill = {
  type: 'solid',
  color: '#00000000',
};

/**
 * Node types whose `fill` represents a FOREGROUND color (text color,
 * icon color) rather than a shape's background. On these types an
 * illegal "none" / "transparent" fill is almost certainly a mistake:
 * the user meant "default text color", not "invisible text". Freezing
 * them to #00000000 would hide the content entirely, so we delete the
 * field and let downstream layers (role defaults, button contrast,
 * style inheritance) supply a visible color.
 */
const FOREGROUND_NODE_TYPES = new Set<string>(['text', 'icon_font']);

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
  // Empty in, empty out — leave unchanged.
  if (raw.length === 0) {
    rec.fill = [] as PenFill[];
    return;
  }
  // Every original entry was a CSS keyword ("none" / "transparent").
  // The correct repair depends on whether `fill` is a background or a
  // foreground color on this node type:
  //
  //   SHAPE types (frame, rectangle, ellipse, path, group, …)
  //     fill = shape background. "no fill" means hollow. Keep the
  //     explicit transparent hex so canvas doesn't fall back to the
  //     default gray fill.
  //
  //   FOREGROUND types (text, icon_font)
  //     fill = text/icon color. "no fill" would hide the content —
  //     almost certainly not what the AI meant. Delete the field so
  //     downstream layers can populate a visible color.
  if (FOREGROUND_NODE_TYPES.has(node.type)) {
    delete rec.fill;
  } else {
    rec.fill = [EXPLICIT_TRANSPARENT_FILL] as PenFill[];
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
