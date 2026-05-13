import { afterAll, describe, it, expect, vi } from 'vitest';
import { canBooleanOp, executeBooleanOp } from '../boolean-ops';
import type { PenNode, RectangleNode, EllipseNode, PathNode, PolygonNode } from '@/types/pen';

type TestBounds = {
  x: number;
  y: number;
  width: number;
  height: number;
  center: { x: number; y: number };
};

const paperMock = vi.hoisted(() => {
  function makeBounds(x: number, y: number, width: number, height: number): TestBounds {
    return {
      x,
      y,
      width,
      height,
      center: { x: x + width / 2, y: y + height / 2 },
    };
  }

  function parseBounds(pathData: string): TestBounds {
    const values = Array.from(pathData.matchAll(/-?\d+(?:\.\d+)?/g), (match) =>
      Number(match[0]),
    );
    const points: Array<[number, number]> = [];
    for (let index = 0; index + 1 < values.length; index += 2) {
      points.push([values[index], values[index + 1]]);
    }
    if (points.length === 0) return makeBounds(0, 0, 0, 0);
    const xs = points.map(([x]) => x);
    const ys = points.map(([, y]) => y);
    const minX = Math.min(...xs);
    const maxX = Math.max(...xs);
    const minY = Math.min(...ys);
    const maxY = Math.max(...ys);
    return makeBounds(minX, minY, maxX - minX, maxY - minY);
  }

  class MockPathItem {
    bounds: TestBounds;

    constructor(bounds: TestBounds) {
      this.bounds = bounds;
    }

    get pathData() {
      const { x, y, width, height } = this.bounds;
      if (width <= 0 || height <= 0) return '';
      return `M ${x} ${y} L ${x + width} ${y} L ${x + width} ${y + height} L ${x} ${
        y + height
      } Z`;
    }

    translate(point: { x: number; y: number }) {
      this.bounds = makeBounds(
        this.bounds.x + point.x,
        this.bounds.y + point.y,
        this.bounds.width,
        this.bounds.height,
      );
    }

    rotate() {}

    unite(path: MockPathItem) {
      const x = Math.min(this.bounds.x, path.bounds.x);
      const y = Math.min(this.bounds.y, path.bounds.y);
      const right = Math.max(this.bounds.x + this.bounds.width, path.bounds.x + path.bounds.width);
      const bottom = Math.max(
        this.bounds.y + this.bounds.height,
        path.bounds.y + path.bounds.height,
      );
      return new MockPathItem(makeBounds(x, y, right - x, bottom - y));
    }

    subtract() {
      return new MockPathItem(this.bounds);
    }

    intersect(path: MockPathItem) {
      const x = Math.max(this.bounds.x, path.bounds.x);
      const y = Math.max(this.bounds.y, path.bounds.y);
      const right = Math.min(this.bounds.x + this.bounds.width, path.bounds.x + path.bounds.width);
      const bottom = Math.min(
        this.bounds.y + this.bounds.height,
        path.bounds.y + path.bounds.height,
      );
      return new MockPathItem(makeBounds(x, y, Math.max(0, right - x), Math.max(0, bottom - y)));
    }

    remove() {}
  }

  return {
    PaperScope: class {
      Size = class {
        constructor(
          public width: number,
          public height: number,
        ) {}
      };

      Point = class {
        constructor(
          public x: number,
          public y: number,
        ) {}
      };

      CompoundPath = {
        create: (pathData: string) => new MockPathItem(parseBounds(pathData)),
      };

      setup() {}

      activate() {}
    },
    Point: class {
      constructor(
        public x: number,
        public y: number,
      ) {}
    },
  };
});

vi.mock('paper', () => paperMock);

const originalRequire = (globalThis as { require?: (id: string) => unknown }).require;
(globalThis as { require?: (id: string) => unknown }).require = (id: string) => {
  if (id === 'paper') return paperMock;
  return originalRequire?.(id);
};

afterAll(() => {
  (globalThis as { require?: (id: string) => unknown }).require = originalRequire;
});

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
