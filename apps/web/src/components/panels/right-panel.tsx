import { useState, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { useCanvasStore } from '@/stores/canvas-store'
import type { RightPanelTab } from '@/stores/canvas-store'
import PropertyPanel from './property-panel'
import CodePanel from './code-panel'

const MIN_WIDTH = 256   // 16rem (w-64)
const MAX_WIDTH = 640   // 40rem
const DEFAULT_WIDTH = 256

export default function RightPanel() {
  const { t } = useTranslation()
  const activeTab = useCanvasStore((s) => s.rightPanelTab)
  const setTab = useCanvasStore((s) => s.setRightPanelTab)
  const [width, setWidth] = useState(DEFAULT_WIDTH)
  const isDragging = useRef(false)
  const startX = useRef(0)
  const startWidth = useRef(0)

  const handleMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault()
    isDragging.current = true
    startX.current = e.clientX
    startWidth.current = width

    const handleMouseMove = (ev: MouseEvent) => {
      if (!isDragging.current) return
      // Dragging left border: moving mouse left => wider
      const delta = startX.current - ev.clientX
      const newWidth = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, startWidth.current + delta))
      setWidth(newWidth)
    }

    const handleMouseUp = () => {
      isDragging.current = false
      document.removeEventListener('mousemove', handleMouseMove)
      document.removeEventListener('mouseup', handleMouseUp)
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
    }

    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'
    document.addEventListener('mousemove', handleMouseMove)
    document.addEventListener('mouseup', handleMouseUp)
  }, [width])

  const tabs: { key: RightPanelTab; label: string }[] = [
    { key: 'design', label: t('rightPanel.design') },
    { key: 'code', label: t('rightPanel.code') },
  ]

  return (
    <div className="bg-card border-l border-border flex flex-col shrink-0 relative" style={{ width }}>
      {/* Resize handle */}
      <div
        className="absolute left-0 top-0 bottom-0 w-1 cursor-col-resize hover:bg-primary/30 active:bg-primary/50 z-10"
        onMouseDown={handleMouseDown}
      />

      {/* Tab bar */}
      <div className="h-8 flex items-center px-2 border-b border-border shrink-0 gap-1">
        {tabs.map((tab) => (
          <button
            key={tab.key}
            type="button"
            onClick={() => setTab(tab.key)}
            className={cn(
              'text-[11px] font-medium px-2 py-0.5 rounded transition-colors',
              activeTab === tab.key
                ? 'bg-secondary text-foreground'
                : 'text-muted-foreground hover:text-foreground',
            )}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {/* Content */}
      {activeTab === 'design' ? (
        <PropertyPanel embedded />
      ) : (
        <CodePanel />
      )}
    </div>
  )
}
