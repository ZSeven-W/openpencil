import { useState, useCallback, useRef } from 'react'
import { Search, Loader2, Image as ImageIcon } from 'lucide-react'
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover'
import { useAgentSettingsStore } from '@/stores/agent-settings-store'
import type { ImageSearchResult, ImageSearchResponse } from '@/types/image-service'

interface ImageSearchPopoverProps {
  initialQuery: string
  onSelect: (url: string) => void
  children: React.ReactNode
}

export default function ImageSearchPopover({ initialQuery, onSelect, children }: ImageSearchPopoverProps) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState(initialQuery)
  const [loading, setLoading] = useState(false)
  const [results, setResults] = useState<ImageSearchResult[]>([])
  const [source, setSource] = useState<'openverse' | 'wikimedia' | null>(null)
  const [hasSearched, setHasSearched] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)

  const openverseOAuth = useAgentSettingsStore((s) => s.openverseOAuth)

  const handleSearch = useCallback(async () => {
    const trimmed = query.trim()
    if (!trimmed || loading) return

    setLoading(true)
    setHasSearched(true)

    try {
      const body: Record<string, unknown> = { query: trimmed, count: 5 }
      if (openverseOAuth) {
        body.openverseClientId = openverseOAuth.clientId
        body.openverseClientSecret = openverseOAuth.clientSecret
      }

      const res = await fetch('/api/ai/image-search', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })

      if (res.ok) {
        const data = (await res.json()) as ImageSearchResponse
        setResults(data.results ?? [])
        setSource(data.source ?? null)
      } else {
        setResults([])
        setSource(null)
      }
    } catch {
      setResults([])
      setSource(null)
    } finally {
      setLoading(false)
    }
  }, [query, loading, openverseOAuth])

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === 'Enter') {
        e.preventDefault()
        void handleSearch()
      }
    },
    [handleSearch],
  )

  const handleSelect = useCallback(
    (url: string) => {
      onSelect(url)
      setOpen(false)
    },
    [onSelect],
  )

  const handleOpenChange = useCallback((next: boolean) => {
    setOpen(next)
    if (next) {
      // Reset search state when re-opening
      setHasSearched(false)
      setResults([])
      setSource(null)
    }
  }, [])

  return (
    <Popover open={open} onOpenChange={handleOpenChange}>
      <PopoverTrigger asChild>{children}</PopoverTrigger>

      <PopoverContent
        className="w-80 p-3"
        side="left"
        align="start"
        sideOffset={8}
      >
        {/* Search bar */}
        <div className="flex gap-1.5 mb-3">
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="Search images..."
            className="flex-1 h-7 px-2 text-xs rounded border border-border bg-background text-foreground placeholder:text-muted-foreground outline-none focus:border-primary transition-colors"
          />
          <button
            type="button"
            onClick={() => void handleSearch()}
            disabled={loading || !query.trim()}
            className="h-7 w-7 flex items-center justify-center rounded border border-border bg-background hover:bg-accent/50 text-foreground disabled:opacity-40 disabled:cursor-not-allowed transition-colors shrink-0"
          >
            {loading ? (
              <Loader2 className="w-3.5 h-3.5 animate-spin" />
            ) : (
              <Search className="w-3.5 h-3.5" />
            )}
          </button>
        </div>

        {/* Results / empty state */}
        {loading ? (
          <div className="flex flex-col items-center justify-center py-8 gap-2">
            <Loader2 className="w-5 h-5 animate-spin text-muted-foreground" />
            <span className="text-xs text-muted-foreground">Searching...</span>
          </div>
        ) : results.length > 0 ? (
          <div className="grid grid-cols-3 gap-1.5">
            {results.map((result) => (
              <button
                key={result.id}
                type="button"
                onClick={() => handleSelect(result.thumbUrl)}
                className="aspect-square w-full overflow-hidden rounded border border-border hover:border-primary transition-colors cursor-pointer"
                title={result.attribution ?? result.license}
              >
                <img
                  src={result.thumbUrl}
                  alt=""
                  className="w-full h-full object-cover"
                  loading="lazy"
                />
              </button>
            ))}
          </div>
        ) : (
          <div className="flex flex-col items-center justify-center py-8 gap-2">
            <ImageIcon className="w-6 h-6 text-muted-foreground opacity-50" />
            <span className="text-xs text-muted-foreground">
              {hasSearched ? 'No results found' : 'Search for images'}
            </span>
          </div>
        )}

        {/* Footer: license + source */}
        {results.length > 0 && source && (
          <div className="mt-2 pt-2 border-t border-border">
            <p className="text-[10px] text-muted-foreground leading-snug">
              Images from{' '}
              <span className="font-medium">
                {source === 'openverse' ? 'Openverse' : 'Wikimedia Commons'}
              </span>
              . Freely licensed — verify license before use.
            </p>
          </div>
        )}
      </PopoverContent>
    </Popover>
  )
}
