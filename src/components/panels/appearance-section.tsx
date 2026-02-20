import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import VariablePicker from '@/components/shared/variable-picker'
import { isVariableRef } from '@/variables/resolve-variables'
import type { PenNode } from '@/types/pen'

interface AppearanceSectionProps {
  node: PenNode
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function AppearanceSection({
  node,
  onUpdate,
}: AppearanceSectionProps) {
  const rawOpacity = node.opacity
  const isBound = typeof rawOpacity === 'string' && isVariableRef(rawOpacity)
  const opacity = typeof rawOpacity === 'number' ? rawOpacity * 100 : 100

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Layer" />
      <div className="flex items-center gap-1">
        <div className="flex-1">
          {isBound ? (
            <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
              {rawOpacity}
            </div>
          ) : (
            <NumberInput
              label="Opacity"
              value={Math.round(opacity)}
              onChange={(v) => onUpdate({ opacity: v / 100 })}
              min={0}
              max={100}
              suffix="%"
            />
          )}
        </div>
        <VariablePicker
          type="number"
          currentValue={isBound ? String(rawOpacity) : undefined}
          onBind={(ref) => onUpdate({ opacity: ref as unknown as number })}
          onUnbind={(val) => onUpdate({ opacity: Number(val) })}
        />
      </div>
    </div>
  )
}
