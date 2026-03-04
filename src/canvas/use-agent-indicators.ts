/**
 * Canvas rendering hook for agent indicators.
 *
 * Draws colored breathing-glow borders and name pills above nodes that
 * are being actively streamed by a sub-agent during concurrent generation.
 *
 * Visual effect for preview nodes (not yet materialized):
 * - Outer glow: thick semi-transparent border for soft glow
 * - Inner border: sharp crisp border
 * - Colored fill tint: visible placeholder
 * - Name pill: agent name badge above the node
 *
 * Uses the established `after:render` + lower canvas 2D context pattern
 * (same as use-frame-labels.ts).
 */

import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { getActiveAgentIndicators, isPreviewNode } from './agent-indicator'
import type { FabricObjectWithPenId } from './canvas-object-factory'

const PILL_FONT_SIZE = 11
const PILL_PAD_X = 7
const PILL_PAD_Y = 4
const PILL_RADIUS = 5
const PILL_OFFSET_Y = 10

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

        const t = Date.now()
        // Breathing: range [0.35, 0.95], faster cycle (400ms period)
        const breath = 0.65 + Math.sin(t / 400 * Math.PI * 2) * 0.30

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

        // Deduplicate pills: for nodes sharing the same agent name,
        // only draw the pill once (on the topmost node in view).
        const pillDrawn = new Set<string>()

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

          // Skip zero-sized objects
          if (w < 1 || h < 1) continue

          const preview = isPreviewNode(entry.nodeId)

          // ---- Outer glow (soft, thick) ----
          const glowWidth = 8 / zoom
          ctx.strokeStyle = entry.color
          ctx.globalAlpha = breath * 0.35
          ctx.lineWidth = glowWidth
          ctx.strokeRect(
            x - glowWidth / 2, y - glowWidth / 2,
            w + glowWidth, h + glowWidth,
          )

          // ---- Inner border (sharp, crisp) ----
          const innerWidth = 2.5 / zoom
          ctx.globalAlpha = breath * 0.9
          ctx.lineWidth = innerWidth
          ctx.strokeRect(x, y, w, h)

          // ---- Preview fill (visible placeholder) ----
          if (preview) {
            ctx.fillStyle = entry.color
            ctx.globalAlpha = 0.10 + Math.sin(t / 500 * Math.PI * 2) * 0.05
            ctx.fillRect(x, y, w, h)
          }

          // ---- Name pill (draw once per agent) ----
          const pillKey = `${entry.name}-${entry.color}`
          if (!pillDrawn.has(pillKey)) {
            pillDrawn.add(pillKey)

            ctx.globalAlpha = 0.95
            const fontSize = PILL_FONT_SIZE / zoom
            const padX = PILL_PAD_X / zoom
            const padY = PILL_PAD_Y / zoom
            const radius = PILL_RADIUS / zoom
            const offsetY = PILL_OFFSET_Y / zoom

            ctx.font = `700 ${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
            const textWidth = ctx.measureText(entry.name).width
            const pillW = textWidth + padX * 2
            const pillH = fontSize + padY * 2
            const pillX = x
            const pillY = y - offsetY - pillH

            // Pill shadow
            ctx.fillStyle = 'rgba(0,0,0,0.25)'
            drawRoundedRect(ctx, pillX + 1 / zoom, pillY + 1 / zoom, pillW, pillH, radius)
            ctx.fill()

            // Pill background
            ctx.fillStyle = entry.color
            drawRoundedRect(ctx, pillX, pillY, pillW, pillH, radius)
            ctx.fill()

            // Pill text
            ctx.fillStyle = '#FFFFFF'
            ctx.textBaseline = 'top'
            ctx.fillText(entry.name, pillX + padX, pillY + padY)
          }
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

// ---------------------------------------------------------------------------
// Rounded rect helper
// ---------------------------------------------------------------------------

function drawRoundedRect(
  ctx: CanvasRenderingContext2D,
  x: number, y: number, w: number, h: number, r: number,
) {
  ctx.beginPath()
  ctx.moveTo(x + r, y)
  ctx.lineTo(x + w - r, y)
  ctx.arcTo(x + w, y, x + w, y + r, r)
  ctx.lineTo(x + w, y + h - r)
  ctx.arcTo(x + w, y + h, x + w - r, y + h, r)
  ctx.lineTo(x + r, y + h)
  ctx.arcTo(x, y + h, x, y + h - r, r)
  ctx.lineTo(x, y + r)
  ctx.arcTo(x, y, x + r, y, r)
  ctx.closePath()
}
