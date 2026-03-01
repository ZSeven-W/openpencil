import { useState, useRef, useEffect } from 'react'
import { Plus, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'

export default function PageTabs() {
  const pages = useDocumentStore((s) => s.document.pages)
  const activePageId = useCanvasStore((s) => s.activePageId)
  const setActivePageId = useCanvasStore((s) => s.setActivePageId)
  const addPage = useDocumentStore((s) => s.addPage)
  const removePage = useDocumentStore((s) => s.removePage)
  const renamePage = useDocumentStore((s) => s.renamePage)
  const reorderPage = useDocumentStore((s) => s.reorderPage)
  const duplicatePage = useDocumentStore((s) => s.duplicatePage)

  const [editingId, setEditingId] = useState<string | null>(null)
  const [editValue, setEditValue] = useState('')
  const [contextMenu, setContextMenu] = useState<{
    x: number
    y: number
    pageId: string
  } | null>(null)
  const inputRef = useRef<HTMLInputElement>(null)

  if (!pages || pages.length === 0) return null

  const canDelete = pages.length > 1

  const handleSwitchPage = (pageId: string) => {
    if (pageId === activePageId) return
    useCanvasStore.getState().clearSelection()
    useCanvasStore.getState().exitAllFrames()
    const canvas = useCanvasStore.getState().fabricCanvas
    if (canvas) {
      canvas.discardActiveObject()
    }
    setActivePageId(pageId)
    requestAnimationFrame(() => zoomToFitContent())
  }

  const handleDoubleClick = (pageId: string, name: string) => {
    setEditingId(pageId)
    setEditValue(name)
    requestAnimationFrame(() => inputRef.current?.select())
  }

  const commitRename = () => {
    if (editingId && editValue.trim()) {
      renamePage(editingId, editValue.trim())
    }
    setEditingId(null)
  }

  const handleContextMenu = (e: React.MouseEvent, pageId: string) => {
    e.preventDefault()
    setContextMenu({ x: e.clientX, y: e.clientY, pageId })
  }

  const handleAdd = () => {
    addPage()
    requestAnimationFrame(() => zoomToFitContent())
  }

  const handleClose = (e: React.MouseEvent, pageId: string) => {
    e.stopPropagation()
    if (!canDelete) return
    removePage(pageId)
    requestAnimationFrame(() => zoomToFitContent())
  }

  return (
    <>
      <div className="flex flex-col">
        {/* Header */}
        <div className="h-8 flex items-center justify-between px-3 border-b border-border">
          <span className="text-xs font-medium text-muted-foreground tracking-wider">
            Pages
          </span>
          <button
            className="w-5 h-5 flex items-center justify-center text-muted-foreground hover:text-foreground rounded transition-colors"
            onClick={handleAdd}
            title="Add page"
          >
            <Plus className="w-3.5 h-3.5" />
          </button>
        </div>

        {/* Page list */}
        <div className="py-1 px-1">
          {pages.map((page) => {
            const isActive = page.id === activePageId
            return (
              <button
                key={page.id}
                className={cn(
                  'group w-full flex items-center h-7 px-2 rounded-md text-xs select-none transition-colors',
                  isActive
                    ? 'bg-accent text-foreground font-medium'
                    : 'text-muted-foreground hover:text-foreground hover:bg-accent/50',
                )}
                onClick={() => handleSwitchPage(page.id)}
                onDoubleClick={() => handleDoubleClick(page.id, page.name)}
                onContextMenu={(e) => handleContextMenu(e, page.id)}
              >
                {editingId === page.id ? (
                  <input
                    ref={inputRef}
                    className="flex-1 bg-transparent outline-none text-xs text-foreground min-w-0"
                    value={editValue}
                    onChange={(e) => setEditValue(e.target.value)}
                    onBlur={commitRename}
                    onKeyDown={(e) => {
                      if (e.key === 'Enter') commitRename()
                      if (e.key === 'Escape') setEditingId(null)
                    }}
                    onClick={(e) => e.stopPropagation()}
                  />
                ) : (
                  <>
                    <span className="flex-1 text-left truncate">{page.name}</span>
                    {canDelete && (
                      <span
                        role="button"
                        className={cn(
                          'shrink-0 inline-flex items-center justify-center w-4 h-4 rounded-sm transition-colors',
                          isActive
                            ? 'opacity-0 group-hover:opacity-60 hover:!opacity-100 hover:bg-foreground/10'
                            : 'opacity-0 group-hover:opacity-60 hover:!opacity-100 hover:bg-foreground/10',
                        )}
                        onClick={(e) => handleClose(e, page.id)}
                      >
                        <X className="w-2.5 h-2.5" />
                      </span>
                    )}
                  </>
                )}
              </button>
            )
          })}
        </div>
      </div>

      {/* Context menu */}
      {contextMenu && (
        <PageContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          canDelete={canDelete}
          onRename={() => {
            const page = pages?.find((p) => p.id === contextMenu.pageId)
            if (page) handleDoubleClick(page.id, page.name)
            setContextMenu(null)
          }}
          onDuplicate={() => {
            duplicatePage(contextMenu.pageId)
            setContextMenu(null)
            requestAnimationFrame(() => zoomToFitContent())
          }}
          onDelete={() => {
            removePage(contextMenu.pageId)
            setContextMenu(null)
            requestAnimationFrame(() => zoomToFitContent())
          }}
          onMoveLeft={() => {
            reorderPage(contextMenu.pageId, 'left')
            setContextMenu(null)
          }}
          onMoveRight={() => {
            reorderPage(contextMenu.pageId, 'right')
            setContextMenu(null)
          }}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  )
}

function PageContextMenu({
  x,
  y,
  canDelete,
  onRename,
  onDuplicate,
  onDelete,
  onMoveLeft,
  onMoveRight,
  onClose,
}: {
  x: number
  y: number
  canDelete: boolean
  onRename: () => void
  onDuplicate: () => void
  onDelete: () => void
  onMoveLeft: () => void
  onMoveRight: () => void
  onClose: () => void
}) {
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    const handleEsc = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', handleClickOutside)
    document.addEventListener('keydown', handleEsc)
    return () => {
      document.removeEventListener('mousedown', handleClickOutside)
      document.removeEventListener('keydown', handleEsc)
    }
  }, [onClose])

  const items = [
    { label: 'Rename', action: onRename },
    { label: 'Duplicate', action: onDuplicate },
    { label: 'Move Up', action: onMoveLeft },
    { label: 'Move Down', action: onMoveRight },
    ...(canDelete
      ? [{ label: 'Delete', action: onDelete, danger: true }]
      : []),
  ]

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-popover border border-border rounded-md shadow-md py-1 min-w-[140px]"
      style={{ left: x, top: y }}
    >
      {items.map((item) => (
        <button
          key={item.label}
          className={cn(
            'w-full text-left px-3 py-1.5 text-xs hover:bg-accent transition-colors',
            'danger' in item && item.danger
              ? 'text-destructive'
              : 'text-popover-foreground',
          )}
          onClick={item.action}
        >
          {item.label}
        </button>
      ))}
    </div>
  )
}
