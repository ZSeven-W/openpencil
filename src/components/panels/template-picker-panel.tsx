import { useState, useCallback } from 'react'
import { CONTENT_TEMPLATES, type TemplateDefinition } from '@/vibekit/content-templates'
import { instantiateTemplate } from '@/vibekit/template-instantiation'
import { cn } from '@/lib/utils'
import { X, FileText, Quote, BarChart3, MessageSquare, Type } from 'lucide-react'

const TEMPLATE_ICONS: Record<string, React.ComponentType<{ className?: string }>> = {
  'tpl-title-intro': Type,
  'tpl-content': FileText,
  'tpl-quote': Quote,
  'tpl-stat-metric': BarChart3,
  'tpl-cta-closing': MessageSquare,
}

interface TemplatePickerProps {
  open: boolean
  onClose: () => void
}

export function TemplatePicker({ open, onClose }: TemplatePickerProps) {
  const [recentlyAdded, setRecentlyAdded] = useState<string | null>(null)

  const handleInsert = useCallback((template: TemplateDefinition) => {
    const pageId = instantiateTemplate(template.id)
    if (pageId) {
      setRecentlyAdded(template.id)
      setTimeout(() => setRecentlyAdded(null), 600)
    }
  }, [])

  if (!open) return null

  return (
    <div className="absolute left-14 top-2 z-20 w-80 bg-card/95 backdrop-blur-sm border border-border rounded-2xl shadow-2xl">
      {/* Header */}
      <div className="flex items-center justify-between px-4 py-3 border-b border-border">
        <h3 className="text-sm font-medium text-foreground">Templates</h3>
        <button
          onClick={onClose}
          className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
        >
          <X className="w-4 h-4" />
        </button>
      </div>

      {/* Template grid */}
      <div className="p-3 grid gap-2">
        {CONTENT_TEMPLATES.map((template) => {
          const Icon = TEMPLATE_ICONS[template.id] ?? FileText
          const isAdded = recentlyAdded === template.id

          return (
            <button
              key={template.id}
              onClick={() => handleInsert(template)}
              className={cn(
                'flex items-start gap-3 w-full p-3 rounded-xl text-left transition-colors',
                'hover:bg-muted/80',
                isAdded && 'bg-primary/10',
              )}
            >
              <div className="flex-shrink-0 w-9 h-9 rounded-lg bg-muted flex items-center justify-center">
                <Icon className="w-4 h-4 text-muted-foreground" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium text-foreground">{template.name}</div>
                <div className="text-xs text-muted-foreground mt-0.5 line-clamp-2">
                  {template.description}
                </div>
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}

export default TemplatePicker
