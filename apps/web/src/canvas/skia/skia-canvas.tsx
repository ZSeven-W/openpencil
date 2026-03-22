import { useRef, useEffect, useState } from 'react'
import { loadCanvasKit } from './skia-init'
import { SkiaEngine } from './skia-engine'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { setSkiaEngineRef } from '../skia-engine-ref'
import type { PenNode } from '@/types/pen'
import { SkiaInteractionManager, type TextEditState } from './skia-interaction'

export default function SkiaCanvas() {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const containerRef = useRef<HTMLDivElement>(null)
  const engineRef = useRef<SkiaEngine | null>(null)
  const [error, setError] = useState<string | null>(null)
  const [editingText, setEditingText] = useState<TextEditState | null>(null)

  // Initialize CanvasKit + engine
  useEffect(() => {
    let disposed = false

    async function init() {
      try {
        const ck = await loadCanvasKit()
        if (disposed) return

        const canvasEl = canvasRef.current
        if (!canvasEl) return

        const engine = new SkiaEngine(ck)
        engine.init(canvasEl)
        engineRef.current = engine
        setSkiaEngineRef(engine)

        // Initial sync
        engine.syncFromDocument()
        requestAnimationFrame(() => engine.zoomToFitContent())

      } catch (err) {
        console.error('SkiaCanvas init failed:', err)
        setError(String(err))
      }
    }

    init()

    return () => {
      disposed = true
      setSkiaEngineRef(null)
      engineRef.current?.dispose()
      engineRef.current = null
    }
  }, [])

  // Resize observer
  useEffect(() => {
    const container = containerRef.current
    if (!container) return
    const observer = new ResizeObserver((entries) => {
      const engine = engineRef.current
      if (!engine) return
      for (const entry of entries) {
        const { width, height } = entry.contentRect
        engine.resize(width, height)
      }
    })
    observer.observe(container)
    return () => observer.disconnect()
  }, [])

  // Document sync: re-render when document changes
  useEffect(() => {
    const unsub = useDocumentStore.subscribe(() => {
      engineRef.current?.syncFromDocument()
    })
    return unsub
  }, [])

  // Page sync: re-render when active page changes
  useEffect(() => {
    let prevPageId = useCanvasStore.getState().activePageId
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.activePageId !== prevPageId) {
        prevPageId = state.activePageId
        engineRef.current?.syncFromDocument()
      }
    })
    return unsub
  }, [])

  // Selection sync: re-render when selection changes
  useEffect(() => {
    let prevIds = useCanvasStore.getState().selection.selectedIds
    const unsub = useCanvasStore.subscribe((state) => {
      if (state.selection.selectedIds !== prevIds) {
        prevIds = state.selection.selectedIds
        engineRef.current?.markDirty()
      }
    })
    return unsub
  }, [])

  // Wheel: zoom + pan
  useEffect(() => {
    const canvasEl = canvasRef.current
    if (!canvasEl) return

    const handleWheel = (e: WheelEvent) => {
      e.preventDefault()
      e.stopPropagation()
      const engine = engineRef.current
      if (!engine) return

      if (e.ctrlKey || e.metaKey) {
        let delta = -e.deltaY
        if (e.deltaMode === 1) delta *= 40
        const factor = Math.pow(1.005, delta)
        const newZoom = engine.zoom * factor
        engine.zoomToPoint(e.clientX, e.clientY, newZoom)
      } else {
        let dx = -e.deltaX
        let dy = -e.deltaY
        if (e.deltaMode === 1) { dx *= 40; dy *= 40 }
        engine.pan(dx, dy)
      }
    }

    canvasEl.addEventListener('wheel', handleWheel, { passive: false })
    return () => canvasEl.removeEventListener('wheel', handleWheel)
  }, [])

  // Mouse/keyboard interactions (select, move, resize, draw, hover, etc.)
  useEffect(() => {
    const canvasEl = canvasRef.current
    if (!canvasEl) return

    const manager = new SkiaInteractionManager(engineRef, canvasEl, setEditingText)
    return manager.attach()
  }, [])

  return (
    <div
      ref={containerRef}
      className="flex-1 relative overflow-hidden bg-muted"
    >
      <canvas
        ref={canvasRef}
        className="absolute inset-0 w-full h-full"
      />
      {editingText && (
        <textarea
          autoFocus
          defaultValue={editingText.content}
          style={{
            position: 'absolute',
            left: editingText.x,
            top: editingText.y,
            width: Math.max(editingText.w, 40),
            minHeight: Math.max(editingText.h, 24),
            fontSize: editingText.fontSize,
            fontFamily: editingText.fontFamily,
            fontWeight: editingText.fontWeight,
            textAlign: editingText.textAlign as CanvasTextAlign,
            color: editingText.color,
            lineHeight: editingText.lineHeight,
            background: 'rgba(255,255,255,0.9)',
            border: '2px solid #0d99ff',
            borderRadius: 2,
            outline: 'none',
            resize: 'none',
            padding: '0 1px',
            margin: 0,
            overflow: 'hidden',
            zIndex: 10,
            boxSizing: 'border-box',
          }}
          onBlur={(e) => {
            const newContent = e.target.value
            if (newContent !== editingText.content) {
              useDocumentStore.getState().updateNode(editingText.nodeId, { content: newContent } as Partial<PenNode>)
            }
            setEditingText(null)
          }}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              setEditingText(null)
            }
            e.stopPropagation()
          }}
        />
      )}

      {error && (
        <div className="absolute inset-0 flex items-center justify-center text-destructive">
          Failed to load CanvasKit: {error}
        </div>
      )}
    </div>
  )
}
