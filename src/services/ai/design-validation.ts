/**
 * Post-generation screenshot validation.
 *
 * After all design sections are generated, captures a screenshot of the root
 * frame and sends it alongside a simplified node tree to the vision API.
 * The LLM correlates visual issues with actual node IDs and returns fixes.
 */

import { nanoid } from 'nanoid'
import { useCanvasStore } from '@/stores/canvas-store'
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import { VALIDATION_TIMEOUT_MS, MAX_VALIDATION_ROUNDS, VALIDATION_QUALITY_THRESHOLD } from './ai-runtime-config'
import type { PenNode } from '@/types/pen'
import type { FabricObjectWithPenId } from '@/canvas/canvas-object-factory'
import type { AIProviderType } from '@/types/agent-settings'
import { getCurrentVisualReference, clearVisualReference } from './visual-ref-orchestrator'
import { lookupIconByName } from './icon-resolver'
import { runPreValidationFixes } from './design-pre-validation'

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
10. MISSING BORDERS: Input fields, cards, or containers that lack a visible border and blend into their parent background. Fix with strokeColor and strokeWidth.
11. STRUCTURAL INCONSISTENCY: Sibling elements that should follow the same pattern but have different child structures. For example, if one input field has a leading icon but a sibling input field does not, or a list item is missing an expected child element. Fix by adding the missing child node.
12. MISSING ELEMENTS: When a reference design is provided, check if important UI elements visible in the reference are missing or absent in the current design. Fix by adding the missing element as a child of the appropriate parent.

Output ONLY a JSON object. No explanation, no markdown fences.
{"qualityScore":8,"issues":["description1","description2"],"fixes":[{"nodeId":"actual-node-id","property":"width","value":"fill_container"}],"structuralFixes":[]}

qualityScore: Rate the overall design quality from 1-10.
- 9-10: Production-ready, polished design
- 7-8: Good design with minor issues
- 5-6: Acceptable but needs improvement
- 1-4: Significant problems

Allowed property fixes (update existing node):
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
- strokeColor: "#hex" (border/stroke color)
- strokeWidth: number (border/stroke width)
- textAlign: "left" | "center" | "right" (text horizontal alignment within its box)
- alignItems: "start" | "center" | "end"
- justifyContent: "start" | "center" | "end" | "space_between"

Structural fixes (add or remove nodes — use sparingly, only for clear structural issues):
- Add child: {"action":"addChild","parentId":"real-parent-id","index":0,"node":{"type":"path","name":"KeyIcon","width":18,"height":18}}
- Add child: {"action":"addChild","parentId":"real-parent-id","node":{"type":"text","name":"Label","content":"text","fontSize":14,"fillColor":"#hex"}}
- Add child: {"action":"addChild","parentId":"real-parent-id","node":{"type":"frame","name":"Divider","width":"fill_container","height":1,"fillColor":"#hex"}}
- Remove node: {"action":"removeNode","nodeId":"real-node-id"}

For addChild nodes:
- type: "frame" | "text" | "path" | "rectangle" | "ellipse"
- For path/icon nodes: set name to the icon name (e.g. "KeyIcon", "LockIcon", "EyeIcon"). The system resolves icon paths automatically.
- index is optional (defaults to 0 = first child). Use it to control insertion position among siblings.
- Specify width, height, fillColor as needed. Other properties are optional.

IMPORTANT:
- Use REAL node IDs from the provided tree — never guess or fabricate IDs.
- For form consistency issues, fix ALL inconsistent siblings, not just one.
- If the design looks correct, return: {"qualityScore":9,"issues":[],"fixes":[],"structuralFixes":[]}
- Keep fixes minimal — only fix clear visual bugs, not stylistic preferences.
- Focus on the most impactful issues first.
- For structuralFixes, only add elements that are clearly needed for consistency or completeness. Do not add decorative elements unless they are present in the reference.
- CRITICAL: When using addChild, ALWAYS include companion property fixes for the parent node to maintain correct layout. For example, if the parent has justifyContent="space_between" and adding a child would break the spacing, also add a property fix to change justifyContent and/or add a gap value. Look at sibling elements with the same pattern and match the parent's layout properties to theirs.`

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
  strokeColor: 'color',
  strokeWidth: 'number',
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

interface StructuralAddChildFix {
  action: 'addChild'
  parentId: string
  index?: number
  node: {
    type: 'frame' | 'text' | 'path' | 'rectangle' | 'ellipse'
    name?: string
    width?: number | string
    height?: number | string
    fillColor?: string
    content?: string
    fontSize?: number
    fontWeight?: number
    layout?: string
    gap?: number
    padding?: number | number[]
    cornerRadius?: number
    alignItems?: string
    justifyContent?: string
  }
}

interface StructuralRemoveNodeFix {
  action: 'removeNode'
  nodeId: string
}

type StructuralFix = StructuralAddChildFix | StructuralRemoveNodeFix

interface ValidationResult {
  issues: string[]
  fixes: ValidationFix[]
  structuralFixes: StructuralFix[]
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
    ? `\n\nA REFERENCE DESIGN screenshot was also provided. Compare the current design against the reference and fix any significant deviations in layout, spacing, proportions, or missing elements. The reference shows the intended design — the current screenshot should match its structure, visual balance, and element completeness. If elements visible in the reference are missing in the current design, use structuralFixes with addChild to add them.`
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
      return { issues: [], fixes: [], structuralFixes: [], qualityScore: 0, skipped: true }
    }

    const data = await response.json() as { text?: string; skipped?: boolean; error?: string }

    if (data.skipped || data.error || !data.text) {
      console.warn(`[Validation] Server response:`, {
        skipped: data.skipped, error: data.error, hasText: !!data.text,
        provider, model,
      })
      return { issues: [], fixes: [], structuralFixes: [], qualityScore: 0, skipped: true }
    }

    const parsed = parseValidationResponse(data.text)
    if (parsed.qualityScore === 0) {
      console.warn(`[Validation] qualityScore=0, raw response (first 500 chars):`, data.text.slice(0, 500))
    }
    return parsed
  } catch (err) {
    console.warn(`[Validation] Fetch error:`, err)
    return { issues: [], fixes: [], structuralFixes: [], qualityScore: 0, skipped: true }
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

function isValidStructuralFix(fix: unknown): fix is StructuralFix {
  if (!fix || typeof fix !== 'object') return false
  const f = fix as Record<string, unknown>
  if (f.action === 'addChild') {
    if (typeof f.parentId !== 'string' || !f.parentId) return false
    if (!f.node || typeof f.node !== 'object') return false
    const node = f.node as Record<string, unknown>
    const validTypes = new Set(['frame', 'text', 'path', 'rectangle', 'ellipse'])
    return typeof node.type === 'string' && validTypes.has(node.type)
  }
  if (f.action === 'removeNode') {
    return typeof f.nodeId === 'string' && !!f.nodeId
  }
  return false
}

function parseValidationResponse(text: string): ValidationResult {
  const tryParse = (json: string): ValidationResult | null => {
    try {
      const parsed = JSON.parse(json) as ValidationResult
      if (!Array.isArray(parsed.fixes)) return null
      parsed.fixes = parsed.fixes.filter(
        (f) => f.nodeId && f.property in SAFE_FIX_PROPERTIES && isValidFixValue(f.property, f.value),
      )
      // Parse and validate structural fixes
      parsed.structuralFixes = Array.isArray(parsed.structuralFixes)
        ? parsed.structuralFixes.filter(isValidStructuralFix)
        : []
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

  return { issues: [], fixes: [], structuralFixes: [], qualityScore: 0 }
}

// ---------------------------------------------------------------------------
// Apply fixes
// ---------------------------------------------------------------------------

async function applyValidationFixes(result: ValidationResult): Promise<number> {
  const hasFixes = result.fixes.length > 0
  const hasStructural = result.structuralFixes.length > 0
  if (!hasFixes && !hasStructural) return 0

  const store = useDocumentStore.getState()
  let applied = 0
  const skipped: string[] = []

  // --- Property fixes ---
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

    // strokeColor — translate to PenStroke
    if (fix.property === 'strokeColor' && typeof fix.value === 'string') {
      const existingNode = store.getNodeById(fix.nodeId)
      const existingStroke = existingNode && 'stroke' in existingNode ? existingNode.stroke : undefined
      const thickness = existingStroke && 'thickness' in existingStroke ? (existingStroke as { thickness?: number }).thickness ?? 1 : 1
      store.updateNode(fix.nodeId, {
        stroke: { thickness, fill: [{ type: 'solid', color: fix.value }] },
      })
      console.log(`[Validation Fix] ${fix.nodeId}: strokeColor → ${fix.value}`)
      applied++
      continue
    }

    // strokeWidth — update thickness in existing stroke or create new stroke
    if (fix.property === 'strokeWidth' && typeof fix.value === 'number') {
      const existingNode = store.getNodeById(fix.nodeId)
      const existingStroke = existingNode && 'stroke' in existingNode ? existingNode.stroke : undefined
      const color = existingStroke && 'fill' in (existingStroke as object)
        ? ((existingStroke as { fill?: Array<{ color?: string }> }).fill?.[0]?.color ?? '#CBD5E1')
        : '#CBD5E1'
      store.updateNode(fix.nodeId, {
        stroke: { thickness: fix.value, fill: [{ type: 'solid', color }] },
      })
      console.log(`[Validation Fix] ${fix.nodeId}: strokeWidth → ${fix.value}`)
      applied++
      continue
    }

    store.updateNode(fix.nodeId, { [fix.property]: fix.value })
    console.log(`[Validation Fix] ${fix.nodeId}: ${fix.property} ${JSON.stringify(oldValue)} → ${JSON.stringify(fix.value)}`)
    applied++
  }

  // --- Structural fixes ---
  for (const sf of result.structuralFixes) {
    if (sf.action === 'addChild') {
      const parent = store.getNodeById(sf.parentId)
      if (!parent) {
        skipped.push(`addChild: parent ${sf.parentId} not found`)
        continue
      }
      const newNode = await buildNodeFromSpec(sf.node)
      if (!newNode) {
        skipped.push(`addChild: could not build node for ${sf.node.type}:${sf.node.name ?? '?'}`)
        continue
      }
      store.addNode(sf.parentId, newNode, sf.index)
      console.log(`[Validation Fix] addChild: ${newNode.type}:${newNode.name ?? newNode.id} → parent ${sf.parentId} at index ${sf.index ?? 0}`)
      applied++

      // Auto-fix parent layout if addChild might break it.
      // When adding a child to a space_between parent, look for a sibling
      // with the same pattern and copy its layout properties.
      autoFixParentLayoutAfterAddChild(store, sf.parentId, parent)
    } else if (sf.action === 'removeNode') {
      const node = store.getNodeById(sf.nodeId)
      if (!node) {
        skipped.push(`removeNode: ${sf.nodeId} not found`)
        continue
      }
      store.removeNode(sf.nodeId)
      console.log(`[Validation Fix] removeNode: ${sf.nodeId}`)
      applied++
    }
  }

  if (skipped.length > 0) {
    console.warn(`[Validation] Skipped fixes:`, skipped)
  }

  return applied
}

/**
 * Build a PenNode from a structural fix spec.
 * For path nodes with icon-like names, resolves via the local icon registry.
 */
async function buildNodeFromSpec(
  spec: StructuralAddChildFix['node'],
): Promise<PenNode | null> {
  const id = nanoid(8)
  const node: Record<string, unknown> = {
    id,
    type: spec.type,
    name: spec.name,
  }

  // Dimensions
  if (spec.width != null) node.width = spec.width
  if (spec.height != null) node.height = spec.height
  if (spec.cornerRadius != null) node.cornerRadius = spec.cornerRadius
  if (spec.layout) node.layout = spec.layout
  if (spec.gap != null) node.gap = spec.gap
  if (spec.padding != null) node.padding = spec.padding
  if (spec.alignItems) node.alignItems = spec.alignItems
  if (spec.justifyContent) node.justifyContent = spec.justifyContent

  // Fill color
  if (spec.fillColor && VALID_HEX_COLOR.test(spec.fillColor)) {
    node.fill = [{ type: 'solid', color: spec.fillColor }]
  }

  // Text-specific
  if (spec.type === 'text') {
    if (spec.content) node.content = spec.content
    if (spec.fontSize) node.fontSize = spec.fontSize
    if (spec.fontWeight) node.fontWeight = spec.fontWeight
  }

  // Path/icon — resolve icon path data
  if (spec.type === 'path' && spec.name) {
    const color = spec.fillColor && VALID_HEX_COLOR.test(spec.fillColor) ? spec.fillColor : '#64748B'
    const icon = lookupIconByName(spec.name)
    if (icon) {
      node.d = icon.d
      node.iconId = icon.iconId
      if (icon.style === 'stroke') {
        node.stroke = { thickness: 2, fill: [{ type: 'solid', color }] }
        node.fill = []
      } else {
        node.fill = [{ type: 'solid', color }]
      }
    } else {
      // Try server-side resolution
      try {
        const res = await fetch(`/api/ai/icon?name=${encodeURIComponent(spec.name)}`)
        if (res.ok) {
          const data = await res.json()
          if (data.icon) {
            node.d = data.icon.d
            node.iconId = data.icon.iconId
            if (data.icon.style === 'stroke') {
              node.stroke = { thickness: 2, fill: [{ type: 'solid', color }] }
              node.fill = []
            } else {
              node.fill = [{ type: 'solid', color }]
            }
          }
        }
      } catch {
        console.warn(`[Validation] Icon resolution failed for ${spec.name}`)
      }
    }
    // Default dimensions for icons if not specified
    if (!spec.width) node.width = 18
    if (!spec.height) node.height = 18
  }

  return node as unknown as PenNode
}

/**
 * After adding a child, auto-fix parent layout if needed.
 * Searches the tree for structurally equivalent nodes (same type, layout,
 * similar name pattern) and copies their layout properties.
 */
function autoFixParentLayoutAfterAddChild(
  store: ReturnType<typeof useDocumentStore.getState>,
  parentId: string,
  parentBeforeAdd: PenNode,
): void {
  const parentNode = parentBeforeAdd as unknown as Record<string, unknown>
  const justify = parentNode.justifyContent as string | undefined
  if (!justify || justify === 'start') return // already fine

  // Search the full flat node list for a structural equivalent:
  // same type, same layout direction, similar name pattern, but different justify
  const flatNodes = store.getFlatNodes()
  const parentNameBase = extractNameBase(parentBeforeAdd.name ?? '')

  // Get current parent (after child was added) to compare child counts
  const currentParent = store.getNodeById(parentId)
  const currentChildCount = currentParent && 'children' in currentParent
    ? (currentParent.children?.length ?? 0)
    : 0

  for (const candidate of flatNodes) {
    if (candidate.id === parentId) continue
    if (candidate.type !== parentBeforeAdd.type) continue

    const cand = candidate as unknown as Record<string, unknown>
    if (cand.layout !== parentNode.layout) continue

    // Check name similarity (e.g. "Email Input" vs "Password Input" share "Input")
    const candNameBase = extractNameBase(candidate.name ?? '')
    if (!parentNameBase || !candNameBase || parentNameBase !== candNameBase) continue

    // Skip if the target parent now has more children than the candidate.
    // The candidate's layout was designed for fewer children and may not
    // be appropriate (e.g. copying "start" from a 2-child sibling would
    // break a 3-child node that needs "space_between" for a trailing icon).
    const candChildCount = 'children' in candidate
      ? ((candidate as { children?: unknown[] }).children?.length ?? 0)
      : 0
    if (currentChildCount > candChildCount) {
      console.log(`[Validation Fix] autoFixParentLayout: skipped ${parentId} — has ${currentChildCount} children vs candidate ${candidate.id} with ${candChildCount}`)
      return
    }

    // Found a structural equivalent with same or more children — copy its justify and gap
    const candJustify = cand.justifyContent as string | undefined
    const candGap = cand.gap as number | undefined

    const updates: Record<string, unknown> = {}
    if ((candJustify ?? 'start') !== justify) {
      updates.justifyContent = candJustify ?? 'start'
    }
    if (candGap != null && candGap !== parentNode.gap) {
      updates.gap = candGap
    }

    if (Object.keys(updates).length > 0) {
      store.updateNode(parentId, updates)
      console.log(`[Validation Fix] autoFixParentLayout: ${parentId} matched ${candidate.id} →`, updates)
    }
    return
  }
}

/** Extract the last word of a node name as a structural key (e.g. "Email Input" → "input") */
function extractNameBase(name: string): string {
  const words = name.trim().toLowerCase().split(/\s+/)
  return words.length > 0 ? words[words.length - 1] : ''
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

  // Pre-validation: pure code checks (no LLM needed)
  emit('streaming', '[pending] Running pre-checks...')
  const preFixCount = runPreValidationFixes()
  if (preFixCount > 0) {
    totalApplied += preFixCount
    log[log.length - 1] = `[done] Pre-checks: fixed ${preFixCount} issue${preFixCount > 1 ? 's' : ''}`
  } else {
    log[log.length - 1] = '[done] Pre-checks: OK'
  }
  emit('streaming')

  for (let round = 1; round <= MAX_VALIDATION_ROUNDS; round++) {
    const isFirstRound = round === 1

    emit('streaming',
      isFirstRound ? '[pending] Capturing screenshot...' : `[pending] Re-capturing screenshot (round ${round})...`,
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
        emit('done', '[error] Screenshot failed')
        clearVisualReference()
        return { applied: 0, skipped: true }
      }
      break
    }

    // Replace the "Capturing..." line with success
    log[log.length - 1] = isFirstRound ? '[done] Screenshot captured' : `[done] Screenshot captured (round ${round})`
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
        ? '[pending] Comparing with design reference...'
        : isFirstRound ? '[pending] Analyzing design...' : `[pending] Analyzing (round ${round})...`,
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
      log[log.length - 1] = '[error] Analysis skipped (timeout or provider error)'
      if (isFirstRound) {
        clearVisualReference()
        emit('done')
        return { applied: 0, skipped: true }
      }
      emit('streaming')
      break
    }

    if (result.qualityScore > 0) {
      lastQualityScore = result.qualityScore
    }

    // Replace "Analyzing..." with result
    const scoreLabel = result.qualityScore > 0 ? ` (quality: ${result.qualityScore}/10)` : ''
    if (result.issues.length > 0) {
      log[log.length - 1] = `[done] Found ${result.issues.length} issue${result.issues.length > 1 ? 's' : ''}${scoreLabel}`
      console.log(`[Validation] Round ${round}: issues found:`, result.issues)
    } else {
      log[log.length - 1] = `[done] No issues found${scoreLabel}`
    }
    emit('streaming')

    // Quality threshold reached — design is good enough
    if (result.qualityScore >= VALIDATION_QUALITY_THRESHOLD) {
      console.log(`[Validation] Round ${round}: quality ${result.qualityScore}/10 >= threshold, stopping`)
      break
    }

    if (result.fixes.length === 0 && result.structuralFixes.length === 0) {
      console.log(`[Validation] Round ${round}: no fixes needed`)
      break
    }

    const totalFixCount = result.fixes.length + result.structuralFixes.length
    emit('streaming', `[pending] Applying ${totalFixCount} fix${totalFixCount > 1 ? 'es' : ''}...`)

    const applied = await applyValidationFixes(result)
    totalApplied += applied
    console.log(`[Validation] Round ${round}: applied ${applied} fixes (quality: ${result.qualityScore}/10):`, result.fixes, result.structuralFixes)

    // Replace "Applying..." with result
    if (applied > 0) {
      log[log.length - 1] = `[done] Applied ${applied} fix${applied > 1 ? 'es' : ''}`
    } else {
      log[log.length - 1] = '[error] No fixes could be applied'
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
    emit('done', `[done] Done: ${totalApplied} fix${totalApplied > 1 ? 'es' : ''} applied${qualityInfo}`)
  } else if (lastQualityScore > 0) {
    emit('done', `[done] Done: no fixes needed${qualityInfo}`)
  } else {
    emit('done')
  }

  return { applied: totalApplied, skipped: false }
}
