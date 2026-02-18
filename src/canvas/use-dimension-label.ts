import { useEffect, useRef, type RefObject } from 'react'
import { useCanvasStore } from '@/stores/canvas-store'

export function useDimensionLabel(
  containerRef: RefObject<HTMLDivElement | null>,
) {
  const labelRef = useRef<HTMLDivElement | null>(null)

  useEffect(() => {
    const container = containerRef.current
    if (!container) return

    // Create the label element
    const label = document.createElement('div')
    label.style.cssText = `
      position: absolute;
      pointer-events: none;
      z-index: 5;
      background: #0d99ff;
      color: #fff;
      font-size: 11px;
      font-weight: 500;
      font-family: -apple-system, BlinkMacSystemFont, sans-serif;
      padding: 2px 8px;
      border-radius: 4px;
      white-space: nowrap;
      display: none;
    `
    container.appendChild(label)
    labelRef.current = label

    const update = () => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas || !label) return

      const active = canvas.getActiveObject()
      if (!active) {
        label.style.display = 'none'
        return
      }

      const bound = active.getBoundingRect()
      const w = Math.round(active.getScaledWidth())
      const h = Math.round(active.getScaledHeight())

      label.textContent = `${w} \u00d7 ${h}`
      label.style.display = 'block'
      label.style.left = `${bound.left + bound.width / 2}px`
      label.style.top = `${bound.top + bound.height + 8}px`
      label.style.transform = 'translateX(-50%)'
    }

    // Poll for canvas availability
    const interval = setInterval(() => {
      const canvas = useCanvasStore.getState().fabricCanvas
      if (!canvas) return
      clearInterval(interval)

      canvas.on('after:render', update)
    }, 100)

    return () => {
      clearInterval(interval)
      const canvas = useCanvasStore.getState().fabricCanvas
      if (canvas) {
        canvas.off('after:render', update)
      }
      label.remove()
      labelRef.current = null
    }
  }, [containerRef])
}
