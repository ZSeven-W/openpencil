import { useState, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import ColorPicker from '@/components/shared/color-picker'
import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import VariablePicker from '@/components/shared/variable-picker'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Button } from '@/components/ui/button'
import { Plus, X, Image as ImageIcon } from 'lucide-react'
import { isVariableRef } from '@/variables/resolve-variables'
import ImageFillPopover from './image-fill-popover'
import type { PenNode } from '@/types/pen'
import type { PenFill, GradientStop, ImageFill } from '@/types/styles'

const FILL_TYPE_OPTIONS = [
  { value: 'solid', labelKey: 'fill.solid' },
  { value: 'linear_gradient', labelKey: 'fill.linear' },
  { value: 'radial_gradient', labelKey: 'fill.radial' },
  { value: 'image', labelKey: 'fill.image' },
]

function defaultStops(): GradientStop[] {
  return [
    { offset: 0, color: '#000000' },
    { offset: 1, color: '#ffffff' },
  ]
}

/** Build a CSS gradient preview string for a gradient fill. */
function gradientPreviewCss(fill: PenFill): string | undefined {
  if (fill.type === 'linear_gradient') {
    const angle = fill.angle ?? 0
    const stops = fill.stops.map(s => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ')
    return `linear-gradient(${angle}deg, ${stops})`
  }
  if (fill.type === 'radial_gradient') {
    const stops = fill.stops.map(s => `${s.color} ${Math.round(s.offset * 100)}%`).join(', ')
    return `radial-gradient(circle, ${stops})`
  }
  return undefined
}

interface FillSectionProps {
  fills?: PenFill[]
  onUpdate: (updates: Partial<PenNode>) => void
}

export default function FillSection({
  fills,
  onUpdate,
}: FillSectionProps) {
  const { t } = useTranslation()
  const firstFill = fills?.[0]
  const fillType = firstFill?.type ?? 'solid'

  const currentColor =
    firstFill?.type === 'solid' ? firstFill.color : '#d1d5db'

  const currentAngle =
    firstFill?.type === 'linear_gradient' ? (firstFill.angle ?? 0) : 0

  const currentStops: GradientStop[] =
    firstFill &&
    (firstFill.type === 'linear_gradient' ||
      firstFill.type === 'radial_gradient')
      ? firstFill.stops
      : defaultStops()

  const fillOpacity = firstFill && 'opacity' in firstFill
    ? Math.round((firstFill.opacity ?? 1) * 100)
    : 100

  const handleTypeChange = (type: string) => {
    let newFills: PenFill[]
    if (type === 'solid') {
      newFills = [{ type: 'solid', color: currentColor }]
    } else if (type === 'linear_gradient') {
      newFills = [{
        type: 'linear_gradient',
        angle: currentAngle,
        stops: currentStops,
      }]
    } else if (type === 'radial_gradient') {
      newFills = [{
        type: 'radial_gradient',
        cx: 0.5, cy: 0.5, radius: 0.5,
        stops: currentStops,
      }]
    } else {
      newFills = [{ type: 'image', url: '' }]
    }
    onUpdate({ fill: newFills } as Partial<PenNode>)
  }

  const handleColorChange = (color: string) => {
    onUpdate({ fill: [{ type: 'solid', color }] } as Partial<PenNode>)
  }

  const handleOpacityChange = (val: number) => {
    if (!firstFill) return
    const opacity = Math.max(0, Math.min(100, val)) / 100
    onUpdate({ fill: [{ ...firstFill, opacity }] } as Partial<PenNode>)
  }

  const handleAngleChange = (angle: number) => {
    if (firstFill?.type === 'linear_gradient') {
      onUpdate({ fill: [{ ...firstFill, angle }] } as Partial<PenNode>)
    }
  }

  const handleStopColorChange = (index: number, color: string) => {
    if (!firstFill || (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')) return
    const newStops = [...firstFill.stops]
    newStops[index] = { ...newStops[index], color }
    onUpdate({ fill: [{ ...firstFill, stops: newStops }] } as Partial<PenNode>)
  }

  const handleStopOffsetChange = (index: number, offset: number) => {
    if (!firstFill || (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')) return
    const newStops = [...firstFill.stops]
    newStops[index] = { ...newStops[index], offset: offset / 100 }
    onUpdate({ fill: [{ ...firstFill, stops: newStops }] } as Partial<PenNode>)
  }

  const handleAddStop = () => {
    if (!firstFill || (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')) return
    const stops = [...firstFill.stops]
    const lastOffset = stops[stops.length - 1]?.offset ?? 0.5
    stops.push({ offset: Math.min(1, lastOffset + 0.1), color: '#888888' })
    onUpdate({ fill: [{ ...firstFill, stops }] } as Partial<PenNode>)
  }

  const handleRemoveStop = (index: number) => {
    if (!firstFill || (firstFill.type !== 'linear_gradient' && firstFill.type !== 'radial_gradient')) return
    if (firstFill.stops.length <= 2) return
    const stops = firstFill.stops.filter((_, i) => i !== index)
    onUpdate({ fill: [{ ...firstFill, stops }] } as Partial<PenNode>)
  }

  const handleRemoveFill = () => {
    onUpdate({ fill: [] } as Partial<PenNode>)
  }

  const handleImageFitChange = (mode: string) => {
    if (firstFill?.type !== 'image') return
    onUpdate({ fill: [{ ...firstFill, mode: mode as ImageFill['mode'] }] } as Partial<PenNode>)
  }

  // Gradient preview swatch
  const gradientCss = firstFill ? gradientPreviewCss(firstFill) : undefined

  return (
    <div className="space-y-1.5">
      <SectionHeader
        title={t('fill.title')}
        actions={
          <Button variant="ghost" size="icon-sm" onClick={() => handleTypeChange('solid')}>
            <Plus className="w-3.5 h-3.5" />
          </Button>
        }
      />

      {/* Fill row: swatch + type label + opacity + remove */}
      {firstFill && (
        <div className="flex items-center gap-1.5 h-7">
          {/* Color/gradient/image swatch */}
          {fillType === 'solid' && !isVariableRef(currentColor) && (
            <div
              className="w-5 h-5 rounded border border-border shrink-0 cursor-pointer"
              style={{ backgroundColor: currentColor }}
            />
          )}
          {(fillType === 'linear_gradient' || fillType === 'radial_gradient') && gradientCss && (
            <div
              className="w-5 h-5 rounded border border-border shrink-0"
              style={{ background: gradientCss }}
            />
          )}
          {fillType === 'image' && (
            <div className="w-5 h-5 rounded border border-border shrink-0 bg-muted flex items-center justify-center">
              <ImageIcon className="w-3 h-3 text-muted-foreground" />
            </div>
          )}

          {/* Type selector */}
          <Select value={fillType} onValueChange={handleTypeChange}>
            <SelectTrigger className="h-6 text-[11px] flex-1 min-w-0">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {FILL_TYPE_OPTIONS.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>
                  {t(opt.labelKey)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>

          {/* Opacity */}
          <NumberInput
            value={fillOpacity}
            onChange={handleOpacityChange}
            min={0}
            max={100}
            suffix="%"
            className="w-14"
          />

          {/* Remove */}
          <Button variant="ghost" size="icon-sm" onClick={handleRemoveFill}>
            <X className="w-3 h-3" />
          </Button>
        </div>
      )}

      {/* Solid fill: color picker + variable picker */}
      {fillType === 'solid' && (
        <div className="flex items-center gap-1">
          <div className="flex-1">
            {isVariableRef(currentColor) ? (
              <div className="h-6 flex items-center px-2 bg-secondary rounded text-[11px] font-mono text-muted-foreground">
                {currentColor}
              </div>
            ) : (
              <ColorPicker value={currentColor} onChange={handleColorChange} />
            )}
          </div>
          <VariablePicker
            type="color"
            currentValue={currentColor}
            onBind={(ref) => onUpdate({ fill: [{ type: 'solid', color: ref }] } as Partial<PenNode>)}
            onUnbind={(val) => onUpdate({ fill: [{ type: 'solid', color: String(val) }] } as Partial<PenNode>)}
          />
        </div>
      )}

      {/* Gradient fill: angle + color stops */}
      {(fillType === 'linear_gradient' || fillType === 'radial_gradient') && (
        <div className="space-y-1.5">
          {fillType === 'linear_gradient' && (
            <NumberInput
              label={t('fill.angle')}
              value={currentAngle}
              onChange={handleAngleChange}
              min={0}
              max={360}
              suffix="°"
            />
          )}

          <div className="space-y-1">
            <div className="flex items-center justify-between">
              <span className="text-[10px] text-muted-foreground">
                {t('fill.stops')}
              </span>
              <Button variant="ghost" size="icon-sm" onClick={handleAddStop}>
                <Plus className="w-3 h-3" />
              </Button>
            </div>
            {currentStops.map((stop, i) => (
              <div key={i} className="flex items-center gap-1">
                <ColorPicker
                  value={stop.color}
                  onChange={(c) => handleStopColorChange(i, c)}
                />
                <NumberInput
                  value={Math.round((Number.isFinite(stop.offset) ? stop.offset : i / Math.max(currentStops.length - 1, 1)) * 100)}
                  onChange={(v) => handleStopOffsetChange(i, v)}
                  min={0}
                  max={100}
                  suffix="%"
                  className="w-[72px]"
                />
                {currentStops.length > 2 && (
                  <Button variant="ghost" size="icon-sm" onClick={() => handleRemoveStop(i)}>
                    <X className="w-3 h-3" />
                  </Button>
                )}
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Image fill: preview + upload + fit mode */}
      {fillType === 'image' && firstFill?.type === 'image' && (
        <ImageFillEditor fill={firstFill} onUpdate={onUpdate} onFitChange={handleImageFitChange} />
      )}
    </div>
  )
}

function ImageFillEditor({
  fill,
  onUpdate,
  onFitChange,
}: {
  fill: ImageFill
  onUpdate: (updates: Partial<PenNode>) => void
  onFitChange: (mode: string) => void
}) {
  const { t } = useTranslation()
  const [triggerRect, setTriggerRect] = useState<DOMRect | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)
  const hasImage = fill.url && !fill.url.startsWith('__')
  const fitMode = fill.mode ?? 'fill'

  const handleClose = useCallback(() => setTriggerRect(null), [])

  const handleToggle = () => {
    if (triggerRect) {
      setTriggerRect(null)
    } else if (triggerRef.current) {
      setTriggerRect(triggerRef.current.getBoundingClientRect())
    }
  }

  return (
    <div>
      <button
        ref={triggerRef}
        type="button"
        onClick={handleToggle}
        className="w-full flex items-center gap-2 h-8 px-1.5 rounded border border-border hover:bg-accent/50 transition-colors cursor-pointer"
      >
        <div className="w-6 h-6 rounded border border-border shrink-0 bg-muted overflow-hidden flex items-center justify-center">
          {hasImage ? (
            <img src={fill.url} alt="" className="w-full h-full object-cover" />
          ) : (
            <ImageIcon className="w-3 h-3 text-muted-foreground" />
          )}
        </div>
        <span className="text-[11px] text-foreground flex-1 text-left truncate">
          {t(`image.${fitMode === 'fit' ? 'fitMode' : fitMode}`)}
        </span>
      </button>

      {triggerRect && (
        <ImageFillPopover
          imageSrc={fill.url}
          fitMode={fitMode}
          triggerRect={triggerRect}
          adjustments={{
            exposure: fill.exposure,
            contrast: fill.contrast,
            saturation: fill.saturation,
            temperature: fill.temperature,
            tint: fill.tint,
            highlights: fill.highlights,
            shadows: fill.shadows,
          }}
          onFitModeChange={(mode) => onFitChange(mode)}
          onAdjustmentChange={(key, value) => {
            onUpdate({ fill: [{ ...fill, [key]: value }] } as Partial<PenNode>)
          }}
          onResetAdjustments={() => {
            onUpdate({ fill: [{ ...fill, exposure: 0, contrast: 0, saturation: 0, temperature: 0, tint: 0, highlights: 0, shadows: 0 }] } as Partial<PenNode>)
          }}
          onImageChange={(dataUrl) => {
            onUpdate({ fill: [{ ...fill, url: dataUrl }] } as Partial<PenNode>)
          }}
          onClose={handleClose}
        />
      )}
    </div>
  )
}
