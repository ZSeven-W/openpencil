import { describe, expect, it } from 'vitest';
import { mapFigmaFills } from './figma-fill-mapper';

describe('mapFigmaFills', () => {
  it('preserves non-identity image transforms for cropped image fills', () => {
    const fills = mapFigmaFills([
      {
        type: 'IMAGE',
        visible: true,
        opacity: 1,
        imageScaleMode: 'STRETCH',
        originalImageWidth: 2644,
        originalImageHeight: 1696,
        transform: {
          m00: 0.9682299494743347,
          m01: 0,
          m02: 0.019307976588606834,
          m10: 0,
          m11: 0.9433962106704712,
          m12: 0.041042111814022064,
        },
        image: {
          hash: Uint8Array.from([
            0x1a, 0x5f, 0x26, 0xdd, 0xcd, 0x1f, 0xf2, 0xdb, 0x35, 0x95, 0xb8, 0x45, 0xfb, 0xe9,
            0xa1, 0x77, 0x1c, 0x46, 0xae, 0x3f,
          ]),
        },
      },
    ]);

    expect(fills).toEqual([
      {
        type: 'image',
        url: '__hash:1a5f26ddcd1ff2db3595b845fbe9a1771c46ae3f',
        mode: 'stretch',
        originalSize: {
          width: 2644,
          height: 1696,
        },
        opacity: 1,
        transform: {
          m00: 0.9682299494743347,
          m01: 0,
          m02: 0.019307976588606834,
          m10: 0,
          m11: 0.9433962106704712,
          m12: 0.041042111814022064,
        },
      },
    ]);
  });

  it('drops identity image transforms to avoid noisy documents', () => {
    const fills = mapFigmaFills([
      {
        type: 'IMAGE',
        visible: true,
        imageScaleMode: 'STRETCH',
        transform: {
          m00: 1,
          m01: 0,
          m02: 0,
          m10: 0,
          m11: 1,
          m12: 0,
        },
        image: {
          dataBlob: 42,
        },
      },
    ]);

    expect(fills).toEqual([
      {
        type: 'image',
        url: '__blob:42',
        mode: 'stretch',
        opacity: undefined,
        transform: undefined,
      },
    ]);
  });
});
