import { useState, useEffect, useRef } from 'react'
import { createPortal } from 'react-dom'
import { Loader2, Search } from 'lucide-react'

const ICONIFY_API = 'https://api.iconify.design'
const DEBOUNCE_MS = 250

function getIconColor(): string {
  const isLight =
    typeof document !== 'undefined' &&
    document.documentElement.classList.contains('light')
  return isLight ? '%23333333' : '%23e4e4e7'
}

function parseIconId(id: string) {
  const idx = id.indexOf(':')
  return { collection: id.slice(0, idx), name: id.slice(idx + 1) }
}

export interface IconPickerPosition {
  top: number
  right: number
}

interface IconPickerDialogProps {
  open: boolean
  onClose: () => void
  onSelect: (svgText: string, iconName: string) => void
  /** Pre-fill the search query when opened */
  initialQuery?: string
  /** Restrict results to this Iconify collection prefix */
  collectionFilter?: string
  /** Highlight this icon ID as the current selection */
  currentIconId?: string
  /** Position of the popover (anchored near trigger element) */
  position?: IconPickerPosition
}

export default function IconPickerDialog({
  open,
  onClose,
  onSelect,
  initialQuery,
  collectionFilter,
  currentIconId,
  position,
}: IconPickerDialogProps) {
  const [query, setQuery] = useState('')
  const [allIcons, setAllIcons] = useState<string[]>([])
  const [totalCount, setTotalCount] = useState<number | null>(null)
  const [collectionLoading, setCollectionLoading] = useState(false)
  // Global search (no collection context)
  const [searchIcons, setSearchIcons] = useState<string[]>([])
  const [searchLoading, setSearchLoading] = useState(false)
  const [searched, setSearched] = useState(false)
  const [fetching, setFetching] = useState<string | null>(null)

  const containerRef = useRef<HTMLDivElement>(null)
  const inputRef = useRef<HTMLInputElement>(null)
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null)

  // The active collection: explicit filter > current icon's collection > default Feather
  const activeCollection =
    collectionFilter ??
    (currentIconId ? currentIconId.split(':')[0] : undefined) ??
    'feather'

  // Load all icons for the active collection
  useEffect(() => {
    if (!activeCollection) return

    setCollectionLoading(true)
    setAllIcons([])
    setTotalCount(null)

    fetch(`${ICONIFY_API}/collection?prefix=${encodeURIComponent(activeCollection)}`)
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (!data) return
        // Iconify collection API stores names in multiple places:
        // - uncategorized: string[]  (most collections, e.g. lucide, ph)
        // - categories: Record<string, string[]>  (e.g. mdi)
        // - icons: Record<string, object>  (legacy / some collections)
        const names = new Set<string>()
        if (Array.isArray(data.uncategorized)) {
          for (const n of data.uncategorized) names.add(n)
        }
        if (data.categories && typeof data.categories === 'object') {
          for (const arr of Object.values(data.categories as Record<string, string[]>)) {
            if (Array.isArray(arr)) for (const n of arr) names.add(n)
          }
        }
        if (data.icons && typeof data.icons === 'object') {
          for (const n of Object.keys(data.icons)) names.add(n)
        }
        const list = [...names]
        setAllIcons(list.map((n) => `${activeCollection}:${n}`))
        setTotalCount(data.total ?? list.length)
      })
      .catch(() => {})
      .finally(() => setCollectionLoading(false))
  }, [activeCollection])

  // Focus + pre-fill on open
  useEffect(() => {
    if (open) {
      setQuery(initialQuery ?? '')
      setTimeout(() => inputRef.current?.focus(), 30)
    } else {
      setQuery('')
      setSearchIcons([])
      setSearched(false)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open])

  // Close on Escape
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

  // Close on outside click
  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    // Delay so the opening click doesn't immediately close it
    const id = setTimeout(() => document.addEventListener('mousedown', handler), 100)
    return () => {
      clearTimeout(id)
      document.removeEventListener('mousedown', handler)
    }
  }, [open, onClose])

  // Global search (used when no collection is loaded)
  const doGlobalSearch = (q: string) => {
    if (timerRef.current) clearTimeout(timerRef.current)
    if (!q.trim()) {
      setSearchIcons([])
      setSearchLoading(false)
      setSearched(false)
      return
    }
    setSearchLoading(true)
    timerRef.current = setTimeout(async () => {
      try {
        const res = await fetch(
          `${ICONIFY_API}/search?query=${encodeURIComponent(q.trim())}&limit=120`,
        )
        if (!res.ok) throw new Error()
        const data = await res.json()
        setSearchIcons(data.icons ?? [])
      } catch {
        setSearchIcons([])
      } finally {
        setSearchLoading(false)
        setSearched(true)
      }
    }, DEBOUNCE_MS)
  }

  const handleQueryChange = (val: string) => {
    setQuery(val)
    if (!activeCollection) doGlobalSearch(val)
  }

  const handleSelect = async (iconId: string) => {
    const { collection, name } = parseIconId(iconId)
    setFetching(iconId)
    try {
      const res = await fetch(`${ICONIFY_API}/${collection}/${name}.svg`)
      if (!res.ok) throw new Error()
      const svgText = await res.text()
      onSelect(svgText, iconId)
    } catch {
      // silently fail
    } finally {
      setFetching(null)
    }
  }

  if (!open || typeof document === 'undefined') return null

  // Determine icons to display
  const trimmedQuery = query.trim().toLowerCase()
  let displayIcons: string[]
  let isLoading: boolean

  if (activeCollection) {
    displayIcons = trimmedQuery
      ? allIcons.filter((id) => (id.split(':')[1] ?? '').includes(trimmedQuery))
      : allIcons
    isLoading = collectionLoading
  } else {
    displayIcons = searchIcons
    isLoading = searchLoading
  }

  const countLabel = activeCollection
    ? trimmedQuery
      ? `${displayIcons.length} icons`
      : `${totalCount ?? displayIcons.length} icons`
    : null

  // Compute popover position: anchored to the left of the property panel
  const PANEL_WIDTH = 256 // property panel w-64
  const PICKER_WIDTH = 280
  const PICKER_MAX_HEIGHT = 480
  const MARGIN = 8

  const top = position
    ? Math.min(
        Math.max(position.top, MARGIN),
        window.innerHeight - PICKER_MAX_HEIGHT - MARGIN,
      )
    : Math.round(window.innerHeight / 2 - PICKER_MAX_HEIGHT / 2)

  const right = PANEL_WIDTH + MARGIN

  const panelStyle: React.CSSProperties = {
    position: 'fixed',
    top,
    right,
    width: PICKER_WIDTH,
    maxHeight: PICKER_MAX_HEIGHT,
    zIndex: 9999,
  }

  return createPortal(
    <div
      ref={containerRef}
      style={panelStyle}
      className="bg-card border border-border rounded-lg shadow-xl"
    >
      {/* Search */}
      <div className="relative p-2 border-b border-border">
        <Search
          size={13}
          className="absolute left-4 top-1/2 -translate-y-1/2 text-muted-foreground"
        />
        <input
          ref={inputRef}
          type="text"
          value={query}
          onChange={(e) => handleQueryChange(e.target.value)}
          placeholder="Search icons..."
          className="w-full bg-secondary border border-transparent rounded pl-7 pr-3 py-1.5 text-[12px] text-foreground placeholder:text-muted-foreground outline-none focus:border-ring"
        />
      </div>

      {/* Count */}
      {!isLoading && countLabel && (
        <p className="text-[11px] text-muted-foreground px-3 py-1.5">{countLabel}</p>
      )}

      {/* Grid */}
      <div className="overflow-y-auto p-2" style={{ maxHeight: 380 }}>
        {isLoading ? (
          <div className="flex items-center justify-center py-10">
            <Loader2 size={18} className="animate-spin text-muted-foreground" />
          </div>
        ) : displayIcons.length > 0 ? (
          <div className="grid grid-cols-6 gap-0.5">
            {displayIcons.map((iconId) => {
              const { collection, name } = parseIconId(iconId)
              const isFetching = fetching === iconId
              const isCurrent = iconId === currentIconId
              return (
                <button
                  key={iconId}
                  title={iconId}
                  onClick={() => handleSelect(iconId)}
                  disabled={isFetching}
                  className={`w-10 h-10 flex items-center justify-center rounded transition-colors disabled:opacity-50 ${
                    isCurrent
                      ? 'bg-primary/15 ring-1 ring-inset ring-primary'
                      : 'hover:bg-accent cursor-pointer'
                  }`}
                >
                  {isFetching ? (
                    <Loader2 size={14} className="animate-spin text-muted-foreground" />
                  ) : (
                    <img
                      src={`${ICONIFY_API}/${collection}/${name}.svg?height=18&color=${getIconColor()}`}
                      alt={name}
                      width={18}
                      height={18}
                      loading="lazy"
                    />
                  )}
                </button>
              )
            })}
          </div>
        ) : searched && !activeCollection ? (
          <p className="text-xs text-muted-foreground text-center py-8">No icons found</p>
        ) : !activeCollection && !trimmedQuery ? (
          <p className="text-xs text-muted-foreground text-center py-8">
            Type to search Iconify icons
          </p>
        ) : null}
      </div>
    </div>,
    document.body,
  )
}
