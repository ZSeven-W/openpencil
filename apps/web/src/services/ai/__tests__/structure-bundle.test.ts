import { describe, expect, it, vi } from 'vitest';
import { parse as parseZip } from 'uzip';
import {
  buildAIStructureBundle,
  encodeAIStructureBundleZip,
  type AIStructureBundleManifest,
  type AIStructureBundleViewFile,
} from '../structure-bundle';

describe('structure-bundle', () => {
  it('builds raw and sanitized views with traceable asset refs', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-11T10:00:00.000Z'));

    const dataUrl = `data:image/png;base64,${'e'.repeat(64)}`;
    const bundle = await buildAIStructureBundle({
      nodes: [
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
      ],
      activePageId: 'page-1',
      selectedIds: ['node-1'],
    });

    expect(bundle.fileName).toBe('ai-structure-bundle.zip');
    expect(bundle.manifest.kind).toBe('ai-structure-bundle');
    expect(bundle.manifest.version).toBe(1);
    expect(bundle.manifest.consumerView).toBe('sanitized');
    expect(bundle.manifest.generatedAt).toBe('2026-04-11T10:00:00.000Z');
    expect(bundle.manifest.scope.mode).toBe('selection');
    expect(bundle.manifest.scope.activePageId).toBe('page-1');
    expect(bundle.manifest.scope.selectedIds).toEqual(['node-1']);
    expect(bundle.manifest.scope.exportedRootIds).toEqual(['node-1']);
    expect(bundle.manifest.scope.exportedNodeCount).toBe(1);

    expect(bundle.rawView.view).toBe('raw');
    expect(bundle.rawView.consumer).toBe(false);
    expect((bundle.rawView.nodes[0] as any).fill[0].url).toBe('asset://asset-1');

    expect(bundle.sanitizedView.view).toBe('sanitized');
    expect(bundle.sanitizedView.consumer).toBe(true);
    expect(bundle.sanitizedView.summary).toContain(
      'This is the sanitized structural view intended for direct AI consumption',
    );
    expect(bundle.sanitizedView.highlights).toContain('Includes image crop/mapping semantics');
    expect((bundle.sanitizedView.nodes[0] as any).fill[0].url).toBe('./assets/hero-card-1.png');
    expect((bundle.sanitizedView.nodes[0] as any).fill[0].originalSize).toEqual({
      width: 2644,
      height: 1696,
    });
    expect((bundle.sanitizedView.nodes[0] as any).fill[0].explain).toBe(
      'This is not a full-image stretch; the source image is cropped before being mapped into the target bounds',
    );

    expect(bundle.manifest.assets).toHaveLength(1);
    expect(bundle.manifest.assets[0]).toMatchObject({
      id: 'asset-1',
      relativePath: './assets/hero-card-1.png',
      zipPath: 'assets/hero-card-1.png',
      sourceNodeId: 'node-1',
      sourceNodeName: 'Hero Card',
      sourceKind: 'image-fill',
    });
    expect(bundle.manifest.assets[0].sha256).toMatch(/^[0-9a-f]{64}$/);
    expect(bundle.manifest.assets[0].rawRefs).toEqual([
      {
        pointer: '#/nodes/0/fill/0/url',
        nodeId: 'node-1',
        field: 'fill.url',
        value: 'asset://asset-1',
      },
    ]);
    expect(bundle.manifest.assets[0].sanitizedRefs).toEqual([
      {
        pointer: '#/nodes/0/fill/0/url',
        nodeId: 'node-1',
        field: 'fill.url',
        value: './assets/hero-card-1.png',
      },
    ]);

    vi.useRealTimers();
  });

  it('encodes a zip with manifest, raw view, sanitized view, and assets only', async () => {
    const dataUrl = `data:image/jpeg;base64,${'f'.repeat(64)}`;
    const bundle = await buildAIStructureBundle({
      nodes: [
        {
          id: 'img-1',
          type: 'image',
          name: 'Preview',
          x: 0,
          y: 0,
          width: 120,
          height: 120,
          src: dataUrl,
        } as any,
      ],
      activePageId: 'page-2',
      selectedIds: [],
    });

    const archive = parseZip(encodeAIStructureBundleZip(bundle.zipEntries));
    const entryNames = Object.keys(archive).sort();

    expect(entryNames).toEqual([
      'assets/preview-1.jpg',
      'manifest.json',
      'views/raw.json',
      'views/sanitized.json',
    ]);
    expect(
      entryNames.some(
        (name) => name.endsWith('.tsx') || name.endsWith('.vue') || name.endsWith('.html'),
      ),
    ).toBe(false);

    const manifest = JSON.parse(
      new TextDecoder().decode(archive['manifest.json']),
    ) as AIStructureBundleManifest;
    const rawView = JSON.parse(
      new TextDecoder().decode(archive['views/raw.json']),
    ) as AIStructureBundleViewFile;
    const sanitizedView = JSON.parse(
      new TextDecoder().decode(archive['views/sanitized.json']),
    ) as AIStructureBundleViewFile;

    expect(manifest.scope.mode).toBe('page');
    expect(manifest.views.raw.path).toBe('views/raw.json');
    expect(manifest.views.sanitized.path).toBe('views/sanitized.json');
    expect(rawView.view).toBe('raw');
    expect(sanitizedView.view).toBe('sanitized');
    expect((rawView.nodes[0] as any).src).toBe('asset://asset-1');
    expect((sanitizedView.nodes[0] as any).src).toBe('./assets/preview-1.jpg');
  });

  it('still exports a zip when there are no image assets', async () => {
    const bundle = await buildAIStructureBundle({
      nodes: [
        {
          id: 'text-1',
          type: 'text',
          name: 'Heading',
          x: 10,
          y: 20,
          width: 200,
          height: 40,
          text: 'Hello',
        } as any,
      ],
      activePageId: null,
      selectedIds: [],
    });

    const archive = parseZip(encodeAIStructureBundleZip(bundle.zipEntries));
    const entryNames = Object.keys(archive).sort();

    expect(entryNames).toEqual(['manifest.json', 'views/raw.json', 'views/sanitized.json']);
    expect(bundle.manifest.assets).toEqual([]);
    expect(bundle.sanitizedView.summary).toContain('containing 1 nodes');
    expect(bundle.sanitizedView.highlights).toEqual([
      'Primarily basic geometry and style structure',
    ]);
    expect((bundle.rawView.nodes[0] as any).explain).toBeUndefined();
    expect((bundle.sanitizedView.nodes[0] as any).explain).toBe(
      'Width is fixed at 200px, Height is fixed at 40px',
    );
  });
});
