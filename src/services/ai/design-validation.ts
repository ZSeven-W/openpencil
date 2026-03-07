/**
 * Post-generation screenshot validation.
 *
 * After all design sections are generated, captures a screenshot of the root
 * frame and sends it alongside a simplified node tree to the vision API.
 * The LLM correlates visual issues with actual node IDs and returns fixes.
 */

import { useCanvasStore } from '@/stores/canvas-store'
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import { VALIDATION_TIMEOUT_MS, MAX_VALIDATION_ROUNDS, VALIDATION_QUALITY_THRESHOLD } from './ai-runtime-config'
import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AIProviderType } from '@/types/agent-settings'
import { getCurrentVisualReference, clearVisualReference } from './visual-ref-orchestrator'

// ---------------------------------------------------------------------------
// System prompt for the vision validator
// ---------------------------------------------------------------------------

const VALIDATION_SYSTEM_PROMPT = `You are a design QA validator. You receive a screenshot of a UI design AND its node tree structure.
Cross-reference the visual issues you see in the screenshot with the node IDs in the tree.

Check for these issues:
1. WIDTH INCONSISTENCY: Form inputs, buttons, cards that are siblings but have different widths. They should all use "fill_container" width to match their parent.
2. ELEMENT TOO NARROW: Buttons or inputs that are much narrower than their parent container. Fix: width="fill_container".
3. SPACING: Uneven padding, elements too close to edges, inconsistent gaps between siblings.
4. OVERFLOW: Text or elements visually clipped or extending beyond their container.
5. ALIGNMENT: Elements that should be aligned but aren't (e.g. form fields not left-aligned).
6. TEXT CENTERING: Text that should be horizontally centered in its container but appears shifted left or right. Common in headings, buttons, divider text ("or continue with"), and footer text. Fix: ensure the parent container has alignItems="center" or the text node has width="fill_container".
7. MISSING ICONS: Path nodes that rendered as empty/invisible rectangles.
8. COLOR ISSUES: Text with poor contrast against its background, wrong background colors, inconsistent color usage across similar elements.
9. TYPOGRAPHY: Inconsistent font sizes between similar elements, wrong font weights for headings vs body text.

Output ONLY a JSON object. No explanation, no markdown fences.
{"qualityScore":8,"issues":["description1","description2"],"fixes":[{"nodeId":"actual-node-id","property":"width","value":"fill_container"}]}

qualityScore: Rate the overall design quality from 1-10.
- 9-10: Production-ready, polished design
- 7-8: Good design with minor issues
- 5-6: Acceptable but needs improvement
- 1-4: Significant problems

Allowed properties and value types:
- width: number | "fill_container" | "fit_content"
- height: number | "fill_container" | "fit_content"
- padding: number | [top,right,bottom,left]
- gap: number
- fontSize: number
- fontWeight: number (300-900)
- letterSpacing: number
- lineHeight: number
- cornerRadius: number
- opacity: number
- fillColor: "#hex" (background/fill color of the node)
- textAlign: "left" | "center" | "right" (text horizontal alignment within its box)
- alignItems: "start" | "center" | "end"
- justifyContent: "start" | "center" | "end" | "space_between"

IMPORTANT:
- Use REAL node IDs from the provided tree — never guess or fabricate IDs.
- For form consistency issues, fix ALL inconsistent siblings, not just one.
- If the design looks correct, return: {"qualityScore":9,"issues":[],"fixes":[]}
- Keep fixes minimal — only fix clear visual bugs, not stylistic preferences.
- Focus on the most impactful issues first.`

// Properties that are safe to auto-fix, with allowed value types
const SAFE_FIX_PROPERTIES: Record<string, 'number' | 'sizing' | 'number_or_array' | 'enum_align' | 'enum_justify' | 'enum_text_align' | 'color' | 'font_weight'> = {
  width: 'sizing',
  height: 'sizing',
  padding: 'number_or_array',
  gap: 'number',
  fontSize: 'number',
  fontWeight: 'font_weight',
  letterSpacing: 'number',
  lineHeight: 'number',
  cornerRadius: 'number',
  opacity: 'number',
  fillColor: 'color',
  textAlign: 'enum_text_align',
  alignItems: 'enum_align',
  justifyContent: 'enum_justify',
}

const VALID_SIZING_STRINGS = new Set(['fill_container', 'fit_content'])
const VALID_ALIGN = new Set(['start', 'center', 'end'])
const VALID_JUSTIFY = new Set(['start', 'center', 'end', 'space_between', 'space_around'])

// ---------------------------------------------------------------------------
// Node tree dump — simplified for LLM context
// ---------------------------------------------------------------------------

function buildNodeTreeDump(rootId: string): string {
  const store = useDocumentStore.getState()
  const lines: string[] = []

  function walk(node: PenNode, depth: number) {
    const indent = '  '.repeat(depth)
    const props: string[] = [`id="${node.id}"`, `type=${node.type}`]

    if (node.name) props.push(`name="${node.name}"`)
    if ('width' in node && node.width != null) props.push(`w=${JSON.stringify(node.width)}`)
    if ('height' in node && node.height != null) props.push(`h=${JSON.stringify(node.height)}`)
    if ('layout' in node && node.layout) props.push(`layout=${node.layout}`)
    if ('gap' in node && node.gap != null) props.push(`gap=${node.gap}`)
    if ('padding' in node && node.padding != null) props.push(`pad=${JSON.stringify(node.padding)}`)
    if ('justifyContent' in node && node.justifyContent) props.push(`justify=${node.justifyContent}`)
    if ('alignItems' in node && node.alignItems) props.push(`align=${node.alignItems}`)
    if ('cornerRadius' in node && node.cornerRadius != null) props.push(`cr=${node.cornerRadius}`)
    if ('opacity' in node && node.opacity != null && node.opacity !== 1) props.push(`opacity=${node.opacity}`)
    if ('fill' in node && Array.isArray(node.fill) && node.fill.length > 0) {
      const firstFill = node.fill[0]
      if (firstFill && 'color' in firstFill && firstFill.color) props.push(`fill="${firstFill.color}"`)
    }
    if ('stroke' in node && Array.isArray(node.stroke) && node.stroke.length > 0) {
      const firstStroke = node.stroke[0]
      if (firstStroke && 'color' in firstStroke) props.push(`stroke="${firstStroke.color}"`)
    }
    if (node.type === 'text') {
      if ('fontSize' in node && node.fontSize) props.push(`fontSize=${node.fontSize}`)
      if ('fontWeight' in node && node.fontWeight) props.push(`fontWeight=${node.fontWeight}`)
      if ('textAlign' in node && node.textAlign) props.push(`textAlign=${node.textAlign}`)
      if ('content' in node) {
        const content = (node as { content?: string }).content ?? ''
        props.push(`text="${content.slice(0, 30)}"`)
      }
    }

    lines.push(`${indent}${props.join(' ')}`)

    if ('children' in node && node.children) {
      for (const child of node.children) {
        walk(child, depth + 1)
      }
    }
  }

  const rootNode = store.getNodeById(rootId)
  if (rootNode) walk(rootNode, 0)
  return lines.join('\n')
}

// ---------------------------------------------------------------------------
// Screenshot capture
// ---------------------------------------------------------------------------

export function captureRootFrameScreenshot(): string | null {
  const canvas = useCanvasStore.getState().fabricCanvas
  if (!canvas) return null

  const store = useDocumentStore.getState()
  const rootNode = store.getNodeById(DEFAULT_FRAME_ID)
  if (!rootNode) return null

  const allFlat = store.getFlatNodes()
  const descendantIds = new Set<string>()
  for (const node of allFlat) {
    if (node.id !== DEFAULT_FRAME_ID && store.isDescendantOf(node.id, DEFAULT_FRAME_ID)) {
      descendantIds.add(node.id)
    }
  }

  const allObjects = canvas.getObjects() as FabricObjectWithPenId[]
  const rootObj = allObjects.find((obj) => obj.penNodeId === DEFAULT_FRAME_ID)
  if (!rootObj) return null

  const originX = rootObj.left ?? 0
  const originY = rootObj.top ?? 0
  const w = (rootObj.width ?? 0) * (rootObj.scaleX ?? 1)
  const h = (rootObj.height ?? 0) * (rootObj.scaleY ?? 1)

  if (w <= 0 || h <= 0) return null

  const allIds = new Set(descendantIds)
  allIds.add(DEFAULT_FRAME_ID)

  const layerObjects = allObjects.filter(
    (obj) => obj.penNodeId && allIds.has(obj.penNodeId),
  )

  const offscreen = document.createElement('canvas')
  offscreen.width = Math.ceil(w)
  offscreen.height = Math.ceil(h)
  const ctx = offscreen.getContext('2d')
  if (!ctx) return null

  ctx.translate(-originX, -originY)

  for (const obj of layerObjects) {
    obj.render(ctx)
  }

  return offscreen.toDataURL('image/png')
}

// ---------------------------------------------------------------------------
// Validation API call
// ---------------------------------------------------------------------------

interface ValidationFix {
  nodeId: string
  property: string
  value: number | string | number[]
}

interface ValidationResult {
  issues: string[]
  fixes: ValidationFix[]
  qualityScore: number
  skipped?: boolean
}

async function validateDesignScreenshot(
  imageBase64: string,
  nodeTreeDump: string,
  model?: string,
  provider?: AIProviderType,
  referenceScreenshot?: string,
  round: number = 1,
): Promise<ValidationResult> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), referenceScreenshot ? VALIDATION_TIMEOUT_MS * 2 : VALIDATION_TIMEOUT_MS)

  const referenceInstruction = referenceScreenshot
    ? `\n\nA REFERENCE DESIGN screenshot was also provided. Compare the current design against the reference and fix any significant deviations in layout, spacing, or proportions. The reference shows the intended design — the current screenshot should match its structure and visual balance.`
    : ''

  const roundInstruction = round > 1
    ? `\n\nThis is validation round ${round}. Previous fixes have already been applied. Focus on remaining issues only — do NOT re-report issues that have already been fixed.`
    : ''

  const message = `Analyze this UI design screenshot. Here is the node tree structure:

\`\`\`
${nodeTreeDump}
\`\`\`

Cross-reference visual issues with the node IDs above. Return JSON fixes using real node IDs from the tree.${referenceInstruction}${roundInstruction}`

  try {
    const response = await fetch('/api/ai/validate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        system: VALIDATION_SYSTEM_PROMPT,
        message,
        imageBase64,
        model,
        provider,
      }),
      signal: controller.signal,
    })

    if (!response.ok) {
      console.warn(`[Validation] HTTP ${response.status}: ${response.statusText}`)
      return { issues: [], fixes: [], qualityScore: 0, skipped: true }
    }

    const data = await response.json() as { text?: string; skipped?: boolean; error?: string }

    if (data.skipped || data.error || !data.text) {
      console.warn(`[Validation] Server response:`, {
        skipped: data.skipped, error: data.error, hasText: !!data.text,
        provider, model,
      })
      return { issues: [], fixes: [], qualityScore: 0, skipped: true }
    }

    const parsed = parseValidationResponse(data.text)
    if (parsed.qualityScore === 0) {
      console.warn(`[Validation] qualityScore=0, raw response (first 500 chars):`, data.text.slice(0, 500))
    }
    return parsed
  } catch (err) {
    console.warn(`[Validation] Fetch error:`, err)
    return { issues: [], fixes: [], qualityScore: 0, skipped: true }
  } finally {
    clearTimeout(timeout)
  }
}

const VALID_HEX_COLOR = /^#(?:[0-9a-fA-F]{3,4}|[0-9a-fA-F]{6}|[0-9a-fA-F]{8})$/
const VALID_FONT_WEIGHTS = new Set([100, 200, 300, 400, 500, 600, 700, 800, 900])
const VALID_TEXT_ALIGN = new Set(['left', 'center', 'right'])

function isValidFixValue(property: string, value: unknown): boolean {
  const type = SAFE_FIX_PROPERTIES[property]
  if (!type) return false

  switch (type) {
    case 'number':
      return typeof value === 'number'
    case 'sizing':
      return typeof value === 'number' || (typeof value === 'string' && VALID_SIZING_STRINGS.has(value))
    case 'number_or_array':
      return typeof value === 'number' || (Array.isArray(value) && value.every((v) => typeof v === 'number'))
    case 'enum_align':
      return typeof value === 'string' && VALID_ALIGN.has(value)
    case 'enum_justify':
      return typeof value === 'string' && VALID_JUSTIFY.has(value)
    case 'color':
      return typeof value === 'string' && VALID_HEX_COLOR.test(value)
    case 'font_weight':
      return typeof value === 'number' && VALID_FONT_WEIGHTS.has(value)
    case 'enum_text_align':
      return typeof value === 'string' && VALID_TEXT_ALIGN.has(value)
    default:
      return false
  }
}

function parseValidationResponse(text: string): ValidationResult {
  const tryParse = (json: string): ValidationResult | null => {
    try {
      const parsed = JSON.parse(json) as ValidationResult
      if (!Array.isArray(parsed.fixes)) return null
      parsed.fixes = parsed.fixes.filter(
        (f) => f.nodeId && f.property in SAFE_FIX_PROPERTIES && isValidFixValue(f.property, f.value),
      )
      const rawScore = parsed.qualityScore
      const numScore = typeof rawScore === 'number' ? rawScore
        : typeof rawScore === 'string' ? Number(rawScore)
        : 0
      parsed.qualityScore = numScore > 0
        ? Math.max(1, Math.min(10, Math.round(numScore)))
        : 0
      return parsed
    } catch {
      return null
    }
  }

  // Strip Agent SDK tool_use XML blocks that may precede the JSON response.
  // The Agent SDK sometimes includes raw tool call XML (e.g. <tool_use>...<input>{...}</input></tool_use>)
  // which confuses the JSON extraction regex.
  const cleaned = text.replace(/<tool_use>[\s\S]*?<\/tool_use>/g, '').trim()

  // Try direct parse
  const direct = tryParse(cleaned)
  if (direct) return direct

  // Try extracting JSON from text
  const match = cleaned.match(/\{[\s\S]*\}/)
  if (match) {
    const extracted = tryParse(match[0])
    if (extracted) return extracted
  }

  return { issues: [], fixes: [], qualityScore: 0 }
}

// ---------------------------------------------------------------------------
// Apply fixes
// ---------------------------------------------------------------------------

function applyValidationFixes(result: ValidationResult): number {
  if (result.fixes.length === 0) return 0

  const store = useDocumentStore.getState()
  let applied = 0
  const skipped: string[] = []

  for (const fix of result.fixes) {
    const node = store.getNodeById(fix.nodeId)
    if (!node) {
      skipped.push(`${fix.nodeId} (not found)`)
      continue
    }
    if (!(fix.property in SAFE_FIX_PROPERTIES)) {
      skipped.push(`${fix.nodeId}.${fix.property} (unsupported property)`)
      continue
    }
    if (!isValidFixValue(fix.property, fix.value)) {
      skipped.push(`${fix.nodeId}.${fix.property}=${JSON.stringify(fix.value)} (invalid value)`)
      continue
    }

    const oldValue = (node as unknown as Record<string, unknown>)[fix.property]

    // fillColor is a virtual property — translate to PenFill array
    if (fix.property === 'fillColor' && typeof fix.value === 'string') {
      store.updateNode(fix.nodeId, { fill: [{ type: 'solid', color: fix.value }] })
      console.log(`[Validation Fix] ${fix.nodeId}: fill → ${fix.value}`)
      applied++
      continue
    }

    store.updateNode(fix.nodeId, { [fix.property]: fix.value })
    console.log(`[Validation Fix] ${fix.nodeId}: ${fix.property} ${JSON.stringify(oldValue)} → ${JSON.stringify(fix.value)}`)
    applied++
  }

  if (skipped.length > 0) {
    console.warn(`[Validation] Skipped fixes:`, skipped)
  }

  return applied
}

// ---------------------------------------------------------------------------
// Public orchestration
// ---------------------------------------------------------------------------

export async function runPostGenerationValidation(
  options?: {
    onStatusUpdate?: (status: 'pending' | 'streaming' | 'done' | 'error', message?: string) => void
    model?: string
    provider?: AIProviderType
  },
): Promise<{ applied: number; skipped: boolean }> {
  let totalApplied = 0
  let lastQualityScore = 0

  // Accumulate a log so the final status retains all validation steps
  const log: string[] = []
  function emit(status: 'pending' | 'streaming' | 'done' | 'error', line?: string) {
    if (line) log.push(line)
    options?.onStatusUpdate?.(status, log.join('\n'))
  }

  for (let round = 1; round <= MAX_VALIDATION_ROUNDS; round++) {
    const isFirstRound = round === 1

    emit('streaming',
      isFirstRound ? '📸 Capturing screenshot...' : `📸 Re-capturing screenshot (round ${round})...`,
    )

    // Wait for canvas render to stabilize.
    // After applying fixes (round 2+), the Zustand → canvas sync pipeline needs
    // more time: subscribe fires → flattenNodes → computeLayout → Fabric render.
    // Use a longer delay for subsequent rounds to ensure fixes are rendered.
    if (round > 1) {
      await new Promise<void>((resolve) => setTimeout(resolve, 500))
    }
    await new Promise<void>((resolve) => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => resolve())
      })
    })

    const imageBase64 = captureRootFrameScreenshot()
    if (!imageBase64) {
      console.warn(`[Validation] Round ${round}: could not capture screenshot — stopping`)
      if (isFirstRound) {
        emit('done', '❌ Screenshot failed')
        clearVisualReference()
        return { applied: 0, skipped: true }
      }
      break
    }

    // Replace the "Capturing..." line with success
    log[log.length - 1] = isFirstRound ? '✅ Screenshot captured' : `✅ Screenshot captured (round ${round})`
    emit('streaming')

    const nodeTreeDump = buildNodeTreeDump(DEFAULT_FRAME_ID)
    if (isFirstRound) {
      console.log(`[Validation] Node tree dump:\n${nodeTreeDump}`)
    }

    // Reference comparison only on first round
    const visualRef = isFirstRound ? getCurrentVisualReference() : null
    const hasReference = visualRef?.screenshot && visualRef.screenshot.length > 0

    emit('streaming',
      hasReference && isFirstRound
        ? '🔍 Comparing with design reference...'
        : isFirstRound ? '🔍 Analyzing design...' : `🔍 Analyzing (round ${round})...`,
    )

    const result = await validateDesignScreenshot(
      imageBase64,
      nodeTreeDump,
      options?.model,
      options?.provider,
      hasReference ? visualRef!.screenshot : undefined,
      round,
    )

    if (result.skipped) {
      console.log(`[Validation] Round ${round}: skipped (see warnings above for details; provider=${options?.provider}, model=${options?.model})`)
      // Replace "Analyzing..." with skipped reason
      log[log.length - 1] = '⚠️ Analysis skipped (timeout or provider error)'
      if (isFirstRound) {
        clearVisualReference()
        emit('done')
        return { applied: 0, skipped: true }
      }
      emit('streaming')
      break
    }

    lastQualityScore = result.qualityScore

    // Replace "Analyzing..." with result
    const scoreLabel = result.qualityScore > 0 ? ` (quality: ${result.qualityScore}/10)` : ''
    if (result.issues.length > 0) {
      log[log.length - 1] = `🔍 Found ${result.issues.length} issue${result.issues.length > 1 ? 's' : ''}${scoreLabel}`
      console.log(`[Validation] Round ${round}: issues found:`, result.issues)
    } else {
      log[log.length - 1] = `✅ No issues found${scoreLabel}`
    }
    emit('streaming')

    // Quality threshold reached — design is good enough
    if (result.qualityScore >= VALIDATION_QUALITY_THRESHOLD) {
      console.log(`[Validation] Round ${round}: quality ${result.qualityScore}/10 >= threshold, stopping`)
      break
    }

    if (result.fixes.length === 0) {
      console.log(`[Validation] Round ${round}: no fixes needed`)
      break
    }

    emit('streaming', `🔧 Applying ${result.fixes.length} fix${result.fixes.length > 1 ? 'es' : ''}...`)

    const applied = applyValidationFixes(result)
    totalApplied += applied
    console.log(`[Validation] Round ${round}: applied ${applied} fixes (quality: ${result.qualityScore}/10):`, result.fixes)

    // Replace "Applying..." with result
    if (applied > 0) {
      log[log.length - 1] = `✅ Applied ${applied} fix${applied > 1 ? 'es' : ''}`
    } else {
      log[log.length - 1] = '⚠️ No fixes could be applied'
      console.log(`[Validation] Round ${round}: no fixes could be applied, stopping`)
      break
    }
    emit('streaming')
  }

  // Cleanup visual reference after all rounds
  clearVisualReference()

  // Final summary line
  const qualityInfo = lastQualityScore > 0 ? ` — quality: ${lastQualityScore}/10` : ''
  if (totalApplied > 0) {
    emit('done', `✨ Done: ${totalApplied} fix${totalApplied > 1 ? 'es' : ''} applied${qualityInfo}`)
  } else if (lastQualityScore > 0) {
    emit('done', `✨ Done: no fixes needed${qualityInfo}`)
  } else {
    emit('done')
  }

  return { applied: totalApplied, skipped: false }
}
