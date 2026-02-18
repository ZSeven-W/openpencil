import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import type { PenNode } from '@/types/pen'

interface AppearanceSectionProps {
  node: PenNode
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function AppearanceSection({
  node,
  onUpdate,
}: AppearanceSectionProps) {
  const opacity =
    typeof node.opacity === 'number' ? node.opacity * 100 : 100

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Layer" />
      <NumberInput
        label="Opacity"
        value={Math.round(opacity)}
        onChange={(v) => onUpdate({ opacity: v / 100 })}
        min={0}
        max={100}
        suffix="%"
      />
    </div>
  )
}
