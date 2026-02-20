import { useState, useRef, useEffect, useMemo } from 'react'
import { Braces, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useDocumentStore } from '@/stores/document-store'
import { isVariableRef, resolveVariableRef, getDefaultTheme } from '@/variables/resolve-variables'
import type { VariableDefinition } from '@/types/variables'

interface VariablePickerProps {
  /** Variable type to filter by */
  type: 'color' | 'number' | 'string'
  /** Current value — if it starts with '$', it's a variable reference */
  currentValue?: string | number
  /** Called when a variable is selected — value will be '$variableName' */
  onBind: (ref: string) => void
  /** Called when the variable binding is removed — should set the resolved concrete value */
  onUnbind: (resolvedValue: string | number) => void
  className?: string
}

export default function VariablePicker({
  type,
  currentValue,
  onBind,
  onUnbind,
  className,
}: VariablePickerProps) {
  const [open, setOpen] = useState(false)
  const popoverRef = useRef<HTMLDivElement>(null)
  const variables = useDocumentStore((s) => s.document.variables)
  const themes = useDocumentStore((s) => s.document.themes)

  const isBound = typeof currentValue === 'string' && isVariableRef(currentValue)
  const boundName = isBound ? (currentValue as string).slice(1) : null

  // Filter variables by matching type
  const matchingVars = useMemo(() => {
    if (!variables) return []
    return Object.entries(variables)
      .filter(([, def]) => def.type === type)
      .sort(([a], [b]) => a.localeCompare(b))
  }, [variables, type])

  // Close on outside click
  useEffect(() => {
    if (!open) return
    const handler = (e: MouseEvent) => {
      if (popoverRef.current && !popoverRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [open])

  const handleBind = (name: string) => {
    onBind(`$${name}`)
    setOpen(false)
  }

  const handleUnbind = () => {
    if (!boundName || !variables?.[boundName]) return
    const activeTheme = getDefaultTheme(themes)
    const resolved = resolveVariableRef(`$${boundName}`, variables, activeTheme)
    const fallback: string | number = type === 'color' ? '#000000' : type === 'number' ? 0 : ''
    const val = resolved != null && typeof resolved !== 'boolean' ? resolved : fallback
    onUnbind(val)
    setOpen(false)
  }

  const getPreview = (def: VariableDefinition): string => {
    const val = def.value
    if (!Array.isArray(val)) return String(val)
    // Show first theme value
    return val[0]?.value != null ? String(val[0].value) : ''
  }

  if (matchingVars.length === 0 && !isBound) return null

  return (
    <div className={cn('relative inline-flex', className)}>
      {isBound ? (
        <button
          type="button"
          onClick={() => setOpen(!open)}
          className="flex items-center gap-1 h-5 px-1.5 rounded bg-primary/15 text-primary text-[10px] font-mono hover:bg-primary/25 transition-colors"
          title={`Bound to --${boundName}`}
        >
          <Braces size={10} />
          <span className="truncate max-w-[80px]">--{boundName}</span>
        </button>
      ) : (
        <button
          type="button"
          onClick={() => setOpen(!open)}
          className="flex items-center justify-center w-5 h-5 rounded text-muted-foreground hover:text-foreground hover:bg-secondary transition-colors"
          title="Bind to variable"
        >
          <Braces size={12} />
        </button>
      )}

      {open && (
        <div
          ref={popoverRef}
          className="absolute right-0 top-full z-50 mt-1 w-52 bg-popover border border-border rounded-md shadow-md py-1 max-h-48 overflow-y-auto"
        >
          {isBound && (
            <>
              <button
                type="button"
                onClick={handleUnbind}
                className="w-full text-left px-3 py-1.5 text-xs text-destructive hover:bg-secondary flex items-center gap-1.5"
              >
                <X size={12} />
                Unbind variable
              </button>
              <div className="border-t border-border my-1" />
            </>
          )}
          {matchingVars.length === 0 ? (
            <div className="px-3 py-2 text-xs text-muted-foreground">
              No {type} variables defined
            </div>
          ) : (
            matchingVars.map(([name, def]) => (
              <button
                key={name}
                type="button"
                onClick={() => handleBind(name)}
                className={cn(
                  'w-full text-left px-3 py-1.5 text-xs hover:bg-secondary flex items-center gap-2',
                  boundName === name && 'bg-secondary',
                )}
              >
                {type === 'color' && (
                  <span
                    className="w-3 h-3 rounded-sm border border-input/50 shrink-0"
                    style={{ backgroundColor: String(getPreview(def)) }}
                  />
                )}
                <span className="font-mono truncate">--{name}</span>
                <span className="text-muted-foreground ml-auto shrink-0">
                  {type === 'color' ? getPreview(def) : getPreview(def)}
                </span>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  )
}
