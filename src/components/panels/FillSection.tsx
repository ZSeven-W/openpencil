import ColorPicker from '@/components/shared/ColorPicker'
import type { PenNode } from '@/types/pen'
import type { PenFill } from '@/types/styles'

interface FillSectionProps {
  fills?: PenFill[]
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function FillSection({
  fills,
  onUpdate,
}: FillSectionProps) {
  const currentColor =
    fills && fills.length > 0 && fills[0].type === 'solid'
      ? fills[0].color
      : '#d1d5db'

  const handleColorChange = (color: string) => {
    const newFills: PenFill[] = [{ type: 'solid', color }]
    onUpdate({ fill: newFills } as Partial<PenNode>)
  }

  return (
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-gray-300 uppercase tracking-wider">
        Fill
      </h4>
      <ColorPicker value={currentColor} onChange={handleColorChange} />
    </div>
  )
}
