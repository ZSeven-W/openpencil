import { describe, it, expect } from 'vitest';
import { buildEllipseArcPath, isArcEllipse } from '../arc-path';

describe('arc-path', () => {
  describe('isArcEllipse', () => {
    it('returns false for full circle with no inner radius', () => {
      expect(isArcEllipse(0, 360, 0)).toBe(false);
    });

    it('returns true for partial sweep', () => {
      expect(isArcEllipse(0, 180, 0)).toBe(true);
    });

    it('returns true for inner radius (donut)', () => {
      expect(isArcEllipse(0, 360, 0.5)).toBe(true);
    });
  });

  describe('buildEllipseArcPath', () => {
    it('builds a full circle path', () => {
      const path = buildEllipseArcPath(100, 100, 0, 360, 0);
      expect(path).toContain('M');
      expect(path).toContain('A');
      expect(path).toContain('Z');
    });

    it('builds a pie slice path', () => {
      const path = buildEllipseArcPath(100, 100, 0, 90, 0);
      expect(path).toContain('M50 50'); // center point for pie
      expect(path).toContain('Z');
    });

    it('builds a donut path with inner radius', () => {
      const path = buildEllipseArcPath(100, 100, 0, 360, 0.5);
      // Should have inner arc commands
      expect(path.split('A').length).toBeGreaterThanOrEqual(3); // outer + inner arcs
    });
  });
});
