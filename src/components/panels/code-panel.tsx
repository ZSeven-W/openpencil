import { useState, useMemo, useCallback, useRef, useEffect } from 'react'
import { Copy, Check, Sparkles, Loader2, RotateCcw, Download, ChevronLeft, ChevronRight } from 'lucide-react'
import { cn } from '@/lib/utils'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore, getActivePageChildren } from '@/stores/document-store'
import { useAIStore } from '@/stores/ai-store'
import { streamChat } from '@/services/ai/ai-service'
import { generateReactCode } from '@/services/codegen/react-generator'
import { generateHTMLCode } from '@/services/codegen/html-generator'
import { generateVueCode } from '@/services/codegen/vue-generator'
import { generateSvelteCode } from '@/services/codegen/svelte-generator'
import { generateSwiftUICode } from '@/services/codegen/swiftui-generator'
import { generateComposeCode } from '@/services/codegen/compose-generator'
import { generateFlutterCode } from '@/services/codegen/flutter-generator'
import { generateReactNativeCode } from '@/services/codegen/react-native-generator'
import { generateCSSVariables } from '@/services/codegen/css-variables-generator'
import { highlightCode } from '@/utils/syntax-highlight'
import type { PenNode } from '@/types/pen'

type CodeTab = 'react' | 'vue' | 'svelte' | 'html' | 'swiftui' | 'compose' | 'flutter' | 'react-native' | 'css-vars'

const ENHANCE_SYSTEM_PROMPT = `You are a code rewriter. You receive auto-generated UI code and rewrite it to be idiomatic, production-ready, and RESPONSIVE.

CRITICAL: Your ENTIRE response must be ONLY the improved source code. Nothing else.
- Do NOT include explanations, commentary, reasoning, or thinking.
- Do NOT include markdown fences (\`\`\`), XML tags, or tool calls.
- Do NOT prefix with "Here is" or any preamble.
- Start your response with the first line of code and end with the last line.

Rewriting rules:
- Preserve visual fidelity — the output must look identical to the input design on desktop.
- Use semantic HTML where appropriate (nav, header, main, section, article, etc.).
- Replace absolute pixel positioning with proper layout (flexbox/grid) where possible.
- Use meaningful class/variable names derived from the node names.
- Keep the same framework and language as the input.
- For CSS-in-JS or scoped styles, keep styles co-located.
- For SwiftUI/Compose/Flutter, use idiomatic patterns (SwiftUI: adaptive stacks, Compose: adaptive layouts, Flutter: LayoutBuilder/MediaQuery).
- Do not add functionality, interactivity, or state beyond what exists.
- If design variables are present as var(--name), preserve them.

RESPONSIVE DESIGN (CRITICAL — make the output adapt gracefully across screen sizes):
- Convert fixed pixel widths to relative units (%, max-w-*, flex-1, w-full) where appropriate. Keep max-width constraints for readability.
- Use responsive Tailwind breakpoints (sm:, md:, lg:) for layout changes:
  - Multi-column rows (horizontal layouts) should stack vertically on small screens (flex-col → sm:flex-row or md:flex-row).
  - Grid layouts: use responsive column counts (grid-cols-1 sm:grid-cols-2 lg:grid-cols-3).
  - Adjust padding/gap with responsive variants (p-4 md:p-8 lg:p-12).
  - Font sizes: scale down on mobile (text-2xl md:text-4xl lg:text-5xl).
- Images: use w-full max-w-* with aspect ratio preservation, never fixed px width.
- Container: use max-w-7xl mx-auto px-4 (or similar) for centered content with side padding.
- Navigation: consider collapsible patterns on small screens.
- For HTML+CSS: use @media queries with mobile-first breakpoints (min-width: 640px, 768px, 1024px).
- For Vue/Svelte: apply the same responsive CSS/class strategies.
- Do NOT break the design on desktop while making it responsive — desktop must remain visually faithful.`

/** Strip markdown fences, reasoning preamble, and tool-call XML from AI output. */
function cleanEnhancedResult(raw: string): string {
  let result = raw

  // Strip everything before the first code-like line if the AI prepended reasoning
  // Detect patterns like "I'll start...", "<tool_call>", "Let me..."
  const reasoningPatterns = /^(I['']ll |Let me |Here['']s |Sure|Okay|<tool_call>|<tool_name>)/im
  if (reasoningPatterns.test(result)) {
    // Try to find the start of actual code after the reasoning
    // Look for common code markers: import, export, <, struct, @, class, fun, <!DOCTYPE
    const codeStart = result.search(/^(import |export |<[!a-zA-Z]|struct |@Composable|@override|class |fun |<!DOCTYPE|package )/m)
    if (codeStart > 0) {
      result = result.slice(codeStart)
    }
  }

  // Strip markdown fences
  if (result.startsWith('```')) {
    const firstNewline = result.indexOf('\n')
    const lastFence = result.lastIndexOf('```')
    if (lastFence > firstNewline) {
      result = result.slice(firstNewline + 1, lastFence).trimEnd()
    }
  }

  return result
}

export default function CodePanel() {
  const { t } = useTranslation()
  const [activeTab, setActiveTab] = useState<CodeTab>('react')
  const [copied, setCopied] = useState(false)
  const [enhancedCode, setEnhancedCode] = useState<Record<string, string>>({})
  const [isEnhancing, setIsEnhancing] = useState(false)
  const enhanceAbortRef = useRef<AbortController | null>(null)
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
    switch (activeTab) {
      case 'css-vars': return generateCSSVariables(document)
      case 'react': return generateReactCode(targetNodes)
      case 'vue': return generateVueCode(targetNodes)
      case 'svelte': return generateSvelteCode(targetNodes)
      case 'swiftui': return generateSwiftUICode(targetNodes)
      case 'compose': return generateComposeCode(targetNodes)
      case 'flutter': return generateFlutterCode(targetNodes)
      case 'react-native': return generateReactNativeCode(targetNodes)
      case 'html': {
        const { html, css } = generateHTMLCode(targetNodes)
        return `<!DOCTYPE html>\n<html lang="en">\n<head>\n  <meta charset="UTF-8" />\n  <meta name="viewport" content="width=device-width, initial-scale=1.0" />\n  <title>Design</title>\n  <style>\n${css.split('\n').map((l) => `    ${l}`).join('\n')}\n  </style>\n</head>\n<body>\n${html.split('\n').map((l) => `  ${l}`).join('\n')}\n</body>\n</html>`
      }
    }
  }, [activeTab, targetNodes, document])

  // Use enhanced code if available for this tab, otherwise the generated code
  const displayCode = enhancedCode[activeTab] ?? generatedCode

  const highlightedHTML = useMemo(() => {
    const langMap: Record<CodeTab, Parameters<typeof highlightCode>[1]> = {
      react: 'jsx',
      vue: 'html',
      svelte: 'html',
      swiftui: 'swift',
      compose: 'kotlin',
      flutter: 'dart',
      'react-native': 'jsx',
      'css-vars': 'css',
      html: 'html',
    }
    // HTML / Vue / Svelte: split at <style to highlight CSS portion separately
    if (activeTab === 'html' || activeTab === 'vue' || activeTab === 'svelte') {
      const styleIdx = displayCode.indexOf('<style')
      if (styleIdx !== -1) {
        const templatePart = displayCode.slice(0, styleIdx)
        const stylePart = displayCode.slice(styleIdx)
        // Highlight the style tag line as HTML, then contents as CSS
        const styleTagEnd = stylePart.indexOf('>\n')
        if (styleTagEnd !== -1) {
          const styleTag = stylePart.slice(0, styleTagEnd + 1)
          const styleBody = stylePart.slice(styleTagEnd + 1)
          const closingIdx = styleBody.lastIndexOf('</style>')
          if (closingIdx !== -1) {
            const cssContent = styleBody.slice(0, closingIdx)
            const closingTag = styleBody.slice(closingIdx)
            return (
              highlightCode(templatePart, 'html') +
              highlightCode(styleTag, 'html') + '\n' +
              highlightCode(cssContent, 'css') +
              highlightCode(closingTag, 'html')
            )
          }
        }
        return highlightCode(templatePart, 'html') + highlightCode(stylePart, 'css')
      }
    }
    return highlightCode(displayCode, langMap[activeTab])
  }, [activeTab, displayCode])

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(displayCode).then(() => {
      setCopied(true)
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
      copyTimeoutRef.current = setTimeout(() => setCopied(false), 2000)
    })
  }, [displayCode])

  const handleDownload = useCallback(() => {
    const extMap: Record<CodeTab, string> = {
      react: 'tsx',
      vue: 'vue',
      svelte: 'svelte',
      html: 'html',
      swiftui: 'swift',
      compose: 'kt',
      flutter: 'dart',
      'react-native': 'tsx',
      'css-vars': 'css',
    }
    const ext = extMap[activeTab]
    const blob = new Blob([displayCode], { type: 'text/plain;charset=utf-8' })
    const url = URL.createObjectURL(blob)
    const a = globalThis.document.createElement('a')
    a.href = url
    a.download = `design.${ext}`
    a.click()
    URL.revokeObjectURL(url)
  }, [activeTab, displayCode])

  const hasAI = useAIStore((s) => s.availableModels.length > 0)

  const handleEnhance = useCallback(async () => {
    if (isEnhancing || activeTab === 'css-vars') return

    const model = useAIStore.getState().model
    const modelGroups = useAIStore.getState().modelGroups
    const provider = modelGroups.find((g) =>
      g.models.some((m) => m.value === model),
    )?.provider

    if (!model || !provider) return

    setIsEnhancing(true)
    const abortController = new AbortController()
    enhanceAbortRef.current = abortController

    // Build a compact node summary for context
    const nodesSummary = JSON.stringify(
      targetNodes.map((n) => {
        const base: Record<string, unknown> = { type: n.type, name: n.name }
        if ('width' in n) base.width = n.width
        if ('height' in n) base.height = n.height
        if ('children' in n && Array.isArray(n.children)) base.childCount = n.children.length
        if ('layout' in n) base.layout = n.layout
        return base
      }),
    )

    const frameworkNames: Record<CodeTab, string> = {
      react: 'React with Tailwind CSS',
      vue: 'Vue 3 SFC',
      svelte: 'Svelte',
      html: 'HTML + CSS',
      swiftui: 'SwiftUI',
      compose: 'Jetpack Compose (Kotlin)',
      flutter: 'Flutter (Dart)',
      'react-native': 'React Native',
      'css-vars': '',
    }

    const userMessage = `Framework: ${frameworkNames[activeTab]}

Design nodes (JSON summary):
${nodesSummary}

Auto-generated code to improve:
${generatedCode}`

    try {
      let result = ''
      for await (const chunk of streamChat(
        ENHANCE_SYSTEM_PROMPT,
        [{ role: 'user', content: userMessage }],
        model,
        { thinkingMode: 'disabled', effort: 'high' },
        provider,
        abortController.signal,
      )) {
        if (chunk.type === 'text') {
          result += chunk.content
          setEnhancedCode((prev) => ({ ...prev, [activeTab]: result }))
        }
        if (chunk.type === 'error') break
      }
      // Clean up artifacts the AI may have included despite instructions
      result = cleanEnhancedResult(result)
      setEnhancedCode((prev) => ({ ...prev, [activeTab]: result }))
    } finally {
      setIsEnhancing(false)
      enhanceAbortRef.current = null
    }
  }, [isEnhancing, activeTab, generatedCode, targetNodes])

  const handleCancelEnhance = useCallback(() => {
    enhanceAbortRef.current?.abort()
    setIsEnhancing(false)
  }, [])

  const handleResetEnhance = useCallback(() => {
    setEnhancedCode((prev) => {
      const next = { ...prev }
      delete next[activeTab]
      return next
    })
  }, [activeTab])

  // Clear enhanced code when nodes change
  useEffect(() => {
    setEnhancedCode({})
  }, [targetNodes])

  useEffect(() => {
    return () => {
      if (copyTimeoutRef.current) clearTimeout(copyTimeoutRef.current)
      enhanceAbortRef.current?.abort()
    }
  }, [])

  const tabs: { key: CodeTab; label: string }[] = [
    { key: 'react', label: 'React' },
    { key: 'vue', label: 'Vue' },
    { key: 'svelte', label: 'Svelte' },
    { key: 'html', label: 'HTML' },
    { key: 'swiftui', label: 'SwiftUI' },
    { key: 'compose', label: 'Compose' },
    { key: 'flutter', label: 'Flutter' },
    { key: 'react-native', label: 'RN' },
    { key: 'css-vars', label: t('code.cssVariables') },
  ]

  const tabsScrollRef = useRef<HTMLDivElement>(null)
  const [canScrollLeft, setCanScrollLeft] = useState(false)
  const [canScrollRight, setCanScrollRight] = useState(false)

  const updateScrollState = useCallback(() => {
    const el = tabsScrollRef.current
    if (!el) return
    setCanScrollLeft(el.scrollLeft > 1)
    setCanScrollRight(el.scrollLeft + el.clientWidth < el.scrollWidth - 1)
  }, [])

  useEffect(() => {
    const el = tabsScrollRef.current
    if (!el) return
    updateScrollState()
    el.addEventListener('scroll', updateScrollState, { passive: true })
    const ro = new ResizeObserver(updateScrollState)
    ro.observe(el)
    return () => {
      el.removeEventListener('scroll', updateScrollState)
      ro.disconnect()
    }
  }, [updateScrollState])

  const scrollTabs = useCallback((dir: 'left' | 'right') => {
    tabsScrollRef.current?.scrollBy({ left: dir === 'left' ? -80 : 80, behavior: 'smooth' })
  }, [])

  return (
    <div className="flex flex-col flex-1 min-h-0">
      {/* Framework tabs + action buttons */}
      <div className="flex items-center pl-1 pr-2 py-1 border-b border-border shrink-0 gap-0.5">
        {canScrollLeft && (
          <button type="button" onClick={() => scrollTabs('left')} className="shrink-0 p-0.5 text-muted-foreground hover:text-foreground">
            <ChevronLeft size={10} />
          </button>
        )}
        <div ref={tabsScrollRef} className="flex items-center gap-1 flex-1 min-w-0 overflow-x-auto scrollbar-none px-1">
          {tabs.map((tab) => (
            <button
              key={tab.key}
              type="button"
              onClick={() => setActiveTab(tab.key)}
              className={cn(
                'text-[10px] px-1.5 py-0.5 rounded transition-colors shrink-0 whitespace-nowrap',
                activeTab === tab.key
                  ? 'bg-secondary text-foreground'
                  : 'text-muted-foreground hover:text-foreground',
              )}
            >
              {tab.label}
            </button>
          ))}
        </div>
        {canScrollRight && (
          <button type="button" onClick={() => scrollTabs('right')} className="shrink-0 p-0.5 text-muted-foreground hover:text-foreground">
            <ChevronRight size={10} />
          </button>
        )}
        <div className="flex items-center gap-0.5 shrink-0">
          {hasAI && activeTab !== 'css-vars' && (
            <>
              {enhancedCode[activeTab] && !isEnhancing && (
                <Tooltip>
                  <TooltipTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon-sm"
                      onClick={handleResetEnhance}
                      className="text-muted-foreground h-5 w-5"
                    >
                      <RotateCcw size={11} />
                    </Button>
                  </TooltipTrigger>
                  <TooltipContent>{t('code.resetEnhance')}</TooltipContent>
                </Tooltip>
              )}
              <Tooltip>
                <TooltipTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    onClick={isEnhancing ? handleCancelEnhance : handleEnhance}
                    className={cn(
                      'h-5 w-5',
                      isEnhancing && 'text-primary',
                      enhancedCode[activeTab] && !isEnhancing && 'text-primary',
                    )}
                  >
                    {isEnhancing ? <Loader2 size={12} className="animate-spin" /> : <Sparkles size={12} />}
                  </Button>
                </TooltipTrigger>
                <TooltipContent>{isEnhancing ? t('code.cancelEnhance') : t('code.aiEnhance')}</TooltipContent>
              </Tooltip>
            </>
          )}
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleDownload}
                className="h-5 w-5"
              >
                <Download size={12} />
              </Button>
            </TooltipTrigger>
            <TooltipContent>{t('code.download')}</TooltipContent>
          </Tooltip>
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon-sm"
                onClick={handleCopy}
                className="h-5 w-5"
              >
                {copied ? <Check size={12} className="text-green-400" /> : <Copy size={12} />}
              </Button>
            </TooltipTrigger>
            <TooltipContent>{copied ? t('code.copied') : t('code.copyClipboard')}</TooltipContent>
          </Tooltip>
        </div>
      </div>

      {/* Code content */}
      <div className="flex-1 overflow-auto p-2">
        <pre className="text-[10px] leading-relaxed font-mono text-foreground/80 whitespace-pre-wrap break-all">
          <code dangerouslySetInnerHTML={{ __html: highlightedHTML }} />
        </pre>
      </div>

      {/* Footer info */}
      <div className="h-5 flex items-center px-2 border-t border-border shrink-0">
        <span className="text-[9px] text-muted-foreground">
          {isEnhancing
            ? t('code.enhancing')
            : enhancedCode[activeTab]
              ? t('code.enhanced')
              : activeTab === 'css-vars'
                ? t('code.genCssVars')
                : selectedIds.length > 0
                  ? t('code.genSelected', { count: selectedIds.length })
                  : t('code.genDocument')}
        </span>
      </div>
    </div>
  )
}
