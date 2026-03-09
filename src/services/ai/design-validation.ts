/**
 * Post-generation screenshot validation.
 *
 * After all design sections are generated, captures a screenshot of the root
 * frame and sends it alongside a simplified node tree to the vision API.
 * The LLM correlates visual issues with actual node IDs and returns fixes.
 */

import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import { VALIDATION_ENABLED, VALIDATION_TIMEOUT_MS, MAX_VALIDATION_ROUNDS, VALIDATION_QUALITY_THRESHOLD } from './ai-runtime-config'
import type { PenNode } from '@/types/pen'
import type { AIProviderType } from '@/types/agent-settings'
import { getCurrentVisualReference, clearVisualReference } from './visual-ref-orchestrator'
import { runPreValidationFixes } from './design-pre-validation'
import { captureRootFrameScreenshot } from './design-screenshot'
import {
  applyValidationFixes,
  isValidFixValue,
  isValidStructuralFix,
  SAFE_FIX_PROPERTIES,
  type ValidationResult,
} from './design-validation-fixes'

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
- textGrowth: "auto" | "fixed-width" | "fixed-width-height" (text wrapping mode — "fixed-width" = wrap text and auto-size height)
- alignItems: "start" | "center" | "end"
- justifyContent: "start" | "center" | "end" | "space_between"

TEXT CLIPPING DETECTION:
- If a text node has an explicit pixel height (h=22, h=30 etc.) AND its content appears visually clipped or overlapping siblings, the fix is: set textGrowth="fixed-width" and height="fit_content". This lets the engine auto-calculate the correct height.
- Text nodes should almost NEVER have explicit pixel heights. The node tree shows textGrowth and lineHeight values — use these to diagnose text issues.
- Button text clipped at bottom: check if the parent frame's padding leaves enough space for the text height (fontSize × lineHeight). Fix the parent's padding or height, not the text's fontSize.

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
- CRITICAL: When using addChild, ALWAYS include companion property fixes for the parent node to maintain correct layout. For example, if the parent has justifyContent="space_between" and adding a child would break the spacing, also add a property fix to change justifyContent and/or add a gap value. Look at sibling elements with the same pattern and match the parent's layout properties to theirs.
- CRITICAL: NEVER change height or width from "fit_content" to a fixed pixel value on a frame that has layout (auto-layout). This creates empty whitespace. If a container appears invisible, fix its opacity, fill color, or border instead — not its height.`


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
    if ('stroke' in node && node.stroke) {
      const s = node.stroke as { thickness?: number | number[]; fill?: Array<{ color?: string }> }
      const strokeColor = s.fill?.[0]?.color
      const strokeW = typeof s.thickness === 'number' ? s.thickness : (Array.isArray(s.thickness) ? s.thickness[0] : 0)
      if (strokeColor) props.push(`stroke="${strokeColor}" strokeW=${strokeW ?? 0}`)
    }
    if (node.type === 'text') {
      if ('fontSize' in node && node.fontSize) props.push(`fontSize=${node.fontSize}`)
      if ('fontWeight' in node && node.fontWeight) props.push(`fontWeight=${node.fontWeight}`)
      if ('lineHeight' in node && node.lineHeight) props.push(`lineHeight=${node.lineHeight}`)
      if ('textGrowth' in node && node.textGrowth) props.push(`textGrowth=${node.textGrowth}`)
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
// Validation API call
// ---------------------------------------------------------------------------


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
  const fixHistory = new Map<string, number>() // "nodeId:property" → round count

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

  // If LLM validation is disabled, stop after pre-checks
  if (!VALIDATION_ENABLED) {
    clearVisualReference()
    emit('done', preFixCount > 0
      ? `[done] Pre-checks: fixed ${preFixCount} issue${preFixCount > 1 ? 's' : ''}`
      : '[done] Pre-checks complete')
    return { applied: totalApplied, skipped: false }
  }

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
    if (result.qualityScore === 0 && result.issues.length === 0) {
      // Parsing failed — not a clean pass
      log[log.length - 1] = `[error] Analysis incomplete (round ${round})`
      console.warn(`[Validation] Round ${round}: qualityScore=0 with no issues — likely parse failure`)
      break
    } else if (result.issues.length > 0) {
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

    // Track fixes for repeated-fix detection
    for (const f of result.fixes) {
      const key = `${f.nodeId}:${f.property}`
      fixHistory.set(key, (fixHistory.get(key) ?? 0) + 1)
    }

    // Filter out repeated fixes that already failed in previous rounds
    if (round > 1) {
      const preFilterLen = result.fixes.length
      result.fixes = result.fixes.filter(f => (fixHistory.get(`${f.nodeId}:${f.property}`) ?? 0) <= 1)
      if (result.fixes.length < preFilterLen) {
        console.log(`[Validation] Round ${round}: filtered ${preFilterLen - result.fixes.length} repeated fix(es)`)
      }
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
