import { useState, useRef, useCallback, useEffect, useMemo, memo } from 'react';
import {
  Copy,
  Download,
  FileJson,
  RefreshCw,
  Sparkles,
  Check,
  AlertTriangle,
  WandSparkles,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import CodeAssetPreviewStrip from './code-asset-preview-strip';
import CodeHistoryList from './code-history-list';
import CodeFileTree from './code-file-tree';
import CodeLocalOutputActions from './code-local-output-actions';
import {
  getFileHighlightLanguage,
  HIGHLIGHT_LANG,
  TAB_LABELS,
  triggerDownload,
} from './code-panel-utils';
import { resolveCodegenModelConfig } from './codegen-model-config';
import { getPreviewReasonKey, openPreviewHtml } from './code-preview-window';
import ProgressItem from './code-progress-item';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import { useAIStore } from '@/stores/ai-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { useCodegenStore, type CodegenHistoryEntry } from '@/stores/codegen-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import { generateCode } from '@/services/ai/code-generation-pipeline';
import { buildCodegenBundleManifest } from '@/services/ai/codegen-assets';
import { buildCodegenFiles, getDefaultCodegenFilePath } from '@/services/ai/codegen-files';
import { buildCodegenTarget, exportCodeGenerationZip, getCodeGenerationHistoryDetail } from '@/services/cloud/codegen-history';
import { buildCodePreviewDocument } from '@/services/cloud/codegen-preview';
import { buildAIStructureBundle, encodeAIStructureBundleZip } from '@/services/ai/structure-bundle';
import { highlightCode } from '@/utils/syntax-highlight';
import type { Framework, CodeGenProgress } from '@zseven-w/pen-types';
import { FRAMEWORKS } from '@zseven-w/pen-types';
import type { PenNode } from '@/types/pen';
import { encode as encodeZip } from 'uzip';
import { useTranslation } from 'react-i18next';

type PanelState = 'empty' | 'generating' | 'complete';
export type CodePanelView = 'code' | 'history';

const EMPTY_HISTORY: CodegenHistoryEntry[] = [];

type CodePanelProps = { generationId?: string };

function CodePanelInner({ generationId }: CodePanelProps) {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [panelView, setPanelView] = useState<CodePanelView>('code');
  const [selectedFilePath, setSelectedFilePath] = useState<string | null>(null);
  const [patchOpen, setPatchOpen] = useState(false);
  const [patchInstruction, setPatchInstruction] = useState('');
  const activeTab = useCodegenStore((s) => s.activeTab);
  const codeCache = useCodegenStore((s) => s.codeCache);
  const isGenerating = useCodegenStore((s) => s.isGenerating);
  const planningStatus = useCodegenStore((s) => s.planningStatus);
  const planningError = useCodegenStore((s) => s.planningError);
  const assemblyStatus = useCodegenStore((s) => s.assemblyStatus);
  const chunks = useCodegenStore((s) => s.chunks);
  const selectionChanged = useCodegenStore((s) => s.selectionChanged);
  const generateError = useCodegenStore((s) => s.generateError);
  const lastSelectionKey = useCodegenStore((s) => s.lastSelectionKey);
  const setActiveTab = useCodegenStore((s) => s.setActiveTab);
  const clearGenerateError = useCodegenStore((s) => s.clearGenerateError);
  const setSelectionChanged = useCodegenStore((s) => s.setSelectionChanged);
  const startGeneration = useCodegenStore((s) => s.startGeneration);
  const applyProgress = useCodegenStore((s) => s.applyProgress);
  const completeGeneration = useCodegenStore((s) => s.completeGeneration);
  const failGeneration = useCodegenStore((s) => s.failGeneration);
  const cancelGeneration = useCodegenStore((s) => s.cancelGeneration);
  const loadHistory = useCodegenStore((s) => s.loadHistory);
  const saveHistory = useCodegenStore((s) => s.saveHistory);
  const selectHistoryEntry = useCodegenStore((s) => s.selectHistoryEntry);
  const promoteHistoryEntry = useCodegenStore((s) => s.promoteHistoryEntry);
  const deleteHistoryEntry = useCodegenStore((s) => s.deleteHistoryEntry);
  const historyLoading = useCodegenStore((s) => s.historyLoading);
  const historyError = useCodegenStore((s) => s.historyError);
  const history = useCodegenStore((s) => s.history[activeTab] ?? EMPTY_HISTORY);
  const selectedHistoryId = useCodegenStore((s) => s.selectedHistoryId[activeTab]);
  const createBackgroundJob = useCodegenJobStore((s) => s.createJob);
  const jobLoading = useCodegenJobStore((s) => s.loading);
  const jobError = useCodegenJobStore((s) => s.error);
  const lastCreatedJobId = useCodegenJobStore((s) => s.lastCreatedJobId);

  const cached = codeCache[activeTab];
  const generatedCode = cached?.code ?? '';
  const isDegraded = cached?.degraded ?? false;
  const exportedAssets = cached?.assets ?? [];
  const exportedAssetsManifest = cached?.assetsManifest ?? [];
  const hasExportedAssets = exportedAssets.length > 0;
  const requiresZipBundle = activeTab === 'uniapp' || hasExportedAssets;
  const panelState: PanelState = isGenerating ? 'generating' : cached ? 'complete' : 'empty';

  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>(null);

  const selectedIds = useCanvasStore((s) => s.selection.selectedIds);
  const activePageId = useCanvasStore((s) => s.activePageId);
  const getNodeById = useDocumentStore((s) => s.getNodeById);
  const children = useDocumentStore((s) => getActivePageChildren(s.document, activePageId));
  const document = useDocumentStore((s) => s.document);
  const cloudFileId = useDocumentStore((s) => s.cloudFileId);
  const cloudRevision = useDocumentStore((s) => s.cloudRevision);
  const variables = document?.variables;
  const model = useAIStore((s) => s.model);
  const modelGroups = useAIStore((s) => s.modelGroups);
  const builtinProviders = useAgentSettingsStore((s) => s.builtinProviders);

  const selectionKey = selectedIds.join(',');
  const modelConfig = useMemo(
    () => resolveCodegenModelConfig(model, modelGroups, builtinProviders),
    [builtinProviders, model, modelGroups],
  );
  const modelUnavailable = !modelConfig;
  const codegenTarget = useMemo(
    () => buildCodegenTarget({ pageId: activePageId, selectedIds }),
    [activePageId, selectedIds],
  );
  const generatedFiles = useMemo(
    () =>
      cached?.files ??
      (generatedCode ? buildCodegenFiles({ framework: activeTab, code: generatedCode }) : []),
    [activeTab, cached?.files, generatedCode],
  );
  const selectedFile = useMemo(
    () => generatedFiles.find((file) => file.path === selectedFilePath) ?? generatedFiles[0],
    [generatedFiles, selectedFilePath],
  );
  const displayedCode = selectedFile?.content ?? generatedCode;
  const nodeCount = selectedIds.length > 0 ? selectedIds.length : children.length;
  const activeHistoryId = cached?.historyId ?? selectedHistoryId ?? null;
  const canQueueBackgroundJob =
    nodeCount > 0 && !modelUnavailable && !!cloudFileId && !!cloudRevision && !jobLoading;
  const targetNodes = useMemo(() => {
    if (selectedIds.length > 0) {
      return selectedIds.map((id) => getNodeById(id)).filter((n): n is PenNode => n !== undefined);
    }
    return children;
  }, [children, getNodeById, selectedIds]);
  const patchTargetLabel = useMemo(() => {
    if (targetNodes.length === 0) return t('codePanel.patch.noSelection');
    if (targetNodes.length === 1) return targetNodes[0]?.name || targetNodes[0]?.id;
    return t('codePanel.patch.multipleSelection', { count: targetNodes.length });
  }, [targetNodes, t]);
  const activeHistoryShortId = activeHistoryId ? activeHistoryId.slice(0, 8) : null;

  useEffect(() => {
    if (generationId) return;
    void loadHistory(activeTab, codegenTarget);
  }, [activeTab, codegenTarget, generationId, loadHistory]);

  useEffect(() => {
    if (!generationId) return;

    let cancelled = false;
    void (async () => {
      try {
        const detail = await getCodeGenerationHistoryDetail(generationId);
        if (cancelled) return;
        const framework = detail.framework;
        setPanelView('history');
        setSelectedFilePath(null);
        setActiveTab(framework);
        await loadHistory(framework, {
          pageId: detail.pageId,
          targetKind: detail.targetKind,
          nodeIds: detail.nodeIds,
          targetHash: detail.targetHash,
        });
        if (cancelled) return;
        selectHistoryEntry(framework, generationId);
        setPanelView('history');
        setSelectedFilePath(null);
      } catch (err) {
        console.error('[CodePanel] failed to open generation history', err);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [generationId, loadHistory, selectHistoryEntry, setActiveTab]);

  useEffect(() => {
    if (generatedFiles.length === 0) {
      setSelectedFilePath(null);
      return;
    }
    if (!selectedFilePath || !generatedFiles.some((file) => file.path === selectedFilePath)) {
      setSelectedFilePath(generatedFiles[0].path);
    }
  }, [generatedFiles, selectedFilePath]);

  useEffect(() => {
    if (panelState === 'complete') {
      setSelectionChanged(selectionKey !== lastSelectionKey);
    }
  }, [lastSelectionKey, panelState, selectionKey, setSelectionChanged]);

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    };
  }, []);

  const getTargetNodes = useCallback((): PenNode[] => {
    return targetNodes;
  }, [targetNodes]);

  const handleGenerate = useCallback(async () => {
    const nodes = getTargetNodes();
    if (nodes.length === 0 || !modelConfig) return;

    const abortController = new AbortController();
    const runId = startGeneration(selectionKey, abortController);
    const generationFramework = activeTab;

    const handleProgress = (event: CodeGenProgress) => {
      applyProgress(runId, generationFramework, event);
    };

    try {
      const result = await generateCode(
        nodes,
        generationFramework,
        variables,
        handleProgress,
        modelConfig?.model ?? model,
        modelConfig?.provider,
        abortController.signal,
      );
      const bundle = {
        code: result.code,
        degraded: result.degraded,
        assets: result.assets,
        files: buildCodegenFiles({ framework: generationFramework, code: result.code }),
      };
      completeGeneration(runId, generationFramework, bundle);
      void saveHistory(
        generationFramework,
        codegenTarget,
        bundle,
        modelConfig?.model ?? model,
        modelConfig?.provider,
      );
      setPanelView('code');
    } catch (err) {
      if (!abortController.signal.aborted) {
        const msg = err instanceof Error ? err.message : t('codePanel.error.generationFailed');
        failGeneration(runId, msg);
      }
    }
  }, [
    activeTab,
    applyProgress,
    completeGeneration,
    failGeneration,
    getTargetNodes,
    model,
    modelConfig,
    codegenTarget,
    saveHistory,
    selectionKey,
    startGeneration,
    t,
    variables,
  ]);

  const handleCreateBackgroundJob = useCallback(async () => {
    const nodes = getTargetNodes();
    if (nodes.length === 0 || !modelConfig || !cloudFileId || !cloudRevision) return;
    await createBackgroundJob({
      jobKind: 'full_generation',
      fileId: cloudFileId,
      framework: activeTab,
      ...codegenTarget,
      documentRevision: cloudRevision,
      model: modelConfig.model ?? model,
      provider: modelConfig.provider,
      nodes,
      variables: variables as Record<string, unknown> | undefined,
    });
  }, [
    activeTab,
    cloudFileId,
    cloudRevision,
    codegenTarget,
    createBackgroundJob,
    getTargetNodes,
    model,
    modelConfig,
    variables,
  ]);

  const handleCreatePatchJob = useCallback(async () => {
    const nodes = getTargetNodes();
    const instruction = patchInstruction.trim();
    if (
      nodes.length === 0 ||
      !modelConfig ||
      !cloudFileId ||
      !cloudRevision ||
      !activeHistoryId ||
      !instruction
    ) {
      return;
    }
    const job = await createBackgroundJob({
      jobKind: 'patch_generation',
      fileId: cloudFileId,
      framework: activeTab,
      ...codegenTarget,
      documentRevision: cloudRevision,
      model: modelConfig.model ?? model,
      provider: modelConfig.provider,
      nodes,
      variables: variables as Record<string, unknown> | undefined,
      baseGenerationId: activeHistoryId,
      patchInstruction: instruction,
    });
    if (job) {
      setPatchInstruction('');
      setPatchOpen(false);
    }
  }, [
    activeHistoryId,
    activeTab,
    cloudFileId,
    cloudRevision,
    codegenTarget,
    createBackgroundJob,
    getTargetNodes,
    model,
    modelConfig,
    patchInstruction,
    variables,
  ]);

  const handleCancel = useCallback(() => {
    cancelGeneration();
  }, [cancelGeneration]);

  const handleRetryChunk = useCallback(
    (_chunkId: string) => {
      void handleGenerate();
    },
    [handleGenerate],
  );

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(displayedCode);
    setCopied(true);
    if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000);
  }, [displayedCode]);

  const handleDownload = useCallback(() => {
    const codeFileName = getDefaultCodegenFilePath(activeTab);
    const assets = exportedAssets;
    const encoder = new TextEncoder();
    const codeBytes = encoder.encode(generatedCode);

    void (async () => {
      if (requiresZipBundle) {
        const codeFiles = activeTab === 'uniapp' ? generatedFiles : [];
        const manifestFileName =
          activeTab === 'uniapp' ? 'openpencil-codegen-manifest.json' : 'manifest.json';
        const zipEntries: Record<string, Uint8Array> = {
          [codeFileName]: codeBytes,
          ...Object.fromEntries(codeFiles.map((file) => [file.path, encoder.encode(file.content)])),
          [manifestFileName]: encoder.encode(
            JSON.stringify(
              await buildCodegenBundleManifest({
                framework: activeTab,
                codeFile: codeFileName,
                codeBytes,
                assets,
              }),
              null,
              2,
            ),
          ),
          ...Object.fromEntries(assets.map((asset) => [asset.zipPath, asset.bytes])),
        };

        triggerDownload(
          new Blob([encodeZip(zipEntries)], { type: 'application/zip' }),
          `design-${activeTab}.zip`,
        );
        return;
      }

      triggerDownload(
        new Blob([generatedCode], { type: 'text/plain;charset=utf-8' }),
        codeFileName,
      );
    })();
  }, [generatedCode, activeTab, exportedAssets, requiresZipBundle, generatedFiles]);

  const handleDownloadStructureBundle = useCallback(() => {
    const nodes = getTargetNodes();
    if (nodes.length === 0) return;

    void (async () => {
      const bundle = await buildAIStructureBundle({
        nodes,
        activePageId,
        selectedIds,
      });

      const blob = new Blob([encodeAIStructureBundleZip(bundle.zipEntries)], {
        type: 'application/zip',
      });

      triggerDownload(blob, bundle.fileName);
    })();
  }, [getTargetNodes, activePageId, selectedIds]);

  const handleTabChange = useCallback(
    (tab: Framework) => {
      setActiveTab(tab);
      clearGenerateError();
      setPanelView('code');
      setSelectedFilePath(null);
    },
    [clearGenerateError, setActiveTab],
  );

  const handleSelectHistory = useCallback(
    (generationId: string) => {
      selectHistoryEntry(activeTab, generationId);
      setPanelView('code');
      setSelectedFilePath(null);
    },
    [activeTab, selectHistoryEntry],
  );

  const handlePreviewHistory = useCallback(
    (entry: CodegenHistoryEntry) => {
      const currentAssets = cached?.historyId === entry.id ? cached.assets : [];
      const preview = buildCodePreviewDocument({
        framework: activeTab,
        code: entry.finalCode ?? '',
        assets: currentAssets ?? [],
        assetsManifest: entry.assetsManifest,
      });

      if (preview.kind === 'unsupported') {
        const reason = t(getPreviewReasonKey(preview.reason));
        window.alert(`${t('codePanel.preview.unavailable')}\n${reason}`);
        return;
      }

      if (!openPreviewHtml(preview.srcDoc)) {
        window.alert(t('codePanel.preview.popupBlocked'));
      }
    },
    [activeTab, cached, t],
  );

  const handlePromoteHistory = useCallback(
    (generationId: string) => {
      void promoteHistoryEntry(activeTab, generationId);
    },
    [activeTab, promoteHistoryEntry],
  );

  const handleDownloadHistory = useCallback((generationId: string) => {
    void (async () => {
      const result = await exportCodeGenerationZip(generationId);
      triggerDownload(result.blob, result.fileName);
    })();
  }, []);

  const handleDeleteHistory = useCallback(
    (generationId: string) => {
      if (!window.confirm(t('codePanel.history.deleteConfirm'))) return;
      void deleteHistoryEntry(activeTab, generationId);
    },
    [activeTab, deleteHistoryEntry, t],
  );

  const highlightedHTML = useMemo(() => {
    if (!displayedCode) return '';
    const lang = selectedFile
      ? getFileHighlightLanguage(selectedFile, HIGHLIGHT_LANG[activeTab])
      : HIGHLIGHT_LANG[activeTab];

    if (
      lang === 'html' &&
      (activeTab === 'html' ||
        activeTab === 'vue' ||
        activeTab === 'svelte' ||
        activeTab === 'uniapp')
    ) {
      const styleIdx = displayedCode.indexOf('<style');
      if (styleIdx !== -1) {
        const templatePart = displayedCode.slice(0, styleIdx);
        const stylePart = displayedCode.slice(styleIdx);
        const styleTagEnd = stylePart.indexOf('>\n');
        if (styleTagEnd !== -1) {
          const styleTag = stylePart.slice(0, styleTagEnd + 1);
          const styleBody = stylePart.slice(styleTagEnd + 1);
          const closingIdx = styleBody.lastIndexOf('</style>');
          if (closingIdx !== -1) {
            const cssContent = styleBody.slice(0, closingIdx);
            const closingTag = styleBody.slice(closingIdx);
            return (
              highlightCode(templatePart, 'html') +
              highlightCode(styleTag, 'html') +
              '\n' +
              highlightCode(cssContent, 'css') +
              highlightCode(closingTag, 'html')
            );
          }
        }
        return highlightCode(templatePart, 'html') + highlightCode(stylePart, 'css');
      }
    }

    return highlightCode(displayedCode, lang);
  }, [activeTab, displayedCode, selectedFile]);

  const totalSteps = 1 + chunks.length + (assemblyStatus !== 'idle' ? 1 : 0);
  const completedSteps =
    (planningStatus === 'done' ? 1 : 0) +
    chunks.filter((c) => c.status === 'done' || c.status === 'degraded').length +
    (assemblyStatus === 'done' ? 1 : 0);
  const progressPct = totalSteps > 0 ? (completedSteps / totalSteps) * 100 : 0;

  return (
    <div className="flex flex-1 min-h-0 flex-col">
      <div className="flex items-center border-b border-border px-1.5 shrink-0">
        <div className="flex gap-0.5 overflow-x-auto py-1 scrollbar-none">
          {FRAMEWORKS.map((fw) => (
            <button
              key={fw}
              type="button"
              className={cn(
                'whitespace-nowrap rounded px-2 py-1 text-[11px] font-medium transition-all duration-150 shrink-0',
                activeTab === fw
                  ? 'bg-primary text-primary-foreground shadow-sm'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted/60',
              )}
              onClick={() => handleTabChange(fw)}
            >
              {TAB_LABELS[fw]}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 min-h-0 flex flex-col">
        {panelState === 'empty' && (
          <div className="flex h-full flex-col items-center justify-center gap-4 p-6 text-center">
            <div className="relative">
              <div className="absolute inset-0 rounded-xl bg-primary/10 blur-xl" />
              <div className="relative rounded-xl border border-border/50 bg-muted/40 p-4">
                <Sparkles className="h-6 w-6 text-primary/70" />
              </div>
            </div>
            <div className="space-y-1">
              <div className="text-xs font-medium text-foreground/80">
                {nodeCount > 0
                  ? t('codePanel.empty.nodeCount', { count: nodeCount })
                  : t('codePanel.empty.noNodes')}
              </div>
              <div className="text-[11px] text-muted-foreground">
                {t('codePanel.empty.description')}
              </div>
            </div>
            <Button
              onClick={handleGenerate}
              disabled={nodeCount === 0 || modelUnavailable}
              size="sm"
              className="h-8 gap-1.5 text-xs shadow-sm"
            >
              <Sparkles className="h-3.5 w-3.5" />
              {t('codePanel.generate', { framework: TAB_LABELS[activeTab] })}
            </Button>
            <Button
              variant="outline"
              onClick={() => void handleCreateBackgroundJob()}
              disabled={!canQueueBackgroundJob}
              size="sm"
              className="h-8 gap-1.5 text-xs"
            >
              <RefreshCw className="h-3.5 w-3.5" />
              {jobLoading ? t('codePanel.background.creating') : t('codePanel.background.create')}
            </Button>
            <Button
              variant="ghost"
              onClick={() => void handleDownloadStructureBundle()}
              disabled={nodeCount === 0}
              size="sm"
              className="h-8 gap-1.5 text-xs"
            >
              <FileJson className="h-3.5 w-3.5" />
              {t('codePanel.exportAiBundle')}
            </Button>
            {lastCreatedJobId && (
              <div className="max-w-[260px] rounded-lg border border-border bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
                {t('codePanel.background.queued')}
              </div>
            )}
            {jobError && (
              <div className="max-w-[260px] rounded-lg border border-destructive/20 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                {jobError}
              </div>
            )}
            {generateError && (
              <div className="max-w-[260px] rounded-lg border border-destructive/20 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                <div className="font-medium">{t('codePanel.generationFailed')}</div>
                <div className="mt-1 break-words opacity-80">{generateError}</div>
              </div>
            )}
            {modelUnavailable && (
              <div className="max-w-[260px] rounded-lg border border-border bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
                {t('codePanel.modelUnavailable')}
              </div>
            )}
            {historyLoading && (
              <div className="text-[11px] text-muted-foreground">{t('codePanel.history.loading')}</div>
            )}
            {historyError && (
              <div className="max-w-[260px] rounded-lg border border-destructive/20 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                {historyError}
              </div>
            )}
            {selectionChanged && (
              <div className="flex items-center gap-1.5 text-[11px] text-amber-500">
                <AlertTriangle className="h-3 w-3" />
                {t('codePanel.selectionChangedSinceLastGeneration')}
              </div>
            )}
          </div>
        )}

        {panelState === 'generating' && (
          <div className="flex flex-1 flex-col">
            <div className="h-[2px] shrink-0 bg-muted">
              <div
                className="h-full bg-primary transition-all duration-500 ease-out"
                style={{ width: `${progressPct}%` }}
              />
            </div>

            <div className="flex flex-col gap-1 p-3">
              <ProgressItem
                label={t('codePanel.step.planning')}
                status={
                  planningStatus === 'running'
                    ? 'running'
                    : planningStatus === 'done'
                      ? 'done'
                      : planningStatus === 'failed'
                        ? 'failed'
                        : 'pending'
                }
                error={planningError}
              />

              {chunks.map((chunk) => (
                <ProgressItem
                  key={chunk.chunkId}
                  label={chunk.name}
                  status={chunk.status}
                  error={chunk.error}
                  onRetry={
                    chunk.status === 'failed' ? () => handleRetryChunk(chunk.chunkId) : undefined
                  }
                />
              ))}

              {assemblyStatus !== 'idle' && (
                <ProgressItem
                  label={t('codePanel.step.assembly')}
                  status={
                    assemblyStatus === 'running'
                      ? 'running'
                      : assemblyStatus === 'done'
                        ? 'done'
                        : 'failed'
                  }
                />
              )}
            </div>

            <div className="mt-auto border-t border-border/50 p-2 shrink-0">
              <button
                type="button"
                onClick={handleCancel}
                className="w-full rounded-md py-1.5 text-[11px] font-medium text-muted-foreground transition-colors hover:text-foreground hover:bg-muted/60"
              >
                {t('common.cancel')}
              </button>
            </div>
          </div>
        )}

        {panelState === 'complete' && (
          <>
            {isDegraded && (
              <div className="flex items-center gap-2 border-b border-amber-500/20 bg-amber-500/8 px-3 py-1.5 text-[11px] text-amber-600 shrink-0">
                <AlertTriangle className="h-3 w-3 shrink-0" />
                {t('codePanel.degradedWarning')}
              </div>
            )}
            {selectionChanged && (
              <div className="flex items-center justify-between border-b border-border/50 bg-muted/40 px-3 py-1 text-[11px] text-muted-foreground shrink-0">
                <span className="flex items-center gap-1.5">
                  <AlertTriangle className="h-3 w-3 text-amber-500" />
                  {t('codePanel.selectionChanged')}
                </span>
                <button
                  type="button"
                  className="text-[11px] font-medium text-primary hover:text-primary/80 transition-colors"
                  onClick={handleGenerate}
                >
                  {t('codePanel.regenerate')}
                </button>
              </div>
            )}
            {requiresZipBundle && (
              <div className="border-b border-border/50 bg-muted/40 px-3 py-2 shrink-0">
                <div className="flex items-center gap-2 text-xs font-medium text-foreground">
                  <Download className="h-3.5 w-3.5 shrink-0" />
                  {activeTab === 'uniapp'
                    ? t('codePanel.bundle.uniappTitle')
                    : t('codePanel.bundle.assetsTitle', { count: exportedAssets.length })}
                </div>
                <div className="mt-1 text-[11px] text-muted-foreground">
                  {activeTab === 'uniapp'
                    ? t('codePanel.bundle.uniappDescription')
                    : t('codePanel.bundle.assetsDescription')}
                </div>
              </div>
            )}
            {history.length > 0 && (
              <div className="border-b border-border/50 bg-muted/30 px-3 py-1.5 text-[11px] text-muted-foreground shrink-0">
                {t('codePanel.history.count', { count: history.length })}
              </div>
            )}
            <div className="border-b border-border/50 bg-card px-1.5 py-1 shrink-0">
              <div className="grid grid-cols-2 rounded-md bg-muted/35 p-0.5">
                {(['code', 'history'] as CodePanelView[]).map((view) => (
                  <button
                    key={view}
                    type="button"
                    onClick={() => setPanelView(view)}
                    className={cn(
                      'rounded px-2 py-1 text-[11px] font-medium capitalize transition-colors',
                      panelView === view
                        ? 'bg-background text-foreground shadow-sm'
                        : 'text-muted-foreground hover:text-foreground',
                    )}
                  >
                    {t(`codePanel.view.${view}`)}
                    {view === 'history' && history.length > 0 ? ` ${history.length}` : ''}
                  </button>
                ))}
              </div>
            </div>
            <div className="flex-1 min-h-0">
              {panelView === 'code' && (
                <div className="flex h-full min-h-0 flex-col">
                  <CodeAssetPreviewStrip
                    assets={exportedAssets}
                    assetsManifest={exportedAssetsManifest}
                  />
                  <CodeLocalOutputActions framework={activeTab} files={generatedFiles} />
                  <CodeFileTree
                    files={generatedFiles}
                    selectedPath={selectedFile?.path ?? null}
                    onSelect={setSelectedFilePath}
                  />
                  <div className="min-h-0 flex-1 overflow-auto p-2">
                    <pre className="text-[10px] leading-relaxed font-mono text-foreground/80 whitespace-pre-wrap break-all">
                      <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
                    </pre>
                  </div>
                </div>
              )}
              {panelView === 'history' && (
                <CodeHistoryList
                  history={history}
                  selectedId={selectedHistoryId}
                  onSelect={handleSelectHistory}
                  onPreview={handlePreviewHistory}
                  onPromote={handlePromoteHistory}
                  onDownload={handleDownloadHistory}
                  onDelete={handleDeleteHistory}
                />
              )}
            </div>
            {panelView !== 'history' && (
              <>
              {patchOpen && (
                <div className="border-t border-border bg-card p-2">
                  <div className="mb-2 rounded-md border border-border bg-muted/25 p-2 text-[11px]">
                    <div className="grid gap-1.5">
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-muted-foreground">{t('codePanel.patch.selectedLayer')}</span>
                        <span className="min-w-0 truncate font-medium text-foreground">
                          {patchTargetLabel}
                        </span>
                      </div>
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-muted-foreground">{t('codePanel.patch.baseGeneration')}</span>
                        <span className="font-mono text-[10px] text-foreground">
                          {activeHistoryShortId ?? '-'}
                        </span>
                      </div>
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-muted-foreground">{t('codePanel.patch.framework')}</span>
                        <span className="font-medium text-foreground">{TAB_LABELS[activeTab]}</span>
                      </div>
                      {patchInstruction.trim() && (
                        <div>
                          <div className="text-muted-foreground">{t('codePanel.patch.preview')}</div>
                          <div className="mt-1 line-clamp-2 whitespace-pre-wrap text-foreground">
                            {patchInstruction.trim()}
                          </div>
                        </div>
                      )}
                    </div>
                  </div>
                  <textarea
                    value={patchInstruction}
                    onChange={(event) => setPatchInstruction(event.target.value)}
                    placeholder={t('codePanel.patch.placeholder')}
                    className="min-h-20 w-full resize-none rounded-md border border-input bg-background p-2 text-xs text-foreground outline-none focus:border-ring"
                  />
                  <div className="mt-2 flex items-center justify-end gap-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => setPatchOpen(false)}
                    >
                      {t('common.cancel')}
                    </Button>
                    <Button
                      size="sm"
                      className="h-7 text-xs"
                      onClick={() => void handleCreatePatchJob()}
                      disabled={
                        !canQueueBackgroundJob || !activeHistoryId || !patchInstruction.trim()
                      }
                    >
                      {jobLoading ? t('codePanel.background.creating') : t('codePanel.patch.queue')}
                    </Button>
                  </div>
                </div>
              )}
              {(lastCreatedJobId || jobError) && (
                <div className="border-t border-border bg-card px-2 py-1.5 text-[11px]">
                  {lastCreatedJobId && (
                    <div className="text-muted-foreground">{t('codePanel.background.queued')}</div>
                  )}
                  {jobError && <div className="text-destructive">{jobError}</div>}
                </div>
              )}
              <div className="flex items-center gap-px border-t border-border px-1 py-1 shrink-0 bg-card">
                <Button
                  variant="ghost"
                  size="sm"
                  className={cn(
                    'h-7 flex-1 px-1 text-[11px] transition-colors',
                    copied ? 'text-green-500' : 'text-muted-foreground hover:text-foreground',
                  )}
                  onClick={handleCopy}
                >
                  {copied ? (
                    <Check className="mr-1 h-3 w-3 shrink-0" />
                  ) : (
                    <Copy className="mr-1 h-3 w-3 shrink-0" />
                  )}
                  <span className="truncate">
                    {copied ? t('codePanel.copied') : t('codePanel.copy')}
                  </span>
                </Button>
                <div className="w-px h-4 bg-border/50" />
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 flex-1 px-1 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={handleDownload}
                >
                  <Download className="mr-1 h-3 w-3 shrink-0" />
                  <span className="truncate">
                    {requiresZipBundle ? t('codePanel.downloadZip') : t('codePanel.download')}
                  </span>
                </Button>
                <div className="w-px h-4 bg-border/50" />
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 flex-1 px-1 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={() => void handleDownloadStructureBundle()}
                >
                  <FileJson className="mr-1 h-3 w-3 shrink-0" />
                  <span className="truncate">{t('codePanel.aiBundle')}</span>
                </Button>
                <div className="w-px h-4 bg-border/50" />
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 flex-1 px-1 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={() => void handleCreateBackgroundJob()}
                  disabled={!canQueueBackgroundJob}
                >
                  <RefreshCw className="mr-1 h-3 w-3 shrink-0" />
                  <span className="truncate">
                    {jobLoading ? t('codePanel.background.creating') : t('codePanel.background.create')}
                  </span>
                </Button>
                <div className="w-px h-4 bg-border/50" />
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 flex-1 px-1 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={() => setPatchOpen((open) => !open)}
                  disabled={!canQueueBackgroundJob || !activeHistoryId}
                >
                  <WandSparkles className="mr-1 h-3 w-3 shrink-0" />
                  <span className="truncate">{t('codePanel.patch.action')}</span>
                </Button>
                <div className="w-px h-4 bg-border/50" />
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 flex-1 px-1 text-[11px] text-muted-foreground hover:text-foreground"
                  onClick={handleGenerate}
                  disabled={modelUnavailable}
                >
                  <RefreshCw className="mr-1 h-3 w-3 shrink-0" />
                  <span className="truncate">{t('codePanel.regenerate')}</span>
                </Button>
              </div>
              </>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default memo(CodePanelInner);
