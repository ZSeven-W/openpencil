import { describe, expect, it, vi } from 'vitest';
import {
  buildCodegenBundleManifest,
  collectChunkAssetHints,
  extractCodegenAssets,
} from '../codegen-assets';

describe('codegen-assets', () => {
  it('extracts image fill data urls into exported asset files', () => {
    const dataUrl = `data:image/png;base64,${'a'.repeat(64)}`;
    const { nodes, assets } = extractCodegenAssets([
      {
        id: 'node-1',
        type: 'rectangle',
        name: 'Hero Card',
        x: 0.05,
        y: -0.39,
        width: 2560,
        height: 1600,
        fill: [
          {
            type: 'image',
            url: dataUrl,
            mode: 'stretch',
            transform: {
              m00: 0.9682299494743347,
              m01: 0,
              m02: 0.019307976588606834,
              m10: 0,
              m11: 0.9433962106704712,
              m12: 0.041042111814022064,
            },
          },
        ],
      } as any,
    ]);

    expect(assets).toHaveLength(1);
    expect(assets[0].relativePath).toMatch(/^\.\/assets\/hero-card-1\.png$/);
    expect((nodes[0] as any).fill[0].url).toBe(assets[0].relativePath);
    expect((nodes[0] as any).fill[0].originalSize).toEqual({
      width: 2644,
      height: 1696,
    });
    expect((nodes[0] as any).fill[0].explain).toBe(
      'This is not a full-image stretch; the source image is cropped before being mapped into the target bounds',
    );
  });

  it('extracts image node src and deduplicates identical data urls', () => {
    const dataUrl = `data:image/jpeg;base64,${'b'.repeat(64)}`;
    const { nodes, assets } = extractCodegenAssets([
      {
        id: 'img-1',
        type: 'image',
        name: 'Preview',
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        src: dataUrl,
      } as any,
      {
        id: 'shape-1',
        type: 'rectangle',
        name: 'Card',
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        fill: [{ type: 'image', url: dataUrl, mode: 'fill' }],
      } as any,
    ]);

    expect(assets).toHaveLength(1);
    expect((nodes[0] as any).src).toBe(assets[0].relativePath);
    expect((nodes[1] as any).fill[0].url).toBe(assets[0].relativePath);
  });

  it('collects chunk-local asset hints for prompt building', () => {
    const dataUrl = `data:image/png;base64,${'c'.repeat(64)}`;
    const { nodes, assets } = extractCodegenAssets([
      {
        id: 'node-1',
        type: 'rectangle',
        name: 'Gallery',
        x: 0,
        y: 0,
        width: 300,
        height: 200,
        fill: [{ type: 'image', url: dataUrl, mode: 'crop' }],
      } as any,
    ]);

    const hints = collectChunkAssetHints(nodes as any, assets);

    expect(hints).toHaveLength(1);
    expect(hints[0].relativePath).toBe(assets[0].relativePath);
    expect(hints[0].sourceKind).toBe('image-fill');
  });

  it('builds a stable manifest for bundle export', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-09T13:07:40.000Z'));

    const dataUrl = `data:image/png;base64,${'d'.repeat(64)}`;
    const { assets } = extractCodegenAssets([
      {
        id: 'node-1',
        type: 'rectangle',
        name: 'Poster',
        x: 0,
        y: 0,
        width: 300,
        height: 200,
        fill: [{ type: 'image', url: dataUrl, mode: 'crop' }],
      } as any,
    ]);

    const manifest = await buildCodegenBundleManifest({
      framework: 'react',
      codeFile: 'design.tsx',
      codeBytes: new TextEncoder().encode('export default function Design() { return null }'),
      assets,
    });

    expect(manifest.version).toBe(2);
    expect(manifest.framework).toBe('react');
    expect(manifest.entry.codeFile).toBe('design.tsx');
    expect(manifest.generatedAt).toBe('2026-04-09T13:07:40.000Z');
    expect(manifest.code.path).toBe('design.tsx');
    expect(manifest.code.size).toBeGreaterThan(10);
    expect(manifest.code.sha256).toMatch(/^[0-9a-f]{64}$/);
    expect(manifest.assets).toHaveLength(1);
    expect(manifest.assets[0].relativePath).toBe(assets[0].relativePath);
    expect(manifest.assets[0].size).toBe(48);
    expect(manifest.assets[0].sha256).toMatch(/^[0-9a-f]{64}$/);

    vi.useRealTimers();
  });
});
