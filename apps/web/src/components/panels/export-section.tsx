import { useState } from 'react'
import { useTranslation } from 'react-i18next'
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

export default function ExportSection({ nodeId: _nodeId, nodeName: _nodeName }: ExportSectionProps) {
  const { t } = useTranslation()
  const [scale, setScale] = useState('1')
  const [format, setFormat] = useState('png')

  const handleExport = () => {
    // TODO: migrate to CanvasKit-based export
    console.warn('[ExportSection] Fabric.js export removed — pending CanvasKit migration')
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
