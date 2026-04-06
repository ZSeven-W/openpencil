import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { normalizeStrokeFillSchema } from '../normalize/normalize-stroke-fill-schema';

const path = (props: Partial<PenNode> = {}): PenNode =>
  ({
    id: 'p',
    type: 'path',
    d: 'M0 0 L100 0',
    width: 100,
    height: 50,
    ...props,
  }) as PenNode;

const ellipse = (props: Partial<PenNode> = {}): PenNode =>
  ({
    id: 'e',
    type: 'ellipse',
    width: 100,
    height: 100,
    ...props,
  }) as PenNode;

const frame = (props: Partial<PenNode> & { children?: PenNode[] } = {}): PenNode =>
  ({
    id: 'f',
    type: 'frame',
    width: 200,
    height: 200,
    ...props,
  }) as PenNode;

const validFill = [{ type: 'solid' as const, color: '#C4F82A' }];

describe('normalizeStrokeFillSchema — stroke array unwrap', () => {
  it('unwraps a stroke that is an array of one proper PenStroke object', () => {
    const node = ellipse({
      stroke: [
        { thickness: 12, fill: validFill },
      ] as unknown as PenNode['stroke'],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { stroke?: { thickness?: number; fill?: unknown } };
    expect(Array.isArray(rec.stroke)).toBe(false);
    expect(rec.stroke?.thickness).toBe(12);
    expect(rec.stroke?.fill).toEqual(validFill);
  });

  it('leaves a stroke that is already a proper object alone', () => {
    const node = ellipse({
      stroke: { thickness: 4, fill: validFill } as PenNode['stroke'],
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
      stroke: [{ type: 'solid', color: '#C4F82A' }] as unknown as PenNode['stroke'],
      strokeWidth: 2.5,
    } as Partial<PenNode> & { strokeWidth?: number });
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
      stroke: [{ type: 'solid', color: '#111' }] as unknown as PenNode['stroke'],
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
      stroke: { type: 'solid', color: '#FFFFFF' } as unknown as PenNode['stroke'],
      strokeWidth: 3,
    } as Partial<PenNode> & { strokeWidth?: number });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as {
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
    };
    expect(rec.stroke?.thickness).toBe(3);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#FFFFFF');
  });
});

describe('normalizeStrokeFillSchema — illegal fill color drops', () => {
  it('drops fills whose color is "none"', () => {
    const node = path({
      fill: [{ type: 'solid', color: 'none' }] as unknown as PenNode['fill'],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    // Either absent or empty array is acceptable — the important thing
    // is that no { color: "none" } entry remains to confuse the renderer.
    const f = rec.fill as unknown[] | undefined;
    expect(f === undefined || (Array.isArray(f) && f.length === 0)).toBe(true);
  });

  it('drops fills whose color is "transparent"', () => {
    const node = path({
      fill: [{ type: 'solid', color: 'transparent' }] as unknown as PenNode['fill'],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: unknown };
    const f = rec.fill as unknown[] | undefined;
    expect(f === undefined || (Array.isArray(f) && f.length === 0)).toBe(true);
  });

  it('keeps legitimate hex fills alone', () => {
    const node = path({
      fill: [{ type: 'solid', color: '#C4F82A' }] as unknown as PenNode['fill'],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#C4F82A');
  });

  it('keeps 8-digit transparent hex (#00000000) alone', () => {
    // The 8-digit hex IS a valid color string (alpha channel). Only the
    // CSS keywords "none" and "transparent" are the unsupported forms.
    const node = ellipse({
      fill: [{ type: 'solid', color: '#00000000' }] as unknown as PenNode['fill'],
    });
    normalizeStrokeFillSchema(node);
    const rec = node as unknown as { fill?: Array<{ color?: string }> };
    expect(rec.fill?.[0]?.color).toBe('#00000000');
  });

  it('also strips illegal colors from stroke.fill arrays', () => {
    const node = path({
      stroke: { thickness: 2, fill: [{ type: 'solid', color: 'none' }] } as unknown as PenNode['stroke'],
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
      stroke: [{ thickness: 8, fill: validFill }] as unknown as PenNode['stroke'],
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
      fill: [{ type: 'solid', color: '#00000000' }] as unknown as PenNode['fill'],
      stroke: [
        { thickness: 12, fill: [{ type: 'solid', color: '#C4F82A' }] },
      ] as unknown as PenNode['stroke'],
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
      fill: [{ type: 'solid', color: 'none' }] as unknown as PenNode['fill'],
      stroke: [{ type: 'solid', color: '#C4F82A' }] as unknown as PenNode['stroke'],
      strokeWidth: 2.5,
    } as Partial<PenNode> & { strokeWidth?: number });
    normalizeStrokeFillSchema(line);
    const rec = line as unknown as {
      fill?: unknown[];
      stroke?: { thickness?: number; fill?: Array<{ color?: string }> };
      strokeWidth?: number;
    };
    expect(rec.stroke?.thickness).toBe(2.5);
    expect(rec.stroke?.fill?.[0]?.color).toBe('#C4F82A');
    // "none" fill is gone
    const f = rec.fill;
    expect(f === undefined || (Array.isArray(f) && f.length === 0)).toBe(true);
    expect(rec.strokeWidth).toBeUndefined();
  });
});
