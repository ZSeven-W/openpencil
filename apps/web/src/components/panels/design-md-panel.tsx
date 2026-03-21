import { useState, useRef, useEffect, useCallback } from 'react'
import { X, Upload, Download, Sparkles, ChevronDown, ChevronRight, Copy, Loader2 } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { cn } from '@/lib/utils'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import { useDesignMdStore } from '@/stores/design-md-store'
import { useCanvasStore } from '@/stores/canvas-store'
import { useAIStore } from '@/stores/ai-store'
import { streamChat } from '@/services/ai/ai-service'
import { importDesignMd, exportDesignMd } from '@/utils/design-md-io'
import { parseDesignMd, designMdColorsToVariables, extractDesignMdFromDocument } from '@/utils/design-md-parser'
import type { DesignMdColor } from '@/types/design-md'
import type { PenNode } from '@/types/pen'

const MIN_WIDTH = 420
const MIN_HEIGHT = 300
const DEFAULT_WIDTH = 560
const DEFAULT_HEIGHT = 520

type SectionId = 'theme' | 'colors' | 'typography' | 'components' | 'layout' | 'notes'

// ---------------------------------------------------------------------------
// AI auto-generate prompt
// ---------------------------------------------------------------------------

const DESIGN_MD_SYSTEM_PROMPT = `You are a Design Systems Lead. Analyze the provided PenNode design tree and generate a comprehensive design.md in the Google Stitch format.

OUTPUT FORMAT — a complete markdown document with these sections:

# Design System: [Project Name]

## 1. Visual Theme & Atmosphere
Describe the mood, density, and aesthetic philosophy using evocative adjectives.

## 2. Color Palette & Roles
For each color found in the design:
- **Descriptive Name** (#HEX) — Functional role (e.g. "Primary CTA", "Background", "Body text")

## 3. Typography Rules
- Font families used, weight hierarchy, size scale, line-height conventions.

## 4. Component Stylings
- **Buttons**: shape, colors, padding, states
- **Cards**: corners, shadows, internal padding
- **Inputs**: borders, backgrounds
- **Navigation**: layout, spacing

## 5. Layout Principles
- Grid system, whitespace strategy, spacing units, responsive breakpoints.

## 6. Design System Notes
- Key language/terms to use when generating new designs in this style.

RULES:
- Use descriptive natural language, NOT technical jargon (e.g. "subtly rounded corners" not "rounded-lg").
- Pair ALL colors with exact hex codes.
- Explain functional roles for every design element.
- Output ONLY the markdown document, starting with "# Design System:".
- NO preamble, NO commentary, NO tool calls, NO code fences around the output.
- Do NOT use <tool_call> tags or any tool invocations. Just output the markdown text directly.`

// ---------------------------------------------------------------------------
// Clean AI response artifacts
// ---------------------------------------------------------------------------

function cleanAIResult(raw: string): string {
  let text = raw.trim()

  // Remove <tool_call>...</tool_call> blocks (XML-style tool calls)
  text = text.replace(/<tool_call>[\s\S]*?<\/tool_call>/g, '')

  // Remove preamble before the first markdown heading
  const headingIdx = text.search(/^#\s+/m)
  if (headingIdx > 0) {
    text = text.substring(headingIdx)
  }

  // Strip wrapping code fences
  if (text.startsWith('```')) {
    text = text.replace(/^```(?:markdown|md)?\n?/, '').replace(/\n?```$/, '')
  }

  // Remove JSON tool-call artifacts (e.g. {"name":"Write","arguments":...})
  text = text.replace(/\{"name"\s*:\s*"(?:Write|Read|Edit|Bash)"[^}]*\}\s*/g, '')

  // Remove lines that are tool call fragments or AI narration
  text = text
    .split('\n')
    .filter((line) => {
      const trimmed = line.trim()
      if (trimmed.startsWith('{"name"') || trimmed.startsWith('{"tool_use_id"')) return false
      if (/^\{"file_path"\s*:/.test(trimmed)) return false
      // Drop leftover tool_call tags
      if (trimmed === '<tool_call>' || trimmed === '</tool_call>') return false
      return true
    })
    .join('\n')

  // Strip code fence blocks containing JSON tool calls
  text = text.replace(/```json\s*\{[^`]*?"(?:file_path|name|arguments)"[^`]*?```/gs, '')

  // Collapse excessive blank lines
  text = text.replace(/\n{4,}/g, '\n\n\n')

  return text.trim()
}

// ---------------------------------------------------------------------------
// Panel component
// ---------------------------------------------------------------------------

export default function DesignMdPanel() {
  const { t } = useTranslation()
  const designMd = useDesignMdStore((s) => s.designMd)
  const setDesignMd = useDesignMdStore((s) => s.setDesignMd)
  const setVariable = useDocumentStore((s) => s.setVariable)
  const togglePanel = useCanvasStore((s) => s.toggleDesignMdPanel)

  const [panelWidth, setPanelWidth] = useState(DEFAULT_WIDTH)
  const [panelHeight, setPanelHeight] = useState(DEFAULT_HEIGHT)
  const [panelX, setPanelX] = useState(() => Math.round((window.innerWidth - DEFAULT_WIDTH) / 2))
  const [panelY, setPanelY] = useState(() => Math.round((window.innerHeight - DEFAULT_HEIGHT) / 2))
  const [expandedSections, setExpandedSections] = useState<Set<SectionId>>(
    new Set(['theme', 'colors', 'typography']),
  )
  const [copiedHex, setCopiedHex] = useState<string | null>(null)
  const [isGenerating, setIsGenerating] = useState(false)

  const panelRef = useRef<HTMLDivElement>(null)
  const generateAbortRef = useRef<AbortController | null>(null)
  const resizeRef = useRef<{
    edge: 'right' | 'bottom' | 'corner'
    startX: number; startY: number; startW: number; startH: number
  } | null>(null)
  const dragRef = useRef<{
    startX: number; startY: number; startPanelX: number; startPanelY: number
  } | null>(null)

  // Drag + resize handlers
  useEffect(() => {
    const onMove = (e: PointerEvent) => {
      const d = dragRef.current
      if (d) {
        setPanelX(Math.max(0, Math.min(window.innerWidth - 100, d.startPanelX + (e.clientX - d.startX))))
        setPanelY(Math.max(0, Math.min(window.innerHeight - 40, d.startPanelY + (e.clientY - d.startY))))
        return
      }
      const r = resizeRef.current
      if (!r) return
      const maxW = window.innerWidth - 72
      const maxH = window.innerHeight - 72
      if (r.edge === 'right' || r.edge === 'corner')
        setPanelWidth(Math.min(maxW, Math.max(MIN_WIDTH, r.startW + (e.clientX - r.startX))))
      if (r.edge === 'bottom' || r.edge === 'corner')
        setPanelHeight(Math.min(maxH, Math.max(MIN_HEIGHT, r.startH + (e.clientY - r.startY))))
    }
    const onUp = () => { dragRef.current = null; resizeRef.current = null }
    window.addEventListener('pointermove', onMove)
    window.addEventListener('pointerup', onUp)
    return () => { window.removeEventListener('pointermove', onMove); window.removeEventListener('pointerup', onUp) }
  }, [])

  // Cleanup abort on unmount
  useEffect(() => () => { generateAbortRef.current?.abort() }, [])

  const startDrag = useCallback((e: React.PointerEvent) => {
    if ((e.target as HTMLElement).closest('button')) return
    e.preventDefault()
    dragRef.current = { startX: e.clientX, startY: e.clientY, startPanelX: panelX, startPanelY: panelY }
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  }, [panelX, panelY])

  const startResize = useCallback((edge: 'right' | 'bottom' | 'corner', e: React.PointerEvent) => {
    e.preventDefault()
    resizeRef.current = { edge, startX: e.clientX, startY: e.clientY, startW: panelWidth, startH: panelHeight }
    ;(e.target as HTMLElement).setPointerCapture(e.pointerId)
  }, [panelWidth, panelHeight])

  const toggleSection = (id: SectionId) => {
    setExpandedSections((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id); else next.add(id)
      return next
    })
  }

  // --- Actions ---

  const handleImport = async () => {
    const spec = await importDesignMd()
    if (spec) setDesignMd(spec)
  }

  const handleExport = async () => {
    const spec = designMd ?? extractDesignMdFromDocument(useDocumentStore.getState().document)
    await exportDesignMd(spec)
  }

  const handleAutoGenerate = useCallback(async () => {
    if (isGenerating) {
      generateAbortRef.current?.abort()
      setIsGenerating(false)
      return
    }

    const model = useAIStore.getState().model
    const modelGroups = useAIStore.getState().modelGroups
    const provider = modelGroups.find((g) =>
      g.models.some((m) => m.value === model),
    )?.provider
    if (!model || !provider) return

    const doc = useDocumentStore.getState().document
    const activePageId = useCanvasStore.getState().activePageId

    // Get nodes from the active page
    const nodes = getActivePageChildren(doc, activePageId)
    if (nodes.length === 0) return

    // Build a compact summary of the design tree
    const summarizeNode = (n: PenNode, depth = 0): string => {
      const indent = '  '.repeat(depth)
      const props: string[] = []
      if (n.name) props.push(`"${n.name}"`)
      if (n.role) props.push(`role=${n.role}`)
      if ('fill' in n && Array.isArray(n.fill)) {
        for (const f of n.fill) {
          if (f.type === 'solid' && f.color) props.push(`fill:${f.color}`)
          if (f.type === 'linear_gradient') props.push('fill:gradient')
        }
      }
      if ('stroke' in n && n.stroke) {
        const sf = n.stroke.fill?.[0]
        if (sf?.type === 'solid' && sf.color) props.push(`stroke:${sf.color}/${n.stroke.thickness}`)
      }
      if ('content' in n && n.content) props.push(`"${String(n.content).slice(0, 40)}"`)
      if ('fontSize' in n) props.push(`${n.fontSize}px`)
      if ('fontFamily' in n) props.push(`font:${n.fontFamily}`)
      if ('fontWeight' in n) props.push(`w:${n.fontWeight}`)
      if ('width' in n) props.push(`w=${n.width}`)
      if ('height' in n) props.push(`h=${n.height}`)
      if ('cornerRadius' in n && n.cornerRadius) props.push(`r=${n.cornerRadius}`)
      if ('gap' in n && n.gap) props.push(`gap=${n.gap}`)
      if ('padding' in n && n.padding) props.push(`pad=${JSON.stringify(n.padding)}`)
      if ('layout' in n && n.layout && n.layout !== 'none') props.push(`layout=${n.layout}`)
      if ('justifyContent' in n && n.justifyContent) props.push(`justify=${n.justifyContent}`)
      if ('alignItems' in n && n.alignItems) props.push(`align=${n.alignItems}`)
      if ('effects' in n && Array.isArray(n.effects) && n.effects.length > 0) {
        props.push(`effects=${n.effects.map(e => e.type).join(',')}`)
      }
      if ('opacity' in n && n.opacity !== undefined && n.opacity !== 1) props.push(`opacity=${n.opacity}`)

      const line = `${indent}${n.type} ${props.join(' ')}`
      const childLines: string[] = []
      if ('children' in n && Array.isArray(n.children) && depth < 5) {
        for (const child of n.children.slice(0, 40)) {
          childLines.push(summarizeNode(child as PenNode, depth + 1))
        }
      }
      return [line, ...childLines].join('\n')
    }

    const treeSummary = nodes.slice(0, 10).map((n) => summarizeNode(n as PenNode)).join('\n\n')

    // Variable summary
    let varSummary = ''
    if (doc.variables && Object.keys(doc.variables).length > 0) {
      varSummary = '\n\nDESIGN VARIABLES:\n' + Object.entries(doc.variables)
        .map(([name, def]) => {
          const val = Array.isArray(def.value) ? String(def.value[0]?.value ?? '') : String(def.value)
          return `- ${name} (${def.type}): ${val}`
        })
        .join('\n')
    }

    const userMessage = `Analyze this PenNode design tree and generate a comprehensive design.md:\n\nProject: ${doc.name ?? 'Untitled'}\n\nDesign tree (PenNode format — type followed by properties):\n${treeSummary}${varSummary}`

    setIsGenerating(true)
    const abortController = new AbortController()
    generateAbortRef.current = abortController

    try {
      let result = ''
      for await (const chunk of streamChat(
        DESIGN_MD_SYSTEM_PROMPT,
        [{ role: 'user', content: userMessage }],
        model,
        { thinkingMode: 'disabled', effort: 'high' },
        provider,
        abortController.signal,
      )) {
        if (chunk.type === 'text') {
          result += chunk.content
        }
        if (chunk.type === 'error') break
      }
      if (result.trim()) {
        const cleaned = cleanAIResult(result)
        if (cleaned) {
          const spec = parseDesignMd(cleaned)
          setDesignMd(spec)
        }
      }
    } finally {
      setIsGenerating(false)
      generateAbortRef.current = null
    }
  }, [isGenerating, setDesignMd])

  const handleCopyHex = (hex: string) => {
    navigator.clipboard.writeText(hex)
    setCopiedHex(hex)
    setTimeout(() => setCopiedHex(null), 1500)
  }

  const handleSyncColor = (color: DesignMdColor) => {
    const vars = designMdColorsToVariables([color])
    for (const [name, def] of Object.entries(vars)) setVariable(name, def)
  }

  const handleSyncAllColors = () => {
    if (!designMd?.colorPalette) return
    const vars = designMdColorsToVariables(designMd.colorPalette)
    for (const [name, def] of Object.entries(vars)) setVariable(name, def)
  }

  const handleClear = () => setDesignMd(undefined)

  const hasAI = useAIStore((s) => s.availableModels.length > 0)

  // Check if designMd has any meaningful content beyond just raw text
  const hasContent = designMd && (
    designMd.visualTheme ||
    (designMd.colorPalette && designMd.colorPalette.length > 0) ||
    designMd.typography ||
    designMd.componentStyles ||
    designMd.layoutPrinciples ||
    designMd.generationNotes
  )

  // --- Render ---

  return (
    <div
      ref={panelRef}
      className="fixed z-50 bg-card border border-border rounded-xl shadow-2xl flex flex-col overflow-hidden"
      style={{ width: panelWidth, height: panelHeight, left: panelX, top: panelY }}
    >
      {/* Header — draggable */}
      <div
        className="flex items-center justify-between px-3 py-2 border-b border-border bg-muted/40 shrink-0 cursor-grab active:cursor-grabbing select-none"
        onPointerDown={startDrag}
      >
        <span className="text-xs font-medium text-foreground">{t('designMd.title')}</span>
        <div className="flex items-center gap-0.5">
          <button onClick={handleImport} className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors" title={t('designMd.import')}>
            <Upload size={14} />
          </button>
          <button onClick={handleExport} className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors" title={t('designMd.export')}>
            <Download size={14} />
          </button>
          {hasAI && (
            <button
              onClick={handleAutoGenerate}
              className={cn('p-1.5 rounded-md transition-colors', isGenerating ? 'text-primary bg-primary/10' : 'text-muted-foreground hover:bg-muted hover:text-foreground')}
              title={t('designMd.autoGenerate')}
            >
              {isGenerating ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} />}
            </button>
          )}
          <button onClick={togglePanel} className="p-1.5 rounded-md hover:bg-muted text-muted-foreground hover:text-foreground transition-colors">
            <X size={14} />
          </button>
        </div>
      </div>

      {/* Content */}
      <div className="flex-1 overflow-y-auto min-h-0">
        {!hasContent ? (
          <div className="flex flex-col items-center justify-center h-full gap-4 px-6">
            {designMd?.raw ? (
              /* designMd exists but parser couldn't extract sections — show raw */
              <div className="w-full h-full flex flex-col gap-3 py-3">
                <p className="text-[11px] text-muted-foreground leading-relaxed whitespace-pre-wrap flex-1 overflow-y-auto font-mono">
                  {designMd.raw}
                </p>
                <div className="flex items-center gap-2 shrink-0">
                  <button
                    onClick={handleAutoGenerate}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
                  >
                    {isGenerating ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                    {t('designMd.autoGenerateCta')}
                  </button>
                  <button onClick={handleClear} className="text-[10px] text-muted-foreground hover:text-destructive transition-colors">
                    {t('designMd.remove')}
                  </button>
                </div>
              </div>
            ) : (
              /* No designMd at all — empty state */
              <>
                <div className="w-10 h-10 rounded-xl bg-muted/50 flex items-center justify-center">
                  <Sparkles size={20} className="text-muted-foreground" />
                </div>
                <p className="text-xs text-muted-foreground text-center">{t('designMd.empty')}</p>
                <div className="flex gap-2">
                  <button
                    onClick={handleImport}
                    className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg bg-primary text-primary-foreground hover:bg-primary/90 transition-colors"
                  >
                    <Upload size={12} /> {t('designMd.importCta')}
                  </button>
                  {hasAI && (
                    <button
                      onClick={handleAutoGenerate}
                      className="flex items-center gap-1.5 px-3 py-1.5 text-xs rounded-lg border border-border text-foreground hover:bg-muted transition-colors"
                    >
                      {isGenerating ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                      {t('designMd.autoGenerateCta')}
                    </button>
                  )}
                </div>
              </>
            )}
          </div>
        ) : (
          <div className="p-3 space-y-2">
            {/* Project name */}
            {designMd.projectName && (
              <div className="px-1 pb-1">
                <h3 className="text-sm font-semibold text-foreground">{designMd.projectName}</h3>
              </div>
            )}

            {/* Visual Theme */}
            {designMd.visualTheme && (
              <Section
                title={t('designMd.visualTheme')}
                expanded={expandedSections.has('theme')}
                onToggle={() => toggleSection('theme')}
              >
                <MdText text={designMd.visualTheme} limit={600} />
              </Section>
            )}

            {/* Color Palette */}
            {designMd.colorPalette && designMd.colorPalette.length > 0 && (
              <Section
                title={`${t('designMd.colors')} (${designMd.colorPalette.length})`}
                expanded={expandedSections.has('colors')}
                onToggle={() => toggleSection('colors')}
                action={
                  <button
                    onClick={(e) => { e.stopPropagation(); handleSyncAllColors() }}
                    className="text-[10px] text-primary hover:text-primary/80 transition-colors"
                  >
                    {t('designMd.syncAllToVariables')}
                  </button>
                }
              >
                <div className="space-y-0.5">
                  {designMd.colorPalette.map((color, i) => (
                    <div key={i} className="flex items-center gap-2 px-1.5 py-1 rounded-md hover:bg-muted/50 group transition-colors">
                      <div
                        className="w-6 h-6 rounded-md border border-border/60 shrink-0 cursor-pointer shadow-sm"
                        style={{ backgroundColor: color.hex }}
                        onClick={() => handleCopyHex(color.hex)}
                        title={t('designMd.copyHex')}
                      />
                      <div className="flex-1 min-w-0">
                        <span className="text-[11px] font-medium text-foreground truncate block">{color.name}</span>
                        <span className="text-[10px] text-muted-foreground truncate block">{color.hex} — {color.role}</span>
                      </div>
                      <div className="flex items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                        <button onClick={() => handleCopyHex(color.hex)} className="p-1 rounded-md hover:bg-muted" title={t('designMd.copyHex')}>
                          <Copy size={11} className={cn('text-muted-foreground', copiedHex === color.hex && 'text-primary')} />
                        </button>
                        <button
                          onClick={() => handleSyncColor(color)}
                          className="text-[9px] px-1.5 py-0.5 rounded-md hover:bg-muted text-muted-foreground font-medium"
                          title={t('designMd.addAsVariable')}
                        >
                          {t('designMd.addAsVariable')}
                        </button>
                      </div>
                    </div>
                  ))}
                </div>
              </Section>
            )}

            {/* Typography */}
            {designMd.typography && (
              <Section
                title={t('designMd.typography')}
                expanded={expandedSections.has('typography')}
                onToggle={() => toggleSection('typography')}
              >
                {designMd.typography.scale ? (
                  <MdText text={designMd.typography.scale} limit={600} />
                ) : (
                  <div className="space-y-1.5 text-[11px] text-muted-foreground">
                    {designMd.typography.fontFamily && (
                      <p><span className="text-foreground font-medium">{t('designMd.font')}:</span> {designMd.typography.fontFamily}</p>
                    )}
                    {designMd.typography.headings && (
                      <p><span className="text-foreground font-medium">{t('designMd.headings')}:</span> {renderInline(designMd.typography.headings)}</p>
                    )}
                    {designMd.typography.body && (
                      <p><span className="text-foreground font-medium">{t('designMd.body')}:</span> {renderInline(designMd.typography.body)}</p>
                    )}
                  </div>
                )}
              </Section>
            )}

            {/* Component Styles */}
            {designMd.componentStyles && (
              <Section title={t('designMd.componentStyles')} expanded={expandedSections.has('components')} onToggle={() => toggleSection('components')}>
                <MdText text={designMd.componentStyles} limit={1000} />
              </Section>
            )}

            {/* Layout Principles */}
            {designMd.layoutPrinciples && (
              <Section title={t('designMd.layoutPrinciples')} expanded={expandedSections.has('layout')} onToggle={() => toggleSection('layout')}>
                <MdText text={designMd.layoutPrinciples} limit={1000} />
              </Section>
            )}

            {/* Generation Notes */}
            {designMd.generationNotes && (
              <Section title={t('designMd.generationNotes')} expanded={expandedSections.has('notes')} onToggle={() => toggleSection('notes')}>
                <MdText text={designMd.generationNotes} limit={600} />
              </Section>
            )}

            {/* Footer: Remove */}
            <div className="pt-2 pb-1 border-t border-border/30">
              <button onClick={handleClear} className="text-[10px] text-muted-foreground hover:text-destructive transition-colors">
                {t('designMd.remove')}
              </button>
            </div>
          </div>
        )}
      </div>

      {/* Generating overlay */}
      {isGenerating && (
        <div className="absolute inset-0 top-[37px] bg-card/80 backdrop-blur-sm flex flex-col items-center justify-center gap-3 z-10">
          <Loader2 size={24} className="text-primary animate-spin" />
          <p className="text-xs text-muted-foreground">{t('ai.generating')}</p>
          <button
            onClick={handleAutoGenerate}
            className="text-[10px] px-2 py-1 rounded-md border border-border hover:bg-muted text-muted-foreground transition-colors"
          >
            {t('ai.stopGenerating')}
          </button>
        </div>
      )}

      {/* Resize handles */}
      <div className="absolute right-0 top-0 w-1.5 h-full cursor-ew-resize" onPointerDown={(e) => startResize('right', e)} />
      <div className="absolute bottom-0 left-0 w-full h-1.5 cursor-ns-resize" onPointerDown={(e) => startResize('bottom', e)} />
      <div className="absolute right-0 bottom-0 w-3 h-3 cursor-nwse-resize" onPointerDown={(e) => startResize('corner', e)} />
    </div>
  )
}

// ---------------------------------------------------------------------------
// Lightweight markdown renderer
// ---------------------------------------------------------------------------

function MdText({ text, limit }: { text: string; limit?: number }) {
  const content = limit && text.length > limit ? text.substring(0, limit) + '...' : text

  // Split into blocks by double newline
  const blocks = content.split(/\n{2,}/)

  return (
    <div className="space-y-2 text-[11px] text-muted-foreground leading-relaxed">
      {blocks.map((block, i) => {
        const trimmed = block.trim()
        if (!trimmed) return null

        // H3 heading
        if (trimmed.startsWith('### ')) {
          return <h4 key={i} className="text-[11px] font-semibold text-foreground mt-1">{renderInline(trimmed.slice(4))}</h4>
        }
        // H4 heading
        if (trimmed.startsWith('#### ')) {
          return <h5 key={i} className="text-[11px] font-medium text-foreground">{renderInline(trimmed.slice(5))}</h5>
        }

        // List block
        const lines = trimmed.split('\n')
        const isList = lines.every((l) => /^\s*[-*]\s/.test(l) || !l.trim())
        if (isList) {
          return (
            <ul key={i} className="space-y-1">
              {lines.filter((l) => l.trim()).map((line, j) => (
                <li key={j} className="flex gap-1.5">
                  <span className="text-muted-foreground/50 shrink-0 mt-px">&#8226;</span>
                  <span>{renderInline(line.replace(/^\s*[-*]\s+/, ''))}</span>
                </li>
              ))}
            </ul>
          )
        }

        // Paragraph
        return <p key={i}>{renderInline(trimmed.replace(/\n/g, ' '))}</p>
      })}
    </div>
  )
}

/** Render inline markdown: **bold**, *italic*, `code`, #HEX color chips */
function renderInline(text: string): React.ReactNode {
  // Split by markdown inline patterns
  const parts: React.ReactNode[] = []
  let remaining = text
  let key = 0

  while (remaining) {
    // Bold: **text**
    const boldMatch = remaining.match(/\*\*(.+?)\*\*/)
    // Code: `text`
    const codeMatch = remaining.match(/`([^`]+)`/)
    // Hex color: #XXXXXX
    const colorMatch = remaining.match(/#([0-9A-Fa-f]{6})\b/)

    // Find earliest match
    const matches = [
      boldMatch && { type: 'bold' as const, index: boldMatch.index!, length: boldMatch[0].length, content: boldMatch[1] },
      codeMatch && { type: 'code' as const, index: codeMatch.index!, length: codeMatch[0].length, content: codeMatch[1] },
      colorMatch && { type: 'color' as const, index: colorMatch.index!, length: colorMatch[0].length, content: `#${colorMatch[1]}` },
    ].filter(Boolean).sort((a, b) => a!.index - b!.index)

    if (matches.length === 0) {
      parts.push(remaining)
      break
    }

    const m = matches[0]!
    if (m.index > 0) parts.push(remaining.substring(0, m.index))

    switch (m.type) {
      case 'bold':
        parts.push(<strong key={key++} className="text-foreground font-medium">{m.content}</strong>)
        break
      case 'code':
        parts.push(<code key={key++} className="px-1 py-0.5 rounded bg-muted text-[10px] font-mono text-foreground">{m.content}</code>)
        break
      case 'color':
        parts.push(
          <span key={key++} className="inline-flex items-center gap-1">
            <span className="inline-block w-2.5 h-2.5 rounded-sm border border-border/50 shrink-0" style={{ backgroundColor: m.content }} />
            <span className="font-mono text-[10px]">{m.content}</span>
          </span>,
        )
        break
    }

    remaining = remaining.substring(m.index + m.length)
  }

  return parts.length === 1 && typeof parts[0] === 'string' ? parts[0] : <>{parts}</>
}

// ---------------------------------------------------------------------------
// Collapsible section
// ---------------------------------------------------------------------------

function Section({
  title,
  expanded,
  onToggle,
  action,
  children,
}: {
  title: string
  expanded: boolean
  onToggle: () => void
  action?: React.ReactNode
  children: React.ReactNode
}) {
  return (
    <div className="rounded-lg border border-border/40 overflow-hidden">
      <button
        onClick={onToggle}
        className="flex items-center justify-between w-full px-2.5 py-2 text-[11px] font-medium text-foreground hover:bg-muted/30 transition-colors"
      >
        <div className="flex items-center gap-1.5">
          {expanded ? <ChevronDown size={12} className="text-muted-foreground" /> : <ChevronRight size={12} className="text-muted-foreground" />}
          {title}
        </div>
        {action && <div onClick={(e) => e.stopPropagation()}>{action}</div>}
      </button>
      {expanded && <div className="px-2.5 pb-2.5 pt-0.5">{children}</div>}
    </div>
  )
}
