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
} from 'lucide-react'
import ToolButton from './tool-button'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
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
