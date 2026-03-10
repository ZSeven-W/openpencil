import { useState } from 'react'
import { Sparkles } from 'lucide-react'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { captureCurrentState } from '@/animation/canvas-bridge'
import type {
  AnimationPresetName,
  EasingPreset,
  SlideDirection,
} from '@/types/animation'

const presetOptions: { value: AnimationPresetName; label: string }[] = [
  { value: 'fade', label: 'Fade' },
  { value: 'slide', label: 'Slide' },
  { value: 'scale', label: 'Scale' },
  { value: 'bounce', label: 'Bounce' },
]

const easingOptions: { value: EasingPreset; label: string }[] = [
  { value: 'smooth', label: 'Smooth' },
  { value: 'snappy', label: 'Snappy' },
  { value: 'bouncy', label: 'Bouncy' },
  { value: 'gentle', label: 'Gentle' },
  { value: 'linear', label: 'Linear' },
]

const directionOptions: { value: SlideDirection; label: string }[] = [
  { value: 'left', label: 'Left' },
  { value: 'right', label: 'Right' },
  { value: 'top', label: 'Top' },
  { value: 'bottom', label: 'Bottom' },
]

export default function PresetPanel() {
  const [easing, setEasing] = useState<EasingPreset>('smooth')
  const [direction, setDirection] = useState<SlideDirection>('left')
  const selectedId = useCanvasStore((s) => s.selection.activeId)
  const canvas = useCanvasStore((s) => s.fabricCanvas)
  const applyPreset = useTimelineStore((s) => s.applyPreset)

  const handleApply = (presetName: AnimationPresetName) => {
    if (!selectedId || !canvas) return

    const nodeState = captureCurrentState(canvas, selectedId)
    if (!nodeState) return

    applyPreset(selectedId, presetName, nodeState, {
      direction,
      easing,
    })
  }

  if (!selectedId) {
    return (
      <div className="px-3 py-2 text-xs text-muted-foreground">
        Select a layer to apply animation
      </div>
    )
  }

  return (
    <div className="flex flex-col gap-2 p-2">
      <div className="text-xs font-medium text-foreground flex items-center gap-1.5">
        <Sparkles className="h-3.5 w-3.5" />
        Animation Presets
      </div>

      {/* Preset buttons */}
      <div className="grid grid-cols-4 gap-1">
        {presetOptions.map((preset) => (
          <Button
            key={preset.value}
            variant="outline"
            size="sm"
            className="h-7 text-xs"
            onClick={() => handleApply(preset.value)}
          >
            {preset.label}
          </Button>
        ))}
      </div>

      {/* Settings row */}
      <div className="flex items-center gap-2">
        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground">Easing</span>
          <Select value={easing} onValueChange={(v) => setEasing(v as EasingPreset)}>
            <SelectTrigger className="h-6 w-24 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {easingOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} className="text-xs">
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="flex items-center gap-1.5">
          <span className="text-xs text-muted-foreground">Direction</span>
          <Select
            value={direction}
            onValueChange={(v) => setDirection(v as SlideDirection)}
          >
            <SelectTrigger className="h-6 w-20 text-xs">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {directionOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} className="text-xs">
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>
      </div>
    </div>
  )
}
