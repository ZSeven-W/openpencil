import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import {
  AlignLeft,
  AlignCenter,
  AlignRight,
  AlignJustify,
  AlignVerticalJustifyStart,
  AlignVerticalJustifyCenter,
  AlignVerticalJustifyEnd,
} from 'lucide-react'
import { cn } from '@/lib/utils'
import type { PenNode, TextNode } from '@/types/pen'

interface TextSectionProps {
  node: TextNode
  onUpdate: (updates: Partial<PenNode>) => void
}

const FONT_OPTIONS = [
  { value: 'Inter, sans-serif', label: 'Inter' },
  { value: 'Arial, sans-serif', label: 'Arial' },
  { value: 'Helvetica, sans-serif', label: 'Helvetica' },
  { value: 'Georgia, serif', label: 'Georgia' },
  { value: 'Times New Roman, serif', label: 'Times' },
  { value: 'Courier New, monospace', label: 'Courier' },
]

const WEIGHT_OPTIONS = [
  { value: '100', label: 'Thin' },
  { value: '300', label: 'Light' },
  { value: '400', label: 'Regular' },
  { value: '500', label: 'Medium' },
  { value: '600', label: 'Semibold' },
  { value: '700', label: 'Bold' },
  { value: '900', label: 'Black' },
]

const H_ALIGN_OPTIONS = [
  { value: 'left', icon: AlignLeft, label: 'Align left' },
  { value: 'center', icon: AlignCenter, label: 'Align center' },
  { value: 'right', icon: AlignRight, label: 'Align right' },
  { value: 'justify', icon: AlignJustify, label: 'Justify' },
]

const V_ALIGN_OPTIONS = [
  { value: 'top', icon: AlignVerticalJustifyStart, label: 'Top' },
  { value: 'middle', icon: AlignVerticalJustifyCenter, label: 'Middle' },
  { value: 'bottom', icon: AlignVerticalJustifyEnd, label: 'Bottom' },
]

const LineHeightIcon = (
  <svg viewBox="0 0 12 12" fill="none" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round">
    <line x1="1" y1="2" x2="1" y2="10" />
    <polyline points="3,4 1,2 -1,4" transform="translate(0,0)" />
    <polyline points="3,8 1,10 -1,8" transform="translate(0,0)" />
    <line x1="5" y1="6" x2="11" y2="6" />
    <line x1="5" y1="3" x2="9" y2="3" />
    <line x1="5" y1="9" x2="9" y2="9" />
  </svg>
)

function AlignButton({
  active,
  onClick,
  icon: Icon,
  label,
}: {
  active: boolean
  onClick: () => void
  icon: React.ComponentType<{ className?: string }>
  label: string
}) {
  return (
    <button
      type="button"
      aria-label={label}
      onClick={onClick}
      className={cn(
        'h-6 w-6 flex items-center justify-center rounded transition-colors',
        active
          ? 'bg-secondary text-foreground'
          : 'text-muted-foreground hover:text-foreground hover:bg-secondary/50',
      )}
    >
      <Icon className="w-3.5 h-3.5" />
    </button>
  )
}

export default function TextSection({
  node,
  onUpdate,
}: TextSectionProps) {
  const fontFamily = node.fontFamily ?? 'Inter, sans-serif'
  const fontSize = node.fontSize ?? 16
  const fontWeight = String(node.fontWeight ?? '400')
  const lineHeight = node.lineHeight ?? 1.2
  const letterSpacing = node.letterSpacing ?? 0
  const textAlign = node.textAlign ?? 'left'
  const textAlignVertical = node.textAlignVertical ?? 'top'

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Typography" />

      {/* Font family */}
      <Select
        value={fontFamily}
        onValueChange={(v) =>
          onUpdate({ fontFamily: v } as Partial<PenNode>)
        }
      >
        <SelectTrigger className="h-6 text-[11px]">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          {FONT_OPTIONS.map((opt) => (
            <SelectItem key={opt.value} value={opt.value}>
              {opt.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>

      {/* Weight + Size */}
      <div className="grid grid-cols-2 gap-1">
        <Select
          value={fontWeight}
          onValueChange={(v) =>
            onUpdate({ fontWeight: Number(v) } as Partial<PenNode>)
          }
        >
          <SelectTrigger className="h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {WEIGHT_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <NumberInput
          label="S"
          value={fontSize}
          onChange={(v) =>
            onUpdate({ fontSize: v } as Partial<PenNode>)
          }
          min={1}
          max={999}
        />
      </div>

      {/* Line height + Letter spacing */}
      <div className="flex items-center justify-between text-[9px] text-muted-foreground px-0.5">
        <span>Line height</span>
        <span>Letter spacing</span>
      </div>
      <div className="grid grid-cols-2 gap-1">
        <NumberInput
          icon={LineHeightIcon}
          value={Math.round(lineHeight * 100)}
          onChange={(v) =>
            onUpdate({ lineHeight: v / 100 } as Partial<PenNode>)
          }
          min={50}
          max={400}
          suffix="%"
        />
        <NumberInput
          label="|A|"
          value={letterSpacing}
          onChange={(v) =>
            onUpdate({ letterSpacing: v } as Partial<PenNode>)
          }
        />
      </div>

      {/* Horizontal alignment */}
      <div className="space-y-0.5">
        <span className="text-[10px] text-muted-foreground">Horizontal</span>
        <div className="flex items-center gap-0.5">
          {H_ALIGN_OPTIONS.map(({ value, icon, label }) => (
            <AlignButton
              key={value}
              active={textAlign === value}
              onClick={() =>
                onUpdate({
                  textAlign: value as TextNode['textAlign'],
                } as Partial<PenNode>)
              }
              icon={icon}
              label={label}
            />
          ))}
        </div>
      </div>

      {/* Vertical alignment */}
      <div className="space-y-0.5">
        <span className="text-[10px] text-muted-foreground">Vertical</span>
        <div className="flex items-center gap-0.5">
          {V_ALIGN_OPTIONS.map(({ value, icon, label }) => (
            <AlignButton
              key={value}
              active={textAlignVertical === value}
              onClick={() =>
                onUpdate({
                  textAlignVertical: value as TextNode['textAlignVertical'],
                } as Partial<PenNode>)
              }
              icon={icon}
              label={label}
            />
          ))}
        </div>
      </div>
    </div>
  )
}
