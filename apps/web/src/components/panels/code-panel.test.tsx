// @vitest-environment jsdom

import { cleanup, fireEvent, render, screen, waitFor } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { PenNode } from '@/types/pen';
import { useCodegenStore } from '@/stores/codegen-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type { CloudCodeGeneration } from '@/types/cloud';
import type { GitAPI } from '@/services/git-types';
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
  cloudRevision: 2 as number | undefined,
};
const pageChildren = [selectedNode];

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: (selector: (state: unknown) => unknown) => selector(canvasState),
}));

const listCodeGenerationHistoryMock = vi.fn(async () => [] as CloudCodeGeneration[]);
const getCodeGenerationHistoryDetailMock = vi.fn(async (generationId: string) => ({
  id: generationId,
  fileId: 'file-1',
  pageId: 'page-2',
  framework: 'uniapp',
  targetKind: 'page',
  nodeIds: [],
  targetHash: 'hash-2',
  documentRevision: 6,
  status: 'done',
  finalCode: '---FILE: pages/index/index.vue---\n<template><view>Target history</view></template>',
  degraded: false,
  assetsManifest: [],
  model: 'target-model',
  provider: 'openai',
  error: null,
  metadata: {},
  createdAt: '2026-05-13T08:00:00.000Z',
  completedAt: '2026-05-13T08:00:03.000Z',
  files: [],
  chunks: [],
} satisfies CloudCodeGeneration & { files: unknown[]; chunks: unknown[] }));
const exportCodeGenerationZipMock = vi.fn(async (_generationId: string) => ({
  blob: new Blob([new Uint8Array([4, 5, 6])], { type: 'application/zip' }),
  fileName: 'design-uniapp.zip',
}));
const promoteCodeGenerationHistoryMock = vi.fn(async (generationId: string) => ({
  id: generationId,
  fileId: 'file-1',
  pageId: 'page-1',
  framework: 'html',
  targetKind: 'selection',
  nodeIds: ['node-1'],
  targetHash: 'hash-1',
  documentRevision: 2,
  status: 'done',
  finalCode: '<main>History tools</main>',
  degraded: false,
  assetsManifest: [],
  model: 'model-a',
  provider: 'builtin',
  error: null,
  metadata: {},
  createdAt: '2026-05-09T10:00:00.000Z',
  completedAt: '2026-05-09T10:00:01.000Z',
  promotedAt: '2026-05-09T10:05:00.000Z',
}));
const deleteCodeGenerationHistoryMock = vi.fn(async (_generationId: string) => {});
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
  metadata: {},
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
    deleteCodeGenerationHistory: (generationId: string) => {
      deleteCodeGenerationHistoryMock(generationId);
      return Promise.resolve();
    },
    exportCodeGenerationZip: (generationId: string) => {
      exportCodeGenerationZipMock(generationId);
      return Promise.resolve({
        blob: new Blob([new Uint8Array([4, 5, 6])], { type: 'application/zip' }),
        fileName: 'design-uniapp.zip',
      });
    },
    listCodeGenerationHistory: (_input: Parameters<typeof actual.listCodeGenerationHistory>[0]) =>
      listCodeGenerationHistoryMock(),
    getCodeGenerationHistoryDetail: (generationId: string) =>
      getCodeGenerationHistoryDetailMock(generationId),
    promoteCodeGenerationHistory: (generationId: string) => {
      promoteCodeGenerationHistoryMock(generationId);
      return Promise.resolve({
        id: generationId,
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'html',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: '<main>History tools</main>',
        degraded: false,
        assetsManifest: [],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        metadata: {},
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
        promotedAt: '2026-05-09T10:05:00.000Z',
      });
    },
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
        cloudRevision: documentState.cloudRevision,
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

let restoreGlobalObjectUrls: (() => void) | undefined;

beforeEach(() => {
  restoreGlobalObjectUrls = mockObjectUrls();
});

afterEach(() => {
  cleanup();
  delete (window as unknown as Record<string, unknown>).electronAPI;
  restoreGlobalObjectUrls?.();
  restoreGlobalObjectUrls = undefined;
  generateCodeMock.mockReset();
  generateCodeMock.mockImplementation(defaultGenerateCodeImplementation);
  listCodeGenerationHistoryMock.mockReset();
  listCodeGenerationHistoryMock.mockResolvedValue([]);
  getCodeGenerationHistoryDetailMock.mockClear();
  promoteCodeGenerationHistoryMock.mockClear();
  deleteCodeGenerationHistoryMock.mockClear();
  exportCodeGenerationZipMock.mockClear();
  saveCodeGenerationHistoryMock.mockClear();
  useCodegenStore.getState().reset();
  useCodegenJobStore.getState().reset();
  aiState.model = 'test-model';
  aiState.modelGroups = [{ provider: 'builtin', models: [{ value: 'test-model' }] }];
  documentState.cloudFileId = undefined;
  documentState.cloudRevision = 2;
});

function mockObjectUrls() {
  const originalCreateObjectURL = URL.createObjectURL;
  const originalRevokeObjectURL = URL.revokeObjectURL;
  Object.defineProperty(URL, 'createObjectURL', {
    configurable: true,
    value: vi.fn(() => 'blob:codegen-asset'),
  });
  Object.defineProperty(URL, 'revokeObjectURL', {
    configurable: true,
    value: vi.fn(),
  });
  return () => {
    Object.defineProperty(URL, 'createObjectURL', {
      configurable: true,
      value: originalCreateObjectURL,
    });
    Object.defineProperty(URL, 'revokeObjectURL', {
      configurable: true,
      value: originalRevokeObjectURL,
    });
  };
}

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
        metadata: {},
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
      expect(screen.getByText('Assets')).toBeTruthy();
      expect(screen.getByAltText('Hero Card')).toBeTruthy();
    });
  });

  it('shows background generation and patch actions after a saved generation exists', async () => {
    documentState.cloudFileId = '33333333-3333-4333-8333-333333333333';
    const createJob = vi.fn(async () => ({ id: 'job-1' }));
    useCodegenJobStore.setState({ createJob } as any);

    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Run in background/i })).toBeTruthy();
      expect(screen.getByRole('button', { name: /Fix part/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /Run in background/i }));

    await waitFor(() => {
      expect(createJob).toHaveBeenCalledWith(
        expect.objectContaining({
          jobKind: 'full_generation',
          fileId: documentState.cloudFileId,
          framework: 'react',
        }),
      );
    });
  });

  it('queues an AI patch job from the current selection and base generation', async () => {
    documentState.cloudFileId = '33333333-3333-4333-8333-333333333333';
    const createJob = vi.fn(async () => ({ id: 'patch-job-1' }));
    useCodegenJobStore.setState({ createJob } as any);

    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Fix part/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /Fix part/i }));
    expect(screen.getByText('Selected layer')).toBeTruthy();
    expect(screen.getByText('node-1')).toBeTruthy();
    expect(screen.getByText('Base generation')).toBeTruthy();
    expect(screen.getByText('saved-ge')).toBeTruthy();
    expect(screen.getByText('Framework')).toBeTruthy();
    expect(screen.getAllByText('React').length).toBeGreaterThan(1);
    fireEvent.change(screen.getByPlaceholderText(/Describe what is wrong/i), {
      target: { value: 'Make the selected card spacing tighter.' },
    });
    expect(screen.getByText('Instruction preview')).toBeTruthy();
    expect(screen.getAllByText('Make the selected card spacing tighter.').length).toBeGreaterThan(
      1,
    );
    fireEvent.click(screen.getByRole('button', { name: /Queue fix/i }));

    await waitFor(() => {
      expect(createJob).toHaveBeenCalledWith(
        expect.objectContaining({
          jobKind: 'patch_generation',
          baseGenerationId: 'saved-generation',
          patchInstruction: 'Make the selected card spacing tighter.',
          nodeIds: selectedIds,
          nodes: [selectedNode],
        }),
      );
    });
  });

  it('marks generation history entries created by AI patch', async () => {
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-patch',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'react',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 3,
        status: 'done',
        finalCode: 'export default function Patched() { return null; }',
        degraded: false,
        assetsManifest: [],
        model: 'patch-model',
        provider: 'builtin',
        error: null,
        metadata: {
          jobKind: 'patch_generation',
          baseGenerationId: 'gen-base',
          patchInstruction: 'Fix spacing.',
        },
        createdAt: '2026-05-09T11:00:00.000Z',
        completedAt: '2026-05-09T11:00:01.000Z',
      },
    ]);

    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByText(/function Patched/i)).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /^History/i }));

    expect(screen.getByText(/generated by patch/i)).toBeTruthy();
  });

  it('generates UniApp code when the UniApp tab is active', async () => {
    useCodegenStore.setState({ activeTab: 'uniapp' });
    const uniappCode = [
      '---FILE: App.vue---',
      '<template><view /></template>',
      '---FILE: pages.json---',
      '{"pages":[{"path":"pages/index/index"}]}',
      '---FILE: manifest.json---',
      '{"name":"OpenPencil"}',
      '---FILE: uni.scss---',
      '$uni-color-primary: #0f172a;',
      '---FILE: pages/index/index.vue---',
      '<template><view>UniApp</view></template>',
    ].join('\n');
    generateCodeMock.mockImplementation(async (args: unknown[]) => {
      const onProgress = args[3] as ((event: Record<string, unknown>) => void) | undefined;
      onProgress?.({
        step: 'complete',
        finalCode: uniappCode,
        degraded: false,
      });
      return {
        code: uniappCode,
        degraded: false,
        assets: [],
      };
    });

    render(<CodePanel />);

    fireEvent.click(screen.getByRole('button', { name: /Generate UniApp/i }));

    await waitFor(() => {
      expect(generateCodeMock).toHaveBeenCalled();
      expect(screen.getByText('Files')).toBeTruthy();
      expect(screen.getByText('App.vue')).toBeTruthy();
      expect(screen.getByText('manifest.json')).toBeTruthy();
      expect(screen.getByText('uni.scss')).toBeTruthy();
      expect(screen.getByText('pages/index/index.vue')).toBeTruthy();
      expect(screen.getByRole('button', { name: /Download ZIP/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByText('pages.json').closest('button')!);

    expect(screen.getByText(/"pages"/i)).toBeTruthy();
    expect(
      screen.getAllByText(/pages\/index\/index/i).some((element) => element.tagName === 'CODE'),
    ).toBe(true);
    expect(generateCodeMock.mock.calls[0]?.[0]?.[1]).toBe('uniapp');
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

  it('writes generated files to a local desktop output folder', async () => {
    const selectOutputDirectory = vi.fn(async () => '/tmp/generated');
    const writeFiles = vi.fn(async () => ({
      rootDir: '/tmp/generated',
      writtenFiles: [
        { path: 'design.tsx', absolutePath: '/tmp/generated/design.tsx', bytes: 48 },
      ],
    }));
    const gitStatus = vi.fn(async () => ({
      mode: 'repo' as const,
      rootDir: '/tmp/generated',
      repoRoot: '/tmp/generated',
      branch: 'main',
      changedFiles: ['design.tsx'],
      diff: '+export default function Design',
      hasRemote: false,
    }));
    (window as unknown as { electronAPI: Partial<ElectronAPI> }).electronAPI = {
      isElectron: true,
      codegen: {
        selectOutputDirectory,
        writeFiles,
        revealPath: vi.fn(async () => {}),
        gitStatus,
        gitCommit: vi.fn(),
        gitPush: vi.fn(),
      },
      git: {
        getSystemAuthor: vi.fn(async () => ({
          name: 'OpenPencil Test',
          email: 'test@openpencil.local',
        })),
      } as unknown as GitAPI,
    };
    render(<CodePanel />);

    fireEvent.click(screen.getAllByRole('button', { name: /Generate React/i })[0]);

    await waitFor(() => {
      expect(screen.getByRole('button', { name: /Write Local/i })).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /Write Local/i }));

    await waitFor(() => {
      expect(writeFiles).toHaveBeenCalledWith({
        rootDir: '/tmp/generated',
        files: [{ path: 'design.tsx', content: generatedResult.code }],
      });
      expect(screen.getByText(/Wrote 1 file/i)).toBeTruthy();
      expect(screen.getByText(/\+export default function Design/i)).toBeTruthy();
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
        metadata: {},
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
        metadata: {},
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
        metadata: {},
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

  it('shows signed history assets in the code view', async () => {
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-html',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'html',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: '<main>History asset</main>',
        degraded: false,
        assetsManifest: [
          {
            id: 'asset-1',
            relativePath: './assets/card.png',
            zipPath: 'assets/card.png',
            mimeType: 'image/png',
            sizeBytes: 3,
            signedUrl: 'https://example.test/card.png',
            sourceNodeId: 'node-1',
            sourceNodeName: 'Card asset',
            sourceKind: 'image-fill',
          },
        ],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        metadata: {},
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
      },
    ]);

    useCodegenStore.setState({ activeTab: 'html' });
    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByText('Assets')).toBeTruthy();
      expect(screen.getByAltText('Card asset').getAttribute('src')).toBe(
        'https://example.test/card.png',
      );
    });
  });

  it('runs promote, download, and delete actions from the history list', async () => {
    listCodeGenerationHistoryMock.mockResolvedValue([
      {
        id: 'gen-html',
        fileId: 'file-1',
        pageId: 'page-1',
        framework: 'html',
        targetKind: 'selection',
        nodeIds: ['node-1'],
        targetHash: 'hash-1',
        documentRevision: 2,
        status: 'done',
        finalCode: '<main>History tools</main>',
        degraded: false,
        assetsManifest: [],
        model: 'model-a',
        provider: 'builtin',
        error: null,
        metadata: {},
        createdAt: '2026-05-09T10:00:00.000Z',
        completedAt: '2026-05-09T10:00:01.000Z',
      },
    ]);
    const promoteSpy = vi.spyOn(useCodegenStore.getState(), 'promoteHistoryEntry');
    const deleteSpy = vi.spyOn(useCodegenStore.getState(), 'deleteHistoryEntry');
    const confirmSpy = vi.spyOn(window, 'confirm').mockReturnValue(true);
    const originalClick = HTMLAnchorElement.prototype.click;
    HTMLAnchorElement.prototype.click = vi.fn();
    const createObjectURL = vi.spyOn(URL, 'createObjectURL').mockReturnValue('blob:history-zip');
    const revokeObjectURL = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {});

    useCodegenStore.setState({ activeTab: 'html' });
    render(<CodePanel />);

    await waitFor(() => {
      expect(screen.getByText(/History tools/i)).toBeTruthy();
    });

    fireEvent.click(screen.getByRole('button', { name: /^History/i }));
    fireEvent.click(screen.getByTitle('Promote'));
    fireEvent.click(screen.getByTitle('Download ZIP'));
    fireEvent.click(screen.getByTitle('Delete'));

    expect(promoteSpy).toHaveBeenCalledWith('html', 'gen-html');
    await waitFor(() => {
      expect(exportCodeGenerationZipMock).toHaveBeenCalledWith('gen-html');
    });
    expect(confirmSpy).toHaveBeenCalled();
    expect(deleteSpy).toHaveBeenCalledWith('html', 'gen-html');

    promoteSpy.mockRestore();
    deleteSpy.mockRestore();
    confirmSpy.mockRestore();
    createObjectURL.mockRestore();
    revokeObjectURL.mockRestore();
    HTMLAnchorElement.prototype.click = originalClick;
  });

  it('opens and selects a generation history entry from a task deep link', async () => {
    listCodeGenerationHistoryMock.mockImplementation(async () => [
      {
        id: 'gen-target',
        fileId: 'file-1',
        pageId: 'page-2',
        framework: 'uniapp',
        targetKind: 'page',
        nodeIds: [],
        targetHash: 'hash-2',
        documentRevision: 6,
        status: 'done',
        finalCode: '---FILE: pages/index/index.vue---\n<template><view>Target history</view></template>',
        degraded: false,
        assetsManifest: [],
        model: 'target-model',
        provider: 'openai',
        error: null,
        metadata: {},
        createdAt: '2026-05-13T08:00:00.000Z',
        completedAt: '2026-05-13T08:00:03.000Z',
      },
    ]);

    render(<CodePanel generationId="gen-target" />);

    await waitFor(() => {
      expect(getCodeGenerationHistoryDetailMock).toHaveBeenCalledWith('gen-target');
      expect(screen.getByText(/target-model/i)).toBeTruthy();
      expect(useCodegenStore.getState().activeTab).toBe('uniapp');
      expect(useCodegenStore.getState().selectedHistoryId.uniapp).toBe('gen-target');
      expect(screen.getByRole('button', { name: /^History/i })).toBeTruthy();
    });
  });
});
