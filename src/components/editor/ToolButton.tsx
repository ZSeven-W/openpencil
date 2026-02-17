import type { ReactNode } from 'react'
import type { ToolType } from '@/types/canvas'
import { useCanvasStore } from '@/stores/canvas-store'

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
    <button
      type="button"
      onClick={() => setActiveTool(tool)}
      title={shortcut ? `${label} (${shortcut})` : label}
      aria-label={label}
      className={`p-1.5 rounded transition-colors ${
        isActive
          ? 'bg-blue-500 text-white'
          : 'text-gray-400 hover:bg-gray-700 hover:text-white'
      }`}
    >
      {icon}
    </button>
  )
}
