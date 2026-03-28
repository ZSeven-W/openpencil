import { useState, useRef, useCallback, useEffect, useMemo } from 'react'
import {
  Copy, Download, RefreshCw, Sparkles,
  Check, Loader2, AlertTriangle, MinusCircle, SkipForward,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import { useAIStore } from '@/stores/ai-store'
import { generateCode } from '@/services/ai/code-generation-pipeline'
import { highlightCode } from '@/utils/syntax-highlight'
import type { Framework, CodeGenProgress, ChunkStatus } from '@zseven-w/pen-codegen'
import { FRAMEWORKS } from '@zseven-w/pen-codegen'
import type { PenNode } from '@/types/pen'
import type { SyntaxLanguage } from '@/utils/syntax-highlight'

type PanelState = 'empty' | 'generating' | 'complete'

interface ChunkProgress {
  chunkId: string
  name: string
  status: ChunkStatus
  error?: string
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
}

const HIGHLIGHT_LANG: Record<Framework, SyntaxLanguage> = {
  react: 'jsx',
  vue: 'html',
  svelte: 'html',
  html: 'html',
  flutter: 'dart',
  swiftui: 'swift',
  compose: 'kotlin',
  'react-native': 'jsx',
}

export default function CodePanel() {
  const [activeTab, setActiveTab] = useState<Framework>('react')
  const [codeCache, setCodeCache] = useState<Partial<Record<Framework, { code: string; degraded: boolean }>>>({})
  const [isDegraded, setIsDegraded] = useState(false)
  const [isGenerating, setIsGenerating] = useState(false)
  const [copied, setCopied] = useState(false)
  const [planningStatus, setPlanningStatus] = useState<'idle' | 'running' | 'done' | 'failed'>('idle')
  const [planningError, setPlanningError] = useState<string>()
  const [assemblyStatus, setAssemblyStatus] = useState<'idle' | 'running' | 'done' | 'failed'>('idle')
  const [chunks, setChunks] = useState<ChunkProgress[]>([])
  const [selectionChanged, setSelectionChanged] = useState(false)
  const [generateError, setGenerateError] = useState<string>()

  const cached = codeCache[activeTab]
  const generatedCode = cached?.code ?? ''
  const panelState: PanelState = isGenerating ? 'generating' : cached ? 'complete' : 'empty'

  const abortRef = useRef<AbortController | null>(null)
  const lastSelectionRef = useRef<string>('')
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>(null)

  const selectedIds = useCanvasStore(s => s.selection.selectedIds)
  const activePageId = useCanvasStore(s => s.activePageId)
  const getNodeById = useDocumentStore(s => s.getNodeById)
  const children = useDocumentStore(s => getActivePageChildren(s.document, activePageId))
  const variables = useDocumentStore(s => s.document?.variables)
  const model = useAIStore(s => s.model)
  const provider = useAIStore(s =>
    s.modelGroups.find(g => g.models.some(m => m.value === s.model))?.provider,
  )

  const selectionKey = selectedIds.join(',')

  // Detect selection changes when code is already generated
  useEffect(() => {
    if (panelState === 'complete' && selectionKey !== lastSelectionRef.current) {
      setSelectionChanged(true)
    }
  }, [panelState, selectionKey])

  // Cleanup on unmount
  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
      abortRef.current?.abort()
    }
  }, [])

  const getTargetNodes = useCallback((): PenNode[] => {
    if (selectedIds.length > 0) {
      return selectedIds
        .map(id => getNodeById(id))
        .filter((n): n is PenNode => n !== undefined)
    }
    return children
  }, [selectedIds, getNodeById, children])

  const handleGenerate = useCallback(async () => {
    const nodes = getTargetNodes()
    if (nodes.length === 0) return

    abortRef.current = new AbortController()
    setIsGenerating(true)
    setPlanningStatus('idle')
    setPlanningError(undefined)
    setAssemblyStatus('idle')
    setChunks([])
    setIsDegraded(false)
    setSelectionChanged(false)
    setGenerateError(undefined)
    lastSelectionRef.current = selectionKey

    const handleProgress = (event: CodeGenProgress) => {
      switch (event.step) {
        case 'planning':
          setPlanningStatus(event.status)
          if (event.error) setPlanningError(event.error)
          break
        case 'chunk':
          setChunks(prev => {
            const existing = prev.findIndex(c => c.chunkId === event.chunkId)
            const entry: ChunkProgress = {
              chunkId: event.chunkId,
              name: event.name,
              status: event.status,
              error: event.error,
            }
            if (existing >= 0) {
              const next = [...prev]
              next[existing] = entry
              return next
            }
            return [...prev, entry]
          })
          break
        case 'assembly':
          setAssemblyStatus(event.status)
          break
        case 'complete':
          setCodeCache(prev => ({ ...prev, [activeTab]: { code: event.finalCode, degraded: event.degraded } }))
          setIsDegraded(event.degraded)
          setIsGenerating(false)
          break
        case 'error':
          setGenerateError(event.message)
          setIsGenerating(false)
          break
      }
    }

    try {
      await generateCode(nodes, activeTab, variables, handleProgress, model, provider, abortRef.current.signal)
    } catch (err) {
      if (!abortRef.current?.signal.aborted) {
        const msg = err instanceof Error ? err.message : 'Code generation failed'
        setGenerateError(msg)
      }
      setIsGenerating(false)
    }
  }, [getTargetNodes, activeTab, variables, selectionKey, model, provider])

  const handleCancel = useCallback(() => {
    abortRef.current?.abort()
    setIsGenerating(false)
  }, [])

  const handleRetryChunk = useCallback((_chunkId: string) => {
    // Re-run the full pipeline (planning is fast, only failed/skipped chunks re-run)
    void handleGenerate()
  }, [handleGenerate])

  const handleCopy = useCallback(async () => {
    await navigator.clipboard.writeText(generatedCode)
    setCopied(true)
    if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
    copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000)
  }, [generatedCode])

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
    }
    const blob = new Blob([generatedCode], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = globalThis.document.createElement('a')
    a.href = url
    a.download = `design${extensions[activeTab]}`
    a.click()
    URL.revokeObjectURL(url)
  }, [generatedCode, activeTab])

  const handleTabChange = useCallback((tab: Framework) => {
    setActiveTab(tab)
    setGenerateError(undefined)
    // isDegraded follows the cached tab's value
    const tabCache = codeCache[tab]
    setIsDegraded(tabCache?.degraded ?? false)
  }, [codeCache])

  const nodeCount = selectedIds.length > 0 ? selectedIds.length : children.length

  const highlightedHTML = useMemo(() => {
    if (!generatedCode) return ''
    const lang = HIGHLIGHT_LANG[activeTab]

    // HTML / Vue / Svelte: split at <style to highlight CSS portion separately
    if (activeTab === 'html' || activeTab === 'vue' || activeTab === 'svelte') {
      const styleIdx = generatedCode.indexOf('<style')
      if (styleIdx !== -1) {
        const templatePart = generatedCode.slice(0, styleIdx)
        const stylePart = generatedCode.slice(styleIdx)
        const styleTagEnd = stylePart.indexOf('>\n')
        if (styleTagEnd !== -1) {
          const styleTag = stylePart.slice(0, styleTagEnd + 1)
          const styleBody = stylePart.slice(styleTagEnd + 1)
          const closingIdx = styleBody.lastIndexOf('</style>')
          if (closingIdx !== -1) {
            const cssContent = styleBody.slice(0, closingIdx)
            const closingTag = styleBody.slice(closingIdx)
            return (
              highlightCode(templatePart, 'html') +
              highlightCode(styleTag, 'html') + '\n' +
              highlightCode(cssContent, 'css') +
              highlightCode(closingTag, 'html')
            )
          }
        }
        return highlightCode(templatePart, 'html') + highlightCode(stylePart, 'css')
      }
    }

    return highlightCode(generatedCode, lang)
  }, [activeTab, generatedCode])

  return (
    <div className="flex flex-1 min-h-0 flex-col">
      {/* Tab Bar */}
      <div className="flex items-center border-b border-border px-2 shrink-0">
        <div className="flex gap-1 overflow-x-auto py-1 scrollbar-none">
          {FRAMEWORKS.map(fw => (
            <button
              key={fw}
              type="button"
              className={cn(
                'whitespace-nowrap rounded-md px-2.5 py-1 text-xs font-medium transition-colors shrink-0',
                activeTab === fw
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:bg-muted',
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
          <div className="flex h-full flex-col items-center justify-center gap-3 p-6 text-center">
            <Sparkles className="h-8 w-8 text-muted-foreground" />
            <div className="text-sm text-muted-foreground">
              {nodeCount > 0
                ? `${nodeCount} node${nodeCount > 1 ? 's' : ''} selected`
                : 'No nodes on page'}
            </div>
            <Button
              onClick={handleGenerate}
              disabled={nodeCount === 0}
              size="sm"
            >
              <Sparkles className="mr-2 h-4 w-4" />
              Generate {TAB_LABELS[activeTab]} Code
            </Button>
            {generateError && (
              <div className="max-w-[260px] rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
                <div className="font-medium">Generation failed</div>
                <div className="mt-1 break-words">{generateError}</div>
              </div>
            )}
            {selectionChanged && (
              <div className="text-xs text-amber-500">
                Selection changed since last generation
              </div>
            )}
          </div>
        )}

        {/* Generating State */}
        {panelState === 'generating' && (
          <div className="flex flex-col gap-2 p-4">
            {/* Planning */}
            <ProgressItem
              label="Planning"
              status={planningStatus === 'running' ? 'running' : planningStatus === 'done' ? 'done' : planningStatus === 'failed' ? 'failed' : 'pending'}
              error={planningError}
            />

            {/* Chunks */}
            {chunks.map(chunk => (
              <ProgressItem
                key={chunk.chunkId}
                label={chunk.name}
                status={chunk.status}
                error={chunk.error}
                onRetry={chunk.status === 'failed' ? () => handleRetryChunk(chunk.chunkId) : undefined}
              />
            ))}

            {/* Assembly */}
            {assemblyStatus !== 'idle' && (
              <ProgressItem
                label="Assembly"
                status={assemblyStatus === 'running' ? 'running' : assemblyStatus === 'done' ? 'done' : 'failed'}
              />
            )}

            <Button variant="ghost" size="sm" onClick={handleCancel} className="mt-2 self-center">
              Cancel
            </Button>
          </div>
        )}

        {/* Complete State */}
        {panelState === 'complete' && (
          <>
            {isDegraded && (
              <div className="flex items-center gap-2 bg-amber-500/10 px-3 py-2 text-xs text-amber-600 shrink-0">
                <AlertTriangle className="h-3.5 w-3.5 shrink-0" />
                Some chunks failed or degraded. Output may not compile.
              </div>
            )}
            {selectionChanged && (
              <div className="flex items-center justify-between bg-muted px-3 py-1.5 text-xs text-muted-foreground shrink-0">
                <span>Selection changed</span>
                <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={handleGenerate}>
                  Regenerate
                </Button>
              </div>
            )}
            <div className="flex-1 min-h-0 overflow-auto p-2">
              <pre className="text-[10px] leading-relaxed font-mono text-foreground/80 whitespace-pre-wrap break-all">
                <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
              </pre>
            </div>
            <div className="flex items-center border-t border-border px-1 py-1 shrink-0 bg-card">
              <Button variant="ghost" size="sm" className="h-7 flex-1 px-1 text-xs text-muted-foreground hover:text-foreground" onClick={handleCopy}>
                {copied ? <Check className="mr-1 h-3 w-3 shrink-0" /> : <Copy className="mr-1 h-3 w-3 shrink-0" />}
                <span className="truncate">{copied ? 'Copied' : 'Copy'}</span>
              </Button>
              <Button variant="ghost" size="sm" className="h-7 flex-1 px-1 text-xs text-muted-foreground hover:text-foreground" onClick={handleDownload}>
                <Download className="mr-1 h-3 w-3 shrink-0" />
                <span className="truncate">Download</span>
              </Button>
              <Button variant="ghost" size="sm" className="h-7 flex-1 px-1 text-xs text-muted-foreground hover:text-foreground" onClick={handleGenerate}>
                <RefreshCw className="mr-1 h-3 w-3 shrink-0" />
                <span className="truncate">Regenerate</span>
              </Button>
            </div>
          </>
        )}
      </div>
    </div>
  )
}

// ── Progress Item Sub-Component ──

function ProgressItem({
  label,
  status,
  error,
  onRetry,
}: {
  label: string
  status: ChunkStatus | 'running' | 'done' | 'failed' | 'pending'
  error?: string
  onRetry?: () => void
}) {
  const icons: Record<string, React.ReactNode> = {
    pending: <div className="h-3.5 w-3.5 rounded-full border border-muted-foreground/30" />,
    running: <Loader2 className="h-3.5 w-3.5 animate-spin text-primary" />,
    done: <Check className="h-3.5 w-3.5 text-green-500" />,
    degraded: <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />,
    failed: <MinusCircle className="h-3.5 w-3.5 text-destructive" />,
    skipped: <SkipForward className="h-3.5 w-3.5 text-muted-foreground" />,
  }

  const sublabels: Record<string, string> = {
    degraded: 'generated without contract',
    skipped: 'skipped (dependency failed)',
  }

  return (
    <div className="flex items-start gap-2 text-sm">
      <div className="mt-0.5">{icons[status]}</div>
      <div className="flex-1">
        <div className="font-medium">{label}</div>
        {sublabels[status] && (
          <div className="text-xs text-muted-foreground">{sublabels[status]}</div>
        )}
        {error && <div className="text-xs text-destructive">{error}</div>}
      </div>
      {onRetry && (
        <Button variant="ghost" size="sm" className="h-6 text-xs" onClick={onRetry}>
          Retry
        </Button>
      )}
    </div>
  )
}
