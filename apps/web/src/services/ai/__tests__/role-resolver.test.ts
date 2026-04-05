import { describe, it, expect } from 'vitest';
import { hexLuminance, hasFill } from '../role-resolver';
import type { PenNode } from '@zseven-w/pen-types';

describe('hexLuminance', () => {
  it('returns 0 for black', () => {
    expect(hexLuminance('#000000')).toBeCloseTo(0, 2);
  });

  it('returns 1 for white', () => {
    expect(hexLuminance('#FFFFFF')).toBeCloseTo(1, 2);
  });

  it('returns ~0.5 for mid-gray', () => {
    const lum = hexLuminance('#808080');
    expect(lum).toBeGreaterThan(0.2);
    expect(lum).toBeLessThan(0.6);
  });

  it('handles lowercase hex', () => {
    expect(hexLuminance('#ffffff')).toBeCloseTo(1, 2);
  });

  it('handles 8-digit hex (with alpha)', () => {
    expect(hexLuminance('#000000FF')).toBeCloseTo(0, 2);
  });

  it('returns < 0.5 for dark blue (#2563EB)', () => {
    expect(hexLuminance('#2563EB')).toBeLessThan(0.5);
  });

  it('returns > 0.5 for light gray (#F8FAFC)', () => {
    expect(hexLuminance('#F8FAFC')).toBeGreaterThan(0.5);
  });
});

describe('hasFill', () => {
  it('returns false for node without fill', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100 } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns false for empty fill array', () => {
    const node = { id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100, fill: [] } as PenNode;
    expect(hasFill(node)).toBe(false);
  });

  it('returns true for node with solid fill', () => {
    const node = {
      id: 'n1', type: 'frame', x: 0, y: 0, width: 100, height: 100,
      fill: [{ type: 'solid', color: '#FFFFFF' }],
    } as PenNode;
    expect(hasFill(node)).toBe(true);
  });
});
