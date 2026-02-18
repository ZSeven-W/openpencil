import NumberInput from '@/components/shared/number-input'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
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
  { value: 'left', label: 'Left' },
  { value: 'center', label: 'Center' },
  { value: 'right', label: 'Right' },
  { value: 'justify', label: 'Justify' },
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
    <div className="space-y-2">
      <h4 className="text-xs font-medium text-muted-foreground uppercase tracking-wider">
        Typography
      </h4>
      <div className="flex items-center gap-2">
        {<span className="text-xs text-muted-foreground">Font</span>}
        <Select
          value={fontFamily}
          onValueChange={(v) =>
            onUpdate({ fontFamily: v } as Partial<PenNode>)
          }
        >
          <SelectTrigger className="flex-1">
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
      </div>
      <div className="grid grid-cols-2 gap-1.5">
        <NumberInput
          label="Sz"
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
          <SelectTrigger>
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
      <div className="flex items-center gap-2">
        <span className="text-xs text-muted-foreground">Align</span>
        <Select
          value={textAlign}
          onValueChange={(v) =>
            onUpdate({
              textAlign: v as TextNode['textAlign'],
            } as Partial<PenNode>)
          }
        >
          <SelectTrigger className="flex-1">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {ALIGN_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
    </div>
  )
}
