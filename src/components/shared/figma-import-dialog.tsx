import { useState, useEffect, useCallback, useRef } from 'react'
import { X, Upload, AlertCircle, Loader2, FileUp } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useDocumentStore } from '@/stores/document-store'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'
import { parseFigFile } from '@/services/figma/fig-parser'
import { figmaToPenDocument, figmaAllPagesToPenDocument, getFigmaPages } from '@/services/figma/figma-node-mapper'
import { resolveImageBlobs } from '@/services/figma/figma-image-resolver'
import type { FigmaDecodedFile, FigmaImportLayoutMode } from '@/services/figma/figma-types'

type ImportState = 'idle' | 'parsing' | 'page-select' | 'converting' | 'done' | 'error'

interface FigmaImportDialogProps {
  open: boolean
  onClose: () => void
}

export default function FigmaImportDialog({ open, onClose }: FigmaImportDialogProps) {
  const [state, setState] = useState<ImportState>('idle')
  const [progress, setProgress] = useState(0)
  const [error, setError] = useState('')
  const [warnings, setWarnings] = useState<string[]>([])
  const [decoded, setDecoded] = useState<FigmaDecodedFile | null>(null)
  const [fileName, setFileName] = useState('')
  const [pages, setPages] = useState<{ id: string; name: string; childCount: number }[]>([])
  const [isDragging, setIsDragging] = useState(false)
  const [layoutMode, setLayoutMode] = useState<FigmaImportLayoutMode>('preserve')
  const fileInputRef = useRef<HTMLInputElement>(null)

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setState('idle')
      setProgress(0)
      setError('')
      setWarnings([])
      setDecoded(null)
      setFileName('')
      setPages([])
      setLayoutMode('preserve')
    }
  }, [open])

  // Escape key to close
  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, onClose])

  const processFile = useCallback(async (file: File) => {
    if (!file.name.endsWith('.fig')) {
      setError('Please select a .fig file')
      setState('error')
      return
    }

    setFileName(file.name.replace(/\.fig$/, ''))
    setState('parsing')
    setProgress(10)

    try {
      const buffer = await file.arrayBuffer()
      setProgress(30)

      const decodedFile = parseFigFile(buffer)
      setDecoded(decodedFile)
      setProgress(50)

      const figmaPages = getFigmaPages(decodedFile)

      if (figmaPages.length === 0) {
        setError('No pages found in the .fig file')
        setState('error')
        return
      }

      // Always show page-select to let user choose layout mode
      setPages(figmaPages)
      setState('page-select')
    } catch (err) {
      console.error('[Figma Import] Parse error:', err)
      setError(err instanceof Error ? err.message : 'Failed to parse .fig file')
      setState('error')
    }
  }, [])

  const convertAndLoad = useCallback(async (
    decodedFile: FigmaDecodedFile,
    name: string,
    pageIndex: number | 'all',
  ) => {
    setState('converting')
    setProgress(60)

    try {
      // Use requestAnimationFrame to avoid blocking UI
      await new Promise((r) => requestAnimationFrame(r))

      const { document: doc, warnings: warns, imageBlobs } = pageIndex === 'all'
        ? figmaAllPagesToPenDocument(decodedFile, name, layoutMode)
        : figmaToPenDocument(decodedFile, name, pageIndex, layoutMode)
      setProgress(80)

      // Resolve image blobs and hash-based images to data URLs across all pages
      let unresolved = 0
      if (doc.pages && doc.pages.length > 0) {
        for (const page of doc.pages) {
          unresolved += resolveImageBlobs(page.children, imageBlobs, decodedFile.imageFiles)
        }
      } else {
        unresolved = resolveImageBlobs(doc.children, imageBlobs, decodedFile.imageFiles)
      }
      if (unresolved > 0) {
        warns.push(`${unresolved} images could not be resolved`)
      }

      setProgress(95)
      setWarnings(warns)

      // Load into the document store
      useDocumentStore.getState().loadDocument(doc, `${name}.op`)
      requestAnimationFrame(() => zoomToFitContent())

      setProgress(100)
      setState('done')

      // Auto-close after brief delay on success
      setTimeout(() => {
        onClose()
      }, 800)
    } catch (err) {
      console.error('[Figma Import] Convert error:', err)
      setError(err instanceof Error ? err.message : 'Failed to convert Figma file')
      setState('error')
    }
  }, [onClose, layoutMode])

  const handlePageSelect = useCallback((pageIndex: number | 'all') => {
    if (!decoded) return
    convertAndLoad(decoded, fileName, pageIndex)
  }, [decoded, fileName, convertAndLoad])

  const handleDragOver = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(true)
  }, [])

  const handleDragLeave = useCallback(() => {
    setIsDragging(false)
  }, [])

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault()
    setIsDragging(false)
    const file = e.dataTransfer.files[0]
    if (file) processFile(file)
  }, [processFile])

  const handleFileInput = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (file) processFile(file)
  }, [processFile])

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div className="relative bg-card rounded-lg border border-border p-5 w-96 shadow-xl">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-foreground">Import from Figma</h3>
          <Button variant="ghost" size="icon-sm" onClick={onClose}>
            <X size={14} />
          </Button>
        </div>

        {/* Idle: file drop zone */}
        {state === 'idle' && (
          <>
            <div
              className={`border-2 border-dashed rounded-lg p-8 text-center cursor-pointer transition-colors ${
                isDragging
                  ? 'border-primary bg-primary/5'
                  : 'border-border hover:border-muted-foreground'
              }`}
              onDragOver={handleDragOver}
              onDragLeave={handleDragLeave}
              onDrop={handleDrop}
              onClick={() => fileInputRef.current?.click()}
            >
              <FileUp size={32} className="mx-auto mb-3 text-muted-foreground" />
              <p className="text-sm text-foreground mb-1">
                Drop a .fig file here
              </p>
              <p className="text-xs text-muted-foreground">
                or click to browse
              </p>
            </div>
            <input
              ref={fileInputRef}
              type="file"
              accept=".fig"
              className="hidden"
              onChange={handleFileInput}
            />
            <p className="text-[10px] text-muted-foreground mt-3">
              Export from Figma: File &rarr; Save local copy (.fig)
            </p>
          </>
        )}

        {/* Parsing / Converting: progress */}
        {(state === 'parsing' || state === 'converting') && (
          <div className="py-4">
            <div className="flex items-center gap-2 mb-3">
              <Loader2 size={16} className="animate-spin text-primary" />
              <span className="text-sm text-foreground">
                {state === 'parsing' ? 'Parsing .fig file...' : 'Converting nodes...'}
              </span>
            </div>
            <div className="w-full bg-secondary rounded-full h-1.5">
              <div
                className="bg-primary rounded-full h-1.5 transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
            </div>
            <p className="text-xs text-muted-foreground mt-2">
              {fileName && `${fileName}.fig`}
            </p>
          </div>
        )}

        {/* Page selection */}
        {state === 'page-select' && (
          <div className="py-2">
            {/* Layout mode toggle */}
            <div className="mb-3">
              <p className="text-xs text-muted-foreground mb-2">Layout mode:</p>
              <div className="flex gap-1">
                <button
                  className={`flex-1 px-3 py-1.5 rounded text-xs transition-colors ${
                    layoutMode === 'preserve'
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary text-foreground hover:bg-secondary/80'
                  }`}
                  onClick={() => setLayoutMode('preserve')}
                >
                  保持 Figma 布局
                </button>
                <button
                  className={`flex-1 px-3 py-1.5 rounded text-xs transition-colors ${
                    layoutMode === 'openpencil'
                      ? 'bg-primary text-primary-foreground'
                      : 'bg-secondary text-foreground hover:bg-secondary/80'
                  }`}
                  onClick={() => setLayoutMode('openpencil')}
                >
                  OpenPencil 自动布局
                </button>
              </div>
            </div>

            {pages.length > 1 ? (
              <>
                <p className="text-xs text-muted-foreground mb-3">
                  This file has {pages.length} pages. Select which to import:
                </p>
                <div className="max-h-48 overflow-y-auto space-y-1 mb-3">
                  {pages.map((page, i) => (
                    <button
                      key={page.id}
                      className="w-full text-left px-3 py-2 rounded text-sm hover:bg-secondary transition-colors flex items-center justify-between"
                      onClick={() => handlePageSelect(i)}
                    >
                      <span className="text-foreground truncate">{page.name}</span>
                      <span className="text-xs text-muted-foreground ml-2 shrink-0">
                        {page.childCount} layers
                      </span>
                    </button>
                  ))}
                </div>
                <Button
                  size="sm"
                  className="w-full"
                  onClick={() => handlePageSelect('all')}
                >
                  Import All Pages
                </Button>
              </>
            ) : (
              <>
                <p className="text-xs text-muted-foreground mb-3">
                  {pages[0]?.name} &middot; {pages[0]?.childCount} layers
                </p>
                <Button
                  size="sm"
                  className="w-full"
                  onClick={() => handlePageSelect(0)}
                >
                  Import
                </Button>
              </>
            )}
          </div>
        )}

        {/* Done */}
        {state === 'done' && (
          <div className="py-4 text-center">
            <Upload size={24} className="mx-auto mb-2 text-primary" />
            <p className="text-sm text-foreground">Import complete!</p>
            {warnings.length > 0 && (
              <div className="mt-3 text-left max-h-24 overflow-y-auto">
                {warnings.slice(0, 10).map((w, i) => (
                  <p key={i} className="text-[10px] text-muted-foreground truncate">
                    {w}
                  </p>
                ))}
                {warnings.length > 10 && (
                  <p className="text-[10px] text-muted-foreground">
                    ...and {warnings.length - 10} more warnings
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {/* Error */}
        {state === 'error' && (
          <div className="py-4">
            <div className="flex items-start gap-2">
              <AlertCircle size={16} className="text-destructive shrink-0 mt-0.5" />
              <p className="text-sm text-destructive">{error}</p>
            </div>
            <Button
              variant="secondary"
              size="sm"
              className="mt-4 w-full"
              onClick={() => {
                setState('idle')
                setError('')
              }}
            >
              Try Again
            </Button>
          </div>
        )}
      </div>
    </div>
  )
}
