import type { ReactNode } from 'react'
import type { ToolType } from '@/types/canvas'
import { useCanvasStore } from '@/stores/canvas-store'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

interface ToolButtonProps {
  tool: ToolType
  icon: ReactNode
  label: string
  shortcut?: string
}

export default function ToolButton({
  tool,
  icon,
  label,
  shortcut,
}: ToolButtonProps) {
  const activeTool = useCanvasStore((s) => s.activeTool)
  const setActiveTool = useCanvasStore((s) => s.setActiveTool)
  const isActive = activeTool === tool

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          onClick={() => setActiveTool(tool)}
          aria-label={label}
          aria-pressed={isActive}
          className={`inline-flex items-center justify-center h-8 min-w-8 px-1.5 rounded-lg transition-colors [&_svg]:size-5 [&_svg]:shrink-0 ${
            isActive
              ? 'bg-primary text-primary-foreground'
              : 'text-muted-foreground hover:bg-muted hover:text-foreground'
          }`}
        >
          {icon}
        </button>
      </TooltipTrigger>
      <TooltipContent side="right">
        {label}
        {shortcut && (
          <kbd className="ml-1.5 inline-flex h-4 items-center rounded border border-border/50 bg-muted px-1 font-mono text-[10px] text-muted-foreground">
            {shortcut}
          </kbd>
        )}
      </TooltipContent>
    </Tooltip>
  )
}
