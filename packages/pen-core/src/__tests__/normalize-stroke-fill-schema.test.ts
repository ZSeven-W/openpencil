import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { normalizeStrokeFillSchema } from '../normalize/normalize-stroke-fill-schema';

// Test helpers accept untyped property bags because most assertions here
// intentionally put malformed (schema-violating) data on the node — that
// is exactly the input shape the normalizer is designed to repair. A
// strongly-typed signature would reject the very cases we need to cover.
type NodeProps = Record<string, unknown>;

const path = (props: NodeProps = {}): PenNode =>
  ({
    id: 'p',
    type: 'path',
    d: 'M0 0 L100 0',
    width: 100,
    height: 50,
    ...props,
  }) as unknown as PenNode;

const ellipse = (props: NodeProps = {}): PenNode =>
  ({
    id: 'e',
    type: 'ellipse',
    width: 100,
    height: 100,
    ...props,
  }) as unknown as PenNode;

const frame = (props: NodeProps & { children?: PenNode[] } = {}): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    width: 200,
    height: 200,
    ...props,
  }) as unknown as PenNode;

const text = (props: NodeProps = {}): PenNode =>
  ({
    id: 't',
    type: 'text',
    content: 'Hello',
    fontSize: 16,
    ...props,
  }) as unknown as PenNode;

const iconFont = (props: NodeProps = {}): PenNode =>
  ({
    id: 'i',
    type: 'icon_font',
    iconFontName: 'heart',
    width: 16,
    height: 16,
    ...props,
  }) as unknown as PenNode;

const validFill = [{ type: 'solid' as const, color: '#C4F82A' }];

describe('normalizeStrokeFillSchema — stroke array unwrap', () => {
  it('unwraps a stroke that is an array of one proper PenStroke object', () => {
    const node = ellipse({
      stroke: [{ thickness: 12, fill: validFill }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(Array.isArray(rec.stroke)).toBe(false);
    expect(rec.stroke?.thickness).toBe(12);
    expect(rec.stroke?.fill).toEqual(validFill);
  });

  it('leaves a stroke that is already a proper object alone', () => {
    const node = ellipse({
      stroke: { thickness: 4, fill: validFill },
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(rec.stroke?.thickness).toBe(4);
  });

  it('converts a fill-shaped stroke entry into a proper PenStroke', () => {
    // Real M2.7 failure: stroke is an array and the inner object has
    // {type, color} (fill shape) instead of {thickness, fill}. strokeWidth
    // lives as a top-level node field.
    const node = path({
      stroke: [{ type: 'solid', color: '#C4F82A' }],
      strokeWidth: 2.5,
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as {
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      strokeWidth?: number;
    };
    expect(rec.stroke?.thickness).toBe(2.5);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // Stray strokeWidth field is cleaned up after migration
    expect(rec.strokeWidth).toBeUndefined();
  });

  it('uses a default thickness when neither strokeWidth nor thickness is present', () => {
    const node = path({
      stroke: [{ type: 'solid', color: '#111' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(rec.stroke?.thickness).toBeGreaterThan(0);
    expect(rec.stroke?.fill).toBeDefined();
  });

  it('handles an object-shaped stroke with a fill-shaped inner value', () => {
    // Some sub-agents emit { stroke: { type: "solid", color: "#fff" } }
    // (object, not array) — still a fill shape. Same recovery rules apply.
    const node = path({
      stroke: { type: 'solid', color: '#FFFFFF' },
      strokeWidth: 3,
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as {
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
    };
    expect(rec.stroke?.thickness).toBe(3);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('migrates SVG-style stroke attrs into a canonical PenStroke', () => {
    const node = path({
      name: 'HR Chart Line',
      'stroke-width': '2',
      'stroke-dasharray': '4 2',
      'stroke-linecap': 'round',
      'stroke-linejoin': 'round',
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: {
        thickness?: number;
        dashPattern?: number[];
        dashOffset?: number;
        cap?: string;
        join?: string;
        fill?: Array<{ color?: string }>;
      };
      'stroke-width'?: unknown;
      'stroke-dasharray'?: unknown;
      'stroke-dashoffset'?: unknown;
    };

    expect(rec.stroke?.thickness).toBe(2);
    expect(rec.stroke?.dashPattern).toEqual([4, 2]);
    expect(rec.stroke?.dashOffset).toBeUndefined();
    expect(rec.stroke?.cap).toBe('round');
    expect(rec.stroke?.join).toBe('round');
    expect(rec.stroke?.fill?.[0]?.color).toBe('#22C55E');
    expect(rec['stroke-width']).toBeUndefined();
    expect(rec['stroke-dasharray']).toBeUndefined();
  });

  it('migrates SVG dash offset into PenStroke.dashOffset', () => {
    const node = path({
      name: 'Steps Progress',
      'stroke-width': '8',
      'stroke-dasharray': '158.4',
      'stroke-dashoffset': '44.4',
      'stroke-linecap': 'round',
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: {
        thickness?: number;
        dashPattern?: number[];
        dashOffset?: number;
        cap?: string;
        fill?: Array<{ color?: string }>;
      };
      'stroke-dashoffset'?: unknown;
    };

    expect(rec.stroke?.thickness).toBe(8);
    expect(rec.stroke?.dashPattern).toEqual([158.4, 158.4]);
    expect(rec.stroke?.dashOffset).toBe(44.4);
    expect(rec.stroke?.cap).toBe('round');
    expect(rec.stroke?.fill?.[0]?.color).toBe('#22C55E');
    expect(rec['stroke-dashoffset']).toBeUndefined();
  });

  it('infers a muted track color for SVG-style track paths with no explicit stroke fill', () => {
    const node = path({
      name: 'Steps Track',
      'stroke-width': '8',
      fill: [{ type: 'solid', color: '#00000000' }],
    });

    normalizeStrokeFillSchema(node);

    const rec = node as unknown as {
      stroke?: { fill?: Array<{ color?: string }> };
    };

    expect(rec.stroke?.fill?.[0]?.color).toBe('#2A2A2A');
  });
});

describe('normalizeStrokeFillSchema — illegal fill color replacement', () => {
  it('replaces a sole "none" fill with explicit transparent hex', () => {
    // "none" and "transparent" are CSS keywords, not valid PenFill colors.
    // But they express an explicit "no fill" intent (e.g. an activity ring
    // that should be hollow). Deleting the fill field entirely would make
    // the canvas renderer fall back to a default gray fill — the opposite
    // of what the AI meant. Replace with the 8-digit transparent hex so
    // the renderer sees an explicit transparent color and respects it.
    const node = path({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ type?: string; color?: string }> };
    expect(rec.fill).toBeDefined();
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.type).toBe('solid');
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
  });

  it('replaces a sole "transparent" fill with explicit transparent hex', () => {
    const node = path({
      fill: [{ type: 'solid', color: 'transparent' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ type?: string; color?: string }> };
    expect(rec.fill).toBeDefined();
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
  });

  it('drops a "none" entry but keeps other legitimate entries in the same array', () => {
    // Mixed array: one illegal, one legal. Just drop the illegal one and
    // keep the real color — no transparent fallback needed.
    const node = path({
      fill: [
        { type: 'solid', color: 'none' },
        { type: 'solid', color: '#FF0000' },
      ],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color).toBe('#FF0000');
  });

  it('keeps legitimate hex fills alone', () => {
    const node = path({
      fill: [{ type: 'solid', color: '#C4F82A' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#C4F82A');
  });

  it('keeps 8-digit transparent hex (#00000000) alone', () => {
    // The 8-digit hex IS a valid color string (alpha channel). Only the
    // CSS keywords "none" and "transparent" are the unsupported forms.
    const node = ellipse({
      fill: [{ type: 'solid', color: '#00000000' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#00000000');
  });

  it('DELETES text node fill when all entries are CSS keywords', () => {
    // text.fill is the foreground color, not a shape background.
    // "none"/"transparent" on a text node would hide the text entirely —
    // almost certainly a mistake. Delete the field so downstream layers
    // (role defaults, button contrast heuristic) can fill in a visible
    // color. Replacing with #00000000 would freeze the text invisible.
    const node = text({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('DELETES text node fill for "transparent" too', () => {
    const node = text({
      fill: [{ type: 'solid', color: 'transparent' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('DELETES icon_font fill when all entries are CSS keywords', () => {
    // Same reasoning: icon_font.fill is the foreground color.
    const node = iconFont({
      fill: [{ type: 'solid', color: 'none' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    expect(rec.fill).toBeUndefined();
  });

  it('still keeps a legitimate text fill unchanged', () => {
    const node = text({
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('still drops illegal entries from text but preserves legal siblings', () => {
    const node = text({
      fill: [
        { type: 'solid', color: 'none' },
        { type: 'solid', color: '#00FF00' },
      ],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill).toHaveLength(1);
    expect(rec.fill?.[0]?.color).toBe('#00FF00');
  });

  it('keeps the transparent-hex replacement for shape fills (frame / ellipse / path)', () => {
    // Sanity guard: the shape-vs-foreground split must not regress the
    // shape branch. Frames, ellipses, and paths still get the explicit
    // #00000000 when an AI writes "none".
    for (const node of [
      frame({ fill: [{ type: 'solid', color: 'none' }] }),
      ellipse({ fill: [{ type: 'solid', color: 'none' }] }),
      path({ fill: [{ type: 'solid', color: 'transparent' }] }),
    ]) {
      normalizeStrokeFillSchema(node);
      const rec = node as unknown as { fill?: Array<{ color?: string }> };
      expect(rec.fill).toHaveLength(1);
      expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
    }
  });

  it('also strips illegal colors from stroke.fill arrays', () => {
    const node = path({
      stroke: { thickness: 2, fill: [{ type: 'solid', color: 'none' }] },
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown[] } };
    // Whole stroke becomes either unset, or stroke.fill is empty — either
    // way the renderer will not try to use "none".
    if (rec.stroke && Array.isArray(rec.stroke.fill)) {
      expect(rec.stroke.fill.length).toBe(0);
    }
  });
});

describe('normalizeStrokeFillSchema — recursion', () => {
  it('recurses into children', () => {
    const inner = ellipse({
      id: 'inner',
      stroke: [{ thickness: 8, fill: validFill }],
    });
    const root = frame({ id: 'root', children: [inner] });
    normalizeStrokeFillSchema(root);
    const rec = inner as unknown as { stroke?: { thickness?: number } };
    expect(Array.isArray((inner as unknown as { stroke?: unknown }).stroke)).toBe(false);
    expect(rec.stroke?.thickness).toBe(8);
  });

  it('reproduces the M2.7 activity rings case end-to-end', () => {
    const ring = ellipse({
      id: 'ring',
      name: 'Steps Circle',
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '#00000000' }],
      stroke: [{ thickness: 12, fill: [{ type: 'solid', color: '#C4F82A' }] }],
    });
    normalizeStrokeFillSchema(ring);
    const rec = ring as unknown as {
      fill?: Array<{ color?: string }>;
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
    };
    // Stroke is now a proper object with the original thickness and color
    expect(rec.stroke?.thickness).toBe(12);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // Transparent 8-digit hex fill is preserved (it's not a CSS keyword)
    expect(rec.fill?.[0]?.color).toBe('#00000000');
  });

  it('reproduces the M2.7 heart-rate chart line case end-to-end', () => {
    const line = path({
      id: 'line',
      name: 'Chart Line',
      d: 'M0 50 L100 20',
      fill: [{ type: 'solid', color: 'none' }],
      stroke: [{ type: 'solid', color: '#C4F82A' }],
      strokeWidth: 2.5,
    });
    normalizeStrokeFillSchema(line);
    const rec = line as unknown as {
      fill?: Array<{ color?: string }>;
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      strokeWidth?: number;
    };
    expect(rec.stroke?.thickness).toBe(2.5);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // "none" fill was replaced with explicit transparent — NOT deleted.
    // Deleting would let the canvas factory fall back to the default gray
    // fill and the chart line's background would bleed through as a
    // solid grey rectangle.
    expect(rec.fill).toBeDefined();
    expect(rec.fill?.[0]?.color?.toLowerCase()).toBe('#00000000');
    expect(rec.strokeWidth).toBeUndefined();
  });
});
