import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import type { PenNode } from '@/types/pen'
import type { PenStroke, PenFill } from '@/types/styles'

interface StrokeSectionProps {
  stroke?: PenStroke
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function StrokeSection({
  stroke,
  onUpdate,
}: StrokeSectionProps) {
  const strokeColor =
    stroke?.fill && stroke.fill.length > 0 && stroke.fill[0].type === 'solid'
      ? stroke.fill[0].color
      : '#374151'

  const strokeWidth =
    stroke && typeof stroke.thickness === 'number'
      ? stroke.thickness
      : 0

  const handleColorChange = (color: string) => {
    const newFill: PenFill[] = [{ type: 'solid', color }]
    const newStroke: PenStroke = {
      ...(stroke ?? { thickness: 1 }),
      fill: newFill,
    }
    onUpdate({ stroke: newStroke } as Partial<PenNode>)
  }

  const handleWidthChange = (width: number) => {
    const newStroke: PenStroke = {
      ...(stroke ?? {
        thickness: 1,
        fill: [{ type: 'solid', color: strokeColor }],
      }),
      thickness: width,
    }
    onUpdate({ stroke: newStroke } as Partial<PenNode>)
  }

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Stroke" />
      <div className="flex items-center gap-1">
        <ColorPicker value={strokeColor} onChange={handleColorChange} />
        <NumberInput
          value={strokeWidth}
          onChange={handleWidthChange}
          min={0}
          max={100}
          step={1}
          className="w-14"
        />
      </div>
    </div>
  )
}
