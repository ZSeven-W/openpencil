import { create } from 'zustand';
import type { CodeGenProgress, ChunkStatus, Framework } from '@zseven-w/pen-types';
import type { CodegenAssetFile } from '@/services/ai/codegen-assets';
import { buildCodegenFiles } from '@/services/ai/codegen-files';
import type {
  CloudCodeGeneration,
  CodegenAssetManifestEntry,
  CodegenTarget,
  SaveCodegenFileInput,
} from '@/types/cloud';
import {
  deleteCodeGenerationHistory,
  listCodeGenerationHistory,
  promoteCodeGenerationHistory,
  saveCodeGenerationHistory,
} from '@/services/cloud/codegen-history';
import { useDocumentStore } from '@/stores/document-store';

export type CodegenStepStatus = 'idle' | 'running' | 'done' | 'failed';

export interface CodegenChunkProgress {
  chunkId: string;
  name: string;
  status: ChunkStatus;
  error?: string;
}

export interface GeneratedCodeBundle {
  code: string;
  degraded: boolean;
  assets: CodegenAssetFile[];
  historyId?: string;
  status?: 'done' | 'degraded' | 'failed';
  assetsManifest?: CodegenAssetManifestEntry[];
  files?: SaveCodegenFileInput[];
  metadata?: Record<string, unknown>;
}

export interface CodegenHistoryEntry {
  id: string;
  createdAt: string;
  completedAt: string | null;
  status: 'done' | 'degraded' | 'failed';
  degraded: boolean;
  finalCode: string | null;
  model: string | null;
  provider: string | null;
  error: string | null;
  documentRevision: number;
  assetsManifest: CodegenAssetManifestEntry[];
  pageId?: string;
  targetKind?: 'page' | 'selection';
  nodeIds?: string[];
  targetHash?: string;
  entryFile?: string | null;
  promotedAt?: string | null;
  metadata?: Record<string, unknown>;
}

interface CodegenState {
  activeTab: Framework;
  codeCache: Partial<Record<Framework, GeneratedCodeBundle>>;
  isGenerating: boolean;
  planningStatus: CodegenStepStatus;
  planningError?: string;
  assemblyStatus: CodegenStepStatus;
  chunks: CodegenChunkProgress[];
  selectionChanged: boolean;
  generateError?: string;
  lastSelectionKey: string;
  currentRunId: string | null;
  abortController: AbortController | null;
  historyLoading: boolean;
  historyError?: string;
  history: Partial<Record<Framework, CodegenHistoryEntry[]>>;
  selectedHistoryId: Partial<Record<Framework, string>>;

  setActiveTab: (tab: Framework) => void;
  clearGenerateError: () => void;
  setSelectionChanged: (changed: boolean) => void;
  startGeneration: (selectionKey: string, abortController: AbortController) => string;
  applyProgress: (runId: string, framework: Framework, event: CodeGenProgress) => void;
  completeGeneration: (runId: string, framework: Framework, bundle: GeneratedCodeBundle) => void;
  failGeneration: (runId: string, message: string) => void;
  cancelGeneration: () => void;
  loadHistory: (framework: Framework, target: CodegenTarget) => Promise<void>;
  selectHistoryEntry: (framework: Framework, generationId: string) => void;
  promoteHistoryEntry: (framework: Framework, generationId: string) => Promise<void>;
  deleteHistoryEntry: (framework: Framework, generationId: string) => Promise<void>;
  saveHistory: (
    framework: Framework,
    target: CodegenTarget,
    bundle: GeneratedCodeBundle,
    model?: string,
    provider?: string,
  ) => Promise<void>;
  reset: () => void;
}

const initialGenerationState = {
  activeTab: 'react' as Framework,
  codeCache: {},
  isGenerating: false,
  planningStatus: 'idle' as CodegenStepStatus,
  planningError: undefined,
  assemblyStatus: 'idle' as CodegenStepStatus,
  chunks: [],
  selectionChanged: false,
  generateError: undefined,
  lastSelectionKey: '',
  currentRunId: null,
  abortController: null,
  historyLoading: false,
  historyError: undefined,
  history: {},
  selectedHistoryId: {},
};

let nextRunId = 0;

function createRunId(): string {
  nextRunId += 1;
  return `codegen-${nextRunId}`;
}

function upsertChunkProgress(
  chunks: CodegenChunkProgress[],
  entry: CodegenChunkProgress,
): CodegenChunkProgress[] {
  const existing = chunks.findIndex((chunk) => chunk.chunkId === entry.chunkId);
  if (existing < 0) return [...chunks, entry];

  const next = [...chunks];
  next[existing] = entry;
  return next;
}

function mapHistoryEntry(entry: CloudCodeGeneration): CodegenHistoryEntry {
  return {
    id: entry.id,
    createdAt: entry.createdAt,
    completedAt: entry.completedAt,
    status: entry.status,
    degraded: entry.degraded,
    finalCode: entry.finalCode,
    model: entry.model,
    provider: entry.provider,
    error: entry.error,
    documentRevision: entry.documentRevision,
    assetsManifest: entry.assetsManifest,
    metadata: entry.metadata,
    pageId: entry.pageId,
    targetKind: entry.targetKind,
    nodeIds: entry.nodeIds,
    targetHash: entry.targetHash,
    entryFile: entry.entryFile,
    promotedAt: entry.promotedAt,
  };
}

export const useCodegenStore = create<CodegenState>((set, get) => ({
  ...initialGenerationState,

  setActiveTab: (activeTab) =>
    set((state) => (state.activeTab === activeTab ? state : { activeTab })),
  clearGenerateError: () =>
    set((state) => (state.generateError === undefined ? state : { generateError: undefined })),
  setSelectionChanged: (selectionChanged) =>
    set((state) => (state.selectionChanged === selectionChanged ? state : { selectionChanged })),

  startGeneration: (lastSelectionKey, abortController) => {
    const previousController = get().abortController;
    if (previousController && previousController !== abortController) {
      previousController.abort();
    }

    const currentRunId = createRunId();
    set({
      currentRunId,
      abortController,
      isGenerating: true,
      planningStatus: 'idle',
      planningError: undefined,
      assemblyStatus: 'idle',
      chunks: [],
      selectionChanged: false,
      generateError: undefined,
      lastSelectionKey,
    });
    return currentRunId;
  },

  applyProgress: (runId, framework, event) =>
    set((state) => {
      if (state.currentRunId !== runId) return {};

      switch (event.step) {
        case 'planning':
          return {
            planningStatus: event.status,
            planningError: event.error ?? state.planningError,
          };
        case 'chunk':
          return {
            chunks: upsertChunkProgress(state.chunks, {
              chunkId: event.chunkId,
              name: event.name,
              status: event.status,
              error: event.error,
            }),
          };
        case 'assembly':
          return { assemblyStatus: event.status };
        case 'complete':
          return {
            isGenerating: false,
            codeCache: {
              ...state.codeCache,
              [framework]: {
                code: event.finalCode,
                degraded: event.degraded,
                assets: [],
                files: buildCodegenFiles({ framework, code: event.finalCode }),
              },
            },
          };
        case 'error':
          return {
            isGenerating: false,
            generateError: event.message,
            abortController: null,
            currentRunId: null,
          };
      }
    }),

  completeGeneration: (runId, framework, bundle) =>
    set((state) => {
      if (state.currentRunId !== runId) return {};
      return {
        isGenerating: false,
        abortController: null,
        currentRunId: null,
        generateError: undefined,
        codeCache: {
          ...state.codeCache,
          [framework]: {
            ...bundle,
            files: bundle.files ?? buildCodegenFiles({ framework, code: bundle.code }),
          },
        },
      };
    }),

  failGeneration: (runId, generateError) =>
    set((state) => {
      if (state.currentRunId !== runId) return {};
      return {
        isGenerating: false,
        abortController: null,
        currentRunId: null,
        generateError,
      };
    }),

  cancelGeneration: () => {
    get().abortController?.abort();
    set({
      isGenerating: false,
      abortController: null,
      currentRunId: null,
    });
  },

  loadHistory: async (framework, target) => {
    const { cloudFileId } = useDocumentStore.getState();
    if (!cloudFileId) return;
    set({ historyLoading: true, historyError: undefined });
    try {
      const history = await listCodeGenerationHistory({
        fileId: cloudFileId,
        framework,
        target,
      });
      const latest = history[0];
      set((state) => {
        const codeCache = { ...state.codeCache };
        if (latest?.finalCode) {
          codeCache[framework] = {
            code: latest.finalCode,
            degraded: latest.degraded,
            assets: [],
            historyId: latest.id,
            status: latest.status,
            assetsManifest: latest.assetsManifest,
            metadata: latest.metadata,
            files: buildCodegenFiles({ framework, code: latest.finalCode }),
          };
        } else {
          delete codeCache[framework];
        }
        const entries = history.map(mapHistoryEntry);

        return {
          historyLoading: false,
          history: {
            ...state.history,
            [framework]: entries,
          },
          selectedHistoryId: {
            ...state.selectedHistoryId,
            [framework]: latest?.id,
          },
          codeCache,
        };
      });
    } catch (err) {
      set({
        historyLoading: false,
        historyError: err instanceof Error ? err.message : 'Failed to load code history',
      });
    }
  },

  selectHistoryEntry: (framework, generationId) =>
    set((state) => {
      const entry = state.history[framework]?.find((item) => item.id === generationId);
      if (!entry) return state;
      const codeCache = { ...state.codeCache };
      if (entry.finalCode) {
        codeCache[framework] = {
          code: entry.finalCode,
          degraded: entry.degraded,
          assets: [],
          historyId: entry.id,
          status: entry.status,
          assetsManifest: entry.assetsManifest,
          metadata: entry.metadata,
          files: buildCodegenFiles({ framework, code: entry.finalCode }),
        };
      } else {
        delete codeCache[framework];
      }

      return {
        codeCache,
        selectedHistoryId: {
          ...state.selectedHistoryId,
          [framework]: entry.id,
        },
      };
    }),

  promoteHistoryEntry: async (framework, generationId) => {
    try {
      const promoted = await promoteCodeGenerationHistory(generationId);
      const promotedEntry = mapHistoryEntry(promoted);
      set((state) => ({
        historyError: undefined,
        history: {
          ...state.history,
          [framework]: (state.history[framework] ?? []).map((entry) =>
            entry.id === generationId
              ? { ...entry, ...promotedEntry }
              : { ...entry, promotedAt: null },
          ),
        },
      }));
    } catch (err) {
      set({
        historyError: err instanceof Error ? err.message : 'Failed to promote code history',
      });
    }
  },

  deleteHistoryEntry: async (framework, generationId) => {
    try {
      await deleteCodeGenerationHistory(generationId);
      set((state) => {
        const nextHistory = (state.history[framework] ?? []).filter(
          (entry) => entry.id !== generationId,
        );
        const currentSelectedId = state.selectedHistoryId[framework];
        const nextSelected =
          currentSelectedId === generationId
            ? nextHistory.find((entry) => entry.finalCode)
            : nextHistory.find((entry) => entry.id === currentSelectedId);
        const codeCache = { ...state.codeCache };
        if (currentSelectedId === generationId) {
          if (nextSelected?.finalCode) {
            codeCache[framework] = {
              code: nextSelected.finalCode,
              degraded: nextSelected.degraded,
              assets: [],
              historyId: nextSelected.id,
              status: nextSelected.status,
              assetsManifest: nextSelected.assetsManifest,
              metadata: nextSelected.metadata,
              files: buildCodegenFiles({ framework, code: nextSelected.finalCode }),
            };
          } else {
            delete codeCache[framework];
          }
        }

        return {
          historyError: undefined,
          history: {
            ...state.history,
            [framework]: nextHistory,
          },
          selectedHistoryId: {
            ...state.selectedHistoryId,
            [framework]: nextSelected?.id,
          },
          codeCache,
        };
      });
    } catch (err) {
      set({
        historyError: err instanceof Error ? err.message : 'Failed to delete code history',
      });
    }
  },

  saveHistory: async (framework, target, bundle, model, provider) => {
    const { cloudFileId, cloudRevision } = useDocumentStore.getState();
    if (!cloudFileId || !cloudRevision) return;
    try {
      const saved = await saveCodeGenerationHistory({
        fileId: cloudFileId,
        framework,
        ...target,
        documentRevision: cloudRevision,
        status: bundle.degraded ? 'degraded' : 'done',
        finalCode: bundle.code,
        degraded: bundle.degraded,
        assets: bundle.assets,
        files: bundle.files ?? buildCodegenFiles({ framework, code: bundle.code }),
        model,
        provider,
        chunks: get().chunks.map((chunk, index) => ({
          chunkId: chunk.chunkId,
          name: chunk.name,
          status: chunk.status,
          error: chunk.error,
          orderIndex: index,
        })),
      });
      const entry = mapHistoryEntry(saved);
      set((state) => ({
        history: {
          ...state.history,
          [framework]: [entry, ...(state.history[framework] ?? [])],
        },
        selectedHistoryId: {
          ...state.selectedHistoryId,
          [framework]: saved.id,
        },
        codeCache: {
          ...state.codeCache,
          [framework]: {
            ...bundle,
            historyId: saved.id,
            status: saved.status,
            assetsManifest: saved.assetsManifest,
            metadata: saved.metadata,
            files:
              saved.files ??
              bundle.files ??
              buildCodegenFiles({ framework, code: saved.finalCode ?? bundle.code }),
          },
        },
      }));
    } catch (err) {
      set({ historyError: err instanceof Error ? err.message : 'Failed to save code history' });
    }
  },

  reset: () => {
    get().abortController?.abort();
    set({
      ...initialGenerationState,
      codeCache: {},
      history: {},
      selectedHistoryId: {},
    });
  },
}));
