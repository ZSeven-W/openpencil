import { useState, useCallback, useRef } from 'react'
import { FORMAT_PRESETS, type FormatPreset } from '@/vibekit/format-presets'
import { switchFormat } from '@/vibekit/format-switch'
import { useCanvasStore } from '@/stores/canvas-store'
import { Monitor, Smartphone, Square, ChevronDown } from 'lucide-react'
import { cn } from '@/lib/utils'

const FORMAT_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  'linkedin-post': Monitor,
  'linkedin-video': Smartphone,
  'linkedin-carousel': Square,
}

export default function FormatSwitcher() {
  const [isOpen, setIsOpen] = useState(false)
  const containerRef = useRef<HTMLDivElement>(null)
  const activeFormat = useCanvasStore((s) => s.activeFormat)

  const handleSelect = useCallback((preset: FormatPreset) => {
    switchFormat(preset)
    setIsOpen(false)
  }, [])

  const handleBlur = useCallback((e: React.FocusEvent) => {
    if (!containerRef.current?.contains(e.relatedTarget as Node)) {
      setIsOpen(false)
    }
  }, [])

  const ActiveIcon = activeFormat ? (FORMAT_ICONS[activeFormat.id] ?? Square) : Square

  return (
    <div ref={containerRef} className="relative" onBlur={handleBlur}>
      {/* Trigger */}
      <button
        onClick={() => setIsOpen((prev) => !prev)}
        className={cn(
          'flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-sm transition-colors',
          'text-muted-foreground hover:text-foreground hover:bg-muted',
          isOpen && 'bg-muted text-foreground',
        )}
      >
        <ActiveIcon className="w-3.5 h-3.5" />
        <span className="max-w-24 truncate">
          {activeFormat?.name ?? 'Format'}
        </span>
        <ChevronDown className="w-3 h-3" />
      </button>

      {/* Dropdown */}
      {isOpen && (
        <div className="absolute top-full left-0 mt-1 w-56 bg-popover border border-border rounded-xl shadow-xl z-30 py-1">
          {FORMAT_PRESETS.map((preset) => {
            const Icon = FORMAT_ICONS[preset.id] ?? Square
            const isActive = activeFormat?.id === preset.id

            return (
              <button
                key={preset.id}
                onClick={() => handleSelect(preset)}
                className={cn(
                  'flex items-center gap-3 w-full px-3 py-2 text-left transition-colors',
                  'hover:bg-muted',
                  isActive && 'bg-primary/10 text-foreground',
                  !isActive && 'text-muted-foreground',
                )}
              >
                <Icon className="w-4 h-4 flex-shrink-0" />
                <div className="flex-1 min-w-0">
                  <div className="text-sm font-medium">{preset.name}</div>
                  <div className="text-xs text-muted-foreground">
                    {preset.width} x {preset.height}
                  </div>
                </div>
              </button>
            )
          })}
        </div>
      )}
    </div>
  )
}
