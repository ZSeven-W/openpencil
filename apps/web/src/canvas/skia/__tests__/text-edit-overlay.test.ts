import { describe, expect, it } from 'vitest';

import { projectTextEditStateToViewport } from '../text-edit-overlay';

describe('projectTextEditStateToViewport', () => {
  it('projects scene-space text editing bounds into the current viewport', () => {
    expect(
      projectTextEditStateToViewport(
        {
          nodeId: 'text-1',
          x: 120,
          y: 80,
          w: 240,
          h: 64,
          content: 'Hello',
          fontSize: 18,
          fontFamily: 'Inter',
          fontWeight: '400',
          textAlign: 'left',
          color: '#111111',
          lineHeight: 1.5,
        },
        {
          zoom: 1.5,
          panX: -30,
          panY: 45,
        },
      ),
    ).toEqual({
      left: 150,
      top: 165,
      width: 360,
      minHeight: 96,
      fontSize: 27,
    });
  });

  it('keeps overlay dimensions positive even when zoomed far out', () => {
    expect(
      projectTextEditStateToViewport(
        {
          nodeId: 'text-1',
          x: 10,
          y: 20,
          w: 0.1,
          h: 0.2,
          content: 'Hello',
          fontSize: 12,
          fontFamily: 'Inter',
          fontWeight: '400',
          textAlign: 'left',
          color: '#111111',
          lineHeight: 1.4,
        },
        {
          zoom: 0.01,
          panX: 0,
          panY: 0,
        },
      ),
    ).toEqual({
      left: 0.1,
      top: 0.2,
      width: 1,
      minHeight: 1,
      fontSize: 0.12,
    });
  });
});
