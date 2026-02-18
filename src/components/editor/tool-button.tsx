import type { ReactNode } from 'react'
import type { ToolType } from '@/types/canvas'
import { useCanvasStore } from '@/stores/canvas-store'
import { Toggle } from '@/components/ui/toggle'
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
        <Toggle
          size="sm"
          pressed={isActive}
          onPressedChange={() => setActiveTool(tool)}
          aria-label={label}
          className="data-[state=on]:bg-primary/15 data-[state=on]:text-primary [&_svg]:size-5"
        >
          {icon}
        </Toggle>
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
