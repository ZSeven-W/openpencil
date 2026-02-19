import { useState, useEffect, useRef, useCallback } from 'react'
import { X, Loader2, Search } from 'lucide-react'
import { Button } from '@/components/ui/button'

const ICONIFY_API = 'https://api.iconify.design'
const SEARCH_LIMIT = 64
const DEBOUNCE_MS = 300

/** Detect dark mode and return a color param for Iconify preview URLs. */
function getIconColor(): string {
  const isLight =
    typeof document !== 'undefined' &&
    document.documentElement.classList.contains('light')
  // Encode # as %23 for URL usage
  return isLight ? '%23333333' : '%23e4e4e7'
}

interface IconPickerDialogProps {
  open: boolean
  onClose: () => void
  onSelect: (svgText: string, iconName: string) => void
}

/** Split "mdi:home" → { collection: "mdi", name: "home" } */
function parseIconId(id: string) {
  const idx = id.indexOf(':')
  return { collection: id.slice(0, idx), name: id.slice(idx + 1) }
}

export default function IconPickerDialog({
  open,
  onClose,
  onSelect,
}: IconPickerDialogProps) {
  const [query, setQuery] = useState('')
  const [icons, setIcons] = useState<string[]>([])
  const [loading, setLoading] = useState(false)
  const [fetching, setFetching] = useState<string | null>(null)
  const [searched, setSearched] = useState(false)
  const inputRef = useRef<HTMLInputElement>(null)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // Focus input on open
  useEffect(() => {
    if (open) {
      setTimeout(() => inputRef.current?.focus(), 50)
    } else {
      setQuery('')
      setIcons([])
      setSearched(false)
    }
  }, [open])

  // Escape key closes dialog
  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        onClose()
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open, onClose])

  // Debounced search
  const doSearch = useCallback((q: string) => {
    if (timerRef.current) clearTimeout(timerRef.current)

    if (!q.trim()) {
      setIcons([])
      setLoading(false)
      setSearched(false)
      return
    }

    setLoading(true)
    timerRef.current = setTimeout(async () => {
      try {
        const res = await fetch(
          `${ICONIFY_API}/search?query=${encodeURIComponent(q.trim())}&limit=${SEARCH_LIMIT}`,
        )
        if (!res.ok) throw new Error('Search failed')
        const data = await res.json()
        setIcons(data.icons ?? [])
      } catch {
        setIcons([])
      } finally {
        setLoading(false)
        setSearched(true)
      }
    }, DEBOUNCE_MS)
  }, [])

  const handleQueryChange = (val: string) => {
    setQuery(val)
    doSearch(val)
  }

  const handleSelect = async (iconId: string) => {
    const { collection, name } = parseIconId(iconId)
    setFetching(iconId)
    try {
      const res = await fetch(`${ICONIFY_API}/${collection}/${name}.svg`)
      if (!res.ok) throw new Error('Fetch failed')
      const svgText = await res.text()
      onSelect(svgText, iconId)
    } catch {
      // silently fail
    } finally {
      setFetching(null)
    }
  }

  if (!open) return null

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-background/80"
        onClick={onClose}
      />

      {/* Dialog */}
      <div className="relative bg-card rounded-lg border border-border p-4 w-[340px] shadow-xl">
        {/* Header */}
        <div className="flex items-center justify-between mb-3">
          <h3 className="text-sm font-semibold text-foreground">Icons</h3>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={onClose}
          >
            <X size={14} />
          </Button>
        </div>

        {/* Search input */}
        <div className="relative mb-3">
          <Search
            size={14}
            className="absolute left-2.5 top-1/2 -translate-y-1/2 text-muted-foreground"
          />
          <input
            ref={inputRef}
            type="text"
            value={query}
            onChange={(e) => handleQueryChange(e.target.value)}
            placeholder="Search icons..."
            className="w-full bg-secondary border border-input rounded-md pl-8 pr-3 py-2 text-sm text-foreground placeholder:text-muted-foreground outline-none focus:ring-1 focus:ring-ring"
          />
        </div>

        {/* Results */}
        <div className="max-h-[360px] overflow-y-auto">
          {loading ? (
            <div className="flex items-center justify-center py-10">
              <Loader2 size={20} className="animate-spin text-muted-foreground" />
            </div>
          ) : icons.length > 0 ? (
            <div className="grid grid-cols-6 gap-1">
              {icons.map((iconId) => {
                const { collection, name } = parseIconId(iconId)
                const isFetching = fetching === iconId
                return (
                  <button
                    key={iconId}
                    title={iconId}
                    onClick={() => handleSelect(iconId)}
                    disabled={isFetching}
                    className="w-10 h-10 flex items-center justify-center rounded hover:bg-accent cursor-pointer transition-colors disabled:opacity-50"
                  >
                    {isFetching ? (
                      <Loader2 size={16} className="animate-spin text-muted-foreground" />
                    ) : (
                      <img
                        src={`${ICONIFY_API}/${collection}/${name}.svg?height=20&color=${getIconColor()}`}
                        alt={iconId}
                        width={20}
                        height={20}
                        loading="lazy"
                      />
                    )}
                  </button>
                )
              })}
            </div>
          ) : searched ? (
            <p className="text-xs text-muted-foreground text-center py-10">
              No icons found
            </p>
          ) : (
            <p className="text-xs text-muted-foreground text-center py-10">
              Type to search Iconify icons
            </p>
          )}
        </div>

        {/* Footer */}
        <p className="text-[10px] text-muted-foreground mt-2 text-right">
          Powered by Iconify
        </p>
      </div>
    </div>
  )
}
