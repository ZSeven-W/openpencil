// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { PenNode } from '@/types/pen';
import { useCodegenStore } from '@/stores/codegen-store';
import type { CloudCodeGeneration } from '@/types/cloud';
import '@/i18n';

const selectedNode: PenNode = {
  id: 'node-1',
  type: 'rectangle',
  x: 0,
  y: 0,
  width: 100,
  height: 80,
};
const selectedIds = ['node-1'];
const canvasState = {
  selection: { selectedIds },
  activePageId: 'page-1',
};
const aiState = {
  model: 'test-model',
  modelGroups: [{ provider: 'builtin', models: [{ value: 'test-model' }] }],
};
const documentState = {
  cloudFileId: undefined as string | undefined,
};
const pageChildren = [selectedNode];

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: (selector: (state: unknown) => unknown) => selector(canvasState),
}));

const listCodeGenerationHistoryMock = vi.fn(async () => [] as CloudCodeGeneration[]);
const saveCodeGenerationHistoryMock = vi.fn(async () => ({
  id: 'saved-generation',
  fileId: 'file-1',
  pageId: 'page-1',
  framework: 'react',
  targetKind: 'selection',
  nodeIds: ['node-1'],
  targetHash: 'target-hash',
  documentRevision: 2,
  status: 'done',
  finalCode: 'export default function Design() { return null; }',
  degraded: false,
  assetsManifest: [],
  model: 'test-model',
  provider: 'builtin',
  error: null,
  createdAt: '2026-05-09T10:00:00.000Z',
  completedAt: '2026-05-09T10:00:01.000Z',
  chunks: [],
}));

vi.mock('@/services/cloud/codegen-history', async () => {
  const actual = await vi.importActual<typeof import('@/services/cloud/codegen-history')>(
    '@/services/cloud/codegen-history',
  );
  return {
    ...actual,
    listCodeGenerationHistory: (_input: Parameters<typeof actual.listCodeGenerationHistory>[0]) =>
      listCodeGenerationHistoryMock(),
    saveCodeGenerationHistory: (_input: Parameters<typeof actual.saveCodeGenerationHistory>[0]) =>
      saveCodeGenerationHistoryMock(),
  };
});

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: Object.assign(
    (selector: (state: unknown) => unknown) =>
    selector({
      getNodeById: (id: string) => (id === 'node-1' ? selectedNode : undefined),
      document: { variables: {} },
      cloudFileId: documentState.cloudFileId,
    }),
    {
      getState: () => ({
        cloudFileId: 'file-1',
        cloudRevision: 2,
      }),
    },
  ),
  getActivePageChildren: () => pageChildren,
}));

vi.mock('@/stores/ai-store', () => ({
  useAIStore: (selector: (state: unknown) => unknown) => selector(aiState),
}));

vi.mock('@/stores/agent-settings-store', () => ({
  useAgentSettingsStore: (selector: (state: unknown) => unknown) =>
    selector({ builtinProviders: [] }),
}));

const generatedResult = {
  code: 'export default function Design() { return null; }',
  degraded: false,
  assets: [
    {
      id: 'asset-1',
      relativePath: './assets/hero-card-1.png',
      zipPath: 'assets/hero-card-1.png',
      mimeType: 'image/png',
      bytes: new Uint8Array([1, 2, 3]),
      sourceNodeId: 'node-1',
      sourceNodeName: 'Hero Card',
      sourceKind: 'image-fill' as const,
    },
  ],
};

const defaultGenerateCodeImplementation = async (args: unknown[]) => {
  const onProgress = args[3] as ((event: Record<string, unknown>) => void) | undefined;
  onProgress?.({
    step: 'complete',
    finalCode: generatedResult.code,
    degraded: generatedResult.degraded,
  });
  return generatedResult;
};

const generateCodeMock = vi.fn(defaultGenerateCodeImplementation);

vi.mock('@/services/ai/code-generation-pipeline', () => ({
  generateCode: (...args: unknown[]) => generateCodeMock(args),
}));

vi.mock('@/services/ai/codegen-assets', () => ({
  buildCodegenBundleManifest: vi.fn(async () => ({ version: 2, assets: [] })),
}));

vi.mock('@/services/ai/structure-bundle', () => ({
  buildAIStructureBundle: vi.fn(async () => ({
    fileName: 'ai-structure-bundle.zip',
    zipEntries: {},
  })),
  encodeAIStructureBundleZip: vi.fn(() => new ArrayBuffer(0)),
}));

vi.mock('@/utils/syntax-highlight', () => ({
  highlightCode: (code: string) => code,
}));

import CodePanel from './code-panel';

afterEach(() => {
  cleanup();
  generateCodeMock.mockReset();
  generateCodeMock.mockImplementation(defaultGenerateCodeImplementation);
  listCodeGenerationHistoryMock.mockReset();
  listCodeGenerationHistoryMock.mockResolvedValue([]);
  saveCodeGenerationHistoryMock.mockClear();
  useCodegenStore.getState().reset();
  aiState.model = 'test-model';
  aiState.modelGroups = [{ provider: 'builtin', models: [{ value: 'test-model' }] }];
  documentState.cloudFileId = undefined;
});

describe('CodePanel export affordances', () => {
  it('shows the AI bundle export action in the empty state', () => {
    render(<CodePanel />);

    expect(screen.getByRole('button', { name: /Export AI Bundle/i })).toBeTruthy();
  });

  it('disables cloud generation when the selected model has no provider', () => {
    documentState.cloudFileId = 'file-1';
    aiState.model = 'claude-sonnet-4-5-20250929';
    aiState.modelGroups = [];

    render(<CodePanel />);

    expect(screen.getByRole('button', { name: /Generate React/i })).toHaveProperty(
      'disabled',
      true,
    );
    expect(screen.getByText(/Connect an AI model/i)).toBeTruthy();
  });

  it('keeps history preview without showing a separate preview tab', async () => {
    documentState.cloudFileId = 'file-1';
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-html',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'react',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: 'export default function Existing() { return null; }',
        degraded: false,
        assetsManifest: [],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
      },
    ]);

    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^History/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /^History/i }));

    expect(screen.getByRole('button', { name: /^Preview$/i })).toBeTruthy();
    expect(screen.queryByRole('button', { name: /^Preview$/i })).toBeTruthy();
  });

  it('shows AI bundle and zip download actions after generation with assets', async () => {
    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getAllByRole('button', { name: /AI Bundle/i }).length).toBeGreaterThan(0);
      expect(screen.getByRole('button', { name: /Download ZIP/i })).toBeTruthy();
    });
  });

  it('keeps generated code when the panel unmounts and mounts again', async () => {
    const firstRender = render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getByText(/export default function Design/i)).toBeTruthy();
    });

    firstRender.unmount();
    render(<CodePanel />);

    expect(screen.getByText(/export default function Design/i)).toBeTruthy();
  });

  it('keeps an in-flight generation running after the panel unmounts', async () => {
    let resolveGeneration: (value: typeof generatedResult) => void = () => {};
    let generationSignal: AbortSignal | undefined;
    generateCodeMock.mockImplementation(
      (args: unknown[]) =>
        new Promise((resolve) => {
          generationSignal = args[6] as AbortSignal;
          resolveGeneration = resolve;
        }),
    );

    const firstRender = render(<CodePanel />);
    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(generationSignal).toBeDefined();
    });

    firstRender.unmount();
    expect(generationSignal?.aborted).toBe(false);

    resolveGeneration(generatedResult);

    await waitFor(() => {
      expect(useCodegenStore.getState().codeCache.react?.code).toContain('Design');
    });

    render(<CodePanel />);
    expect(screen.getByText(/export default function Design/i)).toBeTruthy();
  });

  it('shows code and history views after generation', async () => {
    generateCodeMock.mockImplementation(async (args: unknown[]) => {
      const onProgress = args[3] as ((event: Record<string, unknown>) => void) | undefined;
      onProgress?.({
        step: 'complete',
        finalCode: '<main>Previewable</main>',
        degraded: false,
      });
      return { code: '<main>Previewable</main>', degraded: false, assets: [] };
    });
    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /^Code$/i })).toBeTruthy();
      expect(screen.queryByRole('button', { name: /^Preview$/i })).toBeNull();
      expect(screen.getByRole('button', { name: /^History/i })).toBeTruthy();
    });
  });

  it('switches displayed code when a history item is selected', async () => {
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-new',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'react',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: 'export default function Newer() { return null; }',
        degraded: false,
        assetsManifest: [],
        model: 'new-model',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
      },
      {
        id: 'gen-old',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'react',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 1,
        status: 'done',
        finalCode: 'export default function Older() { return null; }',
        degraded: false,
        assetsManifest: [],
        model: 'old-model',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T09:00:00.000Z',
        completedAt: '2026-05-09T09:00:01.000Z',
      },
    ]);

    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByText(/function Newer/i)).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /^History/i }));
    fireEvent.click(screen.getByText(/old-model/i).closest('button')!);

    expect(screen.getByText(/function Older/i)).toBeTruthy();
    expect(saveCodeGenerationHistoryMock).not.toHaveBeenCalled();
  });

  it('opens a browser preview from the history list without saving', async () => {
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-html',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'react',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: '<main>History preview</main>',
        degraded: false,
        assetsManifest: [],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
      },
    ]);
    const close = vi.fn();
    const write = vi.fn();
    const open = vi.fn();
    const windowOpen = vi.spyOn(window, 'open').mockReturnValue({
      opener: null,
      document: { open, write, close },
    } as unknown as Window);

    useCodegenStore.setState({ activeTab: 'html' });
    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByText(/History preview/i)).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /^History/i }));
    fireEvent.click(screen.getByRole('button', { name: /^Preview$/i }));

    expect(windowOpen).toHaveBeenCalledWith('', '_blank');
    expect(write).toHaveBeenCalledWith(expect.stringContaining('History preview'));
    expect(saveCodeGenerationHistoryMock).not.toHaveBeenCalled();

    windowOpen.mockRestore();
  });
});
