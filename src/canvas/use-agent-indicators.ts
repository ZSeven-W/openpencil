/**
 * Canvas rendering hook for agent indicators.
 *
 * Draws colored breathing-glow borders and name pills above nodes that
 * are being actively streamed by a sub-agent during concurrent generation.
 *
 * Uses the established `after:render` + lower canvas 2D context pattern
 * (same as use-frame-labels.ts).
 */

import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { getActiveAgentIndicators, isPreviewNode } from './agent-indicator'
import type { FabricObjectWithPenId } from './canvas-object-factory'

const PILL_FONT_SIZE = 10
const PILL_PAD_X = 6
const PILL_PAD_Y = 3
const PILL_RADIUS = 4
const PILL_OFFSET_Y = 8
const BORDER_WIDTH = 2

export function useAgentIndicators() {
  useEffect(() => {
    let detach: (() => void) | null = null
    let rafId: number | null = null

    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      const onAfterRender = () => {
        const indicators = getActiveAgentIndicators()
        if (indicators.size === 0) return

        const el = canvas.lowerCanvasEl
        if (!el) return
        const ctx = el.getContext('2d')
        if (!ctx) return

        const vpt = canvas.viewportTransform
        if (!vpt) return
        const zoom = vpt[0]
        if (!Number.isFinite(zoom) || zoom <= 0) return
        if (!el.offsetWidth) return
        const dpr = el.width / el.offsetWidth

        // Breathing opacity: pulses between 0.05 and 0.75
        const breathAlpha = 0.4 + Math.sin(Date.now() / 600) * 0.35

        const objects = canvas.getObjects() as FabricObjectWithPenId[]
        const objMap = new Map<string, FabricObjectWithPenId>()
        for (const obj of objects) {
          if (obj.penNodeId) objMap.set(obj.penNodeId, obj)
        }

        ctx.save()
        ctx.setTransform(
          vpt[0] * dpr, vpt[1] * dpr,
          vpt[2] * dpr, vpt[3] * dpr,
          vpt[4] * dpr, vpt[5] * dpr,
        )

        for (const entry of indicators.values()) {
          const obj = objMap.get(entry.nodeId)
          if (!obj) continue

          const corners = obj.getCoords()
          const xs = corners.map((p) => p.x)
          const ys = corners.map((p) => p.y)
          const x = Math.min(...xs)
          const y = Math.min(...ys)
          const w = Math.max(...xs) - x
          const h = Math.max(...ys) - y

          // -- Breathing glow border --
          const bw = BORDER_WIDTH / zoom
          ctx.strokeStyle = entry.color
          ctx.globalAlpha = breathAlpha
          ctx.lineWidth = bw
          ctx.strokeRect(x - bw, y - bw, w + bw * 2, h + bw * 2)

          // -- Subtle fill for preview nodes (element not yet materialized) --
          if (isPreviewNode(entry.nodeId)) {
            ctx.fillStyle = entry.color
            ctx.globalAlpha = breathAlpha * 0.12
            ctx.fillRect(x, y, w, h)
          }

          // -- Name pill above the node --
          ctx.globalAlpha = 0.9
          const fontSize = PILL_FONT_SIZE / zoom
          const padX = PILL_PAD_X / zoom
          const padY = PILL_PAD_Y / zoom
          const radius = PILL_RADIUS / zoom
          const offsetY = PILL_OFFSET_Y / zoom

          ctx.font = `600 ${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
          const textWidth = ctx.measureText(entry.name).width
          const pillW = textWidth + padX * 2
          const pillH = fontSize + padY * 2
          const pillX = x
          const pillY = y - offsetY - pillH

          // Rounded rect background
          ctx.fillStyle = entry.color
          ctx.beginPath()
          ctx.moveTo(pillX + radius, pillY)
          ctx.lineTo(pillX + pillW - radius, pillY)
          ctx.arcTo(pillX + pillW, pillY, pillX + pillW, pillY + radius, radius)
          ctx.lineTo(pillX + pillW, pillY + pillH - radius)
          ctx.arcTo(pillX + pillW, pillY + pillH, pillX + pillW - radius, pillY + pillH, radius)
          ctx.lineTo(pillX + radius, pillY + pillH)
          ctx.arcTo(pillX, pillY + pillH, pillX, pillY + pillH - radius, radius)
          ctx.lineTo(pillX, pillY + radius)
          ctx.arcTo(pillX, pillY, pillX + radius, pillY, radius)
          ctx.closePath()
          ctx.fill()

          // White text
          ctx.fillStyle = '#FFFFFF'
          ctx.textBaseline = 'top'
          ctx.fillText(entry.name, pillX + padX, pillY + padY)
        }

        ctx.globalAlpha = 1
        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)

      // RAF loop to drive breathing animation while indicators are active
      const tick = () => {
        if (getActiveAgentIndicators().size > 0) {
          canvas.requestRenderAll()
        }
        rafId = requestAnimationFrame(tick)
      }
      rafId = requestAnimationFrame(tick)

      detach = () => {
        canvas.off('after:render', onAfterRender)
        if (rafId !== null) cancelAnimationFrame(rafId)
      }
    }, 100)

    return () => {
      clearInterval(interval)
      detach?.()
    }
  }, [])
}
