import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { MIN_ZOOM, MAX_ZOOM } from './canvas-constants'
import type { ToolType } from '@/types/canvas'

// Precise crosshair cursor (thin +)
const CROSSHAIR_CURSOR = (() => {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><line x1="12" y1="2" x2="12" y2="10" stroke="%23222" stroke-width="1"/><line x1="12" y1="14" x2="12" y2="22" stroke="%23222" stroke-width="1"/><line x1="2" y1="12" x2="10" y2="12" stroke="%23222" stroke-width="1"/><line x1="14" y1="12" x2="22" y2="12" stroke="%23222" stroke-width="1"/></svg>`
  return `url("data:image/svg+xml,${svg}") 12 12, crosshair`
})()

function toolToCursor(tool: ToolType): string {
  switch (tool) {
    case 'hand':
      return 'grab'
    case 'text':
      return 'text'
    case 'select':
      return 'default'
    default:
      return CROSSHAIR_CURSOR
  }
}

export function useCanvasViewport() {
  useEffect(() => {
    // Set up wheel zoom
    const handleWheel = () => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return

      canvas.on('mouse:wheel', (opt) => {
        const e = opt.e as WheelEvent
        e.preventDefault()
        e.stopPropagation()

        // Normalize: trackpads send small pixel deltas, mice send larger line deltas
        let delta = -e.deltaY
        if (e.deltaMode === 1) delta *= 40 // line mode → approx pixels
        const zoom = canvas.getZoom()
        // Smooth exponential zoom — works naturally for both trackpad and mouse
        const factor = Math.pow(1.002, delta)
        let newZoom = zoom * factor

        newZoom = Math.max(MIN_ZOOM, Math.min(MAX_ZOOM, newZoom))

        const point = canvas.getScenePoint(e)
        canvas.zoomToPoint(point, newZoom)

        const vpt = canvas.viewportTransform
        if (vpt) {
          useCanvasStore.getState().setZoom(newZoom)
          useCanvasStore.getState().setPan(vpt[4], vpt[5])
        }

        canvas.requestRenderAll()
      })
    }

    // Set up space + drag panning
    const handlePan = () => {
      let isPanning = false
      let lastX = 0
      let lastY = 0
      let spacePressed = false

      const isHandTool = () =>
        useCanvasStore.getState().activeTool === 'hand'

      const currentToolCursor = () =>
        toolToCursor(useCanvasStore.getState().activeTool)

      const onKeyDown = (e: KeyboardEvent) => {
        if (e.code === 'Space' && !e.repeat) {
          spacePressed = true
          const canvas = useCanvasStore.getState().fabricCanvas
          if (canvas) {
            canvas.defaultCursor = 'grab'
            canvas.selection = false
          }
        }
      }

      const onKeyUp = (e: KeyboardEvent) => {
        if (e.code === 'Space') {
          spacePressed = false
          isPanning = false
          const canvas = useCanvasStore.getState().fabricCanvas
          if (canvas && !isHandTool()) {
            canvas.defaultCursor = currentToolCursor()
            canvas.selection = true
          }
        }
      }

      // Keep cursor in sync when switching tools
      let prevTool = useCanvasStore.getState().activeTool
      const unsubTool = useCanvasStore.subscribe((state) => {
        if (state.activeTool === prevTool) return
        prevTool = state.activeTool
        const canvas = state.fabricCanvas
        if (!canvas) return
        const cursor = toolToCursor(state.activeTool)
        if (state.activeTool === 'hand') {
          canvas.selection = false
        } else if (!spacePressed) {
          canvas.selection = true
        }
        if (!spacePressed) {
          canvas.defaultCursor = cursor
        }
      })

      document.addEventListener('keydown', onKeyDown)
      document.addEventListener('keyup', onKeyUp)

      // Check for canvas periodically since it may not exist yet
      const interval = setInterval(() => {
        const canvas = useCanvasStore.getState().fabricCanvas
        if (!canvas) return

        clearInterval(interval)

        // Set initial cursor
        canvas.defaultCursor = currentToolCursor()

        canvas.on('mouse:down', (opt) => {
          const e = opt.e as MouseEvent
          if (spacePressed || isHandTool() || e.button === 1) {
            isPanning = true
            lastX = e.clientX
            lastY = e.clientY
            canvas.defaultCursor = 'grabbing'
            useCanvasStore.getState().setInteraction({ isPanning: true })
          }
        })

        canvas.on('mouse:move', (opt) => {
          if (!isPanning) return
          const e = opt.e as MouseEvent
          const dx = e.clientX - lastX
          const dy = e.clientY - lastY
          lastX = e.clientX
          lastY = e.clientY

          const vpt = canvas.viewportTransform
          if (vpt) {
            vpt[4] += dx
            vpt[5] += dy
            canvas.setViewportTransform(vpt)
            useCanvasStore.getState().setPan(vpt[4], vpt[5])
          }
        })

        canvas.on('mouse:up', () => {
          if (isPanning) {
            isPanning = false
            canvas.defaultCursor =
              spacePressed || isHandTool() ? 'grab' : currentToolCursor()
            useCanvasStore.getState().setInteraction({ isPanning: false })
          }
        })
      }, 100)

      return () => {
        document.removeEventListener('keydown', onKeyDown)
        document.removeEventListener('keyup', onKeyUp)
        clearInterval(interval)
        unsubTool()
      }
    }

    handleWheel()
    const cleanupPan = handlePan()

    return () => {
      cleanupPan?.()
    }
  }, [])
}
