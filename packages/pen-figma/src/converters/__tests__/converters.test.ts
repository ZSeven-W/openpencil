import { describe, it, expect } from 'vitest';
import { convertNode, collectImageBlobs } from '../index';
import type { ConversionContext } from '../index';

function makeCtx(): ConversionContext {
  let id = 0;
  return {
    componentMap: new Map(),
    symbolTree: new Map(),
    warnings: [],
    generateId: () => `test-${++id}`,
    blobs: [],
    layoutMode: 'openpencil',
  };
}

describe('convertNode', () => {
  it('should convert a RECTANGLE TreeNode to a PenNode rectangle', () => {
    const ctx = makeCtx();
    const treeNode = {
      figma: {
        type: 'RECTANGLE',
        name: 'Test Rect',
        size: { x: 100, y: 50 },
        transform: { m00: 1, m01: 0, m02: 10, m10: 0, m11: 1, m12: 20 },
      },
      children: [],
    };

    const result = convertNode(treeNode as any, undefined, ctx);
    expect(result).not.toBeNull();
    expect(result!.type).toBe('rectangle');
    expect(result!.name).toBe('Test Rect');
    expect(result!.x).toBe(10);
    expect(result!.y).toBe(20);
  });

  it('should convert a TEXT TreeNode', () => {
    const ctx = makeCtx();
    const treeNode = {
      figma: {
        type: 'TEXT',
        name: 'Title',
        size: { x: 200, y: 30 },
        transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 },
        styledTextSegments: [
          { characters: 'Hello World', fontSize: 16, fontWeight: 400, fontFamily: 'Inter' },
        ],
      },
      children: [],
    };

    const result = convertNode(treeNode as any, undefined, ctx);
    expect(result).not.toBeNull();
    expect(result!.type).toBe('text');
  });

  it('should skip SLICE node types', () => {
    const ctx = makeCtx();
    const treeNode = {
      figma: { type: 'SLICE', name: 'Slice', size: { x: 100, y: 100 } },
      children: [],
    };

    const result = convertNode(treeNode as any, undefined, ctx);
    expect(result).toBeNull();
  });

  it('should convert a FRAME TreeNode', () => {
    const ctx = makeCtx();
    const treeNode = {
      figma: {
        type: 'FRAME',
        name: 'Container',
        size: { x: 300, y: 200 },
        transform: { m00: 1, m01: 0, m02: 0, m10: 0, m11: 1, m12: 0 },
      },
      children: [],
    };

    const result = convertNode(treeNode as any, undefined, ctx);
    expect(result).not.toBeNull();
    expect(result!.type).toBe('frame');
  });
});

describe('collectImageBlobs', () => {
  it('should detect PNG blobs', () => {
    const pngHeader = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00]);
    const blobs: (Uint8Array | string)[] = ['text', pngHeader];
    const map = collectImageBlobs(blobs);
    expect(map.size).toBe(1);
    expect(map.has(1)).toBe(true);
  });

  it('should detect JPEG blobs', () => {
    const jpegHeader = new Uint8Array([0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10, 0x4a, 0x46, 0x49]);
    const blobs: (Uint8Array | string)[] = [jpegHeader];
    const map = collectImageBlobs(blobs);
    expect(map.size).toBe(1);
  });
});
