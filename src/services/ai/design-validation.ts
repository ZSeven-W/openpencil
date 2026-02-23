/**
 * Post-generation screenshot validation.
 *
 * After all design sections are generated, captures a screenshot of the root
 * frame and sends it alongside a simplified node tree to the vision API.
 * The LLM correlates visual issues with actual node IDs and returns fixes.
 */

import { useCanvasStore } from '@/stores/canvas-store'
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import { VALIDATION_TIMEOUT_MS } from './ai-runtime-config'
import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'

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
6. MISSING ICONS: Path nodes that rendered as empty/invisible rectangles.

Output ONLY a JSON object. No explanation, no markdown fences.
{"issues":["description1","description2"],"fixes":[{"nodeId":"actual-node-id","property":"width","value":"fill_container"}]}

Allowed properties and value types:
- width: number | "fill_container" | "fit_content"
- height: number | "fill_container" | "fit_content"
- padding: number | [top,right,bottom,left]
- gap: number
- fontSize: number
- cornerRadius: number
- opacity: number
- alignItems: "start" | "center" | "end"
- justifyContent: "start" | "center" | "end" | "space_between"

IMPORTANT:
- Use REAL node IDs from the provided tree — never guess or fabricate IDs.
- For form consistency issues, fix ALL inconsistent siblings, not just one.
- If the design looks correct, return: {"issues":[],"fixes":[]}
- Keep fixes minimal — only fix clear visual bugs, not stylistic preferences.`

// Properties that are safe to auto-fix, with allowed value types
const SAFE_FIX_PROPERTIES: Record<string, 'number' | 'sizing' | 'number_or_array' | 'enum_align' | 'enum_justify'> = {
  width: 'sizing',
  height: 'sizing',
  padding: 'number_or_array',
  gap: 'number',
  fontSize: 'number',
  cornerRadius: 'number',
  opacity: 'number',
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
    if (node.type === 'text' && 'content' in node) {
      const content = (node as { content?: string }).content ?? ''
      props.push(`text="${content.slice(0, 30)}"`)
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
  skipped?: boolean
}

async function validateDesignScreenshot(
  imageBase64: string,
  nodeTreeDump: string,
): Promise<ValidationResult> {
  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), VALIDATION_TIMEOUT_MS)

  const message = `Analyze this UI design screenshot. Here is the node tree structure:

\`\`\`
${nodeTreeDump}
\`\`\`

Cross-reference visual issues with the node IDs above. Return JSON fixes using real node IDs from the tree.`

  try {
    const response = await fetch('/api/ai/validate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        system: VALIDATION_SYSTEM_PROMPT,
        message,
        imageBase64,
      }),
      signal: controller.signal,
    })

    if (!response.ok) {
      return { issues: [], fixes: [], skipped: true }
    }

    const data = await response.json() as { text?: string; skipped?: boolean; error?: string }

    if (data.skipped || data.error || !data.text) {
      return { issues: [], fixes: [], skipped: true }
    }

    return parseValidationResponse(data.text)
  } catch {
    return { issues: [], fixes: [], skipped: true }
  } finally {
    clearTimeout(timeout)
  }
}

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
      return parsed
    } catch {
      return null
    }
  }

  // Try direct parse
  const direct = tryParse(text.trim())
  if (direct) return direct

  // Try extracting JSON from text
  const match = text.match(/\{[\s\S]*\}/)
  if (match) {
    const extracted = tryParse(match[0])
    if (extracted) return extracted
  }

  return { issues: [], fixes: [] }
}

// ---------------------------------------------------------------------------
// Apply fixes
// ---------------------------------------------------------------------------

function applyValidationFixes(result: ValidationResult): number {
  if (result.fixes.length === 0) return 0

  const store = useDocumentStore.getState()
  let applied = 0

  for (const fix of result.fixes) {
    const node = store.getNodeById(fix.nodeId)
    if (!node) continue
    if (!(fix.property in SAFE_FIX_PROPERTIES)) continue
    if (!isValidFixValue(fix.property, fix.value)) continue

    store.updateNode(fix.nodeId, { [fix.property]: fix.value })
    applied++
  }

  return applied
}

// ---------------------------------------------------------------------------
// Public orchestration
// ---------------------------------------------------------------------------

export async function runPostGenerationValidation(
  callbacks?: {
    onStatusUpdate?: (status: 'pending' | 'streaming' | 'done' | 'error', message?: string) => void
  },
): Promise<{ applied: number; skipped: boolean }> {
  callbacks?.onStatusUpdate?.('streaming', 'Capturing screenshot...')

  // Wait for canvas render to stabilize
  await new Promise<void>((resolve) => {
    requestAnimationFrame(() => {
      requestAnimationFrame(() => resolve())
    })
  })

  const imageBase64 = captureRootFrameScreenshot()
  if (!imageBase64) {
    console.warn('[Validation] Could not capture screenshot — skipping')
    callbacks?.onStatusUpdate?.('done', 'Skipped (no screenshot)')
    return { applied: 0, skipped: true }
  }

  // Build simplified node tree for LLM context
  const nodeTreeDump = buildNodeTreeDump(DEFAULT_FRAME_ID)

  callbacks?.onStatusUpdate?.('streaming', 'Analyzing design...')
  const result = await validateDesignScreenshot(imageBase64, nodeTreeDump)

  if (result.skipped) {
    console.log('[Validation] Skipped (provider unsupported)')
    callbacks?.onStatusUpdate?.('done', 'Skipped')
    return { applied: 0, skipped: true }
  }

  if (result.issues.length > 0) {
    console.log('[Validation] Issues found:', result.issues)
  }

  if (result.fixes.length === 0) {
    console.log('[Validation] No fixes needed')
    callbacks?.onStatusUpdate?.('done', 'No issues found')
    return { applied: 0, skipped: false }
  }

  callbacks?.onStatusUpdate?.('streaming', `Applying ${result.fixes.length} fixes...`)
  const applied = applyValidationFixes(result)
  console.log(`[Validation] Applied ${applied} fixes:`, result.fixes)
  callbacks?.onStatusUpdate?.('done', `Applied ${applied} fixes`)
  return { applied, skipped: false }
}
