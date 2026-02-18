import { useEffect, useRef } from 'react'
import {
  Trash2,
  Copy,
  Group,
  Lock,
  EyeOff,
} from 'lucide-react'

interface LayerContextMenuProps {
  x: number
  y: number
  nodeId: string
  canGroup: boolean
  onAction: (action: string) => void
  onClose: () => void
}

const MENU_ITEMS = [
  { action: 'duplicate', label: 'Duplicate', icon: Copy },
  { action: 'delete', label: 'Delete', icon: Trash2 },
  { action: 'group', label: 'Group Selection', icon: Group, requireGroup: true },
  { action: 'lock', label: 'Toggle Lock', icon: Lock },
  { action: 'hide', label: 'Toggle Visibility', icon: EyeOff },
]

export default function LayerContextMenu({
  x,
  y,
  canGroup,
  onAction,
  onClose,
}: LayerContextMenuProps) {
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('mousedown', handleClick)
    document.addEventListener('keydown', handleKey)
    return () => {
      document.removeEventListener('mousedown', handleClick)
      document.removeEventListener('keydown', handleKey)
    }
  }, [onClose])

  return (
    <div
      ref={menuRef}
      className="fixed z-50 bg-gray-800 border border-gray-600 rounded-md shadow-lg py-1 min-w-[160px]"
      style={{ left: x, top: y }}
    >
      {MENU_ITEMS.filter(
        (item) => !item.requireGroup || canGroup,
      ).map((item) => (
        <button
          key={item.action}
          type="button"
          className="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-gray-300 hover:bg-gray-700 hover:text-white text-left"
          onClick={() => onAction(item.action)}
        >
          <item.icon size={12} />
          {item.label}
        </button>
      ))}
    </div>
  )
}
