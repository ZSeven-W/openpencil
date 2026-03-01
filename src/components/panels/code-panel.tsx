import { useState, useMemo, useCallback, useRef, useEffect } from 'react'
import { Copy, Check, X } from 'lucide-react'
import { cn } from '@/lib/utils'
import { Button } from '@/components/ui/button'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import { generateReactCode } from '@/services/codegen/react-generator'
import { generateHTMLCode } from '@/services/codegen/html-generator'
import { generateCSSVariables } from '@/services/codegen/css-variables-generator'
import { highlightCode } from '@/utils/syntax-highlight'
import type { PenNode } from '@/types/pen'

type CodeTab = 'react' | 'html' | 'css-vars'

export default function CodePanel({ onClose }: { onClose: () => void }) {
  const [activeTab, setActiveTab] = useState<CodeTab>('react')
  const [copied, setCopied] = useState(false)
  const copyTimeoutRef = useRef<ReturnType<typeof setTimeout>>(null)
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)
  const activePageId = useCanvasStore((s) => s.activePageId)
  const children = useDocumentStore((s) => getActivePageChildren(s.document, activePageId))
  const getNodeById = useDocumentStore((s) => s.getNodeById)

  // Force re-render when document changes
  void children

  const targetNodes: PenNode[] = useMemo(() => {
    if (selectedIds.length > 0) {
      return selectedIds
        .map((id) => getNodeById(id))
        .filter((n): n is PenNode => n !== undefined)
    }
    return children
  }, [selectedIds, children, getNodeById])

  const document = useDocumentStore((s) => s.document)

  const generatedCode = useMemo(() => {
    if (activeTab === 'css-vars') {
      return generateCSSVariables(document)
    }
    if (activeTab === 'react') {
      return generateReactCode(targetNodes)
    }
    const { html, css } = generateHTMLCode(targetNodes)
    return `<!-- HTML -->\n${html}\n\n/* CSS */\n${css}`
  }, [activeTab, targetNodes, document])

  const highlightedHTML = useMemo(() => {
    if (activeTab === 'css-vars') {
      return highlightCode(generatedCode, 'css')
    }
    if (activeTab === 'react') {
      return highlightCode(generatedCode, 'jsx')
    }
    // Split HTML and CSS sections for mixed highlighting
    const htmlEnd = generatedCode.indexOf('\n\n/* CSS */')
    if (htmlEnd === -1) return highlightCode(generatedCode, 'html')
    const htmlPart = generatedCode.slice(0, htmlEnd)
    const cssPart = generatedCode.slice(htmlEnd + 2)
    return highlightCode(htmlPart, 'html') + '\n\n' + highlightCode(cssPart, 'css')
  }, [activeTab, generatedCode])

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(generatedCode).then(() => {
      setCopied(true)
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
      copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000)
    })
  }, [generatedCode])

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
    }
  }, [])

  const tabs: { key: CodeTab; label: string }[] = [
    { key: 'react', label: 'React + Tailwind' },
    { key: 'html', label: 'HTML + CSS' },
    { key: 'css-vars', label: 'CSS Variables' },
  ]

  return (
    <div className="bg-card border-t border-border flex flex-col" style={{ height: 280 }}>
      {/* Header */}
      <div className="h-8 flex items-center px-3 border-b border-border shrink-0">
        <div className="flex items-center gap-2 flex-1">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              type="button"
              onClick={() => setActiveTab(tab.key)}
              className={cn(
                'text-xs px-2 py-1 rounded transition-colors',
                activeTab === tab.key
                  ? 'bg-secondary text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {tab.label}
            </button>
          ))}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleCopy}
            title="Copy to clipboard"
          >
            {copied ? <Check size={14} className="text-green-400" /> : <Copy size={14} />}
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={onClose}
            title="Close code panel"
          >
            <X size={14} />
          </Button>
        </div>
      </div>

      {/* Code content */}
      <div className="flex-1 overflow-auto p-3">
        <pre className="text-xs leading-relaxed font-mono text-foreground/80 whitespace-pre">
          <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
        </pre>
      </div>

      {/* Footer info */}
      <div className="h-6 flex items-center px-3 border-t border-border shrink-0">
        <span className="text-[10px] text-muted-foreground">
          {activeTab === 'css-vars'
            ? 'Generating CSS variables for entire document'
            : selectedIds.length > 0
              ? `Generating code for ${selectedIds.length} selected element${selectedIds.length > 1 ? 's' : ''}`
              : 'Generating code for entire document'}
        </span>
      </div>
    </div>
  )
}
