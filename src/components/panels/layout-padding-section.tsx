import { useState, useRef, useEffect } from 'react'
import NumberInput from '@/components/shared/number-input'
import type { PenNode } from '@/types/pen'
import { cn } from '@/lib/utils'
import { Settings } from 'lucide-react'

type PaddingMode = 'single' | 'axis' | 'individual'

// ---------------------------------------------------------------------------
// RadioCircle (shared with GapSection)
// ---------------------------------------------------------------------------

export function RadioCircle({
  selected,
  onClick,
}: {
  selected: boolean
  onClick?: () => void
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className={cn(
        'w-[14px] h-[14px] rounded-full border-[1.5px] flex items-center justify-center shrink-0 transition-colors',
        selected ? 'border-primary' : 'border-muted-foreground/40',
      )}
    >
      {selected && <div className="w-2 h-2 rounded-full bg-primary" />}
    </button>
  )
}

// ---------------------------------------------------------------------------
// Padding Icons
// ---------------------------------------------------------------------------

const PadVIcon = (
  <svg viewBox="0 0 12 12" fill="none" stroke="currentColor">
    <rect x="2.5" y="3.5" width="7" height="5" strokeWidth="1.2" rx="0.5" />
    <line x1="4" y1="1" x2="8" y2="1" strokeWidth="1.4" strokeLinecap="round" />
    <line x1="4" y1="11" x2="8" y2="11" strokeWidth="1.4" strokeLinecap="round" />
  </svg>
)

const PadHIcon = (
  <svg viewBox="0 0 12 12" fill="none" stroke="currentColor">
    <rect x="2.5" y="2.5" width="7" height="7" strokeWidth="1.2" rx="0.5" />
    <line x1="1" y1="4" x2="1" y2="8" strokeWidth="1.4" strokeLinecap="round" />
    <line x1="11" y1="4" x2="11" y2="8" strokeWidth="1.4" strokeLinecap="round" />
  </svg>
)

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

export function parsePaddingValues(
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined,
): { mode: PaddingMode; values: [number, number, number, number] } {
  if (typeof padding === 'string' || padding === undefined) {
    return { mode: 'single', values: [0, 0, 0, 0] }
  }
  if (typeof padding === 'number') {
    return {
      mode: 'single',
      values: [padding, padding, padding, padding],
    }
  }
  if (padding.length === 2) {
    return {
      mode: 'axis',
      values: [padding[0], padding[1], padding[0], padding[1]],
    }
  }
  if (padding[0] === padding[2] && padding[1] === padding[3]) {
    return {
      mode: 'axis',
      values: [padding[0], padding[1], padding[2], padding[3]],
    }
  }
  return {
    mode: 'individual',
    values: [padding[0], padding[1], padding[2], padding[3]],
  }
}

// ---------------------------------------------------------------------------
// PaddingSection — Uniform / V-H / T-R-B-L with gear popover
// ---------------------------------------------------------------------------

export default function PaddingSection({
  padding,
  onUpdate,
}: {
  padding:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
    | undefined
  onUpdate: (updates: Partial<PenNode>) => void
}) {
  const parsed = parsePaddingValues(padding)
  const [mode, setMode] = useState<PaddingMode>(parsed.mode)
  const [popoverOpen, setPopoverOpen] = useState(false)
  const popoverRef = useRef<HTMLDivElement>(null)

  useEffect(() => {
    setMode(parsePaddingValues(padding).mode)
  }, [padding])

  useEffect(() => {
    if (!popoverOpen) return
    const handler = (e: MouseEvent) => {
      if (
        popoverRef.current &&
        !popoverRef.current.contains(e.target as Node)
      ) {
        setPopoverOpen(false)
      }
    }
    document.addEventListener('mousedown', handler)
    return () => document.removeEventListener('mousedown', handler)
  }, [popoverOpen])

  const handleModeChange = (newMode: PaddingMode) => {
    setMode(newMode)
    setPopoverOpen(false)
    const vals = parsed.values
    switch (newMode) {
      case 'single':
        onUpdate({ padding: vals[0] } as Partial<PenNode>)
        break
      case 'axis':
        onUpdate({
          padding: [vals[0], vals[1]],
        } as Partial<PenNode>)
        break
      case 'individual':
        onUpdate({
          padding: [vals[0], vals[1], vals[2], vals[3]],
        } as Partial<PenNode>)
        break
    }
  }

  const MODES = [
    { value: 'single' as const, label: 'One value for all sides' },
    { value: 'axis' as const, label: 'Horizontal/Vertical' },
    { value: 'individual' as const, label: 'Top/Right/Bottom/Left' },
  ]

  return (
    <div className="space-y-1.5">
      {/* Label row: "Padding" left, gear right */}
      <div className="flex items-center justify-between">
        <span className="text-[10px] text-muted-foreground">
          Padding
        </span>
        <div ref={popoverRef} className="relative">
          <button
            type="button"
            title="Padding mode"
            onClick={() => setPopoverOpen(!popoverOpen)}
            className="h-5 w-5 flex items-center justify-center rounded text-muted-foreground hover:text-foreground hover:bg-accent transition-colors"
          >
            <Settings className="w-3.5 h-3.5" />
          </button>
          {popoverOpen && (
            <div className="absolute right-0 top-full mt-1 z-50 bg-popover border border-border rounded-lg shadow-md p-3 min-w-[190px]">
              <div className="text-[12px] font-medium mb-3 text-foreground">Padding Values</div>
              <div className="space-y-2.5">
                {MODES.map((opt) => (
                  <div
                    key={opt.value}
                    className="flex items-center gap-2 cursor-pointer"
                    onClick={() => handleModeChange(opt.value)}
                  >
                    <RadioCircle selected={mode === opt.value} />
                    <span className="text-[12px] text-foreground leading-none">{opt.label}</span>
                  </div>
                ))}
              </div>
            </div>
          )}
        </div>
      </div>

      {/* Padding inputs */}
      {mode === 'single' && (
        <NumberInput
          icon={PadVIcon}
          value={parsed.values[0]}
          onChange={(v) =>
            onUpdate({ padding: v } as Partial<PenNode>)
          }
          min={0}
        />
      )}

      {mode === 'axis' && (
        <div className="grid grid-cols-2 gap-1">
          <NumberInput
            icon={PadHIcon}
            value={parsed.values[1]}
            onChange={(v) =>
              onUpdate({
                padding: [parsed.values[0], v],
              } as Partial<PenNode>)
            }
            min={0}
          />
          <NumberInput
            icon={PadVIcon}
            value={parsed.values[0]}
            onChange={(v) =>
              onUpdate({
                padding: [v, parsed.values[1]],
              } as Partial<PenNode>)
            }
            min={0}
          />
        </div>
      )}

      {mode === 'individual' && (
        <div className="grid grid-cols-2 gap-1">
          <NumberInput
            label="T"
            value={parsed.values[0]}
            onChange={(v) =>
              onUpdate({
                padding: [
                  v,
                  parsed.values[1],
                  parsed.values[2],
                  parsed.values[3],
                ],
              } as Partial<PenNode>)
            }
            min={0}
          />
          <NumberInput
            label="R"
            value={parsed.values[1]}
            onChange={(v) =>
              onUpdate({
                padding: [
                  parsed.values[0],
                  v,
                  parsed.values[2],
                  parsed.values[3],
                ],
              } as Partial<PenNode>)
            }
            min={0}
          />
          <NumberInput
            label="B"
            value={parsed.values[2]}
            onChange={(v) =>
              onUpdate({
                padding: [
                  parsed.values[0],
                  parsed.values[1],
                  v,
                  parsed.values[3],
                ],
              } as Partial<PenNode>)
            }
            min={0}
          />
          <NumberInput
            label="L"
            value={parsed.values[3]}
            onChange={(v) =>
              onUpdate({
                padding: [
                  parsed.values[0],
                  parsed.values[1],
                  parsed.values[2],
                  v,
                ],
              } as Partial<PenNode>)
            }
            min={0}
          />
        </div>
      )}
    </div>
  )
}
