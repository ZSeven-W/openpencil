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
  ref: Link,
}

interface LayerItemProps {
  id: string
  name: string
  type: PenNodeType
  depth: number
  selected: boolean
  onSelect: (id: string) => void
  onRename: (id: string, name: string) => void
}

export default function LayerItem({
  id,
  name,
  type,
  depth,
  selected,
  onSelect,
  onRename,
}: LayerItemProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editName, setEditName] = useState(name)
  const [visible, setVisible] = useState(true)
  const [locked, setLocked] = useState(false)

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

  return (
    <div
      className={`flex items-center h-7 px-1 gap-1 cursor-pointer rounded text-xs transition-colors ${
        selected
          ? 'bg-blue-500/20 text-blue-300'
          : 'text-gray-400 hover:bg-gray-700/50'
      }`}
      style={{ paddingLeft: `${depth * 12 + 4}px` }}
      onClick={() => onSelect(id)}
      onDoubleClick={handleDoubleClick}
    >
      <Icon size={12} className="shrink-0 opacity-60" />

      {isEditing ? (
        <input
          type="text"
          value={editName}
          onChange={(e) => setEditName(e.target.value)}
          onBlur={handleRenameBlur}
          onKeyDown={handleRenameKeyDown}
          className="flex-1 bg-gray-700 text-white text-xs px-1 py-0.5 rounded border border-blue-500 focus:outline-none"
          autoFocus
        />
      ) : (
        <span className="flex-1 truncate">{name}</span>
      )}

      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation()
          setVisible(!visible)
        }}
        className="p-0.5 opacity-0 group-hover:opacity-100 hover:opacity-100 transition-opacity"
        title={visible ? 'Hide' : 'Show'}
      >
        {visible ? <Eye size={10} /> : <EyeOff size={10} />}
      </button>
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation()
          setLocked(!locked)
        }}
        className="p-0.5 opacity-0 group-hover:opacity-100 hover:opacity-100 transition-opacity"
        title={locked ? 'Unlock' : 'Lock'}
      >
        {locked ? <Lock size={10} /> : <Unlock size={10} />}
      </button>
    </div>
  )
}
