/**
 * Canvas rendering hook for agent indicators during concurrent generation.
 *
 * Visual effects:
 * 1. Soft colored border on nodes being generated (breathing opacity)
 * 2. Agent badge next to the frame name with a spinning dot animation
 * 3. Preview fill tint on nodes that haven't materialized yet
 */

import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import {
  getActiveAgentIndicators,
  getActiveAgentFrames,
  isPreviewNode,
} from './agent-indicator'
import { isNodeBorderReady } from '@/services/ai/design-animation'
import type { FabricObjectWithPenId } from './canvas-object-factory'

// Badge styling
const BADGE_FONT_SIZE = 11
const BADGE_PAD_X = 6
const BADGE_PAD_Y = 3
const BADGE_RADIUS = 4
const BADGE_GAP = 6
const DOT_RADIUS = 3

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
        const agentFrames = getActiveAgentFrames()
        if (indicators.size === 0 && agentFrames.size === 0) return

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
        // Gentle breathing: range [0.4, 0.8]
        const breath = 0.6 + Math.sin((t / 600) * Math.PI * 2) * 0.2

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

        // ---- Draw node borders (only for nodes whose reveal time has arrived) ----
        for (const entry of indicators.values()) {
          // Sequential queue: only show border when this node's turn arrives
          if (!isNodeBorderReady(entry.nodeId)) continue

          const obj = objMap.get(entry.nodeId)
          if (!obj) continue

          const corners = obj.getCoords()
          const xs = corners.map((p) => p.x)
          const ys = corners.map((p) => p.y)
          const x = Math.min(...xs)
          const y = Math.min(...ys)
          const w = Math.max(...xs) - x
          const h = Math.max(...ys) - y
          if (w < 1 || h < 1) continue

          const preview = isPreviewNode(entry.nodeId)

          // Dashed colored border
          const borderWidth = 1.5 / zoom
          ctx.strokeStyle = entry.color
          ctx.globalAlpha = breath * 0.7
          ctx.lineWidth = borderWidth
          ctx.setLineDash([4 / zoom, 3 / zoom])
          ctx.strokeRect(x, y, w, h)
          ctx.setLineDash([])

          // Preview fill tint (content not yet materialized)
          if (preview) {
            ctx.fillStyle = entry.color
            ctx.globalAlpha = 0.06 + Math.sin((t / 500) * Math.PI * 2) * 0.03
            ctx.fillRect(x, y, w, h)
          }
        }

        // ---- Draw agent badges next to frame names ----
        for (const frame of agentFrames.values()) {
          const obj = objMap.get(frame.frameId)
          if (!obj) continue

          const corners = obj.getCoords()
          const x = Math.min(...corners.map((p) => p.x))
          const y = Math.min(...corners.map((p) => p.y))

          // Measure the existing frame label width to position badge after it
          const fontSize = BADGE_FONT_SIZE / zoom
          const labelFontSize = 12 / zoom
          ctx.font = `500 ${labelFontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
          const docStore = useDocumentStore.getState()
          const frameNode = docStore.getNodeById(frame.frameId)
          const frameName = frameNode?.name ?? frameNode?.type ?? ''
          const labelWidth = frameName ? ctx.measureText(frameName).width + BADGE_GAP / zoom : 0

          // Badge position: right of frame label, above frame
          const padX = BADGE_PAD_X / zoom
          const padY = BADGE_PAD_Y / zoom
          const radius = BADGE_RADIUS / zoom
          const dotR = DOT_RADIUS / zoom
          const labelOffsetY = 6 / zoom

          ctx.font = `600 ${fontSize}px -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif`
          const textWidth = ctx.measureText(frame.name).width
          // Space for spinning dot + text
          const dotSpace = dotR * 2 + 4 / zoom
          const badgeW = dotSpace + textWidth + padX * 2
          const badgeH = fontSize + padY * 2
          const badgeX = x + labelWidth
          const badgeY = y - labelOffsetY - badgeH

          // Badge background (pill shape)
          ctx.globalAlpha = 0.9
          ctx.fillStyle = frame.color
          drawRoundedRect(ctx, badgeX, badgeY, badgeW, badgeH, radius)
          ctx.fill()

          // Spinning dot animation
          const cx = badgeX + padX + dotR
          const cy = badgeY + badgeH / 2
          const angle = (t / 400) * Math.PI * 2

          // Orbiting trail dots (3 dots trailing)
          for (let d = 0; d < 3; d++) {
            const trailAngle = angle - d * 0.6
            const trailR = dotR * 0.8
            const dx = Math.cos(trailAngle) * dotR * 0.7
            const dy = Math.sin(trailAngle) * dotR * 0.7
            ctx.globalAlpha = 0.4 - d * 0.12
            ctx.fillStyle = '#FFFFFF'
            ctx.beginPath()
            ctx.arc(cx + dx, cy + dy, trailR * (1 - d * 0.2), 0, Math.PI * 2)
            ctx.fill()
          }

          // Main spinning dot
          const mainDx = Math.cos(angle) * dotR * 0.7
          const mainDy = Math.sin(angle) * dotR * 0.7
          ctx.globalAlpha = 0.95
          ctx.fillStyle = '#FFFFFF'
          ctx.beginPath()
          ctx.arc(cx + mainDx, cy + mainDy, dotR * 0.6, 0, Math.PI * 2)
          ctx.fill()

          // Agent name text
          ctx.globalAlpha = 1
          ctx.fillStyle = '#FFFFFF'
          ctx.textBaseline = 'top'
          ctx.fillText(frame.name, badgeX + padX + dotSpace, badgeY + padY)
        }

        ctx.globalAlpha = 1
        ctx.restore()
      }

      canvas.on('after:render', onAfterRender)

      // RAF loop to drive breathing + spinning animation while indicators are active
      const tick = () => {
        if (getActiveAgentIndicators().size > 0 || getActiveAgentFrames().size > 0) {
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
