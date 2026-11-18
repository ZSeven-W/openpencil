/**
 * Bounded see-then-fix visual review for the OpenPencil pipeline.
 *
 * The App's built-in design agent looks at a rendered screenshot and fixes
 * what it sees. The DSH pipeline ports the cheapest slice of that loop: one
 * `needs_visual_review` stage between a clean quality gate and atomic
 * publication. The finished PNG rides the existing browser presentation
 * channel, while the model receives a deterministic textual layout digest —
 * the DeepSeek adapter rejects image blocks anywhere in a request, so the
 * digest is the model-facing rendering of the same evidence — plus a fixed
 * review checklist. The model either accepts (calls finish again) or sends
 * exactly one bounded I/K/U correction batch before the final finish.
 * @module dsh-openpencil/design-visual-review
 */

/** Upper bound on I/K/U calls inside the single visual correction batch. */
export const MAX_VISUAL_REVIEW_CALLS = 16
/** Upper bound on the visual correction batch source size. */
export const MAX_VISUAL_REVIEW_BYTES = 6 * 1024

/**
 * The fixed review checklist returned with `needs_visual_review`. Items are
 * phrased as checks over the digest/screenshot rather than open-ended taste
 * so the correction batch stays small and targeted.
 */
export const VISUAL_REVIEW_CHECKLIST: readonly string[] = [
  'Vertical rhythm: section spacing is even; no giant empty band between adjacent regions.',
  'Alignment: content of every region starts on the shared horizontal inset; nothing overflows the canvas width.',
  'Repeated cards: sibling product/category cards share one width mode, media treatment, and text structure.',
  'Media: every visible media slot shows real content; a card whose image collapsed still reads as a complete text card.',
  'Copy fit: no truncated or overlapping text; headline and CTA are the strongest elements above the fold.',
  'Footer: the page ends with the complete footer band (brand, links, copyright) and nothing renders after it.',
]

const MAX_DIGEST_LINES = 30
const MAX_DIGEST_LINE_LENGTH = 160

interface JsonObject {
  [key: string]: unknown
}

function isObject(value: unknown): value is JsonObject {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function nodeChildren(node: JsonObject): JsonObject[] {
  if (!Array.isArray(node.children)) return []
  return node.children.filter(isObject)
}

function dimension(value: unknown): string {
  if (typeof value === 'number' && Number.isFinite(value)) return String(Math.round(value))
  if (typeof value === 'string' && value.length > 0 && value.length <= 16) return value
  return '?'
}

function isHidden(node: JsonObject): boolean {
  return node.visible === false
}

function shortName(node: JsonObject): string {
  const name = typeof node.name === 'string' && node.name.trim().length > 0
    ? node.name.trim()
    : String(node.type ?? 'node')
  return name.length > 40 ? `${name.slice(0, 39)}…` : name
}

interface CardSummary {
  width: string
  hasVisibleImage: boolean
  hasCollapsedImage: boolean
  textCount: number
}

function summarizeCard(card: JsonObject): CardSummary {
  let hasVisibleImage = false
  let hasCollapsedImage = false
  let textCount = 0
  const pending: JsonObject[] = [card]
  let guard = 0
  while (pending.length > 0 && guard < 200) {
    guard += 1
    const node = pending.pop() as JsonObject
    if (node !== card && node.type === 'image') {
      if (isHidden(node)) hasCollapsedImage = true
      else hasVisibleImage = true
    }
    if (node.type === 'text' && !isHidden(node)) textCount += 1
    if (!isHidden(node) || node === card) pending.push(...nodeChildren(node))
  }
  return { width: dimension(card.width), hasVisibleImage, hasCollapsedImage, textCount }
}

function looksLikeCardRail(region: JsonObject, children: JsonObject[]): boolean {
  if (region.layout !== 'horizontal' || children.length < 2) return false
  const frames = children.filter(child => child.type === 'frame' || child.type === 'group')
  return frames.length >= 2 && frames.length === children.length
}

function describeRegion(region: JsonObject, lines: string[]): void {
  const children = nodeChildren(region).filter(child => !isHidden(child))
  const role = typeof region.role === 'string' && region.role.length > 0 ? `/${region.role}` : ''
  lines.push(
    `- ${shortName(region)}${role} ${dimension(region.width)}x${dimension(region.height)}`
    + ` layout:${String(region.layout ?? 'none')} children:${children.length}`,
  )
  // One nested level is enough to expose a card rail hiding inside a section
  // wrapper (Section title + rail is the dominant authored shape).
  const rails = [region, ...children].filter(candidate => (
    looksLikeCardRail(candidate, nodeChildren(candidate).filter(child => !isHidden(child)))
  ))
  for (const rail of rails.slice(0, 2)) {
    const cards = nodeChildren(rail).filter(child => !isHidden(child))
    const summaries = cards.map(summarizeCard)
    const widths = [...new Set(summaries.map(card => card.width))]
    const media = summaries
      .map(card => (card.hasVisibleImage ? 'img' : card.hasCollapsedImage ? 'collapsed' : 'text'))
      .join(',')
    lines.push(
      `  rail ${shortName(rail)}: ${cards.length} cards, widths ${widths.join('/')}, media [${media}]`,
    )
  }
}

/**
 * Deterministic model-facing rendering of the finalized page: one bounded
 * line per top-level region plus card-rail summaries and a footer verdict.
 * Purely structural — derived from the exact document the screenshot shows.
 */
export function buildVisualReviewDigest(documentJson: string): string[] {
  let document: unknown
  try {
    document = JSON.parse(documentJson)
  } catch {
    return ['digest unavailable: the finalized document was not parseable JSON.']
  }
  if (!isObject(document)) return ['digest unavailable: the finalized document was not an object.']
  const roots = Array.isArray(document.children) ? document.children.filter(isObject) : []
  const root = roots[0]
  if (root === undefined) return ['digest unavailable: the document has no root frame.']
  const regions = nodeChildren(root).filter(region => !isHidden(region))
  const lines: string[] = [
    `page ${dimension(root.width)}x${dimension(root.height)} with ${regions.length} visible regions (top to bottom):`,
  ]
  for (const region of regions) {
    if (lines.length >= MAX_DIGEST_LINES - 2) {
      lines.push(`- … ${regions.length - (lines.length - 1)} more regions elided`)
      break
    }
    describeRegion(region, lines)
  }
  const last = regions[regions.length - 1]
  const lastIsFooter = last !== undefined
    && (last.role === 'footer' || /footer|页脚|頁腳/iu.test(String(last.name ?? '')))
  lines.push(lastIsFooter
    ? 'footer: present as the final region.'
    : 'footer: NOT detected as the final region — treat as a defect.')
  return lines.slice(0, MAX_DIGEST_LINES).map(line => (
    line.length > MAX_DIGEST_LINE_LENGTH ? `${line.slice(0, MAX_DIGEST_LINE_LENGTH - 1)}…` : line
  ))
}

/**
 * Validate the single authorized visual-review correction batch. Returns a
 * human-readable violation, or `undefined` when the script is acceptable.
 * The general batch safety checks (wrapper nesting, `I(null, ...)`, binding
 * member mutation) still run in the batch tool itself.
 */
export function visualReviewScriptViolation(script: string): string | undefined {
  if (Buffer.byteLength(script, 'utf8') > MAX_VISUAL_REVIEW_BYTES) {
    return `the visual-review correction batch is limited to ${MAX_VISUAL_REVIEW_BYTES} bytes; send one smaller targeted script`
  }
  if (/\b[CDMRG]\s*\(/u.test(script)) {
    return 'the visual-review correction batch may only use I(parent, node), K(...), and U(nodeId, patch); C/D/M/R/G mutations are not authorized'
  }
  const calls = script.match(/\b[IKU]\s*\(/gu)?.length ?? 0
  if (calls === 0) {
    return 'the visual-review correction batch must contain at least one I/K/U call, or call finish again to accept the preview unchanged'
  }
  if (calls > MAX_VISUAL_REVIEW_CALLS) {
    return `the visual-review correction batch is limited to ${MAX_VISUAL_REVIEW_CALLS} I/K/U calls; fix only what the checklist flagged`
  }
  if (/name\s*:\s*["'](?:Header|Hero)["']/u.test(script)) {
    return 'the visual-review correction batch must not rebuild Header or Hero; patch existing nodes with U(nodeId, patch) instead'
  }
  return undefined
}
