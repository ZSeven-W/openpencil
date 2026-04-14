import { useState, useRef, useCallback, useEffect, useMemo, memo } from 'react';
import {
  Copy,
  Download,
  FileJson,
  RefreshCw,
  Sparkles,
  Check,
  Loader2,
  AlertTriangle,
  MinusCircle,
  SkipForward,
} from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store';
import { useAIStore } from '@/stores/ai-store';
import { generateCode } from '@/services/ai/code-generation-pipeline';
import { buildCodegenBundleManifest, type CodegenAssetFile } from '@/services/ai/codegen-assets';
import { buildAIStructureBundle, encodeAIStructureBundleZip } from '@/services/ai/structure-bundle';
import { highlightCode } from '@/utils/syntax-highlight';
import type { Framework, CodeGenProgress, ChunkStatus } from '@zseven-w/pen-types';
import { FRAMEWORKS } from '@zseven-w/pen-types';
import type { PenNode } from '@/types/pen';
import type { SyntaxLanguage } from '@/utils/syntax-highlight';
import { encode as encodeZip } from 'uzip';

type PanelState = 'empty' | 'generating' | 'complete';

interface ChunkProgress {
  chunkId: string;
  name: string;
  status: ChunkStatus;
  error?: string;
}

interface GeneratedCodeBundle {
  code: string;
  degraded: boolean;
  assets: CodegenAssetFile[];
}

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
  const [activeTab, setActiveTab] = useState<Framework>('react');
  const [codeCache, setCodeCache] = useState<Partial<Record<Framework, GeneratedCodeBundle>>>({});
  const [isDegraded, setIsDegraded] = useState(false);
  const [isGenerating, setIsGenerating] = useState(false);
  const [copied, setCopied] = useState(false);
  const [planningStatus, setPlanningStatus] = useState<'idle' | 'running' | 'done' | 'failed'>(
    'idle',
  );
  const [planningError, setPlanningError] = useState<string>();
  const [assemblyStatus, setAssemblyStatus] = useState<'idle' | 'running' | 'done' | 'failed'>(
    'idle',
  );
  const [chunks, setChunks] = useState<ChunkProgress[]>([]);
  const [selectionChanged, setSelectionChanged] = useState(false);
  const [generateError, setGenerateError] = useState<string>();

  const cached = codeCache[activeTab];
  const generatedCode = cached?.code ?? '';
  const exportedAssets = cached?.assets ?? [];
  const hasExportedAssets = exportedAssets.length > 0;
  const panelState: PanelState = isGenerating ? 'generating' : cached ? 'complete' : 'empty';

  const abortRef = useRef<AbortController | null>(null);
  const lastSelectionRef = useRef<string>('');
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>(null);

  const selectedIds = useCanvasStore((s) => s.selection.selectedIds);
  const activePageId = useCanvasStore((s) => s.activePageId);
  const getNodeById = useDocumentStore((s) => s.getNodeById);
  const children = useDocumentStore((s) => getActivePageChildren(s.document, activePageId));
  const variables = useDocumentStore((s) => s.document?.variables);
  const model = useAIStore((s) => s.model);
  // For builtin models, force provider to 'builtin' — modelGroups may report
  // 'anthropic'/'openai' based on the upstream API type, but streamChat needs
  // 'builtin' to route through streamViaBuiltin on the server. Mirrors
  // ai-chat-handlers.ts behavior so the code panel uses the same model/provider
  // as the chat panel.
  const provider = useAIStore((s) => {
    if (s.model.startsWith('builtin:')) return 'builtin';
    return s.modelGroups.find((g) => g.models.some((m) => m.value === s.model))?.provider;
  });

  const selectionKey = selectedIds.join(',');

  // Detect selection changes when code is already generated
  useEffect(() => {
    if (panelState === 'complete' && selectionKey !== lastSelectionRef.current) {
      setSelectionChanged(true);
    }
  }, [panelState, selectionKey]);

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current);
      abortRef.current?.abort();
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
    if (nodes.length === 0) return;

    abortRef.current = new AbortController();
    setIsGenerating(true);
    setPlanningStatus('idle');
    setPlanningError(undefined);
    setAssemblyStatus('idle');
    setChunks([]);
    setIsDegraded(false);
    setSelectionChanged(false);
    setGenerateError(undefined);
    lastSelectionRef.current = selectionKey;

    const handleProgress = (event: CodeGenProgress) => {
      switch (event.step) {
        case 'planning':
          setPlanningStatus(event.status);
          if (event.error) setPlanningError(event.error);
          break;
        case 'chunk':
          setChunks((prev) => {
            const existing = prev.findIndex((c) => c.chunkId === event.chunkId);
            const entry: ChunkProgress = {
              chunkId: event.chunkId,
              name: event.name,
              status: event.status,
              error: event.error,
            };
            if (existing >= 0) {
              const next = [...prev];
              next[existing] = entry;
              return next;
            }
            return [...prev, entry];
          });
          break;
        case 'assembly':
          setAssemblyStatus(event.status);
          break;
        case 'complete':
          setIsDegraded(event.degraded);
          setIsGenerating(false);
          break;
        case 'error':
          setGenerateError(event.message);
          setIsGenerating(false);
          break;
      }
    };

    try {
      const result = await generateCode(
        nodes,
        activeTab,
        variables,
        handleProgress,
        model,
        provider,
        abortRef.current.signal,
      );
      setCodeCache((prev) => ({
        ...prev,
        [activeTab]: {
          code: result.code,
          degraded: result.degraded,
          assets: result.assets,
        },
      }));
      setIsDegraded(result.degraded);
    } catch (err) {
      if (!abortRef.current?.signal.aborted) {
        const msg = err instanceof Error ? err.message : 'Code generation failed';
        setGenerateError(msg);
      }
      setIsGenerating(false);
    }
  }, [getTargetNodes, activeTab, variables, selectionKey, model, provider]);

  const handleCancel = useCallback(() => {
    abortRef.current?.abort();
    setIsGenerating(false);
  }, []);

  const handleRetryChunk = useCallback(
    (_chunkId: string) => {
      // Re-run the full pipeline (planning is fast, only failed/skipped chunks re-run)
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
      setGenerateError(undefined);
      // isDegraded follows the cached tab's value
      const tabCache = codeCache[tab];
      setIsDegraded(tabCache?.degraded ?? false);
    },
    [codeCache],
  );

  const nodeCount = selectedIds.length > 0 ? selectedIds.length : children.length;

  const highlightedHTML = useMemo(() => {
    if (!generatedCode) return '';
    const lang = HIGHLIGHT_LANG[activeTab];

    // HTML / Vue / Svelte: split at <style to highlight CSS portion separately
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
      {/* Tab Bar */}
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

      {/* Content */}
      <div className="flex-1 min-h-0 flex flex-col">
        {/* Empty State */}
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
              disabled={nodeCount === 0}
              size="sm"
              className="h-8 gap-1.5 text-xs shadow-sm"
            >
              <Sparkles className="h-3.5 w-3.5" />
              Generate {TAB_LABELS[activeTab]}
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
            {selectionChanged && (
              <div className="flex items-center gap-1.5 text-[11px] text-amber-500">
                <AlertTriangle className="h-3 w-3" />
                Selection changed since last generation
              </div>
            )}
          </div>
        )}

        {/* Generating State */}
        {panelState === 'generating' && (
          <div className="flex flex-1 flex-col">
            {/* Progress bar */}
            <div className="h-[2px] shrink-0 bg-muted">
              <div
                className="h-full bg-primary transition-all duration-500 ease-out"
                style={{ width: `${progressPct}%` }}
              />
            </div>

            <div className="flex flex-col gap-1 p-3">
              {/* Planning */}
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

              {/* Chunks */}
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

              {/* Assembly */}
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

        {/* Complete State */}
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
            <div className="flex-1 min-h-0 overflow-auto p-2">
              <pre className="text-[10px] leading-relaxed font-mono text-foreground/80 whitespace-pre-wrap break-all">
                <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
              </pre>
            </div>
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
                <span className="truncate">{hasExportedAssets ? 'Download ZIP' : 'Download'}</span>
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
              >
                <RefreshCw className="mr-1 h-3 w-3 shrink-0" />
                <span className="truncate">Regenerate</span>
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  );
}

export default memo(CodePanelInner);

// ── Progress Item Sub-Component ──

function ProgressItem({
  label,
  status,
  error,
  onRetry,
}: {
  label: string;
  status: ChunkStatus | 'running' | 'done' | 'failed' | 'pending';
  error?: string;
  onRetry?: () => void;
}) {
  const icons: Record<string, React.ReactNode> = {
    pending: <div className="h-3 w-3 rounded-full border border-muted-foreground/20" />,
    running: <Loader2 className="h-3 w-3 animate-spin text-primary" />,
    done: <Check className="h-3 w-3 text-green-500" />,
    degraded: <AlertTriangle className="h-3 w-3 text-amber-500" />,
    failed: <MinusCircle className="h-3 w-3 text-destructive" />,
    skipped: <SkipForward className="h-3 w-3 text-muted-foreground/50" />,
  };

  const sublabels: Record<string, string> = {
    degraded: 'generated without contract',
    skipped: 'skipped (dependency failed)',
  };

  return (
    <div
      className={cn(
        'flex items-start gap-2 rounded-md px-2 py-1.5 text-[12px] transition-colors',
        status === 'running' && 'bg-primary/5',
        status === 'failed' && 'bg-destructive/5',
      )}
    >
      <div className="mt-[3px] shrink-0">{icons[status]}</div>
      <div className="flex-1 min-w-0">
        <div
          className={cn(
            'font-medium truncate',
            status === 'pending' && 'text-muted-foreground/60',
            status === 'running' && 'text-foreground',
            status === 'done' && 'text-foreground/70',
            status === 'failed' && 'text-destructive',
            status === 'degraded' && 'text-amber-600',
            status === 'skipped' && 'text-muted-foreground/50',
          )}
        >
          {label}
        </div>
        {sublabels[status] && (
          <div className="text-[10px] text-muted-foreground/60 mt-0.5">{sublabels[status]}</div>
        )}
        {error && <div className="text-[10px] text-destructive/80 mt-0.5 break-words">{error}</div>}
      </div>
      {onRetry && (
        <button
          type="button"
          className="shrink-0 rounded px-1.5 py-0.5 text-[10px] font-medium text-primary hover:bg-primary/10 transition-colors"
          onClick={onRetry}
        >
          Retry
        </button>
      )}
    </div>
  );
}
