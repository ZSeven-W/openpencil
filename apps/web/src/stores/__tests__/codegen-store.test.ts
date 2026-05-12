import { beforeEach, describe, expect, it, vi } from 'vitest';
import { useCodegenStore } from '../codegen-store';
import { useDocumentStore } from '../document-store';
import * as codegenHistory from '@/services/cloud/codegen-history';

vi.mock('@/services/cloud/codegen-history', async () => {
  const actual = await vi.importActual<typeof import('@/services/cloud/codegen-history')>(
    '@/services/cloud/codegen-history',
  );
  return {
    ...actual,
    deleteCodeGenerationHistory: vi.fn(),
    listCodeGenerationHistory: vi.fn(),
    promoteCodeGenerationHistory: vi.fn(),
    saveCodeGenerationHistory: vi.fn(),
  };
});

describe('codegen-store', () => {
  beforeEach(() => {
    useCodegenStore.getState().reset();
    useDocumentStore.setState({
      cloudFileId: 'file-1',
      cloudRevision: 4,
    } as any);
    vi.mocked(codegenHistory.listCodeGenerationHistory).mockReset();
    vi.mocked(codegenHistory.promoteCodeGenerationHistory).mockReset();
    vi.mocked(codegenHistory.deleteCodeGenerationHistory).mockReset();
    vi.mocked(codegenHistory.saveCodeGenerationHistory).mockReset();
  });

  it('keeps completed code bundles outside the mounted CodePanel lifecycle', () => {
    const controller = new AbortController();
    const runId = useCodegenStore.getState().startGeneration('node-1', controller);

    useCodegenStore.getState().applyProgress(runId, 'react', {
      step: 'complete',
      finalCode: 'export default function Preview() { return null; }',
      degraded: false,
    });
    useCodegenStore.getState().completeGeneration(runId, 'react', {
      code: 'export default function Preview() { return null; }',
      degraded: false,
      assets: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          bytes: new Uint8Array([1]),
          sourceNodeId: 'node-1',
          sourceNodeName: 'Card',
          sourceKind: 'image-fill',
        },
      ],
    });

    const cached = useCodegenStore.getState().codeCache.react;
    expect(cached?.code).toContain('Preview');
    expect(cached?.assets).toHaveLength(1);
    expect(cached?.files?.[0]).toMatchObject({
      path: 'design.tsx',
      language: 'tsx',
      role: 'entry',
    });
    expect(useCodegenStore.getState().isGenerating).toBe(false);
  });

  it('aborts the in-flight run only when the user cancels', () => {
    const controller = new AbortController();
    const abortSpy = vi.spyOn(controller, 'abort');

    useCodegenStore.getState().startGeneration('node-1', controller);
    useCodegenStore.getState().cancelGeneration();

    expect(abortSpy).toHaveBeenCalledTimes(1);
    expect(useCodegenStore.getState().isGenerating).toBe(false);
    expect(useCodegenStore.getState().abortController).toBeNull();
  });

  it('loads full history and selects the latest generation for the current framework', async () => {
    vi.mocked(codegenHistory.listCodeGenerationHistory).mockResolvedValue([
      {
        id: 'gen-new',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'html',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-1',
        documentRevision: 4,
        status: 'done',
        finalCode: '<main>new</main>',
        degraded: false,
        assetsManifest: [],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:05.000Z',
      },
      {
        id: 'gen-old',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'html',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-1',
        documentRevision: 3,
        status: 'degraded',
        finalCode: '<main>old</main>',
        degraded: true,
        assetsManifest: [],
        model: 'model-b',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T09:00:00.000Z',
        completedAt: '2026-05-09T09:00:04.000Z',
      },
    ]);

    await useCodegenStore.getState().loadHistory('html', {
      pageId: 'page-1',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
    });

    const state = useCodegenStore.getState();
    expect(state.history.html).toHaveLength(2);
    expect(state.history.html?.[0]).toMatchObject({
      id: 'gen-new',
      finalCode: '<main>new</main>',
      model: 'model-a',
      documentRevision: 4,
    });
    expect(state.selectedHistoryId.html).toBe('gen-new');
    expect(state.codeCache.html).toMatchObject({
      code: '<main>new</main>',
      degraded: false,
      historyId: 'gen-new',
    });
    expect(state.codeCache.html?.files?.[0]).toMatchObject({
      path: 'design.html',
      content: '<main>new</main>',
    });
  });

  it('selects an older history entry without writing to the cloud', () => {
    useCodegenStore.setState({
      history: {
        html: [
          {
            id: 'gen-new',
            createdAt: '2026-05-09T10:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>new</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 4,
            assetsManifest: [],
          },
          {
            id: 'gen-old',
            createdAt: '2026-05-09T09:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>old</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 3,
            assetsManifest: [],
          },
        ],
      },
    } as any);

    useCodegenStore.getState().selectHistoryEntry('html', 'gen-old');

    expect(codegenHistory.saveCodeGenerationHistory).not.toHaveBeenCalled();
    expect(useCodegenStore.getState().selectedHistoryId.html).toBe('gen-old');
    expect(useCodegenStore.getState().codeCache.html?.code).toBe('<main>old</main>');
    expect(useCodegenStore.getState().codeCache.html?.files?.[0]).toMatchObject({
      path: 'design.html',
      content: '<main>old</main>',
    });
  });

  it('promotes one history entry and clears sibling promotion markers', async () => {
    useCodegenStore.setState({
      history: {
        html: [
          {
            id: 'gen-new',
            createdAt: '2026-05-09T10:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>new</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 4,
            assetsManifest: [],
            promotedAt: '2026-05-09T10:05:00.000Z',
          },
          {
            id: 'gen-old',
            createdAt: '2026-05-09T09:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>old</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 3,
            assetsManifest: [],
            promotedAt: null,
          },
        ],
      },
    } as any);
    vi.mocked(codegenHistory.promoteCodeGenerationHistory).mockResolvedValue({
      id: 'gen-old',
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'html',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 3,
      status: 'done',
      finalCode: '<main>old</main>',
      degraded: false,
      assetsManifest: [],
      model: null,
      provider: null,
      error: null,
      createdAt: '2026-05-09T09:00:00.000Z',
      completedAt: null,
      promotedAt: '2026-05-09T10:10:00.000Z',
    });

    await useCodegenStore.getState().promoteHistoryEntry('html', 'gen-old');

    const entries = useCodegenStore.getState().history.html ?? [];
    expect(codegenHistory.promoteCodeGenerationHistory).toHaveBeenCalledWith('gen-old');
    expect(entries.find((entry) => entry.id === 'gen-new')?.promotedAt).toBeNull();
    expect(entries.find((entry) => entry.id === 'gen-old')?.promotedAt).toBe(
      '2026-05-09T10:10:00.000Z',
    );
  });

  it('deletes the selected history entry and falls back to the next available generation', async () => {
    useCodegenStore.setState({
      history: {
        html: [
          {
            id: 'gen-new',
            createdAt: '2026-05-09T10:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>new</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 4,
            assetsManifest: [],
          },
          {
            id: 'gen-old',
            createdAt: '2026-05-09T09:00:00.000Z',
            completedAt: null,
            status: 'done',
            degraded: false,
            finalCode: '<main>old</main>',
            model: null,
            provider: null,
            error: null,
            documentRevision: 3,
            assetsManifest: [],
          },
        ],
      },
      selectedHistoryId: { html: 'gen-new' },
      codeCache: {
        html: {
          code: '<main>new</main>',
          degraded: false,
          assets: [],
          historyId: 'gen-new',
        },
      },
    } as any);
    vi.mocked(codegenHistory.deleteCodeGenerationHistory).mockResolvedValue();

    await useCodegenStore.getState().deleteHistoryEntry('html', 'gen-new');

    expect(codegenHistory.deleteCodeGenerationHistory).toHaveBeenCalledWith('gen-new');
    expect(useCodegenStore.getState().history.html?.map((entry) => entry.id)).toEqual(['gen-old']);
    expect(useCodegenStore.getState().selectedHistoryId.html).toBe('gen-old');
    expect(useCodegenStore.getState().codeCache.html?.code).toBe('<main>old</main>');
  });

  it('saves a new history entry at the top and selects it', async () => {
    vi.mocked(codegenHistory.saveCodeGenerationHistory).mockResolvedValue({
      id: 'gen-saved',
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'html',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 4,
      status: 'done',
      finalCode: '<main>saved</main>',
      degraded: false,
      assetsManifest: [],
      model: 'model-a',
      provider: 'builtin',
      error: null,
      createdAt: '2026-05-09T11:00:00.000Z',
      completedAt: '2026-05-09T11:00:02.000Z',
      files: [
        {
          id: 'code-file-1',
          generationId: 'gen-saved',
          path: 'index.html',
          language: 'html',
          role: 'entry',
          content: '<main>saved remote</main>',
          orderIndex: 0,
          sizeBytes: 25,
          createdAt: '2026-05-09T11:00:02.000Z',
        },
      ],
      chunks: [],
    });

    await useCodegenStore
      .getState()
      .saveHistory(
        'html',
        { pageId: 'page-1', targetKind: 'page', nodeIds: [], targetHash: 'hash-1' },
        { code: '<main>saved</main>', degraded: false, assets: [] },
        'model-a',
        'builtin',
      );

    const state = useCodegenStore.getState();
    expect(state.history.html?.[0]).toMatchObject({
      id: 'gen-saved',
      finalCode: '<main>saved</main>',
    });
    expect(state.selectedHistoryId.html).toBe('gen-saved');
    expect(state.codeCache.html?.historyId).toBe('gen-saved');
    expect(state.codeCache.html?.files?.[0]).toMatchObject({
      path: 'index.html',
      content: '<main>saved remote</main>',
    });
    expect(codegenHistory.saveCodeGenerationHistory).toHaveBeenCalledWith(
      expect.objectContaining({
        files: [
          expect.objectContaining({
            path: 'design.html',
            content: '<main>saved</main>',
          }),
        ],
      }),
    );
  });

  it('reloads web-saved cloud history after a desktop cold start for the same cloud file', async () => {
    vi.mocked(codegenHistory.saveCodeGenerationHistory).mockResolvedValue({
      id: 'gen-web-saved',
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'uniapp',
      targetKind: 'page',
      nodeIds: [],
      targetHash: 'hash-1',
      documentRevision: 4,
      status: 'done',
      finalCode: '---FILE: pages/index/index.vue---\n<template><view>Web</view></template>',
      degraded: false,
      assetsManifest: [],
      model: 'model-a',
      provider: 'builtin',
      error: null,
      createdAt: '2026-05-12T08:00:00.000Z',
      completedAt: '2026-05-12T08:00:02.000Z',
      promotedAt: null,
      files: [
        {
          id: 'code-file-1',
          generationId: 'gen-web-saved',
          path: 'pages/index/index.vue',
          language: 'vue',
          role: 'page',
          content: '<template><view>Web</view></template>',
          orderIndex: 0,
          sizeBytes: 36,
          createdAt: '2026-05-12T08:00:02.000Z',
        },
      ],
      chunks: [],
    });

    const target = { pageId: 'page-1', targetKind: 'page' as const, nodeIds: [], targetHash: 'hash-1' };
    await useCodegenStore.getState().saveHistory(
      'uniapp',
      target,
      {
        code: '---FILE: pages/index/index.vue---\n<template><view>Web</view></template>',
        degraded: false,
        assets: [],
      },
      'model-a',
      'builtin',
    );

    const remoteHistory = vi.mocked(codegenHistory.saveCodeGenerationHistory).mock.results[0]
      ?.value as Promise<Awaited<ReturnType<typeof codegenHistory.saveCodeGenerationHistory>>>;
    vi.mocked(codegenHistory.listCodeGenerationHistory).mockResolvedValue([await remoteHistory]);

    useCodegenStore.getState().reset();
    useDocumentStore.setState({
      cloudFileId: 'file-1',
      cloudRevision: 4,
    } as any);

    await useCodegenStore.getState().loadHistory('uniapp', target);

    expect(codegenHistory.listCodeGenerationHistory).toHaveBeenCalledWith({
      fileId: 'file-1',
      framework: 'uniapp',
      target,
    });
    expect(useCodegenStore.getState().history.uniapp?.[0]).toMatchObject({
      id: 'gen-web-saved',
      finalCode: '---FILE: pages/index/index.vue---\n<template><view>Web</view></template>',
      model: 'model-a',
    });
    expect(useCodegenStore.getState().selectedHistoryId.uniapp).toBe('gen-web-saved');
    expect(useCodegenStore.getState().codeCache.uniapp).toMatchObject({
      historyId: 'gen-web-saved',
      code: '---FILE: pages/index/index.vue---\n<template><view>Web</view></template>',
    });
  });
});
