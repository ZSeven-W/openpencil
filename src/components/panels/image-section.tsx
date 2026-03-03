import type { ImageNode, ImageFitMode } from '@/types/pen'
import SectionHeader from '@/components/shared/section-header'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'

const FIT_MODE_OPTIONS: { value: string; label: string }[] = [
  { value: 'fill', label: 'Fill' },
  { value: 'fit', label: 'Fit' },
  { value: 'crop', label: 'Crop' },
  { value: 'tile', label: 'Tile' },
]

interface ImageSectionProps {
  node: ImageNode
  onUpdate: (updates: Partial<ImageNode>) => void
}

export default function ImageSection({ node, onUpdate }: ImageSectionProps) {
  return (
    <div className="space-y-1.5">
      <SectionHeader title="Image" />
      <div className="flex items-center gap-1.5">
        <span className="text-[10px] text-muted-foreground shrink-0">Fit</span>
        <Select value={node.objectFit ?? 'fill'} onValueChange={(v) => onUpdate({ objectFit: v as ImageFitMode })}>
          <SelectTrigger className="flex-1 h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {FIT_MODE_OPTIONS.map((opt) => (
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
