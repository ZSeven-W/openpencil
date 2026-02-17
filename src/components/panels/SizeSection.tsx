import NumberInput from '@/components/shared/NumberInput'
import type { PenNode } from '@/types/pen'

interface SizeSectionProps {
  node: PenNode
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function SizeSection({ node, onUpdate }: SizeSectionProps) {
  const x = node.x ?? 0
  const y = node.y ?? 0
  const rotation = node.rotation ?? 0

  const width =
    'width' in node && typeof node.width === 'number'
      ? node.width
      : undefined
  const height =
    'height' in node && typeof node.height === 'number'
      ? node.height
      : undefined

  return (
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-gray-300 uppercase tracking-wider">
        Transform
      </h4>
      <div className="grid grid-cols-2 gap-1.5">
        <NumberInput
          label="X"
          value={Math.round(x)}
          onChange={(v) => onUpdate({ x: v })}
        />
        <NumberInput
          label="Y"
          value={Math.round(y)}
          onChange={(v) => onUpdate({ y: v })}
        />
        {width !== undefined && (
          <NumberInput
            label="W"
            value={Math.round(width)}
            onChange={(v) =>
              onUpdate({ width: v } as Partial<PenNode>)
            }
            min={1}
          />
        )}
        {height !== undefined && (
          <NumberInput
            label="H"
            value={Math.round(height)}
            onChange={(v) =>
              onUpdate({ height: v } as Partial<PenNode>)
            }
            min={1}
          />
        )}
      </div>
      <NumberInput
        label="R"
        value={Math.round(rotation)}
        onChange={(v) => onUpdate({ rotation: v })}
        suffix="°"
      />
    </div>
  )
}
