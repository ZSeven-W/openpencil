import NumberInput from '@/components/shared/number-input'
import type { PenNode } from '@/types/pen'
import { nodeRenderInfo } from '@/canvas/use-canvas-sync'
import { RotateCw } from 'lucide-react'

interface SizeSectionProps {
  node: PenNode
  onUpdate: (updates: Partial<PenNode>) => void
  hasCornerRadius?: boolean
  cornerRadius?: number | [number, number, number, number]
  hideWH?: boolean
}

export default function SizeSection({
  node,
  onUpdate,
  hasCornerRadius,
  cornerRadius,
  hideWH,
}: SizeSectionProps) {
  const info = nodeRenderInfo.get(node.id)
  const offsetX = info?.parentOffsetX ?? 0
  const offsetY = info?.parentOffsetY ?? 0
  const x = (node.x ?? 0) + offsetX
  const y = (node.y ?? 0) + offsetY
  const rotation = node.rotation ?? 0

  const width =
    'width' in node && typeof node.width === 'number'
      ? node.width
      : undefined
  const height =
    'height' in node && typeof node.height === 'number'
      ? node.height
      : undefined

  const cornerRadiusValue =
    typeof cornerRadius === 'number'
      ? cornerRadius
      : Array.isArray(cornerRadius)
        ? cornerRadius[0]
        : 0

  return (
    <div className="space-y-3">
    <span className=" text-[11px] font-medium text-foreground ">
          Position
      </span>
    <div className="grid grid-cols-2 gap-1">
      
      <NumberInput
        label="X"
        value={Math.round(x)}
        onChange={(v) => onUpdate({ x: v - offsetX })}
      />
      <NumberInput
        label="Y"
        value={Math.round(y)}
        onChange={(v) => onUpdate({ y: v - offsetY })}
      />
      {!hideWH && width !== undefined && (
        <NumberInput
          label="W"
          value={Math.round(width)}
          onChange={(v) =>
            onUpdate({ width: v } as Partial<PenNode>)
          }
          min={1}
        />
      )}
      {!hideWH && height !== undefined && (
        <NumberInput
          label="H"
          value={Math.round(height)}
          onChange={(v) =>
            onUpdate({ height: v } as Partial<PenNode>)
          }
          min={1}
        />
      )}
      <NumberInput
        icon={<RotateCw />}
        value={Math.round(rotation)}
        onChange={(v) => onUpdate({ rotation: v })}
        suffix="°"
      />
      {hasCornerRadius && (
        <NumberInput
          label="R"
          value={cornerRadiusValue}
          onChange={(v) =>
            onUpdate({ cornerRadius: v } as Partial<PenNode>)
          }
          min={0}
        />
      )}
    </div>
    </div>
  )
}
