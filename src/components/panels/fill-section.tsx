import { useState } from 'react'
import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
import DropdownSelect from '@/components/shared/dropdown-select'
import type { PenNode } from '@/types/pen'
import type { PenFill, GradientStop } from '@/types/styles'

const FILL_TYPE_OPTIONS = [
  { value: 'solid', label: 'Solid' },
  { value: 'linear_gradient', label: 'Linear' },
  { value: 'radial_gradient', label: 'Radial' },
]

function defaultStops(): GradientStop[] {
  return [
    { offset: 0, color: '#000000' },
    { offset: 1, color: '#ffffff' },
  ]
}

interface FillSectionProps {
  fills?: PenFill[]
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function FillSection({
  fills,
  onUpdate,
}: FillSectionProps) {
  const firstFill = fills?.[0]
  const fillType = firstFill?.type ?? 'solid'
  const [expanded, setExpanded] = useState(false)

  const currentColor =
    firstFill?.type === 'solid' ? firstFill.color : '#d1d5db'

  const currentAngle =
    firstFill?.type === 'linear_gradient' ? (firstFill.angle ?? 0) : 0

  const currentStops: GradientStop[] =
    firstFill &&
    (firstFill.type === 'linear_gradient' ||
      firstFill.type === 'radial_gradient')
      ? firstFill.stops
      : defaultStops()

  const handleTypeChange = (type: string) => {
    let newFills: PenFill[]
    if (type === 'solid') {
      newFills = [{ type: 'solid', color: currentColor }]
    } else if (type === 'linear_gradient') {
      newFills = [
        {
          type: 'linear_gradient',
          angle: currentAngle,
          stops: currentStops,
        },
      ]
    } else {
      newFills = [
        {
          type: 'radial_gradient',
          cx: 0.5,
          cy: 0.5,
          radius: 0.5,
          stops: currentStops,
        },
      ]
    }
    onUpdate({ fill: newFills } as Partial<PenNode>)
  }

  const handleColorChange = (color: string) => {
    onUpdate({ fill: [{ type: 'solid', color }] } as Partial<PenNode>)
  }

  const handleAngleChange = (angle: number) => {
    if (firstFill?.type === 'linear_gradient') {
      onUpdate({
        fill: [{ ...firstFill, angle }],
      } as Partial<PenNode>)
    }
  }

  const handleStopColorChange = (index: number, color: string) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' &&
        firstFill.type !== 'radial_gradient')
    )
      return
    const newStops = [...firstFill.stops]
    newStops[index] = { ...newStops[index], color }
    onUpdate({
      fill: [{ ...firstFill, stops: newStops }],
    } as Partial<PenNode>)
  }

  const handleStopOffsetChange = (index: number, offset: number) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' &&
        firstFill.type !== 'radial_gradient')
    )
      return
    const newStops = [...firstFill.stops]
    newStops[index] = { ...newStops[index], offset: offset / 100 }
    onUpdate({
      fill: [{ ...firstFill, stops: newStops }],
    } as Partial<PenNode>)
  }

  const handleAddStop = () => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' &&
        firstFill.type !== 'radial_gradient')
    )
      return
    const stops = [...firstFill.stops]
    const lastOffset = stops[stops.length - 1]?.offset ?? 0.5
    stops.push({ offset: Math.min(1, lastOffset + 0.1), color: '#888888' })
    onUpdate({
      fill: [{ ...firstFill, stops }],
    } as Partial<PenNode>)
  }

  const handleRemoveStop = (index: number) => {
    if (
      !firstFill ||
      (firstFill.type !== 'linear_gradient' &&
        firstFill.type !== 'radial_gradient')
    )
      return
    if (firstFill.stops.length <= 2) return
    const stops = firstFill.stops.filter((_, i) => i !== index)
    onUpdate({
      fill: [{ ...firstFill, stops }],
    } as Partial<PenNode>)
  }

  return (
    <div className="space-y-2">
      <button
        type="button"
        className="text-xs font-medium text-muted-foreground uppercase tracking-wider w-full text-left"
        onClick={() => setExpanded(!expanded)}
      >
        Fill {expanded ? '-' : '+'}
      </button>

      <DropdownSelect
        value={fillType}
        options={FILL_TYPE_OPTIONS}
        onChange={handleTypeChange}
      />

      {fillType === 'solid' && (
        <ColorPicker value={currentColor} onChange={handleColorChange} />
      )}

      {(fillType === 'linear_gradient' ||
        fillType === 'radial_gradient') && (
        <div className="space-y-2">
          {fillType === 'linear_gradient' && (
            <NumberInput
              label="Angle"
              value={currentAngle}
              onChange={handleAngleChange}
              min={0}
              max={360}
              suffix="deg"
            />
          )}

          <div className="space-y-1.5">
            <span className="text-xs text-muted-foreground">Color Stops</span>
            {currentStops.map((stop, i) => (
              <div key={i} className="flex items-center gap-1">
                <ColorPicker
                  value={stop.color}
                  onChange={(c) => handleStopColorChange(i, c)}
                />
                <NumberInput
                  value={Math.round(stop.offset * 100)}
                  onChange={(v) => handleStopOffsetChange(i, v)}
                  min={0}
                  max={100}
                  suffix="%"
                  className="w-16"
                />
                {currentStops.length > 2 && (
                  <button
                    type="button"
                    onClick={() => handleRemoveStop(i)}
                    className="text-muted-foreground hover:text-red-400 text-xs px-1"
                  >
                    x
                  </button>
                )}
              </div>
            ))}
            <button
              type="button"
              onClick={handleAddStop}
              className="text-xs text-blue-400 hover:text-blue-300"
            >
              + Add Stop
            </button>
          </div>
        </div>
      )}
    </div>
  )
}
