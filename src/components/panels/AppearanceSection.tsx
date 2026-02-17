import SliderInput from '@/components/shared/SliderInput'
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
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-gray-300 uppercase tracking-wider">
        Appearance
      </h4>
      <SliderInput
        label="Opacity"
        value={opacity}
        onChange={(v) => onUpdate({ opacity: v / 100 })}
        min={0}
        max={100}
      />
    </div>
  )
}
