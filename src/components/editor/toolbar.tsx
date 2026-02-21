import { useState, useRef, useCallback } from 'react'
import {
  MousePointer2,
  Type,
  Frame,
  Hand,
  Undo2,
  Redo2,
  Braces,
} from 'lucide-react'
import ToolButton from './tool-button'
import ShapeToolDropdown from './shape-tool-dropdown'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import { parseSvgToNodes } from '@/utils/svg-parser'
import { useHistoryStore } from '@/stores/history-store'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import IconPickerDialog from '@/components/shared/icon-picker-dialog'

export default function Toolbar() {
  const canUndo = useHistoryStore((s) => s.undoStack.length > 0)
  const canRedo = useHistoryStore((s) => s.redoStack.length > 0)
  const variablesPanelOpen = useCanvasStore((s) => s.variablesPanelOpen)
  const toggleVariablesPanel = useCanvasStore((s) => s.toggleVariablesPanel)
  const fileInputRef = useRef<HTMLInputElement>(null)
  const [iconPickerOpen, setIconPickerOpen] = useState(false)

  const handleIconSelect = useCallback((svgText: string, iconName: string) => {
    const nodes = parseSvgToNodes(svgText)
    if (nodes.length === 0) return

    const { viewport, fabricCanvas } = useCanvasStore.getState()
    const canvasEl = fabricCanvas?.getElement()
    const canvasW = canvasEl?.clientWidth ?? 800
    const canvasH = canvasEl?.clientHeight ?? 600
    const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom
    const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom

    for (const node of nodes) {
      const w = ('width' in node ? (typeof node.width === 'number' ? node.width : 100) : 100)
      const h = ('height' in node ? (typeof node.height === 'number' ? node.height : 100) : 100)
      node.x = centerX - w / 2
      node.y = centerY - h / 2
      node.name = iconName
      useDocumentStore.getState().addNode(null, node)
    }
    setIconPickerOpen(false)
  }, [])

  const handleUndo = () => {
    const currentDoc = useDocumentStore.getState().document
    const prev = useHistoryStore.getState().undo(currentDoc)
    if (prev) {
      useDocumentStore.getState().applyHistoryState(prev)
    }
    useCanvasStore.getState().clearSelection()
    const canvas = useCanvasStore.getState().fabricCanvas
    if (canvas) {
      canvas.discardActiveObject()
      canvas.requestRenderAll()
    }
  }

  const handleRedo = () => {
    const currentDoc = useDocumentStore.getState().document
    const next = useHistoryStore.getState().redo(currentDoc)
    if (next) {
      useDocumentStore.getState().applyHistoryState(next)
    }
    useCanvasStore.getState().clearSelection()
    const canvas = useCanvasStore.getState().fabricCanvas
    if (canvas) {
      canvas.discardActiveObject()
      canvas.requestRenderAll()
    }
  }

  const handleAddImage = useCallback(() => {
    fileInputRef.current?.click()
  }, [])

  const handleFileSelected = useCallback((e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file) return
    // Reset input so the same file can be re-selected
    e.target.value = ''

    const isSvg = file.type === 'image/svg+xml'

    if (isSvg) {
      // SVG → parse into editable path/shape nodes
      const reader = new FileReader()
      reader.onload = () => {
        const svgText = reader.result as string
        const nodes = parseSvgToNodes(svgText)
        if (nodes.length === 0) return

        const { viewport, fabricCanvas } = useCanvasStore.getState()
        const canvasEl = fabricCanvas?.getElement()
        const canvasW = canvasEl?.clientWidth ?? 800
        const canvasH = canvasEl?.clientHeight ?? 600
        const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom
        const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom

        for (const node of nodes) {
          const w = ('width' in node ? (typeof node.width === 'number' ? node.width : 100) : 100)
          const h = ('height' in node ? (typeof node.height === 'number' ? node.height : 100) : 100)
          node.x = centerX - w / 2
          node.y = centerY - h / 2
          node.name = file.name.replace(/\.[^.]+$/, '')
          useDocumentStore.getState().addNode(null, node)
        }
      }
      reader.readAsText(file)
    } else {
      // Raster image → ImageNode with data URL
      const reader = new FileReader()
      reader.onload = () => {
        const dataUrl = reader.result as string
        const img = new Image()
        img.onload = () => {
          const { viewport, fabricCanvas } = useCanvasStore.getState()
          const canvasEl = fabricCanvas?.getElement()
          const canvasW = canvasEl?.clientWidth ?? 800
          const canvasH = canvasEl?.clientHeight ?? 600
          const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom
          const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom

          let w = img.naturalWidth
          let h = img.naturalHeight
          const maxDim = 400
          if (w > maxDim || h > maxDim) {
            const scale = maxDim / Math.max(w, h)
            w = Math.round(w * scale)
            h = Math.round(h * scale)
          }

          useDocumentStore.getState().addNode(null, {
            id: generateId(),
            type: 'image',
            name: file.name.replace(/\.[^.]+$/, ''),
            src: dataUrl,
            x: centerX - w / 2,
            y: centerY - h / 2,
            width: w,
            height: h,
          })
        }
        img.src = dataUrl
      }
      reader.readAsDataURL(file)
    }
  }, [])

  return (
    <div className="absolute top-2 left-2 z-10 w-10 bg-card border border-border rounded-xl flex flex-col items-center py-2 gap-1 shadow-lg">
      <ToolButton
        tool="select"
        icon={<MousePointer2 size={20} />}
        label="Select"
        shortcut="V"
      />
      <ShapeToolDropdown
        onIconPickerOpen={() => setIconPickerOpen(true)}
        onImageImport={handleAddImage}
      />
      <ToolButton
        tool="text"
        icon={<Type size={20} />}
        label="Text"
        shortcut="T"
      />
      <ToolButton
        tool="frame"
        icon={<Frame size={20} />}
        label="Frame"
        shortcut="F"
      />
      <ToolButton
        tool="hand"
        icon={<Hand size={20} />}
        label="Hand"
        shortcut="H"
      />

      <Separator className="my-1 w-8" />

      {/* Undo / Redo */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleUndo}
            disabled={!canUndo}
          >
            <Undo2 size={18} />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right">
          Undo
          <kbd className="ml-1.5 inline-flex h-4 items-center rounded border border-border/50 bg-muted px-1 font-mono text-[10px] text-muted-foreground">
            {'\u2318'}Z
          </kbd>
        </TooltipContent>
      </Tooltip>
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleRedo}
            disabled={!canRedo}
          >
            <Redo2 size={18} />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right">
          Redo
          <kbd className="ml-1.5 inline-flex h-4 items-center rounded border border-border/50 bg-muted px-1 font-mono text-[10px] text-muted-foreground">
            {'\u2318\u21e7'}Z
          </kbd>
        </TooltipContent>
      </Tooltip>

      <Separator className="my-1 w-8" />

      {/* Variables */}
      <Tooltip>
        <TooltipTrigger asChild>
          <button
            type="button"
            onClick={toggleVariablesPanel}
            aria-label="Variables"
            aria-pressed={variablesPanelOpen}
            className={`inline-flex items-center justify-center h-8 min-w-8 px-1.5 rounded-lg transition-colors [&_svg]:size-5 [&_svg]:shrink-0 ${
              variablesPanelOpen
                ? 'bg-primary text-primary-foreground'
                : 'text-muted-foreground hover:bg-muted hover:text-foreground'
            }`}
          >
            <Braces size={20} />
          </button>
        </TooltipTrigger>
        <TooltipContent side="right">
          Variables
          <kbd className="ml-1.5 inline-flex h-4 items-center rounded border border-border/50 bg-muted px-1 font-mono text-[10px] text-muted-foreground">
            {'\u2318\u21e7'}V
          </kbd>
        </TooltipContent>
      </Tooltip>

      {/* Hidden file input + icon picker dialog */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/png,image/jpeg,image/svg+xml,image/webp,image/gif"
        className="hidden"
        onChange={handleFileSelected}
      />
      <IconPickerDialog
        open={iconPickerOpen}
        onClose={() => setIconPickerOpen(false)}
        onSelect={handleIconSelect}
      />
    </div>
  )
}
