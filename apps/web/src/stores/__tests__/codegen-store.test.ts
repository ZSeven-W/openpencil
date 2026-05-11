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
    listCodeGenerationHistory: vi.fn(),
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
      chunks: [],
    });

    await useCodegenStore.getState().saveHistory(
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
  });
});
