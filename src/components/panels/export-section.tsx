import { useState } from 'react'
import { useTranslation } from 'react-i18next'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { exportLayerToRaster, type RasterFormat } from '@/utils/export'
import SectionHeader from '@/components/shared/section-header'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Button } from '@/components/ui/button'

const SCALE_OPTIONS = [
  { value: '1', label: '1x' },
  { value: '2', label: '2x' },
  { value: '3', label: '3x' },
]

const FORMAT_OPTIONS = [
  { value: 'png', label: 'PNG' },
  { value: 'jpeg', label: 'JPEG' },
  { value: 'webp', label: 'WEBP' },
]

interface ExportSectionProps {
  nodeId: string
  nodeName: string
}

export default function ExportSection({ nodeId, nodeName }: ExportSectionProps) {
  const { t } = useTranslation()
  const [scale, setScale] = useState('1')
  const [format, setFormat] = useState('png')
  const fabricCanvas = useCanvasStore((s) => s.fabricCanvas)

  const handleExport = () => {
    if (!fabricCanvas) return

    // Collect all descendant IDs for this node
    const { getFlatNodes, isDescendantOf } = useDocumentStore.getState()
    const allNodes = getFlatNodes()
    const descendantIds = new Set<string>()
    for (const n of allNodes) {
      if (n.id !== nodeId && isDescendantOf(n.id, nodeId)) {
        descendantIds.add(n.id)
      }
    }

    const ext = format === 'jpeg' ? 'jpg' : format
    exportLayerToRaster(fabricCanvas, nodeId, descendantIds, {
      format: format as RasterFormat,
      multiplier: Number(scale),
      filename: `${nodeName}.${ext}`,
    })
  }

  return (
    <div className="space-y-1.5">
      <SectionHeader title={t('export.title')} />
      <div className="flex gap-1.5">
        <Select value={scale} onValueChange={setScale}>
          <SelectTrigger className="flex-1 h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {SCALE_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
        <Select value={format} onValueChange={setFormat}>
          <SelectTrigger className="flex-1 h-6 text-[11px]">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            {FORMAT_OPTIONS.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>
                {opt.label}
              </SelectItem>
            ))}
          </SelectContent>
        </Select>
      </div>
      <Button
        variant="outline"
        size="sm"
        className="w-full text-xs"
        onClick={handleExport}
      >
        {t('export.exportLayer')}
      </Button>
    </div>
  )
}
