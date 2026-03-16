import { useState, useCallback, useRef } from 'react'
import { useTranslation } from 'react-i18next'
import type { ImageNode, ImageFitMode } from '@/types/pen'
import SectionHeader from '@/components/shared/section-header'
import { Image as ImageIcon } from 'lucide-react'
import ImageFillPopover from './image-fill-popover'

interface ImageSectionProps {
  node: ImageNode
  onUpdate: (updates: Partial<ImageNode>) => void
}

export default function ImageSection({ node, onUpdate }: ImageSectionProps) {
  const { t } = useTranslation()
  const [triggerRect, setTriggerRect] = useState<DOMRect | null>(null)
  const triggerRef = useRef<HTMLButtonElement>(null)

  const fitMode = node.objectFit ?? 'fill'
  const hasImage = node.src && !node.src.startsWith('__')

  const handleClose = useCallback(() => setTriggerRect(null), [])

  const handleToggle = () => {
    if (triggerRect) {
      setTriggerRect(null)
    } else if (triggerRef.current) {
      setTriggerRect(triggerRef.current.getBoundingClientRect())
    }
  }

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('image.title')} />

      <button
        ref={triggerRef}
        type="button"
        onClick={handleToggle}
        className="w-full flex items-center gap-2 h-8 px-1.5 rounded border border-border hover:bg-accent/50 transition-colors cursor-pointer"
      >
        <div className="w-6 h-6 rounded border border-border shrink-0 bg-muted overflow-hidden flex items-center justify-center">
          {hasImage ? (
            <img src={node.src} alt="" className="w-full h-full object-cover" />
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
          imageSrc={node.src}
          fitMode={fitMode}
          triggerRect={triggerRect}
          adjustments={{
            exposure: node.exposure,
            contrast: node.contrast,
            saturation: node.saturation,
            temperature: node.temperature,
            tint: node.tint,
            highlights: node.highlights,
            shadows: node.shadows,
          }}
          onFitModeChange={(mode) => onUpdate({ objectFit: mode as ImageFitMode })}
          onAdjustmentChange={(key, value) => onUpdate({ [key]: value } as Partial<ImageNode>)}
          onResetAdjustments={() => onUpdate({ exposure: 0, contrast: 0, saturation: 0, temperature: 0, tint: 0, highlights: 0, shadows: 0 } as Partial<ImageNode>)}
          onImageChange={(dataUrl) => onUpdate({ src: dataUrl })}
          onClose={handleClose}
        />
      )}
    </div>
  )
}
