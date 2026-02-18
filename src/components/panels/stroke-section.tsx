import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
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
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
        Stroke
      </h4>
      <ColorPicker value={strokeColor} onChange={handleColorChange} />
      <NumberInput
        label="W"
        value={strokeWidth}
        onChange={handleWidthChange}
        min={0}
        max={100}
        step={1}
      />
    </div>
  )
}
