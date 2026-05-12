import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  buildCodegenTarget,
  deleteCodeGenerationHistory,
  exportCodeGenerationZip,
  getCodeGenerationHistoryDetail,
  listCodeGenerationHistory,
  listCodeGenerationFiles,
  promoteCodeGenerationHistory,
  saveCodeGenerationHistory,
} from '../codegen-history';

const cloudFetchMock = vi.hoisted(() => vi.fn());
const cloudFetchRawMock = vi.hoisted(() => vi.fn());

vi.mock('../cloud-fetch', () => ({
  cloudFetch: cloudFetchMock,
  cloudFetchRaw: cloudFetchRawMock,
}));

describe('buildCodegenTarget', () => {
  beforeEach(() => {
    cloudFetchMock.mockReset();
    cloudFetchRawMock.mockReset();
  });

  it('builds a page target when no nodes are selected', () => {
    const target = buildCodegenTarget({ pageId: 'page-1', selectedIds: [] });

    expect(target).toMatchObject({
      pageId: 'page-1',
      targetKind: 'page',
      nodeIds: [],
    });
    expect(target.targetHash).toHaveLength(8);
  });

  it('sorts selected node ids before hashing', () => {
    const a = buildCodegenTarget({ pageId: 'page-1', selectedIds: ['b', 'a'] });
    const b = buildCodegenTarget({ pageId: 'page-1', selectedIds: ['a', 'b'] });

    expect(a.nodeIds).toEqual(['a', 'b']);
    expect(a.targetHash).toBe(b.targetHash);
  });

  it('serializes generated files when saving code generation history', async () => {
    cloudFetchMock.mockResolvedValueOnce({
      data: {
        id: 'gen-1',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'uniapp',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-1',
        documentRevision: 3,
        status: 'done',
        finalCode: '',
        entryFile: 'pages/index/index.vue',
        degraded: false,
        assetsManifest: [],
        files: [],
        chunks: [],
        model: null,
        provider: null,
        error: null,
        createdAt: '2026-05-11T08:00:00.000Z',
        completedAt: '2026-05-11T08:00:01.000Z',
        promotedAt: null,
      },
    });

    await saveCodeGenerationHistory({
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'uniapp',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 3,
      status: 'done',
      finalCode: [
        '---FILE: pages/index/index.vue---',
        '<template><view>Home</view></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
      ].join('\n'),
      degraded: false,
      assets: [],
      chunks: [],
    });

    const [, request] = cloudFetchMock.mock.calls[0] ?? [];
    const body = JSON.parse((request as RequestInit).body as string);
    expect(body.entryFile).toBe('pages/index/index.vue');
    expect(body.files).toEqual([
      expect.objectContaining({
        path: 'pages/index/index.vue',
        language: 'vue',
        role: 'page',
      }),
      expect.objectContaining({
        path: 'pages.json',
        language: 'json',
        role: 'config',
      }),
    ]);
  });

  it('loads code generation detail and files by generation id', async () => {
    cloudFetchMock
      .mockResolvedValueOnce({
        data: {
          id: 'gen-1',
          files: [{ id: 'file-row-1', path: 'App.vue' }],
          chunks: [],
        },
      })
      .mockResolvedValueOnce({
        data: [{ id: 'file-row-1', path: 'App.vue' }],
      });

    await expect(getCodeGenerationHistoryDetail('gen-1')).resolves.toMatchObject({
      id: 'gen-1',
      files: [{ path: 'App.vue' }],
    });
    await expect(listCodeGenerationFiles('gen-1')).resolves.toEqual([
      { id: 'file-row-1', path: 'App.vue' },
    ]);
    expect(cloudFetchMock).toHaveBeenNthCalledWith(1, '/api/cloud/code-generations/gen-1');
    expect(cloudFetchMock).toHaveBeenNthCalledWith(2, '/api/cloud/code-generations/gen-1/files');
  });

  it('loads generation history from the cloud API for a file and target', async () => {
    cloudFetchMock.mockResolvedValueOnce({
      data: [
        {
          id: 'gen-1',
          fileId: '11111111-1111-4111-8111-111111111111',
          pageId: 'page-1',
          framework: 'uniapp',
          targetKind: 'page',
          nodeIds: [],
          targetHash: 'hash-1',
          documentRevision: 4,
          status: 'done',
          finalCode: '<template><view>Shared</view></template>',
          entryFile: 'pages/index/index.vue',
          degraded: false,
          assetsManifest: [],
          model: 'model-a',
          provider: 'builtin',
          error: null,
          createdAt: '2026-05-12T08:00:00.000Z',
          completedAt: '2026-05-12T08:00:02.000Z',
          promotedAt: null,
        },
      ],
    });

    const result = await listCodeGenerationHistory({
      fileId: '11111111-1111-4111-8111-111111111111',
      framework: 'uniapp',
      target: {
        pageId: 'page-1',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-1',
      },
      limit: 10,
    });

    expect(result).toEqual([
      expect.objectContaining({
        id: 'gen-1',
        fileId: '11111111-1111-4111-8111-111111111111',
        framework: 'uniapp',
      }),
    ]);
    const [path] = cloudFetchMock.mock.calls[0] ?? [];
    expect(path).toBe(
      '/api/cloud/code-generations?fileId=11111111-1111-4111-8111-111111111111&framework=uniapp&pageId=page-1&targetKind=page&targetHash=hash-1&limit=10',
    );
  });

  it('promotes and deletes generation history through cloud APIs', async () => {
    cloudFetchMock.mockResolvedValueOnce({
      data: {
        id: 'gen-1',
        promotedAt: '2026-05-11T09:00:00.000Z',
      },
    });
    cloudFetchRawMock.mockResolvedValueOnce(new Response(null, { status: 204 }));

    await expect(promoteCodeGenerationHistory('gen-1')).resolves.toMatchObject({
      id: 'gen-1',
      promotedAt: '2026-05-11T09:00:00.000Z',
    });
    await expect(deleteCodeGenerationHistory('gen-1')).resolves.toBeUndefined();

    expect(cloudFetchMock).toHaveBeenCalledWith('/api/cloud/code-generations/gen-1/promote', {
      method: 'POST',
    });
    expect(cloudFetchRawMock).toHaveBeenCalledWith('/api/cloud/code-generations/gen-1', {
      method: 'DELETE',
    });
  });

  it('exports generation zip with server-provided file name', async () => {
    const blob = new Blob([new Uint8Array([1, 2, 3])], { type: 'application/zip' });
    cloudFetchRawMock.mockResolvedValueOnce(
      new Response(blob, {
        status: 200,
        headers: {
          'Content-Disposition': 'attachment; filename="design-uniapp.zip"',
        },
      }),
    );

    const result = await exportCodeGenerationZip('gen-1');

    expect(result.fileName).toBe('design-uniapp.zip');
    expect(await result.blob.arrayBuffer()).toEqual(await blob.arrayBuffer());
    expect(cloudFetchRawMock).toHaveBeenCalledWith('/api/cloud/code-generations/gen-1/export-zip', {
      method: 'POST',
    });
  });
});
