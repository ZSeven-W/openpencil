import { useRef, useEffect, useState } from 'react'
import { createPortal } from 'react-dom'
import { useTranslation } from 'react-i18next'
import { X, Upload, RotateCcw } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Slider } from '@/components/ui/slider'
import { Separator } from '@/components/ui/separator'
import type { ImageFitMode } from '@/types/pen'

type FitMode = ImageFitMode | 'stretch'

interface AdjustmentValues {
  exposure?: number
  contrast?: number
  saturation?: number
  temperature?: number
  tint?: number
  highlights?: number
  shadows?: number
}

interface ImageFillPopoverProps {
  imageSrc?: string
  fitMode: FitMode
  adjustments: AdjustmentValues
  /** Bounding rect of the trigger element for positioning */
  triggerRect: DOMRect
  onFitModeChange: (mode: FitMode) => void
  onAdjustmentChange: (key: keyof AdjustmentValues, value: number) => void
  onResetAdjustments?: () => void
  onImageChange?: (dataUrl: string) => void
  onClose: () => void
}

const PANEL_WIDTH = 220
const PANEL_GAP = 8

const ADJUSTMENT_KEYS: { key: keyof AdjustmentValues; labelKey: string }[] = [
  { key: 'exposure', labelKey: 'image.exposure' },
  { key: 'contrast', labelKey: 'image.contrast' },
  { key: 'saturation', labelKey: 'image.saturation' },
  { key: 'temperature', labelKey: 'image.temperature' },
  { key: 'tint', labelKey: 'image.tint' },
  { key: 'highlights', labelKey: 'image.highlights' },
  { key: 'shadows', labelKey: 'image.shadows' },
]

export type { AdjustmentValues, FitMode }

export default function ImageFillPopover({
  imageSrc,
  fitMode,
  adjustments,
  triggerRect,
  onFitModeChange,
  onAdjustmentChange,
  onResetAdjustments,
  onImageChange,
  onClose,
}: ImageFillPopoverProps) {
  const { t } = useTranslation()
  const panelRef = useRef<HTMLDivElement>(null)
  const fileRef = useRef<HTMLInputElement>(null)
  const [panelHeight, setPanelHeight] = useState(0)

  // Measure panel height for vertical centering
  useEffect(() => {
    if (panelRef.current) {
      setPanelHeight(panelRef.current.offsetHeight)
    }
  })

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        onClose()
      }
    }
    // Use setTimeout to avoid the opening click triggering immediate close
    const timer = setTimeout(() => {
      document.addEventListener('mousedown', handler)
    }, 0)
    return () => {
      clearTimeout(timer)
      document.removeEventListener('mousedown', handler)
    }
  }, [onClose])

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0]
    if (!file || !file.type.startsWith('image/')) return
    const reader = new FileReader()
    reader.onload = () => {
      onImageChange?.(reader.result as string)
    }
    reader.readAsDataURL(file)
    e.target.value = ''
  }

  const hasAdjustments = ADJUSTMENT_KEYS.some((a) => (adjustments[a.key] ?? 0) !== 0)
  const handleResetAll = () => {
    if (onResetAdjustments) {
      onResetAdjustments()
    } else {
      for (const a of ADJUSTMENT_KEYS) {
        onAdjustmentChange(a.key, 0)
      }
    }
  }

  const hasImage = imageSrc && !imageSrc.startsWith('__')

  // Position: to the left of the trigger element
  const left = triggerRect.left - PANEL_WIDTH - PANEL_GAP
  // Vertically align with the trigger top, clamped to viewport
  let top = triggerRect.top
  if (panelHeight > 0 && top + panelHeight > window.innerHeight - 8) {
    top = Math.max(8, window.innerHeight - panelHeight - 8)
  }

  return createPortal(
    <div
      ref={panelRef}
      className="fixed z-[100] bg-popover border border-border rounded-lg shadow-lg"
      style={{ left, top, width: PANEL_WIDTH }}
    >
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-2">
        <span className="text-[11px] font-medium text-foreground">{t('image.title')}</span>
        <Button variant="ghost" size="icon-sm" onClick={onClose}>
          <X className="w-3 h-3" />
        </Button>
      </div>

      {/* Fit mode row */}
      <div className="px-3 pb-2">
        <div className="flex items-center gap-0.5 bg-secondary rounded-md p-0.5">
          {(['fill', 'fit', 'crop', 'tile'] as FitMode[]).map((m) => (
            <button
              key={m}
              type="button"
              title={t(`image.${m === 'fit' ? 'fitMode' : m}`)}
              onClick={() => onFitModeChange(m)}
              className={`flex-1 flex items-center justify-center h-6 rounded text-[10px] font-medium transition-colors ${
                fitMode === m
                  ? 'bg-primary text-primary-foreground'
                  : 'text-muted-foreground hover:text-foreground'
              }`}
            >
              {t(`image.${m === 'fit' ? 'fitMode' : m}`)}
            </button>
          ))}
        </div>
      </div>

      {/* Image preview / upload */}
      <div className="px-3 pb-2">
        <input
          ref={fileRef}
          type="file"
          accept="image/*"
          className="hidden"
          onChange={handleFileChange}
        />
        <button
          type="button"
          onClick={() => fileRef.current?.click()}
          className="w-full h-28 rounded-md border border-dashed border-border bg-muted/50 hover:bg-muted transition-colors flex items-center justify-center overflow-hidden cursor-pointer relative group"
        >
          {hasImage ? (
            <>
              <img src={imageSrc} alt="" className="w-full h-full object-contain" />
              <div className="absolute inset-0 bg-black/50 opacity-0 group-hover:opacity-100 transition-opacity flex items-center justify-center">
                <Upload className="w-4 h-4 text-white" />
              </div>
            </>
          ) : (
            <div className="flex flex-col items-center gap-1 text-muted-foreground">
              <Upload className="w-5 h-5" />
              <span className="text-[10px]">{t('image.clickToUpload')}</span>
            </div>
          )}
        </button>
      </div>

      <Separator />

      {/* Adjustments */}
      <div className="px-3 py-2">
        <div className="flex items-center justify-between mb-2">
          <span className="text-[10px] font-medium text-muted-foreground uppercase tracking-wider">
            {t('image.adjustments')}
          </span>
          {hasAdjustments && (
            <button
              type="button"
              onClick={handleResetAll}
              className="text-[10px] text-muted-foreground hover:text-foreground flex items-center gap-0.5"
            >
              <RotateCcw className="w-2.5 h-2.5" />
              {t('image.reset')}
            </button>
          )}
        </div>
        <div className="space-y-2.5">
          {ADJUSTMENT_KEYS.map((a) => (
            <AdjustmentRow
              key={a.key}
              label={t(a.labelKey)}
              value={adjustments[a.key] ?? 0}
              onChange={(v) => onAdjustmentChange(a.key, v)}
            />
          ))}
        </div>
      </div>
    </div>,
    document.body,
  )
}

function AdjustmentRow({
  label,
  value,
  onChange,
}: {
  label: string
  value: number
  onChange: (v: number) => void
}) {
  return (
    <div className="flex items-center gap-2">
      <span className="text-[10px] text-muted-foreground w-[68px] shrink-0 truncate">
        {label}
      </span>
      <Slider
        min={-100}
        max={100}
        step={1}
        value={[value]}
        onValueChange={([v]) => onChange(v)}
        className="flex-1"
      />
      <span className="text-[10px] text-muted-foreground w-7 text-right tabular-nums shrink-0">
        {value}
      </span>
    </div>
  )
}
