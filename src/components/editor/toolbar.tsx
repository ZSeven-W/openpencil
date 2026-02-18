import { useRef, useCallback } from 'react'
import {
  MousePointer2,
  Square,
  Circle,
  Minus,
  Type,
  Frame,
  Hand,
  Undo2,
  Redo2,
  ImagePlus,
} from 'lucide-react'
import ToolButton from './tool-button'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, generateId } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

export default function Toolbar() {
  const canUndo = useHistoryStore((s) => s.undoStack.length > 0)
  const canRedo = useHistoryStore((s) => s.redoStack.length > 0)
  const fileInputRef = useRef<HTMLInputElement>(null)

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

    const reader = new FileReader()
    reader.onload = () => {
      const dataUrl = reader.result as string
      const img = new Image()
      img.onload = () => {
        const { viewport, fabricCanvas } = useCanvasStore.getState()
        // Place image at center of current viewport
        const canvasEl = fabricCanvas?.getElement()
        const canvasW = canvasEl?.clientWidth ?? 800
        const canvasH = canvasEl?.clientHeight ?? 600
        const centerX = (-viewport.panX + canvasW / 2) / viewport.zoom
        const centerY = (-viewport.panY + canvasH / 2) / viewport.zoom

        // Scale down large images to fit reasonably on canvas
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
  }, [])

  return (
    <div className="absolute top-2 left-2 z-10 w-10 bg-card border border-border rounded-xl flex flex-col items-center py-2 gap-1 shadow-lg">
      {/* Drawing Tools */}
      <ToolButton
        tool="select"
        icon={<MousePointer2 size={20} />}
        label="Select"
        shortcut="V"
      />
      <ToolButton
        tool="rectangle"
        icon={<Square size={20} />}
        label="Rectangle"
        shortcut="R"
      />
      <ToolButton
        tool="ellipse"
        icon={<Circle size={20} />}
        label="Ellipse"
        shortcut="O"
      />
      <ToolButton
        tool="line"
        icon={<Minus size={20} />}
        label="Line"
        shortcut="L"
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

      {/* Add Image */}
      <input
        ref={fileInputRef}
        type="file"
        accept="image/png,image/jpeg,image/svg+xml,image/webp,image/gif"
        className="hidden"
        onChange={handleFileSelected}
      />
      <Tooltip>
        <TooltipTrigger asChild>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleAddImage}
          >
            <ImagePlus size={18} />
          </Button>
        </TooltipTrigger>
        <TooltipContent side="right">Add Image</TooltipContent>
      </Tooltip>

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
    </div>
  )
}
