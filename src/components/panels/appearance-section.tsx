import { Slider } from '@/components/ui/slider'
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
      <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
        Appearance
      </h4>
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground w-12 shrink-0">
          Opacity
        </span>
        <Slider
          value={[opacity]}
          onValueChange={([v]) => onUpdate({ opacity: v / 100 })}
          min={0}
          max={100}
          step={1}
          className="flex-1"
        />
        <span className="text-xs text-foreground/70 w-8 text-right tabular-nums">
          {Math.round(opacity)}
        </span>
      </div>
    </div>
  )
}
