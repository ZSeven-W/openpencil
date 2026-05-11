import { useState, useRef, useCallback, useEffect, useMemo, memo } from 'react';
import {
  Copy,
  Download,
  FileJson,
  RefreshCw,
  Sparkles,
  Check,
  AlertTriangle,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import CodeHistoryList from './code-history-list';
import { resolveCodegenModelConfig } from './codegen-model-config';
import { getPreviewReasonKey, openPreviewHtml } from './code-preview-window';
import ProgressItem from './code-progress-item';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import { useAIStore } from '@/stores/ai-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { useCodegenStore, type CodegenHistoryEntry } from '@/stores/codegen-store';
import { generateCode } from '@/services/ai/code-generation-pipeline';
import { buildCodegenBundleManifest } from '@/services/ai/codegen-assets';
import { buildCodegenTarget } from '@/services/cloud/codegen-history';
import { buildCodePreviewDocument } from '@/services/cloud/codegen-preview';
import { buildAIStructureBundle, encodeAIStructureBundleZip } from '@/services/ai/structure-bundle';
import { highlightCode } from '@/utils/syntax-highlight';
import type { Framework, CodeGenProgress } from '@zseven-w/pen-types';
import { FRAMEWORKS } from '@zseven-w/pen-types';
import type { PenNode } from '@/types/pen';
import type { SyntaxLanguage } from '@/utils/syntax-highlight';
import { encode as encodeZip } from 'uzip';
import { useTranslation } from 'react-i18next';

type PanelState = 'empty' | 'generating' | 'complete';
export type CodePanelView = 'code' | 'history';

const TAB_LABELS: Record<Framework, string> = {
  react: 'React',
  vue: 'Vue',
  svelte: 'Svelte',
  html: 'HTML',
  flutter: 'Flutter',
  swiftui: 'SwiftUI',
  compose: 'Compose',
  'react-native': 'RN',
};

const HIGHLIGHT_LANG: Record<Framework, SyntaxLanguage> = {
  react: 'jsx',
  vue: 'html',
  svelte: 'html',
  html: 'html',
  flutter: 'dart',
  swiftui: 'swift',
  compose: 'kotlin',
  'react-native': 'jsx',
};

const EMPTY_HISTORY: CodegenHistoryEntry[] = [];

function triggerDownload(blob: Blob, fileName: string) {
  const url = URL.createObjectURL(blob);
  const link = globalThis.document.createElement('a');
  link.href = url;
  link.download = fileName;
  globalThis.document.body.appendChild(link);
  link.click();
  globalThis.document.body.removeChild(link);
  setTimeout(() => URL.revokeObjectURL(url), 0);
}

function CodePanelInner() {
  const { t } = useTranslation();
  const [copied, setCopied] = useState(false);
  const [panelView, setPanelView] = useState<CodePanelView>('code');
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
  const historyLoading = useCodegenStore((s) => s.historyLoading);
  const historyError = useCodegenStore((s) => s.historyError);
  const history = useCodegenStore((s) => s.history[activeTab] ?? EMPTY_HISTORY);
  const selectedHistoryId = useCodegenStore((s) => s.selectedHistoryId[activeTab]);

  const cached = codeCache[activeTab];
  const generatedCode = cached?.code ?? '';
  const isDegraded = cached?.degraded ?? false;
  const exportedAssets = cached?.assets ?? [];
  const hasExportedAssets = exportedAssets.length > 0;
  const panelState: PanelState = isGenerating ? 'generating' : cached ? 'complete' : 'empty';

  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>(null);

  const selectedIds = useCanvasStore((s) => s.selection.selectedIds);
  const activePageId = useCanvasStore((s) => s.activePageId);
  const getNodeById = useDocumentStore((s) => s.getNodeById);
  const children = useDocumentStore((s) => getActivePageChildren(s.document, activePageId));
  const document = useDocumentStore((s) => s.document);
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

  useEffect(() => {
    void loadHistory(activeTab, codegenTarget);
  }, [activeTab, codegenTarget, loadHistory]);

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
    if (selectedIds.length > 0) {
      return selectedIds.map((id) => getNodeById(id)).filter((n): n is PenNode => n !== undefined);
    }
    return children;
  }, [selectedIds, getNodeById, children]);

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
        const msg = err instanceof Error ? err.message : 'Code generation failed';
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
    await navigator.clipboard.writeText(generatedCode);
    setCopied(true);
    if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
    copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000);
  }, [generatedCode]);

  const handleDownload = useCallback(() => {
    const extensions: Record<Framework, string> = {
      react: '.tsx',
      vue: '.vue',
      svelte: '.svelte',
      html: '.html',
      flutter: '.dart',
      swiftui: '.swift',
      compose: '.kt',
      'react-native': '.tsx',
    };
    const codeFileName = `design${extensions[activeTab]}`;
    const assets = exportedAssets;
    const codeBytes = new TextEncoder().encode(generatedCode);

    void (async () => {
      const blob =
        assets.length > 0
          ? new Blob(
              [
                encodeZip({
                  [codeFileName]: codeBytes,
                  'manifest.json': new TextEncoder().encode(
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
                }),
              ],
              { type: 'application/zip' },
            )
          : new Blob([generatedCode], { type: 'text/plain;charset=utf-8' });

      triggerDownload(blob, assets.length > 0 ? `design-${activeTab}.zip` : codeFileName);
    })();
  }, [generatedCode, activeTab, exportedAssets]);

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
    },
    [clearGenerateError, setActiveTab],
  );

  const handleSelectHistory = useCallback(
    (generationId: string) => {
      selectHistoryEntry(activeTab, generationId);
      setPanelView('code');
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

  const nodeCount = selectedIds.length > 0 ? selectedIds.length : children.length;

  const highlightedHTML = useMemo(() => {
    if (!generatedCode) return '';
    const lang = HIGHLIGHT_LANG[activeTab];

    if (activeTab === 'html' || activeTab === 'vue' || activeTab === 'svelte') {
      const styleIdx = generatedCode.indexOf('<style');
      if (styleIdx !== -1) {
        const templatePart = generatedCode.slice(0, styleIdx);
        const stylePart = generatedCode.slice(styleIdx);
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

    return highlightCode(generatedCode, lang);
  }, [activeTab, generatedCode]);

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
                  ? `${nodeCount} node${nodeCount > 1 ? 's' : ''} selected`
                  : 'No nodes on page'}
              </div>
              <div className="text-[11px] text-muted-foreground">
                Generate production-ready code
              </div>
            </div>
            <Button
              onClick={handleGenerate}
              disabled={nodeCount === 0 || modelUnavailable}
              size="sm"
              className="h-8 gap-1.5 text-xs shadow-sm"
            >
              <Sparkles className="h-3.5 w-3.5" />
              {`Generate ${TAB_LABELS[activeTab]}`}
            </Button>
            <Button
              variant="ghost"
              onClick={() => void handleDownloadStructureBundle()}
              disabled={nodeCount === 0}
              size="sm"
              className="h-8 gap-1.5 text-xs"
            >
              <FileJson className="h-3.5 w-3.5" />
              Export AI Bundle
            </Button>
            {generateError && (
              <div className="max-w-[260px] rounded-lg border border-destructive/20 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                <div className="font-medium">Generation failed</div>
                <div className="mt-1 break-words opacity-80">{generateError}</div>
              </div>
            )}
            {modelUnavailable && (
              <div className="max-w-[260px] rounded-lg border border-border bg-muted/30 px-3 py-2 text-xs text-muted-foreground">
                {t('codePanel.modelUnavailable')}
              </div>
            )}
            {historyLoading && (
              <div className="text-[11px] text-muted-foreground">Loading code history...</div>
            )}
            {historyError && (
              <div className="max-w-[260px] rounded-lg border border-destructive/20 bg-destructive/8 px-3 py-2 text-xs text-destructive">
                {historyError}
              </div>
            )}
            {selectionChanged && (
              <div className="flex items-center gap-1.5 text-[11px] text-amber-500">
                <AlertTriangle className="h-3 w-3" />
                Selection changed since last generation
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
                label="Planning"
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
                  label="Assembly"
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
                Cancel
              </button>
            </div>
          </div>
        )}

        {panelState === 'complete' && (
          <>
            {isDegraded && (
              <div className="flex items-center gap-2 border-b border-amber-500/20 bg-amber-500/8 px-3 py-1.5 text-[11px] text-amber-600 shrink-0">
                <AlertTriangle className="h-3 w-3 shrink-0" />
                Some chunks failed. Output may not compile.
              </div>
            )}
            {selectionChanged && (
              <div className="flex items-center justify-between border-b border-border/50 bg-muted/40 px-3 py-1 text-[11px] text-muted-foreground shrink-0">
                <span className="flex items-center gap-1.5">
                  <AlertTriangle className="h-3 w-3 text-amber-500" />
                  Selection changed
                </span>
                <button
                  type="button"
                  className="text-[11px] font-medium text-primary hover:text-primary/80 transition-colors"
                  onClick={handleGenerate}
                >
                  Regenerate
                </button>
              </div>
            )}
            {hasExportedAssets && (
              <div className="border-b border-border/50 bg-muted/40 px-3 py-2 shrink-0">
                <div className="flex items-center gap-2 text-xs font-medium text-foreground">
                  <Download className="h-3.5 w-3.5 shrink-0" />
                  This generation includes {exportedAssets.length} image asset
                  {exportedAssets.length > 1 ? 's' : ''}. Download will export a ZIP bundle.
                </div>
                <div className="mt-1 text-[11px] text-muted-foreground">
                  The ZIP contains the code file, exported assets, and a
                  <code className="font-mono"> manifest.json </code>
                  index.
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
                <div className="h-full overflow-auto p-2">
                  <pre className="text-[10px] leading-relaxed font-mono text-foreground/80 whitespace-pre-wrap break-all">
                    <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
                  </pre>
                </div>
              )}
              {panelView === 'history' && (
                <CodeHistoryList
                  history={history}
                  selectedId={selectedHistoryId}
                  onSelect={handleSelectHistory}
                  onPreview={handlePreviewHistory}
                />
              )}
            </div>
            {panelView !== 'history' && (
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
                  <span className="truncate">{copied ? 'Copied' : 'Copy'}</span>
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
                    {hasExportedAssets ? 'Download ZIP' : 'Download'}
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
                  <span className="truncate">AI Bundle</span>
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
                  <span className="truncate">Regenerate</span>
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}

export default memo(CodePanelInner);
