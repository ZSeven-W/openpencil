import { describe, expect, it } from 'vitest';

import type { PenPathAnchor } from '@/types/pen';

import {
  bakeSceneAnchorsToPathNode,
  mapAnchorsToScene,
  movePathControl,
  resetPathPointHandles,
  setPathPointType,
} from '../path-editing';

describe('path editing helpers', () => {
  it('maps local path anchors into scene coordinates with node scaling', () => {
    const anchors: PenPathAnchor[] = [
      {
        x: 10,
        y: 20,
        handleIn: null,
        handleOut: { x: 5, y: 10 },
      },
      {
        x: 60,
        y: 120,
        handleIn: { x: -10, y: -20 },
        handleOut: null,
      },
    ];

    const result = mapAnchorsToScene(
      anchors,
      { x: 10, y: 20, width: 50, height: 100 },
      { x: 200, y: 300, width: 100, height: 200 },
    );

    expect(result).toEqual([
      {
        x: 200,
        y: 300,
        handleIn: null,
        handleOut: { x: 10, y: 20 },
      },
      {
        x: 300,
        y: 500,
        handleIn: { x: -20, y: -40 },
        handleOut: null,
      },
    ]);
  });

  it('moves anchors and handles without losing relative handle vectors', () => {
    const anchors: PenPathAnchor[] = [
      {
        x: 20,
        y: 30,
        handleIn: null,
        handleOut: { x: 10, y: 0 },
      },
    ];

    const movedAnchor = movePathControl(anchors, 0, 'anchor', 15, -5);
    expect(movedAnchor[0]).toEqual({
      x: 35,
      y: 25,
      handleIn: null,
      handleOut: { x: 10, y: 0 },
    });

    const movedHandle = movePathControl(movedAnchor, 0, 'handleOut', -3, 7);
    expect(movedHandle[0]).toEqual({
      x: 35,
      y: 25,
      handleIn: null,
      handleOut: { x: 7, y: 7 },
    });
  });

  it('keeps mirrored handles locked together when dragging one side', () => {
    const anchors: PenPathAnchor[] = [
      {
        x: 100,
        y: 100,
        handleIn: { x: -20, y: 0 },
        handleOut: { x: 20, y: 0 },
        pointType: 'mirrored',
      },
    ];

    const movedHandle = movePathControl(anchors, 0, 'handleOut', 10, 5);
    expect(movedHandle[0]).toEqual({
      x: 100,
      y: 100,
      handleIn: { x: -30, y: -5 },
      handleOut: { x: 30, y: 5 },
      pointType: 'mirrored',
    });
  });

  it('can convert a corner point into curve modes and reset its default handles', () => {
    const anchors: PenPathAnchor[] = [
      {
        x: 0,
        y: 0,
        handleIn: null,
        handleOut: null,
      },
      {
        x: 100,
        y: 0,
        handleIn: null,
        handleOut: null,
      },
      {
        x: 200,
        y: 0,
        handleIn: null,
        handleOut: null,
      },
    ];

    const mirrored = setPathPointType(anchors, 1, 'mirrored', false);
    expect(mirrored[1].x).toBe(100);
    expect(mirrored[1].y).toBe(0);
    expect(mirrored[1].pointType).toBe('mirrored');
    expect(mirrored[1].handleIn?.x).toBeCloseTo(-100 / 3, 5);
    expect(mirrored[1].handleIn?.y).toBeCloseTo(0, 5);
    expect(mirrored[1].handleOut?.x).toBeCloseTo(100 / 3, 5);
    expect(mirrored[1].handleOut?.y).toBeCloseTo(0, 5);

    const corner = setPathPointType(mirrored, 1, 'corner', false);
    expect(corner[1]).toEqual({
      x: 100,
      y: 0,
      handleIn: null,
      handleOut: null,
      pointType: 'corner',
    });

    const reset = resetPathPointHandles(corner, 1, false);
    expect(reset[1].x).toBe(100);
    expect(reset[1].y).toBe(0);
    expect(reset[1].pointType).toBe('mirrored');
    expect(reset[1].handleIn?.x).toBeCloseTo(-100 / 3, 5);
    expect(reset[1].handleIn?.y).toBeCloseTo(0, 5);
    expect(reset[1].handleOut?.x).toBeCloseTo(100 / 3, 5);
    expect(reset[1].handleOut?.y).toBeCloseTo(0, 5);
  });

  it('bakes edited scene anchors back into a normalized path node patch', () => {
    const anchors: PenPathAnchor[] = [
      {
        x: 210,
        y: 305,
        handleIn: null,
        handleOut: { x: 20, y: 10 },
      },
      {
        x: 290,
        y: 355,
        handleIn: { x: -15, y: -20 },
        handleOut: null,
      },
    ];

    const result = bakeSceneAnchorsToPathNode(anchors, false, { x: 100, y: 200 });

    expect(result).toEqual({
      x: 110,
      y: 105,
      width: 80,
      height: 50,
      closed: false,
      d: 'M 0 0 C 20 10 65 30 80 50',
      anchors: [
        {
          x: 0,
          y: 0,
          handleIn: null,
          handleOut: { x: 20, y: 10 },
        },
        {
          x: 80,
          y: 50,
          handleIn: { x: -15, y: -20 },
          handleOut: null,
        },
      ],
    });
  });
});
