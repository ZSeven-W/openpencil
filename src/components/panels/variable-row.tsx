import { useState, useRef, useEffect } from 'react'
import { MoreHorizontal, Palette, Hash, Type, Pencil, Trash2 } from 'lucide-react'
import { cn } from '@/lib/utils'
import NumberInput from '@/components/shared/number-input'
import type { VariableDefinition, ThemedValue } from '@/types/variables'

const TYPE_ICONS: Record<string, React.ComponentType<{ size?: number; className?: string }>> = {
  color: Palette,
  number: Hash,
  string: Type,
  boolean: Type,
}

interface VariableRowProps {
  name: string
  definition: VariableDefinition
  themeValues: string[]
  themeAxis: string
  onUpdateValue: (name: string, definition: VariableDefinition) => void
  onRename: (oldName: string, newName: string) => void
  onDelete: (name: string) => void
}

export default function VariableRow({
  name,
  definition,
  themeValues,
  themeAxis,
  onUpdateValue,
  onRename,
  onDelete,
}: VariableRowProps) {
  const [isEditing, setIsEditing] = useState(false)
  const [editName, setEditName] = useState(name)
  const [showMenu, setShowMenu] = useState(false)
  const nameInputRef = useRef<HTMLInputElement>(null)
  const menuRef = useRef<HTMLDivElement>(null)

  useEffect(() => { setEditName(name) }, [name])

  useEffect(() => {
    if (isEditing && nameInputRef.current) {
      nameInputRef.current.focus()
      nameInputRef.current.select()
    }
  }, [isEditing])

  useEffect(() => {
    if (!showMenu) return
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) setShowMenu(false)
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [showMenu])

  const handleNameBlur = () => {
    setIsEditing(false)
    if (editName && editName !== name) onRename(name, editName)
    else setEditName(name)
  }

  const handleNameKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Enter') handleNameBlur()
    if (e.key === 'Escape') { setEditName(name); setIsEditing(false) }
  }

  function getValueForTheme(themeValue: string): string | number | boolean {
    const val = definition.value
    if (!Array.isArray(val)) return val
    const match = (val as ThemedValue[]).find((v) => v.theme?.[themeAxis] === themeValue)
    return match?.value ?? (val as ThemedValue[])[0]?.value ?? ''
  }

  function setValueForTheme(themeValue: string, newValue: string | number | boolean) {
    const val = definition.value
    if (!Array.isArray(val)) {
      const themed: ThemedValue[] = themeValues.map((tv) => ({
        value: tv === themeValue ? newValue : val,
        theme: { [themeAxis]: tv },
      }))
      onUpdateValue(name, { ...definition, value: themed })
      return
    }
    const themed = [...(val as ThemedValue[])]
    const idx = themed.findIndex((v) => v.theme?.[themeAxis] === themeValue)
    if (idx >= 0) themed[idx] = { ...themed[idx], value: newValue }
    else themed.push({ value: newValue, theme: { [themeAxis]: themeValue } })
    onUpdateValue(name, { ...definition, value: themed })
  }

  function getOpacityForTheme(themeValue: string): number {
    const val = getValueForTheme(themeValue)
    if (typeof val === 'string' && val.length === 9) {
      return Math.round((parseInt(val.slice(7, 9), 16) / 255) * 100)
    }
    return 100
  }

  const Icon = TYPE_ICONS[definition.type] ?? Type

  return (
    <div className="flex items-center group hover:bg-secondary/20 transition-colors rounded-lg px-2 min-h-[44px]">
      {/* Name column */}
      <div className="w-[220px] shrink-0 flex items-center gap-2 py-1">
        <Icon size={15} className="text-muted-foreground/60 shrink-0" />
        {isEditing ? (
          <input
            ref={nameInputRef}
            type="text"
            value={editName}
            onChange={(e) => setEditName(e.target.value)}
            onBlur={handleNameBlur}
            onKeyDown={handleNameKeyDown}
            className="bg-secondary text-foreground text-[13px] font-mono px-2 py-1 rounded-lg border border-ring w-full min-w-0 focus:outline-none"
          />
        ) : (
          <div
            className="bg-secondary/50 text-foreground/80 text-[13px] font-mono px-2 py-1 rounded-lg truncate cursor-default flex-1 min-w-0"
            onDoubleClick={() => setIsEditing(true)}
            title={`--${name}`}
          >
            --{name}
          </div>
        )}
      </div>

      {/* Theme value columns */}
      {themeValues.map((tv) => (
        <div key={tv} className="flex-1 min-w-0 flex items-center gap-2.5 pl-4 py-1">
          {definition.type === 'color' && (
            <ColorCell
              value={String(getValueForTheme(tv))}
              opacity={getOpacityForTheme(tv)}
              onChange={(color) => setValueForTheme(tv, color)}
            />
          )}
          {definition.type === 'number' && (
            <NumberInput
              value={Number(getValueForTheme(tv)) || 0}
              onChange={(v) => setValueForTheme(tv, v)}
              className="flex-1 max-w-[140px]"
            />
          )}
          {definition.type === 'string' && (
            <input
              type="text"
              value={String(getValueForTheme(tv))}
              onChange={(e) => setValueForTheme(tv, e.target.value)}
              className="flex-1 max-w-[180px] bg-secondary/50 text-foreground text-[13px] px-2 py-1 rounded-lg border border-transparent hover:border-input focus:border-ring focus:outline-none font-mono min-w-0"
            />
          )}
        </div>
      ))}

      {/* Actions column */}
      <div className="w-[44px] shrink-0 flex items-center justify-center relative">
        <button
          type="button"
          onClick={() => setShowMenu(!showMenu)}
          className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground/40 opacity-0 group-hover:opacity-100 transition-all"
        >
          <MoreHorizontal size={15} />
        </button>
        {showMenu && (
          <div
            ref={menuRef}
            className="absolute right-0 top-full z-50 mt-1 w-40 bg-popover border border-border rounded-xl shadow-xl py-1 animate-in fade-in slide-in-from-top-1 duration-150"
          >
            <button
              type="button"
              onClick={() => { setIsEditing(true); setShowMenu(false) }}
              className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] hover:bg-secondary/60 rounded-lg transition-colors"
            >
              <Pencil size={13} className="text-muted-foreground" />
              Rename
            </button>
            <button
              type="button"
              onClick={() => { onDelete(name); setShowMenu(false) }}
              className="w-full text-left flex items-center gap-2.5 px-3 py-2 text-[13px] text-destructive hover:bg-secondary/60 rounded-lg transition-colors"
            >
              <Trash2 size={13} />
              Delete
            </button>
          </div>
        )}
      </div>
    </div>
  )
}

// --- Color cell ---

function ColorCell({
  value,
  opacity,
  onChange,
}: {
  value: string
  opacity: number
  onChange: (color: string) => void
}) {
  const [hexInput, setHexInput] = useState(value.slice(0, 7))

  useEffect(() => { setHexInput(value.slice(0, 7)) }, [value])

  const handleHexChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const v = e.target.value
    setHexInput(v)
    if (/^#[0-9a-fA-F]{6}$/.test(v)) onChange(v)
  }

  const handleBlur = () => {
    if (!/^#[0-9a-fA-F]{6}$/.test(hexInput)) setHexInput(value.slice(0, 7))
  }

  return (
    <div className="flex items-center gap-2.5 flex-1">
      <input
        type="color"
        value={value.slice(0, 7)}
        onChange={(e) => onChange(e.target.value)}
        className="w-5 h-5 rounded border border-input/40 cursor-pointer bg-transparent p-0 shrink-0"
      />
      <input
        type="text"
        value={hexInput}
        onChange={handleHexChange}
        onBlur={handleBlur}
        className="w-[76px] bg-transparent text-foreground/80 text-[13px] font-mono focus:outline-none"
        placeholder="#000000"
      />
      <span className={cn(
        'text-[13px] tabular-nums whitespace-nowrap',
        opacity < 100 ? 'text-foreground/80' : 'text-muted-foreground/50',
      )}>
        {opacity} %
      </span>
    </div>
  )
}
