import { describe, expect, it } from 'vitest';

import {
  anchorsToPathData,
  getPathBoundsFromAnchors,
  inferPathAnchorPointType,
  pathDataToAnchors,
} from '../path-anchors';

describe('path anchor utilities', () => {
  it('parses cubic and line path commands into editable anchors', () => {
    const result = pathDataToAnchors('M 10 20 C 30 40 50 60 70 80 L 90 100 Z');

    expect(result).not.toBeNull();
    expect(result?.closed).toBe(true);
    expect(result?.anchors).toEqual([
      {
        x: 10,
        y: 20,
        handleIn: null,
        handleOut: { x: 20, y: 20 },
      },
      {
        x: 70,
        y: 80,
        handleIn: { x: -20, y: -20 },
        handleOut: null,
      },
      {
        x: 90,
        y: 100,
        handleIn: null,
        handleOut: null,
      },
    ]);
  });

  it('rebuilds path data from editable anchors', () => {
    const d = anchorsToPathData(
      [
        { x: 0, y: 0, handleIn: null, handleOut: { x: 10, y: 0 } },
        { x: 30, y: 20, handleIn: { x: -5, y: -10 }, handleOut: null },
        { x: 60, y: 20, handleIn: null, handleOut: null },
      ],
      false,
    );

    expect(d).toBe('M 0 0 C 10 0 25 10 30 20 L 60 20');
  });

  it('returns null for unsupported quadratic commands', () => {
    expect(pathDataToAnchors('M 0 0 Q 10 10 20 0')).toBeNull();
  });

  it('measures cubic bounds from the actual bezier curve instead of the control box', () => {
    const bounds = getPathBoundsFromAnchors(
      [
        { x: 0, y: 0, handleIn: null, handleOut: { x: 100, y: 100 } },
        { x: 300, y: 0, handleIn: { x: -100, y: 100 }, handleOut: null },
      ],
      false,
    );

    expect(bounds.x).toBe(0);
    expect(bounds.y).toBe(0);
    expect(bounds.width).toBe(300);
    expect(bounds.height).toBeCloseTo(75, 5);
  });

  it('infers path point types from handle geometry', () => {
    expect(
      inferPathAnchorPointType({
        x: 0,
        y: 0,
        handleIn: null,
        handleOut: null,
      }),
    ).toBe('corner');

    expect(
      inferPathAnchorPointType({
        x: 0,
        y: 0,
        handleIn: { x: -20, y: 0 },
        handleOut: { x: 20, y: 0 },
      }),
    ).toBe('mirrored');

    expect(
      inferPathAnchorPointType({
        x: 0,
        y: 0,
        handleIn: { x: -20, y: 0 },
        handleOut: { x: 10, y: 5 },
      }),
    ).toBe('independent');
  });
});
