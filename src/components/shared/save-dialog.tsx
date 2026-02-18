import { useState, useEffect, useRef } from 'react'
import { X } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { useDocumentStore } from '@/stores/document-store'
import { downloadDocument } from '@/utils/file-operations'
import { syncCanvasPositionsToStore } from '@/canvas/use-canvas-sync'

interface SaveDialogProps {
  open: boolean
  onClose: () => void
}

export default function SaveDialog({ open, onClose }: SaveDialogProps) {
  const [name, setName] = useState('untitled')
  const inputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    if (!open) return
    // Pre-fill with existing name (without extension)
    const fn = useDocumentStore.getState().fileName
    if (fn) {
      setName(fn.replace(/\.pen$|\.json$/, ''))
    } else {
      setName('untitled')
    }
    // Focus + select on open
    requestAnimationFrame(() => inputRef.current?.select())
  }, [open])

  useEffect(() => {
    if (!open) return
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    document.addEventListener('keydown', handleKeyDown)
    return () => document.removeEventListener('keydown', handleKeyDown)
  }, [open, onClose])

  if (!open) return null

  const handleSave = () => {
    const trimmed = name.trim()
    if (!trimmed) return
    // Force-sync all Fabric object positions to the store before serializing
    syncCanvasPositionsToStore()
    const fileName = trimmed.endsWith('.pen') ? trimmed : `${trimmed}.pen`
    const doc = useDocumentStore.getState().document
    downloadDocument(doc, fileName)
    useDocumentStore.setState({ fileName, isDirty: false })
    onClose()
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <div className="absolute inset-0 bg-background/80" onClick={onClose} />
      <div className="relative bg-card rounded-lg border border-border p-4 w-72 shadow-xl">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-sm font-medium text-foreground">Save As</h3>
          <Button variant="ghost" size="icon-sm" onClick={onClose}>
            <X size={14} />
          </Button>
        </div>

        <label className="text-xs text-muted-foreground block mb-1">File name</label>
        <div className="flex items-center gap-1 mb-4">
          <input
            ref={inputRef}
            type="text"
            value={name}
            onChange={(e) => setName(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === 'Enter') handleSave()
            }}
            className="flex-1 bg-secondary border border-input rounded px-2 py-1.5 text-sm text-foreground focus:outline-none focus:border-ring"
            autoFocus
          />
          <span className="text-xs text-muted-foreground">.pen</span>
        </div>

        <div className="flex gap-2">
          <Button
            variant="secondary"
            size="sm"
            onClick={onClose}
            className="flex-1"
          >
            Cancel
          </Button>
          <Button
            size="sm"
            onClick={handleSave}
            disabled={!name.trim()}
            className="flex-1"
          >
            Save
          </Button>
        </div>
      </div>
    </div>
  )
}
