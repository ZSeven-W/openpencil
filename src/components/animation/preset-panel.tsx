import { nanoid } from 'nanoid'
import { Button } from '@/components/ui/button'
import { Separator } from '@/components/ui/separator'
import SectionHeader from '@/components/shared/section-header'
import { useTimelineStore } from '@/stores/timeline-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { captureNodeState, findFabricObject } from '@/animation/canvas-bridge'
import { getEffect, getEffectsByCategory, generateClipFromEffect } from '@/animation/effect-registry'
import '@/animation/effects' // ensure effects are registered
import type { AnimationClipData, VideoClipData, TimedEffectConfig } from '@/types/animation'
import { isVideoClip, isAnimationClip } from '@/types/animation'
import type { PenNode, VideoNode } from '@/types/pen'
import NumberInput from '@/components/shared/number-input'

export default function PresetPanel() {
  const selectedId = useCanvasStore((s) => s.selection.activeId)
  const canvas = useCanvasStore((s) => s.fabricCanvas)
  const duration = useTimelineStore((s) => s.duration)
  const updateNode = useDocumentStore((s) => s.updateNode)

  // Subscribe to document so we re-render when node clips change via updateNode.
  // getNodeById is a stable function ref that won't trigger re-renders on its own.
  const selectedNode = useDocumentStore((s) =>
    selectedId ? s.getNodeById(selectedId) : undefined,
  )
  const isVideo = selectedNode?.type === 'video'
  const videoNode = isVideo ? (selectedNode as VideoNode) : null
  const videoClip = videoNode?.clips?.find(isVideoClip) as VideoClipData | undefined

  // v2: Effect registry — only show enter/exit categories (emphasis deferred)
  const effectCategories = ['enter', 'exit'] as const

  // Find the animation clip on this node (or undefined)
  const animClip = selectedNode?.clips?.find(isAnimationClip) as AnimationClipData | undefined

  // Determine which effects are currently applied via inEffect/outEffect
  const appliedEffectIds = new Set<string>()
  if (animClip?.inEffect) appliedEffectIds.add(animClip.inEffect.effectId)
  if (animClip?.outEffect) appliedEffectIds.add(animClip.outEffect.effectId)
  // Legacy: also check effectId for old-style clips
  if (animClip?.effectId) appliedEffectIds.add(animClip.effectId)

  const handleToggleEffect = (effectId: string) => {
    if (!selectedId || !canvas) return

    const effect = getEffect(effectId)
    if (!effect) return

    const obj = findFabricObject(canvas, selectedId)
    const currentState = obj ? captureNodeState(obj) : {}

    const existing = selectedNode?.clips ?? []
    const existingAnimClip = existing.find(isAnimationClip) as AnimationClipData | undefined

    if (effect.category === 'enter') {
      // Toggle in-effect
      if (existingAnimClip?.inEffect?.effectId === effectId) {
        // Remove in-effect
        const updated = { ...existingAnimClip, inEffect: undefined }
        // Regenerate keyframes without in-effect
        const outKf = updated.outEffect
          ? generateClipFromEffect(updated.outEffect.effectId, updated.outEffect.duration, updated.outEffect.params, currentState)
          : null
        updated.keyframes = outKf?.keyframes ?? []
        const updatedClips = existing.map((c) => c.id === existingAnimClip.id ? updated : c)
        updateNode(selectedId, { clips: updatedClips } as Partial<PenNode>)
        return
      }

      const result = generateClipFromEffect(effectId, undefined, undefined, currentState)
      if (!result) return

      const effectConfig: TimedEffectConfig = {
        effectId,
        duration: result.duration,
      }

      if (existingAnimClip) {
        // Set inEffect on existing clip, merge keyframes
        const updated: AnimationClipData = {
          ...existingAnimClip,
          inEffect: effectConfig,
          keyframes: result.keyframes,
        }
        // If there's an out-effect, append its keyframes
        if (existingAnimClip.outEffect) {
          const outResult = generateClipFromEffect(
            existingAnimClip.outEffect.effectId,
            existingAnimClip.outEffect.duration,
            existingAnimClip.outEffect.params,
            currentState,
          )
          if (outResult) updated.keyframes = [...result.keyframes, ...outResult.keyframes]
        }
        const updatedClips = existing.map((c) => c.id === existingAnimClip.id ? updated : c)
        updateNode(selectedId, { clips: updatedClips } as Partial<PenNode>)
      } else {
        // Create new animation clip with inEffect
        const clip: AnimationClipData = {
          id: nanoid(8),
          kind: 'animation',
          startTime: 0,
          duration: duration,
          inEffect: effectConfig,
          keyframes: result.keyframes,
        }
        updateNode(selectedId, { clips: [...existing, clip] } as Partial<PenNode>)
      }
    } else if (effect.category === 'exit') {
      // Toggle out-effect
      if (existingAnimClip?.outEffect?.effectId === effectId) {
        // Remove out-effect
        const updated = { ...existingAnimClip, outEffect: undefined }
        // Regenerate keyframes without out-effect
        const inKf = updated.inEffect
          ? generateClipFromEffect(updated.inEffect.effectId, updated.inEffect.duration, updated.inEffect.params, currentState)
          : null
        updated.keyframes = inKf?.keyframes ?? []
        const updatedClips = existing.map((c) => c.id === existingAnimClip.id ? updated : c)
        updateNode(selectedId, { clips: updatedClips } as Partial<PenNode>)
        return
      }

      const result = generateClipFromEffect(effectId, undefined, undefined, currentState)
      if (!result) return

      const effectConfig: TimedEffectConfig = {
        effectId,
        duration: result.duration,
      }

      if (existingAnimClip) {
        // Set outEffect on existing clip, merge keyframes
        const updated: AnimationClipData = {
          ...existingAnimClip,
          outEffect: effectConfig,
          keyframes: existingAnimClip.keyframes,
        }
        // If there's an in-effect, keep its keyframes and append out keyframes
        if (existingAnimClip.inEffect) {
          const inResult = generateClipFromEffect(
            existingAnimClip.inEffect.effectId,
            existingAnimClip.inEffect.duration,
            existingAnimClip.inEffect.params,
            currentState,
          )
          updated.keyframes = [...(inResult?.keyframes ?? []), ...result.keyframes]
        } else {
          updated.keyframes = result.keyframes
        }
        const updatedClips = existing.map((c) => c.id === existingAnimClip.id ? updated : c)
        updateNode(selectedId, { clips: updatedClips } as Partial<PenNode>)
      } else {
        // Create new animation clip with outEffect
        const clip: AnimationClipData = {
          id: nanoid(8),
          kind: 'animation',
          startTime: 0,
          duration: duration,
          outEffect: effectConfig,
          keyframes: result.keyframes,
        }
        updateNode(selectedId, { clips: [...existing, clip] } as Partial<PenNode>)
      }
    }
  }

  const handleEffectDurationChange = (segment: 'in' | 'out', newDurationSec: number) => {
    if (!selectedId || !canvas || !animClip) return

    const newDurationMs = Math.round(newDurationSec * 1000)
    const effectConfig = segment === 'in' ? animClip.inEffect : animClip.outEffect
    if (!effectConfig) return

    // Clamp: in + out cannot exceed clip duration
    const otherDuration = segment === 'in'
      ? (animClip.outEffect?.duration ?? 0)
      : (animClip.inEffect?.duration ?? 0)
    const maxDuration = animClip.duration - otherDuration - 50 // leave 50ms min hold
    const clampedMs = Math.max(50, Math.min(newDurationMs, maxDuration))

    const obj = findFabricObject(canvas, selectedId)
    const currentState = obj ? captureNodeState(obj) : {}

    const existing = selectedNode?.clips ?? []
    const updatedEffect: TimedEffectConfig = { ...effectConfig, duration: clampedMs }

    const updated: AnimationClipData = {
      ...animClip,
      [segment === 'in' ? 'inEffect' : 'outEffect']: updatedEffect,
    }

    // Regenerate keyframes for both effects
    const inConfig = segment === 'in' ? updatedEffect : animClip.inEffect
    const outConfig = segment === 'out' ? updatedEffect : animClip.outEffect

    const inKf = inConfig
      ? generateClipFromEffect(inConfig.effectId, inConfig.duration, inConfig.params, currentState)
      : null
    const outKf = outConfig
      ? generateClipFromEffect(outConfig.effectId, outConfig.duration, outConfig.params, currentState)
      : null

    updated.keyframes = [...(inKf?.keyframes ?? []), ...(outKf?.keyframes ?? [])]

    const updatedClips = existing.map((c) => c.id === animClip.id ? updated : c)
    updateNode(selectedId, { clips: updatedClips } as Partial<PenNode>)
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
        {videoNode && videoClip && (
          <>
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Video Clip" />
              <div className="space-y-1.5">
                <div className="grid grid-cols-2 gap-1.5">
                  <NumberInput
                    label="In"
                    value={Math.round(videoClip.sourceStart / 100) / 10}
                    onChange={(v) => {
                      const newSourceStart = Math.round(v * 1000)
                      const updatedClips = (selectedNode?.clips ?? []).map((c) =>
                        c.id === videoClip.id ? { ...c, sourceStart: newSourceStart } : c,
                      )
                      updateNode(selectedId!, { clips: updatedClips } as Partial<PenNode>)
                    }}
                    min={0}
                    max={videoClip.sourceEnd / 1000}
                    step={0.1}
                    suffix="s"
                  />
                  <NumberInput
                    label="Out"
                    value={Math.round(videoClip.sourceEnd / 100) / 10}
                    onChange={(v) => {
                      const newSourceEnd = Math.round(v * 1000)
                      const updatedClips = (selectedNode?.clips ?? []).map((c) =>
                        c.id === videoClip.id ? { ...c, sourceEnd: newSourceEnd } : c,
                      )
                      updateNode(selectedId!, { clips: updatedClips } as Partial<PenNode>)
                    }}
                    min={videoClip.sourceStart / 1000}
                    max={(videoNode.videoDuration ?? 0) / 1000}
                    step={0.1}
                    suffix="s"
                  />
                </div>
                <NumberInput
                  label="Offset"
                  value={Math.round(videoClip.startTime / 100) / 10}
                  onChange={(v) => {
                    const newStartTime = Math.round(v * 1000)
                    const updatedClips = (selectedNode?.clips ?? []).map((c) =>
                      c.id === videoClip.id ? { ...c, startTime: newStartTime } : c,
                    )
                    updateNode(selectedId!, { clips: updatedClips } as Partial<PenNode>)
                  }}
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
                    {(videoClip.duration / 1000).toFixed(1)}s
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
                      {effects.map((effect) => {
                        const isActive = appliedEffectIds.has(effect.id)
                        return (
                          <Button
                            key={effect.id}
                            variant={isActive ? 'default' : 'outline'}
                            size="sm"
                            className="h-7 text-[11px]"
                            onClick={() => handleToggleEffect(effect.id)}
                          >
                            {effect.name}
                          </Button>
                        )
                      })}
                    </div>
                  </div>
                )
              })}
            </div>
          </>
        )}

        {/* In/Out duration controls */}
        {animClip && (animClip.inEffect || animClip.outEffect) && (
          <>
            <Separator />
            <div className="px-3 py-2 space-y-1.5">
              <SectionHeader title="Timing" />
              <div className="grid grid-cols-2 gap-1.5">
                {animClip.inEffect && (
                  <NumberInput
                    label="In"
                    value={Math.round(animClip.inEffect.duration / 100) / 10}
                    onChange={(v) => handleEffectDurationChange('in', v)}
                    min={0.05}
                    step={0.1}
                    suffix="s"
                  />
                )}
                {animClip.outEffect && (
                  <NumberInput
                    label="Out"
                    value={Math.round(animClip.outEffect.duration / 100) / 10}
                    onChange={(v) => handleEffectDurationChange('out', v)}
                    min={0.05}
                    step={0.1}
                    suffix="s"
                  />
                )}
              </div>
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
                      {clip.kind === 'animation'
                        ? (clip.inEffect || clip.outEffect
                          ? [clip.inEffect?.effectId, clip.outEffect?.effectId].filter(Boolean).join(' → ')
                          : (clip.effectId ?? 'Custom'))
                        : 'Video'}
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
