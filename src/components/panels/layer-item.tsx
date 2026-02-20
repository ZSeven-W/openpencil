import { useState } from 'react'
import {
  Square,
  Circle,
  Type,
  Minus,
  Frame,
  Eye,
  EyeOff,
  Lock,
  Unlock,
  FolderOpen,
  Hexagon,
  Spline,
  Link,
  ImageIcon,
  ChevronDown,
  ChevronRight,
  Diamond,
} from 'lucide-react'
import type { PenNodeType } from '@/types/pen'

const TYPE_ICONS: Record<PenNodeType, typeof Square> = {
  rectangle: Square,
  ellipse: Circle,
  text: Type,
  line: Minus,
  frame: Frame,
  group: FolderOpen,
  polygon: Hexagon,
  path: Spline,
  image: ImageIcon,
  ref: Link,
}

export type DropPosition = 'above' | 'below' | 'inside' | null

interface LayerItemProps {
  id: string
  name: string
  type: PenNodeType
  depth: number
  selected: boolean
  visible: boolean
  locked: boolean
  hasChildren: boolean
  expanded: boolean
  isReusable: boolean
  isInstance: boolean
  dropPosition: DropPosition
  onSelect: (id: string) => void
  onRename: (id: string, name: string) => void
  onToggleVisibility: (id: string) => void
  onToggleLock: (id: string) => void
  onToggleExpand: (id: string) => void
  onContextMenu: (e: React.MouseEvent, id: string) => void
  onDragStart: (id: string) => void
  onDragOver: (id: string, e: React.PointerEvent) => void
  onDragEnd: () => void
}

export default function LayerItem({
  id,
  name,
  type,
  depth,
  selected,
  visible,
  locked,
  hasChildren,
  expanded,
  isReusable,
  isInstance,
  dropPosition,
  onSelect,
  onRename,
  onToggleVisibility,
  onToggleLock,
  onToggleExpand,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDragEnd,
}: LayerItemProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editName, setEditName] = useState(name)

  const Icon = isReusable || isInstance ? Diamond : (TYPE_ICONS[type] ?? Square)

  const handleDoubleClick = () => {
    setEditName(name)
    setIsEditing(true)
  }

  const handleRenameBlur = () => {
    setIsEditing(false)
    if (editName.trim() && editName !== name) {
      onRename(id, editName.trim())
    }
  }

  const handleRenameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleRenameBlur()
    if (e.key === 'Escape') {
      setIsEditing(false)
      setEditName(name)
    }
  }

  const handlePointerDown = (e: React.PointerEvent) => {
    if (isEditing) return
    // Prevent browser text selection during drag
    e.preventDefault()
    onDragStart(id)
  }

  const dropInsideHighlight =
    dropPosition === 'inside' ? 'ring-2 ring-inset ring-blue-500 bg-blue-500/10' : ''

  return (
    <div className="relative">
      {dropPosition === 'above' && (
        <div className="absolute top-0 left-2 right-2 h-0.5 bg-blue-500 rounded-full z-10" />
      )}
      <div
        className={`group/layer flex items-center h-7 px-1 gap-1 cursor-pointer rounded text-xs transition-colors ${
          selected
            ? isReusable
              ? 'bg-purple-500/15 text-purple-400'
              : isInstance
                ? 'bg-[#9281f7]/10 text-[#9281f7]'
                : 'bg-primary/15 text-primary'
            : isReusable
              ? 'text-purple-400 hover:bg-purple-500/10'
              : isInstance
                ? 'text-[#9281f7] hover:bg-[#9281f7]/10'
                : 'text-muted-foreground hover:bg-accent/50'
        } ${!visible ? 'opacity-40' : ''} ${dropInsideHighlight}`}
        style={{ paddingLeft: `${depth * 12 + 4}px` }}
        onClick={() => onSelect(id)}
        onDoubleClick={handleDoubleClick}
        onContextMenu={(e) => onContextMenu(e, id)}
        onPointerDown={handlePointerDown}
        onPointerMove={(e) => onDragOver(id, e)}
        onPointerUp={onDragEnd}
      >
        {hasChildren ? (
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation()
              onToggleExpand(id)
            }}
            className="shrink-0 opacity-60 hover:opacity-100"
          >
            {expanded ? <ChevronDown size={12} /> : <ChevronRight size={12} />}
          </button>
        ) : (
          <span className="shrink-0 w-3" />
        )}

        <Icon size={12} className={`shrink-0 ${isReusable ? 'text-purple-400' : isInstance ? 'text-[#9281f7]' : 'opacity-60'}`} />

        {isEditing ? (
          <input
            type="text"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onBlur={handleRenameBlur}
            onKeyDown={handleRenameKeyDown}
            className="flex-1 bg-secondary text-foreground text-xs px-1 py-0.5 rounded border border-ring focus:outline-none"
            autoFocus
          />
        ) : (
          <span className="flex-1 truncate">{name}</span>
        )}

        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation()
            onToggleVisibility(id)
          }}
          className={`p-0.5 transition-opacity ${
            !visible
              ? 'opacity-100 text-yellow-400'
              : 'opacity-0 group-hover/layer:opacity-100'
          }`}
          title={visible ? 'Hide' : 'Show'}
        >
          {visible ? <Eye size={10} /> : <EyeOff size={10} />}
        </button>
        <button
          type="button"
          onClick={(e) => {
            e.stopPropagation()
            onToggleLock(id)
          }}
          className={`p-0.5 transition-opacity ${
            locked
              ? 'opacity-100 text-orange-400'
              : 'opacity-0 group-hover/layer:opacity-100'
          }`}
          title={locked ? 'Unlock' : 'Lock'}
        >
          {locked ? <Lock size={10} /> : <Unlock size={10} />}
        </button>
      </div>
      {dropPosition === 'below' && (
        <div className="absolute bottom-0 left-2 right-2 h-0.5 bg-blue-500 rounded-full z-10" />
      )}
    </div>
  )
}
