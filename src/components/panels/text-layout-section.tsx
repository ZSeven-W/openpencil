import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import {
  MoveHorizontal,
  WrapText,
  Maximize2,
  Check,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import { useTranslation } from 'react-i18next'
import type { PenNode, TextNode, SizingBehavior } from '@/types/pen'

interface TextLayoutSectionProps {
  node: TextNode
  onUpdate: (updates: Partial<PenNode>) => void
}

type TextResizing = 'auto' | 'fixed-width' | 'fixed-width-height'

function resolveTextGrowth(node: TextNode): TextResizing {
  if (node.textGrowth) return node.textGrowth
  const w = node.width
  if (typeof w === 'number' && w > 0) return 'fixed-width'
  if (typeof w === 'string' && w.startsWith('fill_container')) return 'fixed-width'
  return 'auto'
}

function extractNumericSize(value: SizingBehavior | undefined): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const match = value.match(/\((\d+)\)/)
    if (match) return parseInt(match[1], 10)
  }
  return 100
}

function ResizingToggle({
  active,
  onClick,
  children,
  title,
}: {
  active: boolean
  onClick: () => void
  children: React.ReactNode
  title: string
}) {
  return (
    <button
      type="button"
      title={title}
      onClick={onClick}
      className={cn(
        'h-7 flex-1 flex items-center justify-center gap-1 rounded text-[10px] transition-colors',
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent',
      )}
    >
      {children}
    </button>
  )
}

function SizingCheckbox({
  label,
  checked,
  onChange,
}: {
  label: string
  checked: boolean
  onChange: (checked: boolean) => void
}) {
  return (
    <label className="flex items-center gap-1.5 cursor-pointer group">
      <button
        type="button"
        role="checkbox"
        aria-checked={checked}
        onClick={() => onChange(!checked)}
        className={cn(
          'w-4 h-4 rounded-[3px] border-[1.5px] flex items-center justify-center transition-colors shrink-0',
          checked
            ? 'bg-primary border-primary'
            : 'border-muted-foreground/40 group-hover:border-muted-foreground',
        )}
      >
        {checked && (
          <Check className="w-3 h-3 text-primary-foreground" strokeWidth={3} />
        )}
      </button>
      <span className="text-[11px] text-muted-foreground select-none">
        {label}
      </span>
    </label>
  )
}

export default function TextLayoutSection({
  node,
  onUpdate,
}: TextLayoutSectionProps) {
  const { t } = useTranslation()
  const resizing = resolveTextGrowth(node)
  const widthStr = typeof node.width === 'string' ? node.width : ''
  const heightStr = typeof node.height === 'string' ? node.height : ''
  const fillWidth = widthStr.startsWith('fill_container')
  const fillHeight = heightStr.startsWith('fill_container')
  const numericWidth = typeof node.width === 'number' && node.width > 0 ? node.width : undefined
  const numericHeight = typeof node.height === 'number' && node.height > 0 ? node.height : undefined
  const fallbackW = extractNumericSize(node.width)
  const fallbackH = extractNumericSize(node.height)

  const handleResizingChange = (mode: TextResizing) => {
    const updates: Record<string, unknown> = { textGrowth: mode }
    switch (mode) {
      case 'auto':
        updates.width = 0
        updates.height = 0
        break
      case 'fixed-width':
        if (!fillWidth && (typeof node.width !== 'number' || node.width <= 0)) {
          updates.width = fallbackW > 0 ? fallbackW : 200
        }
        if (!fillHeight && (typeof node.height !== 'number' || node.height <= 0)) {
          updates.height = fallbackH > 0 ? fallbackH : 44
        }
        break
      case 'fixed-width-height':
        if (!fillWidth && (typeof node.width !== 'number' || node.width <= 0)) {
          updates.width = fallbackW > 0 ? fallbackW : 200
        }
        if (!fillHeight && (typeof node.height !== 'number' || node.height <= 0)) {
          updates.height = fallbackH > 0 ? fallbackH : 100
        }
        break
    }
    onUpdate(updates as Partial<PenNode>)
  }

  // Always show dimensions — read-only when not directly editable
  const canEditWidth = resizing !== 'auto' && !fillWidth && numericWidth !== undefined
  const canEditHeight = resizing === 'fixed-width-height' && !fillHeight && numericHeight !== undefined
  // Display value: prefer numeric, fallback to extracted size from fill_container(N)
  const displayW = numericWidth ?? fallbackW
  const displayH = numericHeight ?? fallbackH

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('textLayout.title')} />

      {/* Dimensions — always visible, read-only when auto/fill */}
      <div>
        <span className="text-[10px] text-muted-foreground mb-1 block">
          {t('textLayout.dimensions')}
        </span>
        <div className="grid grid-cols-2 gap-1">
          <NumberInput
            label="W"
            value={Math.round(displayW)}
            onChange={(v) =>
              onUpdate({ width: v } as Partial<PenNode>)
            }
            min={1}
            readOnly={!canEditWidth}
          />
          <NumberInput
            label="H"
            value={Math.round(displayH)}
            onChange={(v) =>
              onUpdate({ height: v } as Partial<PenNode>)
            }
            min={1}
            readOnly={!canEditHeight}
          />
        </div>
      </div>

      {/* Fill Width / Fill Height */}
      {resizing !== 'auto' && (
        <div className="grid grid-cols-2 gap-y-1.5">
          <SizingCheckbox
            label={t('textLayout.fillWidth')}
            checked={fillWidth}
            onChange={(v) =>
              onUpdate({
                width: v ? 'fill_container' : (fallbackW > 0 ? fallbackW : 200),
              } as Partial<PenNode>)
            }
          />
          <SizingCheckbox
            label={t('textLayout.fillHeight')}
            checked={fillHeight}
            onChange={(v) =>
              onUpdate({
                height: v ? 'fill_container' : (fallbackH > 0 ? fallbackH : 100),
              } as Partial<PenNode>)
            }
          />
        </div>
      )}

      {/* Resizing mode */}
      <div>
        <span className="text-[10px] text-muted-foreground mb-1 block">
          {t('textLayout.resizing')}
        </span>
        <div className="flex gap-0.5">
          <ResizingToggle
            active={resizing === 'auto'}
            onClick={() => handleResizingChange('auto')}
            title={t('textLayout.autoWidthDesc')}
          >
            <MoveHorizontal className="w-3 h-3" />
            <span>{t('textLayout.autoWidth')}</span>
          </ResizingToggle>
          <ResizingToggle
            active={resizing === 'fixed-width'}
            onClick={() => handleResizingChange('fixed-width')}
            title={t('textLayout.autoHeightDesc')}
          >
            <WrapText className="w-3 h-3" />
            <span>{t('textLayout.autoHeight')}</span>
          </ResizingToggle>
          <ResizingToggle
            active={resizing === 'fixed-width-height'}
            onClick={() => handleResizingChange('fixed-width-height')}
            title={t('textLayout.fixedDesc')}
          >
            <Maximize2 className="w-3 h-3" />
            <span>{t('textLayout.fixed')}</span>
          </ResizingToggle>
        </div>
      </div>
    </div>
  )
}
