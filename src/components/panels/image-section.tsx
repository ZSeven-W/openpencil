import type { ImageNode, ImageFitMode } from '@/types/pen'
import SectionHeader from '@/components/shared/section-header'
import DropdownSelect from '@/components/shared/dropdown-select'

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
      <DropdownSelect
        label="Fit"
        value={node.objectFit ?? 'fill'}
        options={FIT_MODE_OPTIONS}
        onChange={(v) => onUpdate({ objectFit: v as ImageFitMode })}
      />
    </div>
  )
}
