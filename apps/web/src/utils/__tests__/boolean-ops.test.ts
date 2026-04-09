import { describe, it, expect } from 'vitest';
import { canBooleanOp, executeBooleanOp } from '../boolean-ops';
import type { PenNode, RectangleNode, EllipseNode, PathNode, PolygonNode } from '@/types/pen';

function makeRect(id: string, x: number, y: number, w: number, h: number): RectangleNode {
  return {
    id,
    type: 'rectangle',
    name: `Rect ${id}`,
    x,
    y,
    width: w,
    height: h,
    fill: [{ type: 'solid', color: '#ff0000' }],
  };
}

function makeEllipse(id: string, x: number, y: number, w: number, h: number): EllipseNode {
  return { id, type: 'ellipse', name: `Ellipse ${id}`, x, y, width: w, height: h };
}

function makePolygon(
  id: string,
  x: number,
  y: number,
  w: number,
  h: number,
  count = 6,
): PolygonNode {
  return {
    id,
    type: 'polygon',
    name: `Polygon ${id}`,
    x,
    y,
    width: w,
    height: h,
    polygonCount: count,
  };
}

function makePath(id: string, d: string, x = 0, y = 0): PathNode {
  return { id, type: 'path', name: `Path ${id}`, d, x, y };
}

describe('canBooleanOp', () => {
  it('returns false for fewer than 2 nodes', () => {
    expect(canBooleanOp([])).toBe(false);
    expect(canBooleanOp([makeRect('a', 0, 0, 50, 50)])).toBe(false);
  });

  it('returns true for 2+ shape nodes', () => {
    expect(canBooleanOp([makeRect('a', 0, 0, 50, 50), makeRect('b', 25, 25, 50, 50)])).toBe(true);
  });

  it('returns true for mixed shape types', () => {
    expect(canBooleanOp([makeRect('a', 0, 0, 50, 50), makeEllipse('b', 25, 25, 50, 50)])).toBe(
      true,
    );
  });

  it('returns false if text or image nodes are included', () => {
    const textNode: PenNode = { id: 't', type: 'text', content: 'hi' };
    expect(canBooleanOp([makeRect('a', 0, 0, 50, 50), textNode])).toBe(false);
  });
});

describe('executeBooleanOp', () => {
  it('performs union of two overlapping rectangles', () => {
    const r1 = makeRect('a', 0, 0, 100, 100);
    const r2 = makeRect('b', 50, 50, 100, 100);
    const result = executeBooleanOp([r1, r2], 'union');
    expect(result).not.toBeNull();
    expect(result!.type).toBe('path');
    expect(result!.d).toBeTruthy();
    expect(result!.name).toBe('Union');
    expect(result!.x).toBeCloseTo(0, 0);
    expect(result!.y).toBeCloseTo(0, 0);
    // Union should be larger than either original
    expect(result!.width).toBeGreaterThanOrEqual(149);
    expect(result!.height).toBeGreaterThanOrEqual(149);
  });

  it('performs subtract of two overlapping rectangles', () => {
    const r1 = makeRect('a', 0, 0, 100, 100);
    const r2 = makeRect('b', 50, 50, 100, 100);
    const result = executeBooleanOp([r1, r2], 'subtract');
    expect(result).not.toBeNull();
    expect(result!.type).toBe('path');
    expect(result!.name).toBe('Subtract');
  });

  it('performs intersect of two overlapping rectangles', () => {
    const r1 = makeRect('a', 0, 0, 100, 100);
    const r2 = makeRect('b', 50, 50, 100, 100);
    const result = executeBooleanOp([r1, r2], 'intersect');
    expect(result).not.toBeNull();
    expect(result!.type).toBe('path');
    expect(result!.name).toBe('Intersect');
    // Intersection should be 50x50 area
    expect(result!.width).toBeCloseTo(50, 0);
    expect(result!.height).toBeCloseTo(50, 0);
  });

  it('preserves fill from first operand', () => {
    const r1 = makeRect('a', 0, 0, 100, 100);
    const r2 = makeRect('b', 50, 50, 100, 100);
    const result = executeBooleanOp([r1, r2], 'union');
    expect(result!.fill).toEqual([{ type: 'solid', color: '#ff0000' }]);
  });

  it('handles ellipse + rectangle boolean', () => {
    const e = makeEllipse('a', 0, 0, 100, 100);
    const r = makeRect('b', 25, 25, 50, 50);
    const result = executeBooleanOp([e, r], 'subtract');
    expect(result).not.toBeNull();
    expect(result!.d).toBeTruthy();
  });

  it('handles polygon + rectangle boolean', () => {
    const p = makePolygon('a', 0, 0, 100, 100, 6);
    const r = makeRect('b', 25, 25, 50, 50);
    const result = executeBooleanOp([p, r], 'intersect');
    expect(result).not.toBeNull();
  });

  it('handles path + path boolean', () => {
    const p1 = makePath('a', 'M 0 0 L 100 0 L 100 100 L 0 100 Z');
    const p2 = makePath('b', 'M 50 50 L 150 50 L 150 150 L 50 150 Z');
    const result = executeBooleanOp([p1, p2], 'union');
    expect(result).not.toBeNull();
  });

  it('handles 3+ nodes (fold left)', () => {
    const r1 = makeRect('a', 0, 0, 100, 100);
    const r2 = makeRect('b', 50, 0, 100, 100);
    const r3 = makeRect('c', 100, 0, 100, 100);
    const result = executeBooleanOp([r1, r2, r3], 'union');
    expect(result).not.toBeNull();
    expect(result!.width).toBeCloseTo(200, 0);
    expect(result!.height).toBeCloseTo(100, 0);
  });

  it('returns null for non-overlapping intersect', () => {
    const r1 = makeRect('a', 0, 0, 50, 50);
    const r2 = makeRect('b', 200, 200, 50, 50);
    const result = executeBooleanOp([r1, r2], 'intersect');
    // Non-overlapping: either null or empty path
    if (result) {
      expect(result.width).toBeLessThan(1);
    }
  });

  it('handles rotated shapes', () => {
    const r1: RectangleNode = { ...makeRect('a', 50, 50, 100, 100), rotation: 45 };
    const r2 = makeRect('b', 50, 50, 100, 100);
    const result = executeBooleanOp([r1, r2], 'intersect');
    expect(result).not.toBeNull();
  });
});
