import NumberInput from '@/components/shared/number-input'
import type { PenNode, ContainerProps, SizingBehavior } from '@/types/pen'
import { cn } from '@/lib/utils'
import {
  Columns3,
  Rows3,
  LayoutGrid,
  Check,
} from 'lucide-react'
import PaddingSection from './layout-padding-section'
import { RadioCircle } from './layout-padding-section'

interface LayoutSectionProps {
  node: PenNode & ContainerProps
  onUpdate: (updates: Partial<PenNode>) => void
}

const POSITIONS = ['start', 'center', 'end'] as const

type GapMode = 'numeric' | 'space_between' | 'space_around'
type JustifyValue = 'start' | 'center' | 'end' | 'space_between' | 'space_around'
type AlignValue = 'start' | 'center' | 'end'

function normalizeJustifyValue(value: unknown): JustifyValue {
  if (typeof value !== 'string') return 'start'
  const v = value.trim().toLowerCase()
  switch (v) {
    case 'start':
    case 'flex-start':
    case 'left':
    case 'top':
      return 'start'
    case 'center':
    case 'middle':
      return 'center'
    case 'end':
    case 'flex-end':
    case 'right':
    case 'bottom':
      return 'end'
    case 'space_between':
    case 'space-between':
      return 'space_between'
    case 'space_around':
    case 'space-around':
      return 'space_around'
    default:
      return 'start'
  }
}

function normalizeAlignValue(value: unknown): AlignValue {
  if (typeof value !== 'string') return 'start'
  const v = value.trim().toLowerCase()
  switch (v) {
    case 'start':
    case 'flex-start':
    case 'left':
    case 'top':
      return 'start'
    case 'center':
    case 'middle':
      return 'center'
    case 'end':
    case 'flex-end':
    case 'right':
    case 'bottom':
      return 'end'
    default:
      return 'start'
  }
}

// ---------------------------------------------------------------------------
// ToggleButton
// ---------------------------------------------------------------------------

function ToggleButton({
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
        'h-7 w-7 flex items-center justify-center rounded transition-colors',
        active
          ? 'bg-primary text-primary-foreground'
          : 'bg-secondary text-muted-foreground hover:text-foreground hover:bg-accent',
      )}
    >
      {children}
    </button>
  )
}

// ---------------------------------------------------------------------------
// AlignmentGrid — 3×3 interactive alignment picker
// ---------------------------------------------------------------------------

function AlignmentGrid({
  layout,
  justifyContent,
  alignItems,
  isSpaceMode,
  onUpdate,
}: {
  layout: 'none' | 'vertical' | 'horizontal'
  justifyContent: JustifyValue
  alignItems: AlignValue
  isSpaceMode: boolean
  onUpdate: (updates: Partial<PenNode>) => void
}) {
  const isFreedom = layout === 'none'
  const isVertical = layout === 'vertical'

  return (
    <div className="grid grid-cols-3 gap-[3px] p-2 bg-secondary rounded">
      {[0, 1, 2].map((row) =>
        [0, 1, 2].map((col) => {
          const rowPos = POSITIONS[row]
          const colPos = POSITIONS[col]
          const cellJustify = isVertical ? rowPos : colPos
          const cellAlign = isVertical ? colPos : rowPos
          const isActive =
            !isFreedom &&
            !isSpaceMode &&
            justifyContent === cellJustify &&
            alignItems === cellAlign
          const cellCrossPos = isVertical ? colPos : rowPos
          const isOnActiveCross =
            isSpaceMode && cellCrossPos === alignItems

          return (
            <button
              key={`${row}-${col}`}
              type="button"
              disabled={isFreedom}
              className={cn(
                'w-7 h-5 rounded-[2px] flex items-center justify-center transition-colors',
                isFreedom && 'cursor-default',
                !isFreedom && 'cursor-pointer hover:bg-accent/50',
              )}
              onClick={() => {
                if (isFreedom) return
                if (isSpaceMode) {
                  onUpdate({
                    alignItems: cellAlign,
                  } as Partial<PenNode>)
                } else {
                  onUpdate({
                    justifyContent: cellJustify,
                    alignItems: cellAlign,
                  } as Partial<PenNode>)
                }
              }}
            >
              {isFreedom ? (
                <div className="w-[3px] h-[3px] rounded-full bg-muted-foreground/30" />
              ) : isSpaceMode && isOnActiveCross ? (
                <div
                  className={cn(
                    'rounded-[1px] bg-primary',
                    isVertical
                      ? 'w-[10px] h-[2px]'
                      : 'w-[2px] h-[10px]',
                  )}
                />
              ) : isActive ? (
                <div className="w-2.5 h-2.5 rounded-[2px] bg-primary" />
              ) : (
                <div className="w-[3px] h-[3px] rounded-full bg-muted-foreground/40" />
              )}
            </button>
          )
        }),
      )}
    </div>
  )
}

// ---------------------------------------------------------------------------
// GapSection — Radio: Numeric / Space Between / Space Around
// ---------------------------------------------------------------------------

function GapSection({
  gap,
  gapMode,
  onGapModeChange,
  onUpdate,
}: {
  gap: number
  gapMode: GapMode
  onGapModeChange: (mode: GapMode) => void
  onUpdate: (updates: Partial<PenNode>) => void
}) {
  return (
    <div className="space-y-1.5">
      <div
        className="flex items-center gap-1.5 cursor-pointer"
        onClick={() => onGapModeChange('numeric')}
      >
        <RadioCircle selected={gapMode === 'numeric'} />
        <div
          className="flex-1"
          onClick={(e) => e.stopPropagation()}
        >
          <NumberInput
            value={gap}
            onChange={(v) =>
              onUpdate({ gap: v } as Partial<PenNode>)
            }
            min={0}
          />
        </div>
      </div>
      <div
        className="flex items-center gap-1.5 cursor-pointer"
        onClick={() => onGapModeChange('space_between')}
      >
        <RadioCircle selected={gapMode === 'space_between'} />
        <span className="text-[10px] text-muted-foreground select-none">
          Space Between
        </span>
      </div>
      <div
        className="flex items-center gap-1.5 cursor-pointer"
        onClick={() => onGapModeChange('space_around')}
      >
        <RadioCircle selected={gapMode === 'space_around'} />
        <span className="text-[10px] text-muted-foreground select-none">
          Space Around
        </span>
      </div>
    </div>
  )
}

// ---------------------------------------------------------------------------
// SizingCheckbox
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// SizingCheckboxes — Fill / Hug per axis + Clip Content
// ---------------------------------------------------------------------------

function extractNumericSize(
  value: SizingBehavior | undefined,
): number {
  if (typeof value === 'number') return value
  if (typeof value === 'string') {
    const match = value.match(/\((\d+)\)/)
    if (match) return parseInt(match[1], 10)
  }
  return 100
}

function SizingCheckboxes({
  node,
  onUpdate,
}: {
  node: PenNode & ContainerProps
  onUpdate: (updates: Partial<PenNode>) => void
}) {
  const widthStr =
    typeof node.width === 'string' ? node.width : ''
  const heightStr =
    typeof node.height === 'string' ? node.height : ''
  const fillWidth = widthStr.startsWith('fill_container')
  const fillHeight = heightStr.startsWith('fill_container')
  const hugWidth = widthStr.startsWith('fit_content')
  const hugHeight = heightStr.startsWith('fit_content')
  const clipContent = node.clipContent === true
  const fallbackW = extractNumericSize(node.width)
  const fallbackH = extractNumericSize(node.height)

  return (
    <div className="space-y-1.5">
      <div className="grid grid-cols-2 gap-y-1.5">
        <SizingCheckbox
          label="Fill Width"
          checked={fillWidth}
          onChange={(v) =>
            onUpdate({
              width: v ? 'fill_container' : fallbackW,
            } as Partial<PenNode>)
          }
        />
        <SizingCheckbox
          label="Fill Height"
          checked={fillHeight}
          onChange={(v) =>
            onUpdate({
              height: v ? 'fill_container' : fallbackH,
            } as Partial<PenNode>)
          }
        />
        <SizingCheckbox
          label="Hug Width"
          checked={hugWidth}
          onChange={(v) =>
            onUpdate({
              width: v ? 'fit_content' : fallbackW,
            } as Partial<PenNode>)
          }
        />
        <SizingCheckbox
          label="Hug Height"
          checked={hugHeight}
          onChange={(v) =>
            onUpdate({
              height: v ? 'fit_content' : fallbackH,
            } as Partial<PenNode>)
          }
        />
      </div>
      <SizingCheckbox
        label="Clip Content"
        checked={clipContent}
        onChange={(v) =>
          onUpdate({ clipContent: v } as Partial<PenNode>)
        }
      />
    </div>
  )
}

// ---------------------------------------------------------------------------
// Main LayoutSection
// ---------------------------------------------------------------------------

export default function LayoutSection({
  node,
  onUpdate,
}: LayoutSectionProps) {
  const layout = node.layout ?? 'none'
  const hasLayout = layout !== 'none'

  const justifyContent = normalizeJustifyValue(node.justifyContent)
  const alignItems = normalizeAlignValue(node.alignItems)
  const rawGap = node.gap
  const gap = typeof rawGap === 'number' ? rawGap : 0

  const gapMode: GapMode =
    justifyContent === 'space_between'
      ? 'space_between'
      : justifyContent === 'space_around'
        ? 'space_around'
        : 'numeric'

  const isSpaceMode =
    gapMode === 'space_between' || gapMode === 'space_around'

  const handleGapModeChange = (mode: GapMode) => {
    switch (mode) {
      case 'numeric':
        onUpdate({
          justifyContent: 'start',
        } as Partial<PenNode>)
        break
      case 'space_between':
        onUpdate({
          justifyContent: 'space_between',
        } as Partial<PenNode>)
        break
      case 'space_around':
        onUpdate({
          justifyContent: 'space_around',
        } as Partial<PenNode>)
        break
    }
  }

  const width =
    typeof node.width === 'number' ? node.width : undefined
  const height =
    typeof node.height === 'number' ? node.height : undefined

  return (
    <div className="space-y-3">
      {/* Header */}
      <span className="text-[11px] font-medium text-foreground">
        Flex Layout
      </span>

      {/* Direction row — no label, just buttons */}
      <div className="flex jusfity-between gap-0.5">
        <ToggleButton
          active={layout === 'none'}
          onClick={() =>
            onUpdate({ layout: 'none' } as Partial<PenNode>)
          }
          title="Freedom (no layout)"
        >
          <LayoutGrid className="w-3.5 h-3.5" />
        </ToggleButton>
        <ToggleButton
          active={layout === 'vertical'}
          onClick={() =>
            onUpdate({ layout: 'vertical' } as Partial<PenNode>)
          }
          title="Vertical layout"
        >
          <Rows3 className="w-3.5 h-3.5" />
        </ToggleButton>
        <ToggleButton
          active={layout === 'horizontal'}
          onClick={() =>
            onUpdate({
              layout: 'horizontal',
            } as Partial<PenNode>)
          }
          title="Horizontal layout"
        >
          <Columns3 className="w-3.5 h-3.5" />
        </ToggleButton>
      </div>

      {/* Alignment + Gap side by side */}
      {hasLayout && (
        <>
          <div className="flex gap-2">
            {/* Left: Alignment */}
            <div className="w-[160px]">
              <span className="text-[10px] w-full text-muted-foreground mb-1.5 block">
                Alignment
              </span>
              <AlignmentGrid
                layout={layout}
                justifyContent={justifyContent}
                alignItems={alignItems}
                isSpaceMode={isSpaceMode}
                onUpdate={onUpdate}
              />
            </div>
            {/* Right: Gap */}
            <div>
              <span className="text-[10px] text-muted-foreground mb-1.5 block">
                Gap
              </span>
              <GapSection
                gap={gap}
                gapMode={gapMode}
                onGapModeChange={handleGapModeChange}
                onUpdate={onUpdate}
              />
            </div>
          </div>

          {/* Padding */}
          <PaddingSection
            padding={node.padding}
            onUpdate={onUpdate}
          />
        </>
      )}

      {/* Dimensions */}
      {(width !== undefined || height !== undefined) && (
        <div>
          <span className="text-[10px] text-muted-foreground mb-1.5 block">
            Dimensions
          </span>
          <div className="grid grid-cols-2 gap-1">
            {width !== undefined && (
              <NumberInput
                label="W"
                value={Math.round(width)}
                onChange={(v) =>
                  onUpdate({ width: v } as Partial<PenNode>)
                }
                min={1}
              />
            )}
            {height !== undefined && (
              <NumberInput
                label="H"
                value={Math.round(height)}
                onChange={(v) =>
                  onUpdate({ height: v } as Partial<PenNode>)
                }
                min={1}
              />
            )}
          </div>
        </div>
      )}

      {/* Sizing checkboxes */}
      <SizingCheckboxes node={node} onUpdate={onUpdate} />
    </div>
  )
}
