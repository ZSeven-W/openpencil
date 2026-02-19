import { useState, useRef, useEffect, type ReactNode } from 'react'
import {
  Square,
  Circle,
  Minus,
  PenTool,
  Sparkles,
  ImagePlus,
  ChevronDown,
} from 'lucide-react'
import type { ToolType } from '@/types/canvas'
import { useCanvasStore } from '@/stores/canvas-store'
import { Toggle } from '@/components/ui/toggle'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'

const SHAPE_TOOLS: ToolType[] = ['rectangle', 'ellipse', 'line', 'path']

interface ToolItem {
  type: 'tool'
  tool: ToolType
  icon: ReactNode
  label: string
}

interface ActionItem {
  type: 'action'
  key: string
  icon: ReactNode
  label: string
  onAction: () => void
}

type DropdownItem = ToolItem | ActionItem

interface ShapeToolDropdownProps {
  onIconPickerOpen: () => void
  onImageImport: () => void
}

const TOOL_ICON_MAP: Record<string, ReactNode> = {
  rectangle: <Square size={20} />,
  ellipse: <Circle size={20} />,
  line: <Minus size={20} />,
  path: <PenTool size={20} />,
}

export default function ShapeToolDropdown({
  onIconPickerOpen,
  onImageImport,
}: ShapeToolDropdownProps) {
  const [open, setOpen] = useState(false)
  const lastShapeTool = useRef<ToolType>('rectangle')
  const wrapperRef = useRef<HTMLDivElement>(null)

  const activeTool = useCanvasStore((s) => s.activeTool)
  const setActiveTool = useCanvasStore((s) => s.setActiveTool)

  const isGroupActive = SHAPE_TOOLS.includes(activeTool)

  // Track last used shape tool
  useEffect(() => {
    if (SHAPE_TOOLS.includes(activeTool)) {
      lastShapeTool.current = activeTool
    }
  }, [activeTool])

  // Close on Escape
  useEffect(() => {
    if (!open) return
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation()
        setOpen(false)
      }
    }
    document.addEventListener('keydown', handler)
    return () => document.removeEventListener('keydown', handler)
  }, [open])

  const displayIcon =
    isGroupActive
      ? TOOL_ICON_MAP[activeTool]
      : TOOL_ICON_MAP[lastShapeTool.current]

  const items: DropdownItem[] = [
    { type: 'tool', tool: 'rectangle', icon: <Square size={18} />, label: 'Rectangle' },
    { type: 'tool', tool: 'ellipse', icon: <Circle size={18} />, label: 'Ellipse' },
    { type: 'tool', tool: 'line', icon: <Minus size={18} />, label: 'Line' },
    { type: 'action', key: 'icon', icon: <Sparkles size={18} />, label: 'Icon', onAction: onIconPickerOpen },
    { type: 'action', key: 'image', icon: <ImagePlus size={18} />, label: 'Import Image or SVG…', onAction: onImageImport },
    { type: 'tool', tool: 'path', icon: <PenTool size={18} />, label: 'Pen' },
  ]

  const handleSelect = (item: DropdownItem) => {
    if (item.type === 'tool') {
      setActiveTool(item.tool)
    } else {
      item.onAction()
    }
    setOpen(false)
  }

  return (
    <div ref={wrapperRef} className="relative flex flex-col items-center">
      {/* Main shape tool button */}
      <Tooltip>
        <TooltipTrigger asChild>
          <Toggle
            size="sm"
            pressed={isGroupActive}
            onPressedChange={() => setActiveTool(lastShapeTool.current)}
            aria-label="Shape tools"
            className="data-[state=on]:bg-primary/15 data-[state=on]:text-primary [&_svg]:size-5"
          >
            {displayIcon}
          </Toggle>
        </TooltipTrigger>
        <TooltipContent side="right">
          Shape tools
        </TooltipContent>
      </Tooltip>

      {/* Chevron button below — opens dropdown */}
      <button
        onClick={() => setOpen((v) => !v)}
        className="flex items-center justify-center w-5 h-3 rounded-sm hover:bg-muted text-muted-foreground transition-colors cursor-pointer"
        aria-label="More shape tools"
      >
        <ChevronDown size={10} />
      </button>

      {open && (
        <>
          {/* Invisible backdrop to catch click-away */}
          <div
            className="fixed inset-0 z-40"
            onClick={() => setOpen(false)}
          />

          {/* Dropdown panel — below the chevron, offset to the right */}
          <div className="absolute top-full left-[calc(100%+8px)] mt-1 z-50 bg-card border border-border rounded-lg shadow-xl py-1.5 min-w-[220px]">
            {items.map((item) => {
              const key = item.type === 'tool' ? item.tool : item.key
              const isActive = item.type === 'tool' && activeTool === item.tool
              return (
                <button
                  key={key}
                  onClick={() => handleSelect(item)}
                  className={`w-full flex items-center gap-3 px-3 py-2 text-sm transition-colors cursor-pointer ${
                    isActive
                      ? 'bg-primary/15 text-primary'
                      : 'text-foreground hover:bg-accent'
                  }`}
                >
                  <span className="flex-shrink-0">{item.icon}</span>
                  <span className="flex-1 text-left">{item.label}</span>
                </button>
              )
            })}
          </div>
        </>
      )}
    </div>
  )
}
