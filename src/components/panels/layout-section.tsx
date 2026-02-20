import NumberInput from '@/components/shared/number-input'
import DropdownSelect from '@/components/shared/dropdown-select'
import VariablePicker from '@/components/shared/variable-picker'
import { isVariableRef } from '@/variables/resolve-variables'
import type { PenNode, ContainerProps } from '@/types/pen'
import { cn } from '@/lib/utils'
import {
  Rows3,
  Columns3,
  LayoutGrid,
  AlignStartHorizontal,
  AlignCenterHorizontal,
  AlignEndHorizontal,
  AlignStartVertical,
  AlignCenterVertical,
  AlignEndVertical,
} from 'lucide-react'

interface LayoutSectionProps {
  node: PenNode & ContainerProps
  onUpdate: (updates: Partial<PenNode>) => void
}

const JUSTIFY_OPTIONS = [
  { value: 'start', label: 'Start' },
  { value: 'center', label: 'Center' },
  { value: 'end', label: 'End' },
  { value: 'space_between', label: 'Between' },
  { value: 'space_around', label: 'Around' },
]

const ALIGN_OPTIONS = [
  { value: 'start', label: 'Start' },
  { value: 'center', label: 'Center' },
  { value: 'end', label: 'End' },
]

function ToggleButton({
  active,
  onClick,
  children,
  title,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
  title: string
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        'h-6 w-6 flex items-center justify-center rounded transition-colors',
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent',
      )}
    >
      {children}
    </button>
  )
}

export default function LayoutSection({ node, onUpdate }: LayoutSectionProps) {
  const layout = node.layout ?? 'none'
  const rawGap = node.gap
  const rawPadding = node.padding
  const gapIsBound = typeof rawGap === 'string' && isVariableRef(rawGap)
  const paddingIsBound = typeof rawPadding === 'string' && isVariableRef(rawPadding)
  const gap = typeof rawGap === 'number' ? rawGap : 0
  const padding = typeof rawPadding === 'number'
    ? rawPadding
    : Array.isArray(rawPadding)
      ? rawPadding[0]
      : 0
  const justifyContent = node.justifyContent ?? 'start'
  const alignItems = node.alignItems ?? 'start'
  const hasLayout = layout !== 'none'

  return (
    <div className="space-y-1.5">
      <span className="text-[10px] font-medium text-muted-foreground  tracking-wider">
        Layout
      </span>

      {/* Layout direction */}
      <div className="flex items-center gap-1.5">
        <span className="text-[10px] text-muted-foreground w-8 shrink-0">Dir</span>
        <div className="flex gap-0.5">
          <ToggleButton
            active={layout === 'none'}
            onClick={() => onUpdate({ layout: 'none' } as Partial<PenNode>)}
            title="No layout"
          >
            <LayoutGrid className="w-3 h-3" />
          </ToggleButton>
          <ToggleButton
            active={layout === 'vertical'}
            onClick={() => onUpdate({ layout: 'vertical' } as Partial<PenNode>)}
            title="Vertical layout"
          >
            <Rows3 className="w-3 h-3" />
          </ToggleButton>
          <ToggleButton
            active={layout === 'horizontal'}
            onClick={() => onUpdate({ layout: 'horizontal' } as Partial<PenNode>)}
            title="Horizontal layout"
          >
            <Columns3 className="w-3 h-3" />
          </ToggleButton>
        </div>
      </div>

      {hasLayout && (
        <>
          {/* Gap & Padding */}
          <div className="space-y-1">
            <div className="flex items-center gap-1">
              <div className="flex-1">
                {gapIsBound ? (
                  <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
                    {String(rawGap)}
                  </div>
                ) : (
                  <NumberInput
                    label="Gap"
                    value={gap}
                    onChange={(v) => onUpdate({ gap: v } as Partial<PenNode>)}
                    min={0}
                  />
                )}
              </div>
              <VariablePicker
                type="number"
                currentValue={gapIsBound ? String(rawGap) : undefined}
                onBind={(ref) => onUpdate({ gap: ref as unknown as number } as Partial<PenNode>)}
                onUnbind={(val) => onUpdate({ gap: Number(val) } as Partial<PenNode>)}
              />
            </div>
            <div className="flex items-center gap-1">
              <div className="flex-1">
                {paddingIsBound ? (
                  <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
                    {String(rawPadding)}
                  </div>
                ) : (
                  <NumberInput
                    label="Pad"
                    value={padding}
                    onChange={(v) => onUpdate({ padding: v } as Partial<PenNode>)}
                    min={0}
                  />
                )}
              </div>
              <VariablePicker
                type="number"
                currentValue={paddingIsBound ? String(rawPadding) : undefined}
                onBind={(ref) => onUpdate({ padding: ref as unknown as number } as Partial<PenNode>)}
                onUnbind={(val) => onUpdate({ padding: Number(val) } as Partial<PenNode>)}
              />
            </div>
          </div>

          {/* Justify Content */}
          <DropdownSelect
            label="Justify"
            value={justifyContent}
            options={JUSTIFY_OPTIONS}
            onChange={(v) =>
              onUpdate({ justifyContent: v } as Partial<PenNode>)
            }
          />

          {/* Align Items */}
          <div className="flex items-center gap-1.5">
            <span className="text-[10px] text-muted-foreground w-8 shrink-0">Align</span>
            <div className="flex gap-0.5">
              {ALIGN_OPTIONS.map((opt) => {
                const icons = layout === 'horizontal'
                  ? { start: AlignStartVertical, center: AlignCenterVertical, end: AlignEndVertical }
                  : { start: AlignStartHorizontal, center: AlignCenterHorizontal, end: AlignEndHorizontal }
                const Icon = icons[opt.value as keyof typeof icons]
                return (
                  <ToggleButton
                    key={opt.value}
                    active={alignItems === opt.value}
                    onClick={() => onUpdate({ alignItems: opt.value } as Partial<PenNode>)}
                    title={`Align ${opt.label}`}
                  >
                    <Icon className="w-3 h-3" />
                  </ToggleButton>
                )
              })}
            </div>
          </div>
        </>
      )}
    </div>
  )
}
