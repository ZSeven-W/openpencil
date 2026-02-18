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
  GripVertical,
  ImageIcon,
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

interface LayerItemProps {
  id: string
  name: string
  type: PenNodeType
  depth: number
  selected: boolean
  visible: boolean
  locked: boolean
  onSelect: (id: string) => void
  onRename: (id: string, name: string) => void
  onToggleVisibility: (id: string) => void
  onToggleLock: (id: string) => void
  onContextMenu: (e: React.MouseEvent, id: string) => void
  onDragStart: (id: string) => void
  onDragOver: (id: string) => void
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
  onSelect,
  onRename,
  onToggleVisibility,
  onToggleLock,
  onContextMenu,
  onDragStart,
  onDragOver,
  onDragEnd,
}: LayerItemProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editName, setEditName] = useState(name)

  const Icon = TYPE_ICONS[type] ?? Square

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
    if ((e.target as HTMLElement).closest('[data-drag-handle]')) {
      onDragStart(id)
    }
  }

  return (
    <div
      className={`flex items-center h-7 px-1 gap-1 cursor-pointer rounded text-xs transition-colors ${
        selected
          ? 'bg-primary/15 text-primary'
          : 'text-muted-foreground hover:bg-accent/50'
      } ${!visible ? 'opacity-40' : ''}`}
      style={{ paddingLeft: `${depth * 12 + 4}px` }}
      onClick={() => onSelect(id)}
      onDoubleClick={handleDoubleClick}
      onContextMenu={(e) => onContextMenu(e, id)}
      onPointerDown={handlePointerDown}
      onPointerEnter={() => onDragOver(id)}
      onPointerUp={onDragEnd}
    >
      <div
        data-drag-handle
        className="cursor-grab opacity-0 group-hover:opacity-60 hover:opacity-100 shrink-0"
      >
        <GripVertical size={10} />
      </div>

      <Icon size={12} className="shrink-0 opacity-60" />

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
            : 'opacity-0 group-hover:opacity-100'
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
            : 'opacity-0 group-hover:opacity-100'
        }`}
        title={locked ? 'Unlock' : 'Lock'}
      >
        {locked ? <Lock size={10} /> : <Unlock size={10} />}
      </button>
    </div>
  )
}
