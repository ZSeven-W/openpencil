import { nanoid } from 'nanoid'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import SectionHeader from '@/components/shared/section-header'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { captureNodeState, findFabricObject } from '@/animation/canvas-bridge'
import { getEffectsByCategory, generateClipFromEffect } from '@/animation/effect-registry'
import '@/animation/effects' // ensure effects are registered
import type { AnimationClipData } from '@/types/animation'
import type { PenNode, VideoNode } from '@/types/pen'
import NumberInput from '@/components/shared/number-input'

export default function PresetPanel() {
  const selectedId = useCanvasStore((s) => s.selection.activeId)
  const canvas = useCanvasStore((s) => s.fabricCanvas)
  const duration = useTimelineStore((s) => s.duration)
  const getNodeById = useDocumentStore((s) => s.getNodeById)
  const updateNode = useDocumentStore((s) => s.updateNode)

  const selectedNode = selectedId ? getNodeById(selectedId) : undefined
  const isVideo = selectedNode?.type === 'video'
  const videoNode = isVideo ? (selectedNode as VideoNode) : null

  // v2: Effect registry
  const effectCategories = ['enter', 'exit', 'emphasis'] as const
  const handleApplyEffect = (effectId: string) => {
    if (!selectedId || !canvas) return

    const obj = findFabricObject(canvas, selectedId)
    const currentState = obj ? captureNodeState(obj) : {}

    const result = generateClipFromEffect(effectId, undefined, undefined, currentState)
    if (!result) return

    const clip: AnimationClipData = {
      id: nanoid(8),
      kind: 'animation',
      startTime: 0,
      duration: result.duration,
      effectId,
      keyframes: result.keyframes,
    }

    const existing = selectedNode?.clips ?? []
    updateNode(selectedId, { clips: [...existing, clip] } as Partial<PenNode>)
  }

  return (
    <>
      {/* Layer name header */}
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

        {/* v2: Effect registry section */}
        {selectedId && (
          <>
            <Separator />
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Effects" />
              {effectCategories.map((category) => {
                const effects = getEffectsByCategory(category)
                if (effects.length === 0) return null
                return (
                  <div key={category} className="space-y-1">
                    <span className="text-[10px] uppercase tracking-wide text-muted-foreground">
                      {category}
                    </span>
                    <div className="grid grid-cols-2 gap-1">
                      {effects.map((effect) => (
                        <Button
                          key={effect.id}
                          variant="outline"
                          size="sm"
                          className="h-7 text-[11px]"
                          onClick={() => handleApplyEffect(effect.id)}
                        >
                          {effect.name}
                        </Button>
                      ))}
                    </div>
                  </div>
                )
              })}
            </div>
          </>
        )}

        {/* Node clips summary */}
        {selectedNode?.clips && selectedNode.clips.length > 0 && (
          <>
            <Separator />
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Clips" />
              <div className="space-y-1">
                {selectedNode.clips.map((clip) => (
                  <div key={clip.id} className="flex items-center justify-between">
                    <span className="text-[11px] text-muted-foreground truncate">
                      {clip.kind === 'animation' ? (clip.effectId ?? 'Custom') : 'Video'}
                    </span>
                    <span className="text-[11px] text-foreground tabular-nums">
                      {(clip.duration / 1000).toFixed(1)}s
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </>
        )}
      </div>
    </>
  )
}
