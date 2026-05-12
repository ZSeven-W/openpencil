import { describe, expect, it, vi } from 'vitest';
import { parse as parseZip } from 'uzip';

const getCodeGenerationDetailMock = vi.hoisted(() => vi.fn());

vi.mock('../cloud-code-generation-records', () => ({
  getCodeGenerationDetail: getCodeGenerationDetailMock,
}));

import { buildCodeGenerationExportZip } from '../cloud-code-generation-exports';

describe('buildCodeGenerationExportZip', () => {
  it('builds a zip that keeps the generated UniApp file tree', async () => {
    getCodeGenerationDetailMock.mockResolvedValueOnce({
      id: 'gen-1',
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'uniapp',
      targetKind: 'selection',
      nodeIds: ['node-1'],
      targetHash: 'hash-1',
      documentRevision: 3,
      status: 'done',
      finalCode: [
        '---FILE: App.vue---',
        '<template><view>App</view></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
        '---FILE: pages/index/index.vue---',
        '<template><view>Home</view></template>',
      ].join('\n'),
      entryFile: 'pages/index/index.vue',
      degraded: false,
      assetsManifest: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          sizeBytes: 3,
          storagePath: 'user-1/file-1/code-generations/gen-1/assets/card.png',
          sourceNodeId: 'node-1',
          sourceNodeName: 'Card',
          sourceKind: 'image-fill',
        },
      ],
      model: 'model-a',
      provider: 'builtin',
      error: null,
      createdAt: '2026-05-11T08:00:00.000Z',
      completedAt: '2026-05-11T08:00:01.000Z',
      promotedAt: null,
      files: [
        {
          id: 'file-row-1',
          generationId: 'gen-1',
          path: 'App.vue',
          language: 'vue',
          role: 'entry',
          content: '<template><view>App</view></template>',
          sizeBytes: 37,
          orderIndex: 0,
          createdAt: '2026-05-11T08:00:01.000Z',
        },
        {
          id: 'file-row-2',
          generationId: 'gen-1',
          path: 'pages.json',
          language: 'json',
          role: 'config',
          content: '{"pages":[{"path":"pages/index/index"}]}',
          sizeBytes: 40,
          orderIndex: 1,
          createdAt: '2026-05-11T08:00:01.000Z',
        },
        {
          id: 'file-row-3',
          generationId: 'gen-1',
          path: 'pages/index/index.vue',
          language: 'vue',
          role: 'page',
          content: '<template><view>Home</view></template>',
          sizeBytes: 39,
          orderIndex: 2,
          createdAt: '2026-05-11T08:00:01.000Z',
        },
        {
          id: 'file-row-4',
          generationId: 'gen-1',
          path: 'uni.scss',
          language: 'scss',
          role: 'style',
          content: '$uni-color-primary: #0f172a;',
          sizeBytes: 28,
          orderIndex: 3,
          createdAt: '2026-05-11T08:00:01.000Z',
        },
        {
          id: 'file-row-5',
          generationId: 'gen-1',
          path: 'manifest.json',
          language: 'json',
          role: 'config',
          content: '{"name":"OpenPencil"}',
          sizeBytes: 21,
          orderIndex: 4,
          createdAt: '2026-05-11T08:00:01.000Z',
        },
      ],
      chunks: [
        {
          chunkId: 'chunk-1',
          name: 'App.vue',
          status: 'done',
          code: '<template><view>App</view></template>',
          contract: null,
          error: null,
          orderIndex: 0,
        },
      ],
    });

    const download = vi.fn(async () => ({
      data: new Blob([new Uint8Array([1, 2, 3])], { type: 'image/png' }),
      error: null,
    }));
    const supabase = {
      storage: {
        from: vi.fn(() => ({ download })),
      },
    };

    const result = await buildCodeGenerationExportZip({
      supabase: supabase as never,
      userId: 'user-1',
      generationId: 'gen-1',
    });

    expect(result.fileName).toBe('design-uniapp.zip');
    expect(download).toHaveBeenCalledWith('user-1/file-1/code-generations/gen-1/assets/card.png');

    const archive = parseZip(result.bytes);
    expect(Object.keys(archive).sort()).toEqual([
      'App.vue',
      'assets/card.png',
      'manifest.json',
      'openpencil-codegen-manifest.json',
      'pages.json',
      'pages/index/index.vue',
      'uni.scss',
      'uniapp-bundle.txt',
    ]);

    expect(new TextDecoder().decode(archive['App.vue'])).toContain('<template><view>App</view>');
    expect(new TextDecoder().decode(archive['pages/index/index.vue'])).toContain('Home');
    expect(new TextDecoder().decode(archive['manifest.json'])).toContain('OpenPencil');

    const bundleManifest = JSON.parse(
      new TextDecoder().decode(archive['openpencil-codegen-manifest.json']),
    );
    expect(bundleManifest.framework).toBe('uniapp');
    expect(bundleManifest.entry.codeFile).toBe('uniapp-bundle.txt');
    expect(bundleManifest.assets).toHaveLength(1);
  });
});
