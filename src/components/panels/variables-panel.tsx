import { useState, useMemo, useCallback, useRef, useEffect } from 'react'
import { X, Plus, ChevronDown, Search, Pencil, Trash2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import VariableRow from './variable-row'
import type { VariableDefinition, ThemedValue } from '@/types/variables'

const DEFAULT_THEME_AXIS = 'Theme-1'
const DEFAULT_THEME_VALUES = ['Default']
const MIN_WIDTH = 480
const MIN_HEIGHT = 240
const DEFAULT_WIDTH = 820
const DEFAULT_HEIGHT = 480

export default function VariablesPanel() {
  const variables = useDocumentStore((s) => s.document.variables)
  const themes = useDocumentStore((s) => s.document.themes)
  const setVariable = useDocumentStore((s) => s.setVariable)
  const removeVariable = useDocumentStore((s) => s.removeVariable)
  const renameVariable = useDocumentStore((s) => s.renameVariable)
  const setThemes = useDocumentStore((s) => s.setThemes)
  const toggleVariablesPanel = useCanvasStore((s) => s.toggleVariablesPanel)

  const [search, setSearch] = useState('')
  const [showAddMenu, setShowAddMenu] = useState(false)
  const [activeAxis, setActiveAxis] = useState<string | null>(null)
  // Theme tab dropdown (Rename/Delete)
  const [activeThemeMenu, setActiveThemeMenu] = useState<string | null>(null)
  const [renamingTheme, setRenamingTheme] = useState<string | null>(null)
  const [renameThemeValue, setRenameThemeValue] = useState('')
  // Variant column dropdown (Rename/Delete)
  const [activeColumnMenu, setActiveColumnMenu] = useState<string | null>(null)
  const [renamingColumn, setRenamingColumn] = useState<string | null>(null)
  const [renameColumnValue, setRenameColumnValue] = useState('')
  // Panel size
  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH)
  const [panelHeight, setPanelHeight] = useState(DEFAULT_HEIGHT)

  const themeMenuRef = useRef<HTMLDivElement>(null)
  const addMenuRef = useRef<HTMLDivElement>(null)
  const columnMenuRef = useRef<HTMLDivElement>(null)
  const renameInputRef = useRef<HTMLInputElement>(null)
  const themeRenameInputRef = useRef<HTMLInputElement>(null)
  const panelRef = useRef<HTMLDivElement>(null)
  const resizeRef = useRef<{
    edge: 'right' | 'bottom' | 'corner'
    startX: number; startY: number; startW: number; startH: number
  } | null>(null)

  // Close menus on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (activeThemeMenu && themeMenuRef.current && !themeMenuRef.current.contains(e.target as Node))
        { setActiveThemeMenu(null); setRenamingTheme(null) }
      if (showAddMenu && addMenuRef.current && !addMenuRef.current.contains(e.target as Node))
        setShowAddMenu(false)
      if (activeColumnMenu && columnMenuRef.current && !columnMenuRef.current.contains(e.target as Node))
        setActiveColumnMenu(null)
    }
    if (activeThemeMenu || showAddMenu || activeColumnMenu) {
      document.addEventListener('mousedown', handler)
      return () => document.removeEventListener('mousedown', handler)
    }
  }, [activeThemeMenu, showAddMenu, activeColumnMenu])

  useEffect(() => {
    if (renamingColumn && renameInputRef.current) {
      renameInputRef.current.focus()
      renameInputRef.current.select()
    }
  }, [renamingColumn])

  useEffect(() => {
    if (renamingTheme && themeRenameInputRef.current) {
      themeRenameInputRef.current.focus()
      themeRenameInputRef.current.select()
    }
  }, [renamingTheme])

  /* --- Resize --- */
  const handleResizeStart = useCallback((edge: 'right' | 'bottom' | 'corner', e: React.PointerEvent) => {
    e.preventDefault()
    e.stopPropagation()
    resizeRef.current = { edge, startX: e.clientX, startY: e.clientY, startW: panelWidth, startH: panelHeight }
    e.currentTarget.setPointerCapture(e.pointerId)
  }, [panelWidth, panelHeight])

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

  /* --- Theme axes & variants --- */
  const themeAxes = useMemo(() => {
    if (!themes) return []
    return Object.keys(themes)
  }, [themes])

  const currentAxis = useMemo(() => {
    if (activeAxis && themes?.[activeAxis]) return activeAxis
    if (themeAxes.length > 0) return themeAxes[0]
    return null
  }, [activeAxis, themes, themeAxes])

  const themeValues = useMemo(() => {
    if (!currentAxis || !themes?.[currentAxis]) return DEFAULT_THEME_VALUES
    return themes[currentAxis].length > 0 ? themes[currentAxis] : DEFAULT_THEME_VALUES
  }, [themes, currentAxis])

  const themeAxis = currentAxis ?? DEFAULT_THEME_AXIS

  const ensureThemes = useCallback(() => {
    if (!themes || Object.keys(themes).length === 0) {
      setThemes({ [DEFAULT_THEME_AXIS]: DEFAULT_THEME_VALUES })
    }
  }, [themes, setThemes])

  const entries = useMemo(() => {
    if (!variables) return []
    return Object.entries(variables)
      .filter(([n]) => !search || n.toLowerCase().includes(search.toLowerCase()))
      .sort(([a], [b]) => a.localeCompare(b))
  }, [variables, search])

  /* --- Theme actions --- */
  const handleAddTheme = () => {
    const current = themes ?? {}
    let counter = 1
    let name = `Theme-${counter}`
    while (current[name]) { counter++; name = `Theme-${counter}` }
    setThemes({ ...current, [name]: ['Default'] })
    setActiveAxis(name)
  }

  const handleDeleteTheme = (axis: string) => {
    if (!themes) return
    const updated = { ...themes }
    delete updated[axis]
    setThemes(updated)
    if (activeAxis === axis) setActiveAxis(null)
    setActiveThemeMenu(null)
  }

  const handleRenameTheme = (oldName: string, newName: string) => {
    setRenamingTheme(null)
    setActiveThemeMenu(null)
    if (!newName.trim() || newName === oldName) return
    if (themes?.[newName]) return
    const current = themes ?? {}
    const values = current[oldName] ?? DEFAULT_THEME_VALUES
    const updated: Record<string, string[]> = {}
    for (const key of Object.keys(current)) {
      if (key === oldName) updated[newName] = values
      else updated[key] = current[key]
    }
    setThemes(updated)
    if (activeAxis === oldName) setActiveAxis(newName)
  }

  /* --- Variant actions --- */
  const handleAddVariant = () => {
    ensureThemes()
    const axis = currentAxis ?? DEFAULT_THEME_AXIS
    const currentValues = themes?.[axis] ?? DEFAULT_THEME_VALUES
    let counter = 1
    let n = `Variant-${counter}`
    while (currentValues.includes(n)) { counter++; n = `Variant-${counter}` }
    const updatedThemes = { ...(themes ?? { [DEFAULT_THEME_AXIS]: DEFAULT_THEME_VALUES }) }
    updatedThemes[axis] = [...currentValues, n]
    setThemes(updatedThemes)
  }

  const handleRemoveVariant = (value: string) => {
    if (!currentAxis || !themes) return
    const currentValues = themes[currentAxis] ?? []
    if (currentValues.length <= 1) return
    setThemes({ ...themes, [currentAxis]: currentValues.filter((v) => v !== value) })
    setActiveColumnMenu(null)
  }

  const handleRenameVariant = (oldName: string, newName: string) => {
    if (!newName.trim() || newName === oldName) { setRenamingColumn(null); return }
    if (!currentAxis || !themes) { setRenamingColumn(null); return }
    const currentValues = themes[currentAxis] ?? []
    if (currentValues.includes(newName)) { setRenamingColumn(null); return }
    setThemes({ ...themes, [currentAxis]: currentValues.map((v) => v === oldName ? newName : v) })
    setRenamingColumn(null)
  }

  const startRenameVariant = (tv: string) => {
    setRenameColumnValue(tv)
    setRenamingColumn(tv)
    setActiveColumnMenu(null)
  }

  /* --- Add variable --- */
  const handleAdd = (type: VariableDefinition['type']) => {
    ensureThemes()
    const existing = variables ? Object.keys(variables) : []
    let counter = 1
    const baseName = type === 'color' ? 'color' : type === 'number' ? 'number' : 'string'
    let varName = `${baseName}-${counter}`
    while (existing.includes(varName)) { counter++; varName = `${baseName}-${counter}` }
    const currentTV = themes?.[themeAxis] ?? DEFAULT_THEME_VALUES
    let defaultValue: VariableDefinition['value']
    if (currentTV.length > 1) {
      defaultValue = currentTV.map((tv) => ({
        value: type === 'color' ? '#000000' : type === 'number' ? 0 : '',
        theme: { [themeAxis]: tv },
      })) as ThemedValue[]
    } else {
      defaultValue = type === 'color' ? '#000000' : type === 'number' ? 0 : ''
    }
    setVariable(varName, { type, value: defaultValue })
    setShowAddMenu(false)
  }

  return (
    <div
      ref={panelRef}
      className="absolute left-14 top-2 z-20 flex flex-col select-none"
      style={{ width: panelWidth, height: panelHeight }}
    >
      {/* Background layer with rounded corners — sits behind everything */}
      <div className="absolute inset-0 bg-card/95 backdrop-blur-sm border border-border/80 rounded-2xl shadow-2xl pointer-events-none" />

      {/* ── Header: Theme-1 | Theme-2 | ... | + | spacer | X ── */}
      <div className="relative h-11 flex items-center px-4 shrink-0 gap-1 z-10">
        {/* Theme tabs — all equal, active one has chevron dropdown */}
        {themeAxes.map((axis) => (
          <div key={axis} className="relative shrink-0" ref={activeThemeMenu === axis ? themeMenuRef : undefined}>
            {renamingTheme === axis ? (
              <input
                ref={themeRenameInputRef}
                type="text"
                value={renameThemeValue}
                onChange={(e) => setRenameThemeValue(e.target.value)}
                onBlur={() => handleRenameTheme(axis, renameThemeValue)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleRenameTheme(axis, renameThemeValue)
                  if (e.key === 'Escape') { setRenamingTheme(null); setActiveThemeMenu(null) }
                }}
                className="text-[13px] text-foreground bg-secondary px-2 py-0.5 rounded-lg border border-ring focus:outline-none w-24"
              />
            ) : (
              <button
                type="button"
                onClick={() => {
                  if (axis === currentAxis) {
                    // Already active → toggle dropdown
                    setActiveThemeMenu(activeThemeMenu === axis ? null : axis)
                  } else {
                    // Switch to this theme
                    setActiveAxis(axis)
                    setActiveThemeMenu(null)
                  }
                }}
                className={cn(
                  'flex items-center gap-1 text-[13px] px-2 py-1 rounded-lg transition-colors whitespace-nowrap',
                  axis === currentAxis
                    ? 'text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground',
                )}
              >
                {axis}
                {axis === currentAxis && (
                  <ChevronDown size={11} className={cn(
                    'text-muted-foreground/60 transition-transform',
                    activeThemeMenu === axis && 'rotate-180',
                  )} />
                )}
              </button>
            )}
            {/* Theme dropdown: Rename / Delete */}
            {activeThemeMenu === axis && !renamingTheme && (
              <div className="absolute left-0 top-full z-50 mt-1 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150">
                <button
                  type="button"
                  onClick={() => { setRenameThemeValue(axis); setRenamingTheme(axis); setActiveThemeMenu(null) }}
                  className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                >
                  <Pencil size={14} className="text-muted-foreground" />
                  Rename
                </button>
                {themeAxes.length > 1 && (
                  <button
                    type="button"
                    onClick={() => handleDeleteTheme(axis)}
                    className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                  >
                    <Trash2 size={14} className="text-muted-foreground" />
                    Delete
                  </button>
                )}
              </div>
            )}
          </div>
        ))}

        {/* + add theme */}
        <button
          type="button"
          onClick={handleAddTheme}
          className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors shrink-0"
          title="Add theme"
        >
          <Plus size={15} />
        </button>

        <div className="flex-1" />

        <button
          type="button"
          onClick={toggleVariablesPanel}
          className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-secondary/60 transition-colors shrink-0"
          title="Close (⌘⇧V)"
        >
          <X size={16} />
        </button>
      </div>

      {/* ── Column headers: Name | Default | Variant-1 | ... | + ── */}
      <div className="relative flex items-center px-4 h-9 shrink-0 border-t border-b border-border/40 z-10">
        <div className="w-[220px] shrink-0">
          <span className="text-[13px] font-medium text-muted-foreground">Name</span>
        </div>
        {themeValues.map((tv) => (
          <div key={tv} className="flex-1 min-w-0 pl-4 relative" ref={activeColumnMenu === tv ? columnMenuRef : undefined}>
            {renamingColumn === tv ? (
              <input
                ref={renameInputRef}
                type="text"
                value={renameColumnValue}
                onChange={(e) => setRenameColumnValue(e.target.value)}
                onBlur={() => handleRenameVariant(tv, renameColumnValue)}
                onKeyDown={(e) => {
                  if (e.key === 'Enter') handleRenameVariant(tv, renameColumnValue)
                  if (e.key === 'Escape') setRenamingColumn(null)
                }}
                className="text-[13px] font-medium text-foreground bg-secondary px-1.5 py-0.5 rounded border border-ring focus:outline-none w-32"
              />
            ) : (
              <button
                type="button"
                onClick={() => setActiveColumnMenu(activeColumnMenu === tv ? null : tv)}
                className="flex items-center gap-1 text-[13px] font-medium text-muted-foreground hover:text-foreground transition-colors"
              >
                {tv}
                <ChevronDown size={11} className={cn(
                  'text-muted-foreground/60 transition-transform',
                  activeColumnMenu === tv && 'rotate-180',
                )} />
              </button>
            )}
            {activeColumnMenu === tv && (
              <div className="absolute left-4 top-full z-50 mt-1 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150">
                <button
                  type="button"
                  onClick={() => startRenameVariant(tv)}
                  className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                >
                  <Pencil size={14} className="text-muted-foreground" />
                  Rename
                </button>
                {themeValues.length > 1 && (
                  <button
                    type="button"
                    onClick={() => handleRemoveVariant(tv)}
                    className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
                  >
                    <Trash2 size={14} className="text-muted-foreground" />
                    Delete
                  </button>
                )}
              </div>
            )}
          </div>
        ))}
        <div className="w-[44px] shrink-0 flex justify-center">
          <button
            type="button"
            onClick={handleAddVariant}
            className="p-1 rounded text-muted-foreground/50 hover:text-foreground transition-colors"
            title="Add variant"
          >
            <Plus size={14} />
          </button>
        </div>
      </div>

      {/* ── Search ── */}
      {entries.length > 6 && (
        <div className="relative px-4 py-2 shrink-0 border-b border-border/30">
          <div className="flex items-center gap-2 bg-secondary/40 rounded-lg px-2.5 h-7 border border-transparent focus-within:border-ring transition-colors">
            <Search size={13} className="text-muted-foreground/60 shrink-0" />
            <input
              type="text"
              placeholder="Search variables..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="flex-1 bg-transparent text-foreground text-[12px] focus:outline-none placeholder:text-muted-foreground/40"
            />
          </div>
        </div>
      )}

      {/* ── Variable rows ── */}
      <div className="relative flex-1 overflow-y-auto overflow-x-auto min-h-0 px-2 py-0.5">
        {entries.length === 0 && (
          <div className="flex flex-col items-center justify-center h-full gap-1.5">
            <span className="text-[13px] text-muted-foreground/50">
              {search ? 'No variables match your search' : 'No variables defined'}
            </span>
          </div>
        )}
        {entries.map(([varName, def]) => (
          <VariableRow
            key={varName}
            name={varName}
            definition={def}
            themeValues={themeValues}
            themeAxis={themeAxis}
            onUpdateValue={(n, d) => setVariable(n, d)}
            onRename={(o, n) => renameVariable(o, n)}
            onDelete={(n) => removeVariable(n)}
          />
        ))}
      </div>

      {/* ── Footer ── */}
      <div className="relative h-10 flex items-center px-4 shrink-0 border-t border-border/30 z-10" ref={addMenuRef}>
        <button
          type="button"
          onClick={() => setShowAddMenu(!showAddMenu)}
          className={cn(
            'flex items-center gap-2 text-[13px] transition-colors',
            showAddMenu ? 'text-foreground' : 'text-muted-foreground hover:text-foreground',
          )}
        >
          <Plus size={14} />
          Add variable
          <ChevronDown size={11} className={cn('transition-transform', showAddMenu && 'rotate-180')} />
        </button>
        {showAddMenu && (
          <div className="absolute left-4 bottom-full z-50 mb-1.5 w-44 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-bottom-1 duration-150">
            {(['color', 'number', 'string'] as const).map((t) => (
              <button
                key={t}
                type="button"
                onClick={() => handleAdd(t)}
                className="w-full text-left px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg capitalize transition-colors"
              >
                {t}
              </button>
            ))}
          </div>
        )}
      </div>

      {/* ── Resize handles ── */}
      <div
        className="absolute top-0 right-0 w-1.5 h-full cursor-ew-resize hover:bg-primary/10 transition-colors z-10"
        onPointerDown={(e) => handleResizeStart('right', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute bottom-0 left-0 w-full h-1.5 cursor-ns-resize hover:bg-primary/10 transition-colors z-10"
        onPointerDown={(e) => handleResizeStart('bottom', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
      <div
        className="absolute bottom-0 right-0 w-3 h-3 cursor-nwse-resize hover:bg-primary/15 transition-colors rounded-br-2xl z-10"
        onPointerDown={(e) => handleResizeStart('corner', e)}
        onPointerMove={handleResizeMove}
        onPointerUp={handleResizeEnd}
      />
    </div>
  )
}
