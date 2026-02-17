import { useEffect } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'
import { MIN_ZOOM, MAX_ZOOM } from './canvas-constants'

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

        const delta = -e.deltaY
        const zoom = canvas.getZoom()
        const factor = delta > 0 ? 1.05 : 0.95
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
            canvas.defaultCursor = 'default'
            canvas.selection = true
          }
        }
      }

      // Keep cursor in sync when switching to/from hand tool
      let prevTool = useCanvasStore.getState().activeTool
      const unsubTool = useCanvasStore.subscribe((state) => {
        if (state.activeTool === prevTool) return
        prevTool = state.activeTool
        const canvas = state.fabricCanvas
        if (!canvas) return
        if (state.activeTool === 'hand') {
          canvas.defaultCursor = 'grab'
          canvas.selection = false
        } else if (!spacePressed) {
          canvas.defaultCursor = 'default'
          canvas.selection = true
        }
      })

      document.addEventListener('keydown', onKeyDown)
      document.addEventListener('keyup', onKeyUp)

      // Check for canvas periodically since it may not exist yet
      const interval = setInterval(() => {
        const canvas = useCanvasStore.getState().fabricCanvas
        if (!canvas) return

        clearInterval(interval)

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
              spacePressed || isHandTool() ? 'grab' : 'default'
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
