import NumberInput from '@/components/shared/number-input'
import SectionHeader from '@/components/shared/section-header'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { AlignLeft, AlignCenter, AlignRight, AlignJustify } from 'lucide-react'
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

const ALIGN_OPTIONS = [
  { value: 'left', icon: AlignLeft, label: 'Align left' },
  { value: 'center', icon: AlignCenter, label: 'Align center' },
  { value: 'right', icon: AlignRight, label: 'Align right' },
  { value: 'justify', icon: AlignJustify, label: 'Justify' },
]

export default function TextSection({
  node,
  onUpdate,
}: TextSectionProps) {
  const fontFamily = node.fontFamily ?? 'Inter, sans-serif'
  const fontSize = node.fontSize ?? 16
  const fontWeight = String(node.fontWeight ?? '400')
  const textAlign = node.textAlign ?? 'left'

  return (
    <div className="space-y-1.5">
      <SectionHeader title="Text" />

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

      <div className="grid grid-cols-2 gap-1">
        <NumberInput
          value={fontSize}
          onChange={(v) =>
            onUpdate({ fontSize: v } as Partial<PenNode>)
          }
          min={1}
          max={999}
        />
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
      </div>

      <div className="flex items-center gap-0.5">
        {ALIGN_OPTIONS.map(({ value, icon: Icon, label }) => (
          <button
            key={value}
            type="button"
            aria-label={label}
            onClick={() =>
              onUpdate({
                textAlign: value as TextNode['textAlign'],
              } as Partial<PenNode>)
            }
            className={cn(
              'h-6 w-6 flex items-center justify-center rounded transition-colors',
              textAlign === value
                ? 'bg-secondary text-foreground'
                : 'text-muted-foreground hover:text-foreground hover:bg-secondary/50',
            )}
          >
            <Icon className="w-3.5 h-3.5" />
          </button>
        ))}
      </div>
    </div>
  )
}
