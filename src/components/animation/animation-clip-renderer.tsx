/**
 * Custom action renderer for animation clip actions.
 * Shows In / Hold / Out segments within a single clip bar.
 * Segment widths are proportional to their duration relative to total clip duration.
 * In/Out divider edges are draggable to adjust segment durations.
 * Colors sourced from --clip-animation design tokens.
 */

import { useRef, useCallback } from 'react'
import { useDocumentStore } from '@/stores/document-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { isAnimationClip } from '@/types/animation'
import type { AnimationClipData, TimedEffectConfig } from '@/types/animation'
import type { PenNode } from '@/types/pen'
import { getEffect } from '@/animation/effect-registry'
import { generateClipFromEffect } from '@/animation/effect-registry'
import { captureNodeState, findFabricObject } from '@/animation/canvas-bridge'
import '@/animation/effects'

interface AnimationClipRendererProps {
  clipId: string
}

/** Minimum segment duration in ms */
const MIN_SEGMENT_MS = 50

export default function AnimationClipRenderer({
  clipId,
}: AnimationClipRendererProps) {
  const containerRef = useRef<HTMLDivElement>(null)

  // Read clip data from document store for in/out segment sizing
  const clip = useDocumentStore((s) => {
    for (const node of s.getFlatNodes()) {
      const found = node.clips?.find((c) => c.id === clipId)
      if (found && isAnimationClip(found)) return found
    }
    return undefined
  })

  // Find the node that owns this clip (for updating)
  const nodeId = useDocumentStore((s) => {
    for (const node of s.getFlatNodes()) {
      if (node.clips?.some((c) => c.id === clipId)) return node.id
    }
    return undefined
  })

  const updateNode = useDocumentStore((s) => s.updateNode)

  const handleDividerDrag = useCallback(
    (segment: 'in' | 'out', e: React.PointerEvent) => {
      e.stopPropagation()
      e.preventDefault()

      if (!clip || !nodeId || !containerRef.current) return

      // Capture pointer so all subsequent pointer events route to this element,
      // preventing the timeline library's @use-gesture from interpreting the
      // drag as a clip move.
      const target = e.currentTarget as HTMLElement
      target.setPointerCapture(e.nativeEvent.pointerId)

      const containerWidth = containerRef.current.getBoundingClientRect().width
      if (containerWidth <= 0) return

      const msPerPx = clip.duration / containerWidth
      const startX = e.clientX
      const startDuration = segment === 'in'
        ? (clip.inEffect?.duration ?? 0)
        : (clip.outEffect?.duration ?? 0)

      const otherDuration = segment === 'in'
        ? (clip.outEffect?.duration ?? 0)
        : (clip.inEffect?.duration ?? 0)
      const maxDuration = clip.duration - otherDuration - MIN_SEGMENT_MS

      const onMove = (moveEvent: PointerEvent) => {
        moveEvent.stopPropagation()
        const dx = moveEvent.clientX - startX
        // For 'in' segment: dragging right increases duration
        // For 'out' segment: dragging left increases duration
        const direction = segment === 'in' ? 1 : -1
        const deltaMs = dx * msPerPx * direction
        const newDuration = Math.round(
          Math.max(MIN_SEGMENT_MS, Math.min(startDuration + deltaMs, maxDuration)),
        )

        // Update the clip in the store
        const effectConfig = segment === 'in' ? clip.inEffect : clip.outEffect
        if (!effectConfig) return

        const updatedEffect: TimedEffectConfig = { ...effectConfig, duration: newDuration }

        // Get canvas state for keyframe regeneration
        const canvas = useCanvasStore.getState().fabricCanvas
        const obj = canvas ? findFabricObject(canvas, nodeId) : null
        const currentState = obj ? captureNodeState(obj) : {}

        const inConfig = segment === 'in' ? updatedEffect : clip.inEffect
        const outConfig = segment === 'out' ? updatedEffect : clip.outEffect

        const inKf = inConfig
          ? generateClipFromEffect(inConfig.effectId, inConfig.duration, inConfig.params, currentState)
          : null
        const outKf = outConfig
          ? generateClipFromEffect(outConfig.effectId, outConfig.duration, outConfig.params, currentState)
          : null

        const updated: AnimationClipData = {
          ...clip,
          [segment === 'in' ? 'inEffect' : 'outEffect']: updatedEffect,
          keyframes: [...(inKf?.keyframes ?? []), ...(outKf?.keyframes ?? [])],
        }

        const node = useDocumentStore.getState().getNodeById(nodeId)
        if (!node?.clips) return
        const updatedClips = node.clips.map((c) => c.id === clipId ? updated : c)
        updateNode(nodeId, { clips: updatedClips } as Partial<PenNode>)
      }

      const onUp = (upEvent: PointerEvent) => {
        upEvent.stopPropagation()
        target.releasePointerCapture(upEvent.pointerId)
        target.removeEventListener('pointermove', onMove)
        target.removeEventListener('pointerup', onUp)
      }

      // With pointer capture, events route to the target element, not document
      target.addEventListener('pointermove', onMove)
      target.addEventListener('pointerup', onUp)
    },
    [clip, clipId, nodeId, updateNode],
  )

  if (!clip) {
    return (
      <div className="h-full w-full flex items-center justify-center rounded-[14px] bg-clip-animation-bg border border-clip-animation-border">
        <span className="text-[9px] text-clip-animation/60">{clipId.slice(0, 6)}</span>
      </div>
    )
  }

  const inDur = clip.inEffect?.duration ?? 0
  const outDur = clip.outEffect?.duration ?? 0
  const totalDur = clip.duration
  const holdDur = Math.max(0, totalDur - inDur - outDur)

  const inPct = totalDur > 0 ? (inDur / totalDur) * 100 : 0
  const outPct = totalDur > 0 ? (outDur / totalDur) * 100 : 0
  const holdPct = 100 - inPct - outPct

  const inLabel = clip.inEffect ? getEffect(clip.inEffect.effectId)?.name : undefined
  const outLabel = clip.outEffect ? getEffect(clip.outEffect.effectId)?.name : undefined

  const hasSegments = inDur > 0 || outDur > 0

  return (
    <div
      ref={containerRef}
      className="h-full w-full flex items-stretch overflow-hidden rounded-[14px] bg-clip-animation-bg border border-clip-animation-border hover:border-clip-animation hover:border-[1.5px] transition-colors"
    >
      {hasSegments ? (
        <>
          {/* In segment */}
          {inPct > 0 && (
            <div
              className="relative flex items-center justify-center bg-clip-animation/20"
              style={{ width: `${inPct}%` }}
            >
              <span className="text-[8px] text-clip-animation truncate px-0.5">
                {inLabel ?? 'In'}
              </span>
              {/* Draggable right edge */}
              <div
                className="absolute right-0 top-0 w-[6px] h-full cursor-ew-resize z-10 group/edge"
                onPointerDown={(e) => handleDividerDrag('in', e)}
              >
                <div className="absolute right-[2px] top-0 w-[2px] h-full bg-clip-animation-border/50 group-hover/edge:bg-clip-animation transition-colors" />
              </div>
            </div>
          )}

          {/* Hold segment */}
          <div
            className="flex-1 min-w-0 flex items-center justify-center"
            style={{ width: `${holdPct}%` }}
          >
            {holdDur > 100 && (
              <span className="text-[9px] text-clip-animation/60 truncate px-0.5">
                {(holdDur / 1000).toFixed(1)}s
              </span>
            )}
          </div>

          {/* Out segment */}
          {outPct > 0 && (
            <div
              className="relative flex items-center justify-center bg-clip-animation/20"
              style={{ width: `${outPct}%` }}
            >
              {/* Draggable left edge */}
              <div
                className="absolute left-0 top-0 w-[6px] h-full cursor-ew-resize z-10 group/edge"
                onPointerDown={(e) => handleDividerDrag('out', e)}
              >
                <div className="absolute left-[2px] top-0 w-[2px] h-full bg-clip-animation-border/50 group-hover/edge:bg-clip-animation transition-colors" />
              </div>
              <span className="text-[8px] text-clip-animation truncate px-0.5">
                {outLabel ?? 'Out'}
              </span>
            </div>
          )}
        </>
      ) : (
        /* No in/out effects — show simple label */
        <div className="flex-1 min-w-0 flex items-center justify-center px-1.5">
          <span className="text-[9px] text-clip-animation truncate">
            {clip.effectId ? (getEffect(clip.effectId)?.name ?? clipId.slice(0, 6)) : 'Custom'}
          </span>
        </div>
      )}
    </div>
  )
}
