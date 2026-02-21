import { useState, useRef, useCallback, useMemo } from 'react'
import { X, Search, Upload, Download, Trash2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useUIKitStore } from '@/stores/uikit-store'
import { useDocumentStore } from '@/stores/document-store'
import { importKitFromFile, exportKit } from '@/uikit/kit-import-export'
import ComponentBrowserGrid from './component-browser-grid'
import type { ComponentCategory } from '@/types/uikit'

const MIN_WIDTH = 420
const MIN_HEIGHT = 300
const DEFAULT_WIDTH = 520
const DEFAULT_HEIGHT = 460

const CATEGORIES: { value: ComponentCategory | null; label: string }[] = [
  { value: null, label: 'All' },
  { value: 'buttons', label: 'Buttons' },
  { value: 'inputs', label: 'Inputs' },
  { value: 'cards', label: 'Cards' },
  { value: 'navigation', label: 'Nav' },
  { value: 'layout', label: 'Layout' },
  { value: 'feedback', label: 'Feedback' },
  { value: 'data-display', label: 'Data' },
  { value: 'other', label: 'Other' },
]

export default function ComponentBrowserPanel() {
  const kits = useUIKitStore((s) => s.kits)
  const searchQuery = useUIKitStore((s) => s.searchQuery)
  const setSearchQuery = useUIKitStore((s) => s.setSearchQuery)
  const activeCategory = useUIKitStore((s) => s.activeCategory)
  const setActiveCategory = useUIKitStore((s) => s.setActiveCategory)
  const activeKitId = useUIKitStore((s) => s.activeKitId)
  const setActiveKitId = useUIKitStore((s) => s.setActiveKitId)
  const toggleBrowser = useUIKitStore((s) => s.toggleBrowser)
  const importKitAction = useUIKitStore((s) => s.importKit)
  const removeKit = useUIKitStore((s) => s.removeKit)

  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH)
  const [panelHeight, setPanelHeight] = useState(DEFAULT_HEIGHT)
  const [confirmDeleteKitId, setConfirmDeleteKitId] = useState<string | null>(null)

  const panelRef = useRef<HTMLDivElement>(null)
  const resizeRef = useRef<{
    edge: 'right' | 'bottom' | 'corner'
    startX: number; startY: number; startW: number; startH: number
  } | null>(null)

  /* --- Resize --- */
  const handleResizeStart = useCallback(
    (edge: 'right' | 'bottom' | 'corner', e: React.PointerEvent) => {
      e.preventDefault()
      e.stopPropagation()
      resizeRef.current = {
        edge,
        startX: e.clientX,
        startY: e.clientY,
        startW: panelWidth,
        startH: panelHeight,
      }
      e.currentTarget.setPointerCapture(e.pointerId)
    },
    [panelWidth, panelHeight],
  )

  const handleResizeMove = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return
    e.preventDefault()
    const { edge, startX, startY, startW, startH } = resizeRef.current
    const container = panelRef.current?.parentElement
    const maxW = container ? container.clientWidth - 72 : 1400
    const maxH = container ? container.clientHeight - 16 : 900
    if (edge === 'right' || edge === 'corner')
      setPanelWidth(Math.max(MIN_WIDTH, Math.min(maxW, startW + e.clientX - startX)))
    if (edge === 'bottom' || edge === 'corner')
      setPanelHeight(Math.max(MIN_HEIGHT, Math.min(maxH, startH + e.clientY - startY)))
  }, [])

  const handleResizeEnd = useCallback((e: React.PointerEvent) => {
    if (!resizeRef.current) return
    resizeRef.current = null
    e.currentTarget.releasePointerCapture(e.pointerId)
  }, [])

  /* --- Import --- */
  const handleImport = useCallback(async () => {
    const kit = await importKitFromFile()
    if (kit) {
      importKitAction(kit)
    }
  }, [importKitAction])

  /* --- Export --- */
  const handleExport = useCallback(async () => {
    const doc = useDocumentStore.getState().document
    await exportKit(doc, [], doc.name ?? 'My Kit')
  }, [])

  /* --- Delete imported kit --- */
  const handleDeleteKit = useCallback(
    (kitId: string) => {
      removeKit(kitId)
      setConfirmDeleteKitId(null)
    },
    [removeKit],
  )

  /* --- Imported kits for delete list --- */
  const importedKits = useMemo(() => kits.filter((k) => !k.builtIn), [kits])

  /* --- Visible categories (only show tabs that have components) --- */
  const visibleCategories = useMemo(() => {
    const categorySet = new Set<ComponentCategory>()
    const targetKits = activeKitId ? kits.filter((k) => k.id === activeKitId) : kits
    for (const kit of targetKits) {
      for (const comp of kit.components) {
        categorySet.add(comp.category)
      }
    }
    return CATEGORIES.filter((c) => c.value === null || categorySet.has(c.value))
  }, [kits, activeKitId])

  return (
    <div
      ref={panelRef}
      className="absolute left-14 top-2 z-20 flex flex-col select-none"
      style={{ width: panelWidth, height: panelHeight }}
    >
      {/* Background */}
      <div className="absolute inset-0 bg-card/95 backdrop-blur-sm border border-border rounded-2xl shadow-2xl" />

      {/* Header */}
      <div className="relative h-10 flex items-center justify-between px-3 border-b border-border shrink-0">
        <span className="text-sm font-medium text-foreground">UIKit Browser</span>
        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={handleExport}
            className="inline-flex items-center justify-center h-6 w-6 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
            title="Export kit"
          >
            <Download size={14} />
          </button>
          <button
            type="button"
            onClick={handleImport}
            className="inline-flex items-center justify-center h-6 w-6 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
            title="Import kit"
          >
            <Upload size={14} />
          </button>
          <button
            type="button"
            onClick={toggleBrowser}
            className="inline-flex items-center justify-center h-6 w-6 rounded hover:bg-muted text-muted-foreground hover:text-foreground transition-colors"
          >
            <X size={14} />
          </button>
        </div>
      </div>

      {/* Kit filter + Category tabs */}
      <div className="relative shrink-0">
        {/* Kit selector row */}
        <div className="flex items-center gap-1.5 px-3 py-1.5 border-b border-border">
          <span className="text-xs text-muted-foreground shrink-0">Kit:</span>
          <select
            value={activeKitId ?? ''}
            onChange={(e) => setActiveKitId(e.target.value || null)}
            className="text-xs bg-transparent border border-border rounded px-1.5 py-0.5 text-foreground outline-none min-w-0"
          >
            <option value="">All</option>
            {kits.map((k) => (
              <option key={k.id} value={k.id}>
                {k.name}{k.builtIn ? '' : ' (imported)'}
              </option>
            ))}
          </select>
        </div>

        {/* Imported kits list — each with individual delete */}
        {importedKits.length > 0 && (
          <div className="flex flex-col gap-0 border-b border-border">
            {importedKits.map((kit) => (
              <div
                key={kit.id}
                className="flex items-center gap-2 px-3 py-1 hover:bg-muted/50 transition-colors"
              >
                <span className="text-xs text-foreground truncate flex-1">{kit.name}</span>
                <span className="text-[10px] text-muted-foreground shrink-0">
                  {kit.components.length} components
                </span>
                {confirmDeleteKitId === kit.id ? (
                  <div className="flex items-center gap-1 shrink-0">
                    <button
                      type="button"
                      onClick={() => handleDeleteKit(kit.id)}
                      className="text-[10px] px-1.5 py-0.5 rounded bg-destructive text-destructive-foreground hover:bg-destructive/90 transition-colors"
                    >
                      Delete
                    </button>
                    <button
                      type="button"
                      onClick={() => setConfirmDeleteKitId(null)}
                      className="text-[10px] px-1.5 py-0.5 rounded bg-muted text-muted-foreground hover:text-foreground transition-colors"
                    >
                      Cancel
                    </button>
                  </div>
                ) : (
                  <button
                    type="button"
                    onClick={() => setConfirmDeleteKitId(kit.id)}
                    className="inline-flex items-center justify-center h-5 w-5 rounded hover:bg-destructive/10 text-muted-foreground hover:text-destructive transition-colors shrink-0"
                    title={`Delete ${kit.name}`}
                  >
                    <Trash2 size={12} />
                  </button>
                )}
              </div>
            ))}
          </div>
        )}

        {/* Category pills */}
        <div className="flex items-center gap-1 px-3 py-1.5 border-b border-border overflow-x-auto">
          {visibleCategories.map((cat) => (
            <button
              key={cat.label}
              type="button"
              onClick={() => setActiveCategory(cat.value)}
              className={cn(
                'px-2.5 py-1 text-xs rounded-full whitespace-nowrap transition-colors',
                activeCategory === cat.value
                  ? 'bg-primary text-primary-foreground'
                  : 'bg-muted text-muted-foreground hover:text-foreground',
              )}
            >
              {cat.label}
            </button>
          ))}
        </div>
      </div>

      {/* Search */}
      <div className="relative shrink-0 px-3 py-2 border-b border-border">
        <div className="flex items-center gap-2 bg-muted rounded-lg px-2.5 py-1.5">
          <Search size={14} className="text-muted-foreground shrink-0" />
          <input
            type="text"
            placeholder="Search components..."
            value={searchQuery}
            onChange={(e) => setSearchQuery(e.target.value)}
            className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground outline-none"
          />
        </div>
      </div>

      {/* Grid */}
      <div className="relative flex-1 overflow-y-auto overflow-x-hidden min-h-0">
        <ComponentBrowserGrid
          kits={kits}
          searchQuery={searchQuery}
          activeCategory={activeCategory}
          activeKitId={activeKitId}
        />
      </div>

      {/* Resize handles */}
      <div
        className="absolute top-0 -right-1 w-2 h-full cursor-ew-resize"
        onPointerDown={(e) => handleResizeStart('right', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute -bottom-1 left-0 w-full h-2 cursor-ns-resize"
        onPointerDown={(e) => handleResizeStart('bottom', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute -bottom-1 -right-1 w-4 h-4 cursor-nwse-resize"
        onPointerDown={(e) => handleResizeStart('corner', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
    </div>
  )
}
