import { describe, expect, it } from 'vitest';
import { hasVisibleStroke, shouldUseTransparentFallbackFill } from '../paint-utils';

describe('hasVisibleStroke', () => {
  it('returns true for a normal colored stroke', () => {
    expect(
      hasVisibleStroke({
        thickness: 4,
        fill: [{ type: 'solid', color: '#60A5FA' }],
      }),
    ).toBe(true);
  });

  it('returns false when stroke is missing a visible color', () => {
    expect(
      hasVisibleStroke({
        thickness: 4,
        fill: [{ type: 'solid', color: '#00000000' }],
      }),
    ).toBe(true);
    expect(
      hasVisibleStroke({
        thickness: 4,
        fill: [],
      }),
    ).toBe(false);
  });

  it('returns false when stroke width resolves to zero', () => {
    expect(
      hasVisibleStroke({
        thickness: 0,
        fill: [{ type: 'solid', color: '#60A5FA' }],
      }),
    ).toBe(false);
  });
});

describe('shouldUseTransparentFallbackFill', () => {
  it('keeps stroke-only shapes hollow instead of falling back to default fill', () => {
    expect(
      shouldUseTransparentFallbackFill(undefined, {
        thickness: 12,
        fill: [{ type: 'solid', color: '#22C55E' }],
      }),
    ).toBe(true);
  });

  it('keeps fill-less containers transparent', () => {
    expect(shouldUseTransparentFallbackFill(undefined, undefined, true)).toBe(true);
  });

  it('does not override explicit fills', () => {
    expect(
      shouldUseTransparentFallbackFill([{ type: 'solid', color: '#00000000' }], {
        thickness: 4,
        fill: [{ type: 'solid', color: '#60A5FA' }],
      }),
    ).toBe(false);
  });
});
