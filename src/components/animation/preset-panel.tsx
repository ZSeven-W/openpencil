import { useState } from 'react'
import { Button } from '@/components/ui/button'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import SectionHeader from '@/components/shared/section-header'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { captureCurrentState } from '@/animation/canvas-bridge'
import { cn } from '@/lib/utils'
import type {
  AnimationPresetName,
  EasingPreset,
  SlideDirection,
} from '@/types/animation'
import type { VideoNode } from '@/types/pen'
import NumberInput from '@/components/shared/number-input'

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
  const tracks = useTimelineStore((s) => s.tracks)
  const duration = useTimelineStore((s) => s.duration)
  const getNodeById = useDocumentStore((s) => s.getNodeById)

  const handleApply = (presetName: AnimationPresetName) => {
    if (!selectedId || !canvas) return

    const nodeState = captureCurrentState(canvas, selectedId)
    if (!nodeState) return

    applyPreset(selectedId, presetName, nodeState, {
      direction,
      easing,
    })
  }

  const selectedNode = selectedId ? getNodeById(selectedId) : undefined
  const selectedTrack = selectedId ? tracks[selectedId] : undefined
  const updateNode = useDocumentStore((s) => s.updateNode)
  const isVideo = selectedNode?.type === 'video'
  const videoNode = isVideo ? (selectedNode as VideoNode) : null

  return (
    <>
      {/* Layer name header — matches property panel pattern */}
      <div className="h-8 flex items-center px-2 border-b border-border gap-1 shrink-0">
        <span className="text-[11px] font-medium text-foreground flex-1 truncate px-1">
          {selectedNode ? (selectedNode.name ?? selectedNode.type) : 'No selection'}
        </span>
      </div>

      <div className="flex-1 overflow-y-auto">
        {/* Video clip controls */}
        {videoNode && (
          <>
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Video Clip" />
              <div className="space-y-1.5">
                <div className="grid grid-cols-2 gap-1.5">
                  <NumberInput
                    label="In"
                    value={Math.round((videoNode.inPoint ?? 0) / 100) / 10}
                    onChange={(v) =>
                      updateNode(selectedId!, { inPoint: Math.round(v * 1000) } as Partial<VideoNode>)
                    }
                    min={0}
                    max={(videoNode.outPoint ?? videoNode.videoDuration ?? 0) / 1000}
                    step={0.1}
                    suffix="s"
                  />
                  <NumberInput
                    label="Out"
                    value={Math.round((videoNode.outPoint ?? videoNode.videoDuration ?? 0) / 100) / 10}
                    onChange={(v) =>
                      updateNode(selectedId!, { outPoint: Math.round(v * 1000) } as Partial<VideoNode>)
                    }
                    min={(videoNode.inPoint ?? 0) / 1000}
                    max={(videoNode.videoDuration ?? 0) / 1000}
                    step={0.1}
                    suffix="s"
                  />
                </div>
                <NumberInput
                  label="Offset"
                  value={Math.round((videoNode.timelineOffset ?? 0) / 100) / 10}
                  onChange={(v) =>
                    updateNode(selectedId!, { timelineOffset: Math.round(v * 1000) } as Partial<VideoNode>)
                  }
                  min={0}
                  step={0.1}
                  suffix="s"
                />
                <div className="flex items-center justify-between pt-0.5">
                  <span className="text-[11px] text-muted-foreground">Source duration</span>
                  <span className="text-[11px] text-foreground tabular-nums">
                    {((videoNode.videoDuration ?? 0) / 1000).toFixed(1)}s
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-[11px] text-muted-foreground">Clip duration</span>
                  <span className="text-[11px] text-foreground tabular-nums">
                    {(((videoNode.outPoint ?? videoNode.videoDuration ?? 0) - (videoNode.inPoint ?? 0)) / 1000).toFixed(1)}s
                  </span>
                </div>
              </div>
            </div>
            <Separator />
          </>
        )}

        {/* Presets section */}
        <div className="px-3 py-2 space-y-1.5">
          <SectionHeader title="Presets" />
          {selectedId ? (
            <div className="grid grid-cols-2 gap-1">
              {presetOptions.map((preset) => (
                <Button
                  key={preset.value}
                  variant="outline"
                  size="sm"
                  className={cn(
                    'h-7 text-[11px]',
                    selectedTrack && selectedTrack.keyframes.length > 0
                      ? ''
                      : '',
                  )}
                  onClick={() => handleApply(preset.value)}
                >
                  {preset.label}
                </Button>
              ))}
            </div>
          ) : (
            <p className="text-[11px] text-muted-foreground">
              Select a layer to apply a preset
            </p>
          )}
        </div>

        <Separator />

        {/* Easing section */}
        <div className="px-3 py-2 space-y-1.5">
          <SectionHeader title="Easing" />
          <Select value={easing} onValueChange={(v) => setEasing(v as EasingPreset)}>
            <SelectTrigger className="h-7 text-[11px] w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {easingOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} className="text-[11px]">
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <Separator />

        {/* Direction section */}
        <div className="px-3 py-2 space-y-1.5">
          <SectionHeader title="Direction" />
          <Select
            value={direction}
            onValueChange={(v) => setDirection(v as SlideDirection)}
          >
            <SelectTrigger className="h-7 text-[11px] w-full">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {directionOptions.map((opt) => (
                <SelectItem key={opt.value} value={opt.value} className="text-[11px]">
                  {opt.label}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <Separator />

        {/* Duration section */}
        <div className="px-3 py-2 space-y-1.5">
          <SectionHeader title="Duration" />
          <div className="flex items-center gap-1">
            <span className="text-[11px] text-foreground tabular-nums">
              {(duration / 1000).toFixed(1)}s
            </span>
            <span className="text-[11px] text-muted-foreground">total</span>
          </div>
        </div>

        {selectedTrack && (
          <>
            <Separator />

            {/* Track info section */}
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Phases" />
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-emerald-500" />
                    <span className="text-[11px] text-muted-foreground">In</span>
                  </div>
                  <span className="text-[11px] text-foreground tabular-nums">
                    {(selectedTrack.phases.in.duration / 1000).toFixed(2)}s
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-blue-500" />
                    <span className="text-[11px] text-muted-foreground">While</span>
                  </div>
                  <span className="text-[11px] text-foreground tabular-nums">
                    {(selectedTrack.phases.while.duration / 1000).toFixed(2)}s
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-1.5">
                    <span className="w-2 h-2 rounded-full bg-red-500" />
                    <span className="text-[11px] text-muted-foreground">Out</span>
                  </div>
                  <span className="text-[11px] text-foreground tabular-nums">
                    {(selectedTrack.phases.out.duration / 1000).toFixed(2)}s
                  </span>
                </div>
              </div>
              <div className="flex items-center justify-between pt-0.5">
                <span className="text-[11px] text-muted-foreground">Keyframes</span>
                <span className="text-[11px] text-foreground tabular-nums">
                  {selectedTrack.keyframes.length}
                </span>
              </div>
            </div>
          </>
        )}
      </div>
    </>
  )
}
