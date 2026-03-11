import { useState, useCallback, useEffect } from 'react'
import { X, Globe, Loader2, Check, AlertCircle } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { applyKit } from '@/vibekit/kit-applicator'
import { useVibeKitStore } from '@/stores/vibekit-store'
import { VIBE_KIT_SCHEMA } from '@/vibekit/schema'
import type { VibeKit } from '@/types/vibekit'
import type { VariableDefinition } from '@/types/variables'

type ExtractionState = 'idle' | 'loading' | 'preview' | 'error'

interface ExtractionPreviewProps {
  open: boolean
  onClose: () => void
}

interface ExtractedData {
  variables: Record<string, VariableDefinition>
  tokens: Record<string, unknown>
  cached: boolean
}

export default function ExtractionPreview({ open, onClose }: ExtractionPreviewProps) {
  const [state, setState] = useState<ExtractionState>('idle')
  const [url, setUrl] = useState('')
  const [error, setError] = useState('')
  const [extracted, setExtracted] = useState<ExtractedData | null>(null)

  // Reset state when dialog opens
  useEffect(() => {
    if (open) {
      setState('idle')
      setUrl('')
      setError('')
      setExtracted(null)
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

  const handleExtract = useCallback(async () => {
    if (!url.trim()) return

    setState('loading')
    setError('')

    try {
      const res = await fetch('/api/vibekit/extract', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ url: url.trim() }),
      })

      if (!res.ok) {
        const msg = await res.text().catch(() => 'Extraction failed')
        throw new Error(msg)
      }

      const data: ExtractedData = await res.json()
      setExtracted(data)
      setState('preview')
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Extraction failed')
      setState('error')
    }
  }, [url])

  const handleApply = useCallback(() => {
    if (!extracted) return

    // Build a complete VibeKit from extracted variables + schema fallbacks
    const variables: Record<string, VariableDefinition> = {}

    // Start with schema fallbacks
    for (const [name, entry] of Object.entries(VIBE_KIT_SCHEMA)) {
      variables[name] = {
        type: entry.type,
        value: entry.fallback,
      }
    }

    // Override with extracted variables
    for (const [name, def] of Object.entries(extracted.variables)) {
      variables[name] = def
    }

    const kit: VibeKit = {
      id: `kit-extracted-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`,
      name: new URL(url).hostname,
      version: '1.0.0',
      sourceUrl: url,
      variables,
      assets: {},
      metadata: {
        createdAt: new Date().toISOString(),
        extractedFrom: url,
        generatedBy: 'extraction',
      },
    }

    applyKit(kit)
    useVibeKitStore.getState().saveKit(kit)
    onClose()
  }, [extracted, url, onClose])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === 'Enter' && state === 'idle') {
        handleExtract()
      }
    },
    [state, handleExtract],
  )

  if (!open) return null

  // Collect color and font variables for preview
  const colorVars = extracted
    ? Object.entries(extracted.variables).filter(([, def]) => def.type === 'color')
    : []
  const fontVars = extracted
    ? Object.entries(extracted.variables).filter(
        ([name]) => name.startsWith('font-'),
      )
    : []
  const totalCount = extracted ? Object.keys(extracted.variables).length : 0

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div className="relative bg-card border border-border rounded-2xl shadow-2xl p-5 w-full max-w-md">
        {/* Header */}
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-2">
            <Globe size={16} className="text-muted-foreground" />
            <h3 className="text-sm font-medium text-foreground">Extract from website</h3>
          </div>
          <Button variant="ghost" size="icon-sm" onClick={onClose}>
            <X size={14} />
          </Button>
        </div>

        {/* Idle: URL input */}
        {state === 'idle' && (
          <div className="space-y-3">
            <input
              type="url"
              placeholder="https://example.com"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              onKeyDown={handleKeyDown}
              className="w-full px-3 py-2 rounded-lg border border-border bg-secondary text-foreground text-sm placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-primary"
              autoFocus
            />
            <Button
              size="sm"
              className="w-full"
              disabled={!url.trim()}
              onClick={handleExtract}
            >
              Extract
            </Button>
          </div>
        )}

        {/* Loading */}
        {state === 'loading' && (
          <div className="py-8 flex flex-col items-center gap-3">
            <Loader2 size={24} className="animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">Extracting tokens...</p>
          </div>
        )}

        {/* Preview */}
        {state === 'preview' && extracted && (
          <div className="space-y-4">
            {/* Color swatches */}
            {colorVars.length > 0 && (
              <div>
                <p className="text-xs text-muted-foreground mb-2">Colors</p>
                <div className="grid grid-cols-6 gap-2">
                  {colorVars.map(([name, def]) => (
                    <div key={name} className="flex flex-col items-center gap-1">
                      <div
                        className="w-8 h-8 rounded-md border border-border"
                        style={{ backgroundColor: String(def.value) }}
                      />
                      <span className="text-[10px] text-muted-foreground truncate w-full text-center">
                        {name.replace('color-', '')}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Font previews */}
            {fontVars.length > 0 && (
              <div>
                <p className="text-xs text-muted-foreground mb-2">Fonts</p>
                <div className="space-y-2">
                  {fontVars.map(([name, def]) => (
                    <div key={name} className="flex items-baseline justify-between">
                      <span
                        className="text-sm text-foreground"
                        style={{ fontFamily: String(def.value) }}
                      >
                        {String(def.value).split(',')[0]}
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        {name.replace('font-', '')}
                      </span>
                    </div>
                  ))}
                </div>
              </div>
            )}

            {/* Total count */}
            <p className="text-xs text-muted-foreground">
              {totalCount} variable{totalCount !== 1 ? 's' : ''} extracted
              {extracted.cached ? ' (cached)' : ''}
            </p>

            {/* Actions */}
            <div className="flex gap-2">
              <Button
                variant="secondary"
                size="sm"
                className="flex-1"
                onClick={onClose}
              >
                Cancel
              </Button>
              <Button
                size="sm"
                className="flex-1"
                onClick={handleApply}
              >
                <Check size={14} className="mr-1" />
                Apply
              </Button>
            </div>
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
