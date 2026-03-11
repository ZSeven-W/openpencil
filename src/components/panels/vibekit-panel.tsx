import { useState, useCallback } from 'react'
import {
  VIBE_CATEGORIES,
  type VibeCategory,
  getSchemaByCategory,
} from '@/vibekit/schema'
import { useDocumentStore } from '@/stores/document-store'
import { useVibeKitStore } from '@/stores/vibekit-store'
import { cn } from '@/lib/utils'
import {
  X,
  Palette,
  Type as TypeIcon,
  Ruler,
  Box,
  Minus,
  Play,
  ArrowRight,
  Image,
  SunMoon,
  Volume2,
  Shapes,
} from 'lucide-react'
import type { VariableDefinition } from '@/types/variables'

const CATEGORY_ICONS: Record<VibeCategory, React.ComponentType<{ className?: string }>> = {
  color: Palette,
  typography: TypeIcon,
  size: Ruler,
  space: Box,
  stroke: Minus,
  animation: Play,
  transition: ArrowRight,
  texture: Image,
  lut: SunMoon,
  sfx: Volume2,
  graphic: Shapes,
}

const CATEGORY_LABELS: Record<VibeCategory, string> = {
  color: 'Color',
  typography: 'Type',
  size: 'Size',
  space: 'Space',
  stroke: 'Stroke',
  animation: 'Anim',
  transition: 'Trans',
  texture: 'Texture',
  lut: 'LUT',
  sfx: 'SFX',
  graphic: 'Graphic',
}

interface VibeKitPanelProps {
  open: boolean
  onClose: () => void
}

export default function VibeKitPanel({ open, onClose }: VibeKitPanelProps) {
  const [activeCategory, setActiveCategory] = useState<VibeCategory>('color')
  const variables = useDocumentStore((s) => s.document.variables ?? {})
  const setVariable = useDocumentStore((s) => s.setVariable)
  const activeKit = useVibeKitStore((s) => s.getActiveKit())

  const schemaEntries = getSchemaByCategory(activeCategory)

  const handleValueChange = useCallback(
    (name: string, def: VariableDefinition, newValue: string | number | boolean) => {
      setVariable(name, { ...def, value: newValue })
    },
    [setVariable],
  )

  if (!open) return null

  return (
    <div className="absolute left-14 top-2 z-20 w-[400px] max-h-[500px] flex flex-col bg-card/95 backdrop-blur-sm border border-border rounded-2xl shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border flex-shrink-0">
        <div className="flex items-center gap-2">
          <h3 className="text-sm font-medium text-foreground">Vibe Kit</h3>
          {activeKit && (
            <span className="text-xs text-muted-foreground bg-muted px-2 py-0.5 rounded-full">
              {activeKit.name}
            </span>
          )}
        </div>
        <button
          onClick={onClose}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Category tabs */}
      <div className="flex gap-1 px-3 py-2 border-b border-border overflow-x-auto flex-shrink-0">
        {VIBE_CATEGORIES.map((cat) => {
          const Icon = CATEGORY_ICONS[cat]
          const isActive = activeCategory === cat
          return (
            <button
              key={cat}
              onClick={() => setActiveCategory(cat)}
              className={cn(
                'flex items-center gap-1 px-2 py-1 rounded-lg text-xs whitespace-nowrap transition-colors',
                isActive
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground hover:bg-muted',
              )}
            >
              <Icon className="w-3 h-3" />
              {CATEGORY_LABELS[cat]}
            </button>
          )
        })}
      </div>

      {/* Variable rows */}
      <div className="flex-1 overflow-y-auto p-3 space-y-1">
        {Object.entries(schemaEntries).map(([name, schema]) => {
          const def: VariableDefinition = variables[name] ?? {
            type: schema.type,
            value: schema.fallback,
          }
          const value = typeof def.value === 'object' ? schema.fallback : def.value

          return (
            <VariableValueRow
              key={name}
              name={name}
              description={schema.description}
              type={schema.type}
              value={value}
              onChange={(v) => handleValueChange(name, def, v)}
            />
          )
        })}

        {Object.keys(schemaEntries).length === 0 && (
          <div className="text-xs text-muted-foreground text-center py-8">
            No variables in this category
          </div>
        )}
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// Inline variable row — simpler than the full VariableRow component
// ---------------------------------------------------------------------------

interface VariableValueRowProps {
  name: string
  description?: string
  type: 'color' | 'number' | 'boolean' | 'string'
  value: string | number | boolean
  onChange: (value: string | number | boolean) => void
}

function VariableValueRow({ name, description, type, value, onChange }: VariableValueRowProps) {
  return (
    <div className="flex items-center gap-2 px-2 py-1.5 rounded-lg hover:bg-muted/50 group">
      {/* Name */}
      <div className="flex-1 min-w-0">
        <div className="text-xs font-medium text-foreground truncate">{name}</div>
        {description && (
          <div className="text-[10px] text-muted-foreground truncate">{description}</div>
        )}
      </div>

      {/* Value editor */}
      <div className="flex-shrink-0">
        {type === 'color' && (
          <div className="flex items-center gap-1.5">
            <input
              type="color"
              value={String(value)}
              onChange={(e) => onChange(e.target.value)}
              className="w-6 h-6 rounded border border-border cursor-pointer bg-transparent p-0"
            />
            <input
              type="text"
              value={String(value)}
              onChange={(e) => onChange(e.target.value)}
              className="w-20 text-xs bg-transparent border border-border rounded px-1.5 py-0.5 text-foreground font-mono"
            />
          </div>
        )}

        {type === 'number' && (
          <input
            type="number"
            value={Number(value)}
            onChange={(e) => onChange(parseFloat(e.target.value) || 0)}
            className="w-16 text-xs bg-transparent border border-border rounded px-1.5 py-0.5 text-foreground text-right"
          />
        )}

        {type === 'string' && (
          <input
            type="text"
            value={String(value)}
            onChange={(e) => onChange(e.target.value)}
            className="w-32 text-xs bg-transparent border border-border rounded px-1.5 py-0.5 text-foreground"
          />
        )}

        {type === 'boolean' && (
          <button
            onClick={() => onChange(!value)}
            className={cn(
              'w-8 h-4 rounded-full transition-colors relative',
              value ? 'bg-primary' : 'bg-muted',
            )}
          >
            <span
              className={cn(
                'absolute top-0.5 w-3 h-3 rounded-full bg-primary-foreground transition-transform',
                value ? 'left-4' : 'left-0.5',
              )}
            />
          </button>
        )}
      </div>
    </div>
  )
}
