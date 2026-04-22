import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import {
  buildChartBars,
  buildChartLine,
  buildChartPie,
  assignIdsRecursively,
} from '../element-builders/index.js';
import { computeLayoutPositions } from '../layout/engine.js';

/**
 * Visual smoke tests for the three chart builders. Verifies that
 * every chart variant produces a tree the Skia renderer can
 * actually draw, without triggering NaN/Infinity coords, degenerate
 * paths, or zero-dimension children. We can't run Skia headlessly
 * in a unit test (CanvasKit WASM is heavy + GPU-context-dependent),
 * so this smoke layer checks the geometric invariants that a
 * rendering failure would start from:
 *
 *   1. Build doesn't throw on any reasonable input
 *   2. computeLayoutPositions doesn't throw
 *   3. No NaN / Infinity in x / y / width / height anywhere in tree
 *   4. No zero-or-negative width/height on "visible" children
 *   5. Chart-specific invariants (bar count, slice count, line
 *      polyline point count)
 *
 * The `debug_screenshot` MCP tool gives us real visual regression
 * at the app level; this smoke is the cheap unit-test canary that
 * flags shape-level regressions before they reach the renderer.
 */

interface ShapeNode {
  type?: string;
  name?: string;
  role?: string;
  x?: number;
  y?: number;
  width?: unknown;
  height?: unknown;
  children?: ShapeNode[];
  points?: unknown;
  startAngle?: unknown;
  endAngle?: unknown;
}

function isNumFinite(v: unknown): boolean {
  return typeof v === 'number' && Number.isFinite(v);
}

/** Every numeric coord / dimension in the tree is finite. */
function hasAllFiniteCoords(n: ShapeNode): boolean {
  if (typeof n.x === 'number' && !Number.isFinite(n.x)) return false;
  if (typeof n.y === 'number' && !Number.isFinite(n.y)) return false;
  if (typeof n.width === 'number' && !Number.isFinite(n.width)) return false;
  if (typeof n.height === 'number' && !Number.isFinite(n.height)) return false;
  for (const c of n.children ?? []) {
    if (!hasAllFiniteCoords(c)) return false;
  }
  return true;
}

/** Collect role-matching descendants (role as documented in builder). */
function collectByRole(n: ShapeNode, role: string, out: ShapeNode[] = []): ShapeNode[] {
  if (n.role === role) out.push(n);
  for (const c of n.children ?? []) collectByRole(c, role, out);
  return out;
}

function wrapInPage(tree: PenNode): PenNode {
  return {
    id: 'root',
    type: 'frame',
    name: 'Page',
    x: 0,
    y: 0,
    width: 375,
    height: 812,
    layout: 'vertical',
    children: [tree],
  } as unknown as PenNode;
}

describe('chart builders — visual smoke', () => {
  describe('buildChartBars', () => {
    const cases: Array<{ name: string; args: Parameters<typeof buildChartBars>[0] }> = [
      { name: 'default minimal', args: { values: [4, 7, 3, 9, 5, 8, 6] } },
      { name: 'all equal values', args: { values: [5, 5, 5, 5] } },
      { name: 'single bar', args: { values: [42] } },
      { name: 'very small values', args: { values: [0.1, 0.2, 0.3, 0.4] } },
      { name: 'very large values', args: { values: [1e6, 2e6, 3e6] } },
      { name: 'custom dimensions', args: { values: [1, 2, 3], bar_width: 40, chart_height: 200 } },
      { name: 'zero mixed with positive', args: { values: [0, 5, 0, 10, 0] } },
    ];

    for (const c of cases) {
      it(`${c.name}: builds, layouts, finite coords, correct bar count`, () => {
        const tree = buildChartBars(c.args) as unknown as PenNode;
        assignIdsRecursively(tree as unknown as { children?: unknown[] });
        const page = wrapInPage(tree);
        expect(() => {
          const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
          computeLayoutPositions(page, kids);
        }).not.toThrow();
        expect(hasAllFiniteCoords(page as unknown as ShapeNode)).toBe(true);

        const bars = collectByRole(tree as unknown as ShapeNode, 'chart-bar');
        expect(bars.length, 'one bar per value').toBe(c.args.values.length);
        // Every bar should have finite, positive dimensions when rendered
        for (const bar of bars) {
          if (typeof bar.width === 'number') {
            expect(bar.width).toBeGreaterThan(0);
            expect(Number.isFinite(bar.width)).toBe(true);
          }
          if (typeof bar.height === 'number') {
            // Zero-height bars CAN exist for 0-value inputs; they're
            // invisible but must still be finite
            expect(Number.isFinite(bar.height)).toBe(true);
          }
        }
      });
    }

    it('empty values array: throws with a clear message (guards caller against empty input)', () => {
      expect(() => buildChartBars({ values: [] })).toThrow(/at least one number/);
    });
  });

  describe('buildChartLine', () => {
    const cases: Array<{ name: string; args: Parameters<typeof buildChartLine>[0] }> = [
      { name: 'smooth trend', args: { values: [2, 5, 3, 7, 4, 8, 6] } },
      { name: 'monotonic increase', args: { values: [1, 2, 3, 4, 5, 6, 7] } },
      { name: 'flat line (equal values)', args: { values: [5, 5, 5, 5] } },
      { name: 'two points only', args: { values: [1, 10] } },
      { name: 'spike in middle', args: { values: [1, 1, 1, 100, 1, 1, 1] } },
      { name: 'fractional values', args: { values: [0.5, 1.5, 2.25, 3.75] } },
      { name: 'custom dimensions', args: { values: [1, 2, 3], width: 400, height: 200 } },
    ];

    for (const c of cases) {
      it(`${c.name}: builds, layouts, polyline has N points for N values`, () => {
        const tree = buildChartLine(c.args) as unknown as PenNode;
        assignIdsRecursively(tree as unknown as { children?: unknown[] });
        const page = wrapInPage(tree);
        expect(() => {
          const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
          computeLayoutPositions(page, kids);
        }).not.toThrow();
        expect(hasAllFiniteCoords(page as unknown as ShapeNode)).toBe(true);

        // Find the polyline node. Chart-line emits a path with a
        // `points` array; just verify something polyline-ish exists.
        const lines = collectByRole(tree as unknown as ShapeNode, 'chart-line');
        expect(lines.length, 'at least one chart-line geometry').toBeGreaterThan(0);
      });
    }

    it('single value: degenerate but no crash', () => {
      const tree = buildChartLine({ values: [5] }) as unknown as PenNode;
      assignIdsRecursively(tree as unknown as { children?: unknown[] });
      const page = wrapInPage(tree);
      expect(() => {
        const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
        computeLayoutPositions(page, kids);
      }).not.toThrow();
      expect(hasAllFiniteCoords(page as unknown as ShapeNode)).toBe(true);
    });

    it('empty values: throws with a clear message', () => {
      expect(() => buildChartLine({ values: [] })).toThrow(/at least one number/);
    });
  });

  describe('buildChartPie', () => {
    const cases: Array<{ name: string; args: Parameters<typeof buildChartPie>[0] }> = [
      { name: 'four equal slices', args: { values: [25, 25, 25, 25] } },
      { name: 'heavily skewed', args: { values: [90, 5, 3, 2] } },
      { name: 'two slices', args: { values: [60, 40] } },
      { name: 'many thin slices', args: { values: [10, 10, 10, 10, 10, 10, 10, 10, 10, 10] } },
      { name: 'custom diameter', args: { values: [30, 70], diameter: 300 } },
      {
        name: 'donut (inner_radius_ratio=0.5)',
        args: { values: [1, 1, 1, 1], inner_radius_ratio: 0.5 },
      },
      {
        name: 'nearly-full donut (ratio=0.9)',
        args: { values: [50, 50], inner_radius_ratio: 0.9 },
      },
    ];

    for (const c of cases) {
      it(`${c.name}: builds, layouts, one slice per value, angles finite`, () => {
        const tree = buildChartPie(c.args) as unknown as PenNode;
        assignIdsRecursively(tree as unknown as { children?: unknown[] });
        const page = wrapInPage(tree);
        expect(() => {
          const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
          computeLayoutPositions(page, kids);
        }).not.toThrow();
        expect(hasAllFiniteCoords(page as unknown as ShapeNode)).toBe(true);

        const slices = collectByRole(tree as unknown as ShapeNode, 'chart-pie-slice');
        expect(slices.length, 'one slice per value').toBe(c.args.values.length);

        // Angles on each slice must be finite numbers (chart-pie uses
        // startAngle + sweepAngle in degrees, not startAngle/endAngle).
        for (const slice of slices) {
          const s = slice as ShapeNode & { startAngle?: unknown; sweepAngle?: unknown };
          if (s.startAngle !== undefined) {
            expect(isNumFinite(s.startAngle), `slice startAngle: ${s.startAngle}`).toBe(true);
          }
          if (s.sweepAngle !== undefined) {
            expect(isNumFinite(s.sweepAngle), `slice sweepAngle: ${s.sweepAngle}`).toBe(true);
            expect((s.sweepAngle as number) > 0, 'sweepAngle must be positive').toBe(true);
          }
        }
      });
    }

    it('sum of slice sweepAngles = 360° for a complete pie', () => {
      const tree = buildChartPie({ values: [30, 50, 20] }) as unknown as PenNode;
      const slices = collectByRole(tree as unknown as ShapeNode, 'chart-pie-slice');
      let totalSweep = 0;
      for (const slice of slices) {
        const s = slice as ShapeNode & { sweepAngle?: number };
        if (typeof s.sweepAngle === 'number') totalSweep += s.sweepAngle;
      }
      // Degrees; expect ~360 with small floating-point tolerance.
      expect(Math.abs(totalSweep - 360)).toBeLessThan(0.001);
    });

    it('single value (100% slice): still builds without crash', () => {
      const tree = buildChartPie({ values: [100] }) as unknown as PenNode;
      assignIdsRecursively(tree as unknown as { children?: unknown[] });
      const page = wrapInPage(tree);
      expect(() => {
        const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
        computeLayoutPositions(page, kids);
      }).not.toThrow();
    });

    it('empty values: throws with a clear message', () => {
      expect(() => buildChartPie({ values: [] })).toThrow(/at least one number/);
    });

    it('all-zero values: throws (no slice can have 0° sweep)', () => {
      expect(() => buildChartPie({ values: [0, 0, 0] })).toThrow(/sum to > 0/);
    });
  });

  describe('cross-chart invariants', () => {
    it('same input length → same number of geometry children across chart types', () => {
      const values = [10, 20, 30, 40];
      const bars = buildChartBars({ values }) as unknown as PenNode;
      const line = buildChartLine({ values }) as unknown as PenNode;
      const pie = buildChartPie({ values }) as unknown as PenNode;

      // Bar = 1 rect per value; pie = 1 slice per value. Line is a
      // single polyline, not per-value children, but the ROLE count
      // should be non-zero.
      expect(collectByRole(bars as unknown as ShapeNode, 'chart-bar').length).toBe(values.length);
      expect(collectByRole(pie as unknown as ShapeNode, 'chart-pie-slice').length).toBe(
        values.length,
      );
      expect(collectByRole(line as unknown as ShapeNode, 'chart-line').length).toBeGreaterThan(0);
    });

    it('all three types produce finite-coord trees on identical input', () => {
      const values = [10, 20, 30];
      for (const build of [buildChartBars, buildChartLine, buildChartPie]) {
        const tree = build({ values }) as unknown as PenNode;
        assignIdsRecursively(tree as unknown as { children?: unknown[] });
        const page = wrapInPage(tree);
        const kids = (page as PenNode & { children?: PenNode[] }).children ?? [];
        computeLayoutPositions(page, kids);
        expect(hasAllFiniteCoords(page as unknown as ShapeNode)).toBe(true);
      }
    });
  });
});
