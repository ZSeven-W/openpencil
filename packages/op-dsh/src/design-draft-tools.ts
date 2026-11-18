/** Model-facing orchestration for OpenPencil's complete, unpublished design pipeline. */

import { createHash, randomBytes } from 'node:crypto'
import { copyFile, lstat, mkdir, readFile, rm, writeFile } from 'node:fs/promises'
import { basename, dirname, extname, isAbsolute, join } from 'node:path'
import type FileSystem from '@deepseek-ai/dsh-fs'
import type { FsObservation, FsTarget } from '@deepseek-ai/dsh-fs'
import type SandboxPolicyService from '@deepseek-ai/dsh-sandbox-policy'
import { defineTool, type ToolRunContext } from '@deepseek-ai/dsh-tools'
import type { JsonValue } from './json-value.js'
import {
  inspectGeneratedDraftStructureReport,
  inspectGeneratedDesignQualityReport,
  type GeneratedDesignQualityRepairTarget,
  type GeneratedDesignQualityReport,
} from './design-quality.js'
import {
  selectDomainGuidance,
  selectStyleGuide,
  type SelectedStyleGuide,
} from './design-knowledge.js'
import {
  VISUAL_REVIEW_CHECKLIST,
  buildVisualReviewDigest,
  visualReviewScriptViolation,
} from './design-visual-review.js'
import type {
  DesignDraftCallResult,
  DesignDraftController,
  DesignDraftScreenshot,
  DesignDraftSnapshot,
} from './design-draft-controller.js'
import type { EditorHostController } from './editor-host.js'
import {
  type DocumentSnapshot,
  RenderAccessController,
  createRenderOutput,
  createDocumentSnapshotFromText,
  projectImageArtifactGrant,
  projectDocumentGrant,
  renderDir,
  stateRoot,
  verifyRenderOutput,
  type RenderFrame,
} from './renderer.js'
import {
  OPENPENCIL_PIPELINE_ABORT_TOOL_NAME,
  OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME,
  OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
  OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
  OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME,
  OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
  OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
} from './tool-names.js'
import {
  AGENT_RUN_PROVIDER_GUIDANCE,
  isProviderConfigurationError,
  parsePipelineEngine,
  summarizeAgentRun,
  validateAgentRunMaxTurns,
  validateAgentRunTimeoutSeconds,
} from './design-agent-run.js'
import type { PipelineEngine } from './design-agent-run.js'
import {
  asSelectedStyleGuide,
  loadInstalledStyleCatalog,
  selectedStyleOf,
  type SelectedStyleSection,
  type StyleCatalogGuide,
  type StyleCatalogSnapshot,
} from './style-catalog.js'
import {
  createStylePlannerTools,
  createStyleSelectionStore,
  type StyleRecommendationProvider,
} from './style-planner.js'

const MAX_BRIEF_LENGTH = 64 * 1024
const MAX_BATCH_LENGTH = 256 * 1024
const MAX_FIRST_GENERATION_BYTES = 8 * 1024
const MAX_FIRST_GENERATION_CALLS = 32
const VERIFIED_LUCIDE_ICONS = 'home/search/shopping-bag/shopping-cart/user/heart/star/plus/arrow-right/sparkles/sun/apple/snowflake/droplet/cookie/leaf/coffee/package/gift/baby/spray-can/lamp/sofa/armchair/shirt/smartphone/camera/utensils/sandwich/headphones/laptop/monitor/gamepad-2/watch/palette/croissant/cake/bed/bed-double/lamp-desk/flower/truck/shield-check/credit-card/map-pin/menu/x/check/chevron-down/phone/mail/facebook/instagram/youtube/table-2/gem/music/tv/car/globe/clock/calendar/tag/percent/store/users/rocket/layers/database/cloud/lock/shield/chart-bar/chart-line/chart-pie/trending-up/activity/gauge/target/code/workflow'
const MAX_SCREENSHOT_BYTES = 16 * 1024 * 1024
const MAX_TARGETED_CONTEXT_CALLS = 4
const MAX_CORRECTION_DIAGNOSTICS = 64
// One repair round can legitimately surface a second round of repairable
// geometry (collapsing failed media reflows siblings). Two bounded rounds
// convert those runs into publishes; anything deeper is a real defect.
const MAX_FINISH_REPAIR_ROUNDS = 2
const PNG_SIGNATURE = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])
const EXPLICIT_MOBILE_BRIEF = /(?:\b(?:mobile|phone|iphone|ios|android)\b|移动(?:端|应用|界面)?|手机(?:端|应用|界面)?)/iu
const EXPLICIT_CANVAS_SIZE = /(?:^|\D)(\d{3,4})\s*(?:x|×|✕|\*)\s*(\d{3,5})(?:\D|$)/iu
export const COMMERCE_BRIEF = /(?:\b(?:e-?commerce|shop|shopping|storefront|retail)\b|电商|商城|购物|商品|商店)/iu

/**
 * Heal the canonical page-wrapper slip: a script whose FIRST creation call is
 * `const <name> = I(null, {...});` rebinds that name to the existing root id
 * (the wrapper object is dropped — the root frame already carries the canvas
 * contract). Returns the healed source, or undefined when the pattern is not
 * the safe canonical form.
 */
function healRootWrapperScript(script: string, rootNodeId: string): string | undefined {
  const assignment = /(const|let|var)\s+[A-Za-z_$][\w$]*\s*=\s*I\s*\(\s*null\s*,\s*\{/u.exec(script)
  if (assignment === null) return undefined
  const callStart = script.indexOf('I', assignment.index + assignment[1].length)
  // Only heal when this is the first I( call in the script — a later
  // I(null, ...) is not the page-wrapper shape.
  const firstCreation = script.search(/\bI\s*\(/u)
  if (firstCreation === -1 || firstCreation < assignment.index) return undefined
  const braceStart = script.indexOf('{', callStart)
  let depth = 0
  let index = braceStart
  let inString: string | undefined
  for (; index < script.length; index += 1) {
    const ch = script[index]
    if (inString !== undefined) {
      if (ch === '\\') index += 1
      else if (ch === inString) inString = undefined
      continue
    }
    if (ch === '"' || ch === "'" || ch === '`') inString = ch
    else if (ch === '{') depth += 1
    else if (ch === '}') {
      depth -= 1
      if (depth === 0) break
    }
  }
  if (depth !== 0) return undefined
  const close = script.indexOf(')', index)
  if (close === -1) return undefined
  return script.slice(0, callStart) + JSON.stringify(rootNodeId) + script.slice(close + 1)
}

const SECOND_BATCH_SCOPE_INSTRUCTION = 'Fresh QuickJS; opaque I/K ids parent-only, no mutation/locals. Text and text-background colors come from begin continuationStyle.palette slots only; never invent hexes for text surfaces. No new Page/App Content/Header/Hero. On begin.rootNodeId add <=3 regions/cards ending with the required Footer; exact old wrapper id only. Category helper: card=I(rail,96x112 vertical); face=I(card,{type:"frame",name:"Category glyph surface",width:56,height:56}); I(face,{type:"icon_font",name:label+" icon",iconFontName:glyph,width:24,height:24}); I(card,{type:"text",name:label+" label",content:label,...}); rail justifyContent:"space_between"; never art/media/image names. Semantic match: 数码/electronics=smartphone/camera, 食品/food=utensils/sandwich/croissant; never lamp/coffee for those. Desktop: complete three equal fill_container product cards from one coherent collection spanning the rail, one large image/name/price each, gap24, no unused right tail. Mobile: the product rail uses at most 2 fill_container cards OR equal numeric-width cards (e.g. 3x240px) inside an explicit clipped scroller rail — never three fill_container cards. Badge/pill text on #C2410C must be >=15px fontWeight>=600 (or #C2410C text on #FFF7ED). Mobile sections keep >=16px vertical separation; category tiles and search rows never butt against neighboring surfaces. Generic home uses exactly `gray armchair isolated photo`, `artemide tolomeo lamp photo`, `potted plant isolated photo`; label the third as a potted plant, not a vase. That trio is ONLY for a generic home/furniture brief: any other vertical (coffee, food, fashion, electronics, ...) must give every card an English query naming the exact product shown in that card visible name, never the home trio. Every query <=4 English words, exactly one product, no lifestyle/collection/category. No lone small icon in large media. The last root region is the required Footer that completes the page: a role footer frame width:"fill_container" height:"fit_content" vertical gap 24 padding:[48,160] (mobile [40,24]) fill #1C1917; through its binding add the brand text #FFFFFF, one role nav-links horizontal row of >=3 role nav-link frames (minWidth 44, height 44) each holding 14px #D6D3D1 text, then one 13px #A8A29E copyright line. Never finish without the Footer.'

const CONTEXT_TOOLS = [
  'get_guidelines',
  'get_style_guide',
  'list_style_guides',
  'get_variables',
  'get_active_theme',
  'set_variables',
  'set_themes',
  'set_active_axis_value',
  'list_ui_kits',
  'list_components',
  'get_component',
  'batch_get',
  'read_nodes',
  'find_empty_space',
  'get_canvas_bounds',
  'apply_design_system',
  'enrich_images',
] as const

type ContextTool = typeof CONTEXT_TOOLS[number]

export interface DesignDraftToolServices {
  fs: FileSystem
  sandboxPolicy: SandboxPolicyService
  render: RenderAccessController
  observe(target: FsTarget, observation: FsObservation, exec: ToolRunContext): void
  /** Test seam and pre-commit artifact builder; defaults to the real cache. */
  createDocumentSnapshot?: typeof createDocumentSnapshotFromText
  /** Optional advisory provider fixture; the planner ships off by default. */
  styleProvider?: StyleRecommendationProvider
  /** Test seam for handle expiry; defaults to the wall clock. */
  styleClock?: () => number
  /** Test seam over the installed catalog snapshot (value or loader). */
  styleCatalog?: StyleCatalogSnapshot | (() => StyleCatalogSnapshot)
  /** Whether begin exposes the style_selection handoff input; default true. */
  styleHandoff?: { enabled: boolean }
}

interface PendingPublication {
  ownerSessionId: string
  requestedPath: string
  processPath: string
  target: FsTarget
  canvas: DraftCanvasContract
  canvasValidated: boolean
  contextCalls: Set<string>
  contextEnrichmentUsed: boolean
  finalEnrichmentUsed: boolean
  rootNodeId: string
  commerceIntent: boolean
  generationScriptCount: number
  generationCorrectionAttempted: Set<number>
  engine: PipelineEngine
  brief: string
  /** Immutable confirmed style section fixed for the whole draft (§4). */
  selectedStyle?: JsonValue
  agentRunUsed: boolean
  repairAuthorized: boolean
  repairAttemptCount: number
  skipVisualReview: boolean
  visualReviewDone: boolean
  visualReviewAuthorized: boolean
  latestRootScreenshot?: InspectionScreenshotArtifact & {
    version: number
    documentSha256: string
    finalized: boolean
  }
  finalization?: DraftFinalizationCheckpoint
}

interface InspectionScreenshotArtifact {
  path: string
  filename: string
  mimeType: 'image/png'
  bytes: number
  sha256: string
  width?: number
  height?: number
}

interface DraftFinalizationCheckpoint {
  version: number
  documentSha256: string
  beforeVersion: number
  versionChanged: boolean
  documentChanged: boolean
  result: JsonValue
  validation?: DraftValidationCheckpoint
}

interface DraftValidationCheckpoint {
  canvasDiagnostics: string[]
  canvasRepairTargets: GeneratedDesignQualityRepairTarget[]
  nativeDiagnostics: string[]
  nativeSources: {
    finalize: number
    quality: number
    lint: number
    layout: number
  }
  dsh: GeneratedDesignQualityReport
}

export interface DraftCanvasContract {
  platform: 'web' | 'mobile'
  width: number
  seedHeight: number
  finalHeight: number | 'fit_content'
  fixedViewport: boolean
  rootCount: 1
  rootType: 'frame'
}


const DESIGN_PALETTE = {
  page: '#F4F0E8',
  panel: '#FFFFFF',
  surface: '#111318',
  onSurface: '#FAF8F3',
  mutedOnSurface: '#C9C5BC',
  accent: '#A84300',
  accentHighlight: '#FFD9A8',
  onAccent: '#FFFFFF',
  ink: '#17191D',
  muted: '#66635E',
  line: '#DED8CE',
  surfaceLine: '#8F929B',
} as const

// Compact host-selected counterpart of OpenPencil's bundled
// `ecommerce-modern-light` style guide. Ordinary DSH turns must not spend a
// model/tool round trip discovering a style, but they still need the same
// concrete visual direction the App resolves before it starts drawing.
const ECOMMERCE_PALETTE = {
  page: '#FFFFFF',
  panel: '#FFFFFF',
  surface: '#1C1917',
  onSurface: '#FFFFFF',
  mutedOnSurface: '#A8A29E',
  accent: '#C2410C',
  accentHighlight: '#FFF7ED',
  onAccent: '#FFFFFF',
  ink: '#1C1917',
  muted: '#57534E',
  line: '#E7E5E4',
  surfaceLine: '#F5F5F4',
} as const

export interface PublishedDraft {
  draftId: string
  path: string
  filename: string
  bytes: number
  sha256: string
  created: true
  applied: true
  saved: true
  published: true
  sourceTool: typeof OPENPENCIL_PIPELINE_FINISH_TOOL_NAME
  previewIntent: 'document'
  editable: true
  autoOpenEditor: true
  preview: RenderFrame
  document: DocumentSnapshot
  note: string
  /** Immutable confirmed style section retained through publication (§4). */
  selectedStyle?: JsonValue
}

/** Compact unpublished result projected into the live editor workbench. */
export interface BegunDraft {
  draftId: string
  path: string
  version: number
  createdAt?: number
  platform: 'web' | 'mobile'
  canvas: DraftCanvasContract
  buildContract: JsonValue
  rootNodeId: string
  continuationStyle: JsonValue
  editorState: JsonValue
  styleGuideTags: JsonValue
  document: DocumentSnapshot
  sourceTool: typeof OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME
  previewIntent: 'document'
  editable: true
  autoOpenEditor: true
  liveCanvas: true
  published: false
  next: string
}

class DesignDraftJsQualityError extends Error {
  readonly report: GeneratedDesignQualityReport

  constructor(report: GeneratedDesignQualityReport) {
    super('OpenPencil draft failed the DSH publication quality gate')
    this.report = report
  }
}

const renderJson = (_args: unknown, value: unknown): [{ type: 'text'; text: string }] => [{
  type: 'text',
  text: JSON.stringify(value, null, 2),
}]

function ownerSessionId(exec: ToolRunContext): string {
  if (exec.agent === undefined) {
    throw new Error('OpenPencil design drafts require an agent-owned DSH execution')
  }
  // Draft lifetime is tied to `session/disposed`, whose event carries this
  // same identity. DSH currently enforces agent.id === session.id, but using
  // the session directly keeps the lifecycle contract explicit.
  return String(exec.agent.session.id)
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function asJson(value: unknown): JsonValue {
  return (value ?? null) as JsonValue
}

function publicBatchReceipt(value: unknown): JsonValue {
  if (!isRecord(value)) return {}
  const receipt: Record<string, JsonValue> = {}
  if (Array.isArray(value.results)) {
    receipt.results = value.results.flatMap((entry) => {
      if (!isRecord(entry)
        || typeof entry.binding !== 'string'
        || typeof entry.nodeId !== 'string') return []
      return [{ binding: entry.binding, nodeId: entry.nodeId }]
    })
  }
  if (typeof value.nodeCount === 'number' && Number.isFinite(value.nodeCount)) {
    receipt.nodeCount = value.nodeCount
  }
  if (typeof value.postProcessed === 'boolean') receipt.postProcessed = value.postProcessed
  if (typeof value.applied === 'boolean') receipt.applied = value.applied
  return receipt
}

function creationCallCount(script: string): number {
  return [...script.matchAll(/\b[IK]\s*\(/gu)].length
}

/**
 * Direct QuickJS bindings are opaque node-id strings, not mutable PenNode
 * objects. Assigning `card.x = ...` throws after earlier I/K calls have
 * already been recorded; older native runners intentionally salvaged that
 * prefix, which made a visibly incomplete batch look successful. Creation
 * scripts do not need member mutation, so reject it before any draft change.
 */
function hasMemberMutation(script: string): boolean {
  return /(?:^|[;{}\n])\s*[A-Za-z_$][\w$]*\s*(?:\.[A-Za-z_$][\w$]*|\[[^\]\n]+\])\s*(?:=(?!=)|\+=|-=|\*=|\/=|%=|\+\+|--)/mu.test(script)
}

function designContinuationStyle(
  canvas: DraftCanvasContract,
  commerceIntent: boolean,
  guide?: SelectedStyleGuide,
): JsonValue {
  const mobile = canvas.platform === 'mobile'
  const commerceDesktop = commerceIntent && !mobile
  const displayDefault: [number, number, number] = [mobile ? 42 : commerceDesktop ? 56 : 64, commerceDesktop ? 700 : 800, commerceDesktop ? 1.05 : 1.04]
  const headingDefault: [number, number, number] = [mobile ? 28 : commerceDesktop ? 36 : 40, commerceDesktop ? 700 : 750, commerceDesktop ? 1.2 : 1.12]
  const bodyDefault: [number, number, number] = [mobile ? 16 : 18, 400, commerceIntent ? 1.6 : 1.5]
  const labelDefault: [number, number, number] = [14, commerceIntent ? 500 : 650, commerceIntent ? 1.4 : 1.3]
  const scaled = (row: [number, number] | undefined, fallback: [number, number, number]): [number, number, number] =>
    row === undefined ? fallback : [row[0], row[1], fallback[2]]
  return {
    version: 'openpencil-continuation-style-v1',
    styleGuide: guide?.name ?? (commerceIntent ? 'ecommerce-modern-light' : 'dsh-editorial-warm'),
    palette: guide?.palette ?? (commerceIntent ? ECOMMERCE_PALETTE : DESIGN_PALETTE),
    typography: {
      fontFamily: 'Inter, system-ui, sans-serif',
      display: scaled(guide?.typeScale.display, displayDefault),
      heading: scaled(guide?.typeScale.heading, headingDefault),
      body: scaled(guide?.typeScale.body, bodyDefault),
      label: scaled(guide?.typeScale.label, labelDefault),
    },
    spacing: {
      pageInset: mobile ? 24 : commerceDesktop ? 160 : 80,
      headerInset: mobile ? 24 : commerceDesktop ? 160 : 72,
      sectionPadding: mobile ? [40, 24, 36, 24] : commerceDesktop ? [64, 160] : [72, 80],
      gaps: [8, 12, 16, 24, 32, 48, 64],
    },
    ...(!commerceIntent && guide !== undefined
      ? {
          recipe: {
            direction: guide.direction,
            ...(guide.surfaces.length > 0 ? { surfaces: guide.surfaces } : {}),
            language: 'Visible copy follows the user request language; keep the invented brand name consistent and do not mix interface languages.',
          },
        }
      : {}),
    ...(commerceIntent
      ? {
          recipe: {
            direction: 'Modern direct-to-consumer storefront: clean white base, warm-tinted section rhythm, product-first hierarchy; orange only for CTA, active state, and price.',
            surfaces: 'Structural wrappers stay transparent. Cards are white with 16px radius and optional #E7E5E4 hairline; buttons use 12px radius. Never stack decorative wrapper cards.',
            hero: mobile
              ? 'One dominant message, one primary CTA, and one recognizable focal visual; avoid generic floating rectangles.'
              : '1120px row: 512px copy + 64px gap + 448px product visual; headline/subtitle fill the copy width; one primary CTA, 56px display, 64-96px vertical rhythm.',
            rails: 'Category rail spans its content width with space_between. Product rail is a three-column equal-width grid with 24px gaps, large image areas, 24px card content padding, and no unused right tail.',
            language: 'Visible copy follows the user request language; keep the invented brand name consistent and do not mix interface languages.',
          },
        }
      : {}),
    iconSizes: [18, 20, 24],
    controls: {
      height: mobile ? 52 : 48,
      buttonHeight: mobile ? 52 : 48,
      iconButton: 44,
    },
  } as JsonValue
}

function defaultDraftPath(brief: string): string {
  const stem = COMMERCE_BRIEF.test(brief) ? 'shop-home' : 'openpencil-design'
  return `${stem}-${Date.now().toString(36)}-${randomBytes(2).toString('hex')}.op`
}

function seedCanvasScript(
  canvas: DraftCanvasContract,
  commerceIntent: boolean,
  pageColor?: string,
): string {
  const palette = pageColor !== undefined
    ? { page: pageColor }
    : commerceIntent ? ECOMMERCE_PALETTE : DESIGN_PALETTE
  const root = {
    type: 'frame',
    name: 'Generated Page',
    width: canvas.width,
    height: canvas.fixedViewport ? canvas.seedHeight : 'fit_content',
    ...(canvas.fixedViewport ? {} : { minHeight: canvas.seedHeight }),
    layout: 'vertical',
    padding: 0,
    gap: 0,
    fill: [{ type: 'solid', color: palette.page }],
  }
  return `I(null, ${JSON.stringify(root)});`
}

function documentSha256(documentJson: string): string {
  return createHash('sha256').update(documentJson).digest('hex')
}

function clearFinalizationCheckpoint(pending: PendingPublication): void {
  pending.finalization = undefined
  pending.latestRootScreenshot = undefined
}

function finalizationNote(checkpoint: DraftFinalizationCheckpoint): string {
  if (checkpoint.documentChanged) {
    return `Native finalization changed document bytes and advanced version from ${checkpoint.beforeVersion} to ${checkpoint.version}. The finalized checkpoint is informational only and does not request a repair.`
  }
  if (checkpoint.versionChanged) {
    return `Native finalization advanced version from ${checkpoint.beforeVersion} to ${checkpoint.version} without changing document bytes. The finalized checkpoint is informational only and does not request a repair.`
  }
  return 'Native finalization preserved the current version and document bytes. The finalized checkpoint is informational only and does not request a repair.'
}

function publicFinalizationCheckpoint(
  checkpoint: DraftFinalizationCheckpoint,
  reused: boolean,
): JsonValue {
  return {
    version: checkpoint.version,
    changed: checkpoint.versionChanged,
    documentChanged: checkpoint.documentChanged,
    reused,
    documentSha256: checkpoint.documentSha256,
    note: finalizationNote(checkpoint),
  }
}

function publicScreenshot(
  screenshot: PendingPublication['latestRootScreenshot'],
  version: number,
  documentSha?: string,
): JsonValue | undefined {
  if (
    screenshot === undefined
    || screenshot.version !== version
    || (documentSha !== undefined && screenshot.documentSha256 !== documentSha)
  ) return undefined
  const {
    version: _version,
    documentSha256: _documentSha256,
    finalized: _finalized,
    ...artifact
  } = screenshot
  return artifact as unknown as JsonValue
}

export function latestDirectUserText(exec: ToolRunContext): string | undefined {
  const session = exec.agent?.session
  if (session === undefined || typeof session.deriveMessages !== 'function') return undefined
  const messages = session.deriveMessages()
  for (let index = messages.length - 1; index >= 0; index -= 1) {
    const message = messages[index]
    if (message.role !== 'user' || message.source.kind !== 'user') continue
    const text = message.content
      .filter(block => block.type === 'text')
      .map(block => block.text)
      .join('\n')
      .trim()
    if (text.length > 0) return text.slice(0, MAX_BRIEF_LENGTH)
  }
  return undefined
}

/** Choice-only replies do not declare a new viewport or platform. */
export function hasExplicitCanvasIntent(text: string): boolean {
  return EXPLICIT_MOBILE_BRIEF.test(text) || EXPLICIT_CANVAS_SIZE.test(text)
    || /\b(?:desktop|web|website|webapp)\b|桌面|电脑|网页|网站|官网/iu.test(text)
}

export function draftCanvasContract(brief: string, directUserText?: string): DraftCanvasContract {
  // Platform and viewport are user intent, not design-agent creative choices.
  // The model-facing `brief` may expand the requested content, but it must not
  // silently turn an ordinary desktop/web request into a mobile composition.
  const authoritativeIntent = directUserText ?? brief
  const explicit = EXPLICIT_CANVAS_SIZE.exec(authoritativeIntent)
  const explicitWidth = explicit === null ? undefined : Number(explicit[1])
  const explicitHeight = explicit === null ? undefined : Number(explicit[2])
  const validExplicit = explicitWidth !== undefined
    && explicitHeight !== undefined
    && Number.isSafeInteger(explicitWidth)
    && Number.isSafeInteger(explicitHeight)
    && explicitWidth >= 240
    && explicitWidth <= 3_840
    && explicitHeight >= 240
    && explicitHeight <= 20_000
  const mobile = EXPLICIT_MOBILE_BRIEF.test(authoritativeIntent) || (validExplicit && explicitWidth <= 500)
  const width = validExplicit ? explicitWidth! : mobile ? 390 : 1_440
  const seedHeight = validExplicit ? explicitHeight! : mobile ? 844 : 900
  return {
    platform: mobile ? 'mobile' : 'web',
    width,
    seedHeight,
    finalHeight: validExplicit ? seedHeight : 'fit_content',
    fixedViewport: validExplicit,
    rootCount: 1,
    rootType: 'frame',
  }
}

/**
 * Small, version-pinned subset of the native design contract needed to write
 * at most two direct QuickJS generation scripts without re-reading native sources.
 */
function compactBuildContract(
  canvas: DraftCanvasContract,
  commerceIntent: boolean,
  domainGuidance?: Record<string, string>,
): JsonValue {
  const mobile = canvas.platform === 'mobile'
  return {
    version: 'openpencil-script-v12',
    // App-alignment: the same domain-skill essentials OpenPencil's built-in
    // design agent loads for this brief (built from the vendored corpus).
    ...(domainGuidance === undefined ? {} : { domainGuidance }),
    canvas: {
      width: canvas.width,
      rootHeight: canvas.finalHeight,
      rule: canvas.fixedViewport
        ? 'Keep numeric root viewport.'
        : 'Keep root height:"fit_content".',
    },
    script: {
      runtime: 'sandboxed QuickJS',
      create: 'rootNodeId is page. Use I(parent,node)/K(realKitId,...); only returned frame/group bindings parent children. No Page/root, I(null,...), or children on leaves including text_input. Round art parent=frame+cornerRadius, not ellipse.',
      wrapper: 'const draftId="<exact begin.draftId>";\nconst script=String.raw`...`;\nconst r=await tools.openpencil_pipeline_batch({draftId,script});\nreturn r;',
      wrapperRule: 'Quote exact begin.draftId first; call contains only draftId,script; no fields/return inside.',
      safe: 'QuickJS loops/helpers. I/K return opaque ids: parent-only, never mutate a binding. No imports/console/host APIs/edit/delete.',
    },
    generation: {
      first: mobile
        ? 'First<=32 I/K: compact Header/Search/Cart + one complete mobile Hero; short ASCII brand. Mobile vertical rhythm: root gap>=16 or every section keeps >=16px vertical padding; the Search row keeps >=12px clearance above and below and never touches a bordered neighbor. One dominant message/CTA and one recognizable focal visual; no desktop two-column rule and no generic floating rectangles. Below-fold/images batch2; else one.'
        : commerceIntent
          ? 'First<=32 I/K: use literal hex colors directly in node objects, no palette alias variables; text and text-background colors must be copied from begin continuationStyle.palette slots, never invented hexes. const header=I(root,{type:"frame",name:"Header",role:"navbar",width:"fill_container",height:64,layout:"horizontal",padding:[0,160],justifyContent:"space_between",alignItems:"center"}); insert Brand text, Nav role nav-links, and Actions through that header binding, never as root siblings — an empty role container invalidates the batch. Every nav item MUST be a frame role nav-link minWidth44 height44 containing its text child; never put role nav-link on text. Header actions role toolbar (Search/Cart each 44x44 role icon-button frame with a 20px icon). Hero is width:"fill_container" horizontal padding:[64,160], containing copy512+gap64+image448; never set Hero width1120 together with padding. Headline/subtitle width:"fill_container". Short ASCII brand; all other copy follows the user language (Chinese request => Chinese copy). One headline/subtitle/primary CTA; CTA role button 160x48 #C2410C/#FFFFFF and label must be inserted through its CTA binding. Generic commerce MUST use this direct visual pattern: const visual=I(hero,{type:"image",name:"Hero product image",width:448,height:360,imageSearchQuery:"gray loveseat isolated photo"}); imageSearchQuery is a direct node field and must differ from every product-card query. Never wrap the image, use image:{...}, or mix it with shapes. A layout:none 4-6-layer ellipse/path fallback is allowed only when the user explicitly requests illustration/no photos. Below-fold product images batch2.'
          : 'First<=32 I/K: Header/Nav(Brand/Search/Cart)+one polished Hero; short ASCII brand. Use one dominant message, one primary CTA, and one recognizable focal visual with no blank field or overflow. Below-fold/images batch2; else one.',
      second: 'Second=fresh parent-only I/K; no mutation/new Page/App Content/Header/Hero. rootNodeId<=3 regions, old wrapper. Product=media-or-none+name+price. Category=96x112 -> 56x56 Category glyph surface -> semantic 24x24 icon+label; names not art/media/image; rail spans width with justifyContent:"space_between". 数码=smartphone/camera; 食品=utensils/sandwich/croissant; not lamp/coffee. Commerce desktop=3 equal fill_container product cards spanning the rail, 24px gap, large media, name, price, no unused right tail; mobile product rail=at most 2 fill_container cards OR equal numeric-width cards (e.g. 3x240px) inside an explicit clipped scroller rail, never 3 fill_container. Badge/pill text on the #C2410C accent must be >=15px fontWeight>=600 or use #C2410C text on #FFF7ED. Generic home ONLY exact queries=gray armchair isolated photo/artemide tolomeo lamp photo/potted plant isolated photo; third card is potted plant. Other verticals: each query names the exact product in that card name (coffee=>latte cup photo), never the home trio. Images=node.image. Query<=4 English words, one product, no lifestyle/collection/category. Last region=required Footer: role footer fill_container/fit_content vertical gap24 padding[48,160] (mobile[40,24]) fill #1C1917: brand #FFFFFF, role nav-links row of >=3 role nav-link frames minWidth44 height44 14px #D6D3D1, 13px #A8A29E copyright. Loops; finish.',
      limit: 'Exactly two I/K generation scripts; do not expand short briefs into gallery/promo variants beyond the contract sections; the single closing Footer is required, not an expansion.',
    },
    continuationStyle: {
      rule: 'Use returned values only.',
    },
    quality: {
      textDefaults: 'Generated text: Inter, system-ui, sans-serif / 16 / 1.5.',
      contrast: 'Use AA text pairs from the returned continuationStyle palette.',
    },
    repair: 'Each complete non-empty finish.repairTargets allows one U(nodeId,patch)-only script + one finish retry (host caps repair rounds at 2). A clean gate returns one needs_visual_review round: accept by calling finish again, or send one bounded I/K/U correction batch (<=16 calls, <=6KB, never rebuild Header/Hero) then finish. Otherwise stop.',
    node: {
      parents: 'Only frame/group parent. Use rectangle not rect; ellipse/image/icon_font/text/path are leaves.',
      container: 'width/height: number, fill_container, or fit_content; padding:[vertical,horizontal] or [top,right,bottom,left].',
      text: 'Text portable "Inter, system-ui, sans-serif"/16/1.5; CJK same. height omitted/fit_content; never bare Inter.',
      paint: 'fill:[solid #RRGGBB]; stroke:{thickness,fill}; effects[]; shadow blur<=40.',
      icon: `{type:'icon_font',name:'Search icon',iconFontName:'search',width:20,height:20}; name=layer label; iconFontName=glyph from ${VERIFIED_LUCIDE_ICONS}; else shapes.`,
      image: 'Photos:default1; commerce=Hero1+product3, all four queries distinct. Hero exact leaf: I(parent,{type:"image",name:"Hero product image",width:"fill_container",height:300,imageSearchQuery:"gray loveseat isolated photo"}); imageSearchQuery is direct, never image:{...}, never wrapper. Query<=4 English words, one product, no lifestyle/collection/category. Generic desktop Hero uses the direct image leaf; shapes only when user explicitly requests illustration/no photos. Media=image XOR shapes; no tiny icon.',
      control: 'Controls >=44x44 (or fill width) and bind visible children. Button/CTA role button; icon buttons are role icon-button 44x44 frames containing a 20px icon. Nav collection role nav-links; each item role nav-link minWidth44 height44 with text. text_input is a leaf: placeholder only; never I(text_input,...). Icon+hint search uses a Search frame. password text_input secure:true.',
    },
    layoutRules: [
      'No x/y in horizontal/vertical flow. An explicit layout:"none" visual stack has numeric width/height and every direct child has numeric x/y/width/height; never mutate returned bindings.',
      'No fill_container on fit_content same-axis parent.',
      'No empty shells.',
      'Visible copy uses the user request language consistently; brand identity stays identical.',
      ...(canvas.platform === 'mobile'
        ? [
            'Mobile: only full-width root children; text/icons/controls/titles use a bound 24px-gutter rail.',
            'Mobile category item: vertical fit_content item -> 56x56 frame tile -> icon, then label; never put the icon directly in the item.',
            'Mobile product rail: height:"fit_content"; either at most 2 all-fill_container cards, or equal numeric-width cards in an explicit clipped/scroller rail; never mix numeric and fill_container card widths.',
          ]
        : []),
    ],
  } as JsonValue
}

function draftRootNodes(document: Record<string, unknown>): unknown[] {
  if (Array.isArray(document.children) && document.children.length > 0) return document.children
  if (!Array.isArray(document.pages)) return []
  return document.pages.flatMap(page => isRecord(page) && Array.isArray(page.children) ? page.children : [])
}

function draftRootNodeId(documentJson: string): string | undefined {
  let document: unknown
  try {
    document = JSON.parse(documentJson)
  } catch {
    return undefined
  }
  if (!isRecord(document)) return undefined
  const roots = draftRootNodes(document)
  if (roots.length !== 1 || !isRecord(roots[0])) return undefined
  const id = roots[0].id
  return typeof id === 'string' && /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(id)
    ? id
    : undefined
}

function hasUnresolvedImageSearchQuery(documentJson: string): boolean {
  let document: unknown
  try {
    document = JSON.parse(documentJson)
  } catch {
    return false
  }
  const pending: unknown[] = [document]
  while (pending.length > 0) {
    const value = pending.pop()
    if (Array.isArray(value)) {
      pending.push(...value)
      continue
    }
    if (!isRecord(value)) continue
    if (typeof value.imageSearchQuery === 'string' && value.imageSearchQuery.trim() !== '') {
      const source = typeof value.src === 'string' ? value.src.trim() : ''
      if (source === '' || /^placeholder:\/\//iu.test(source)) return true
    }
    pending.push(...Object.values(value))
  }
  return false
}

function canvasContractDiagnostics(
  documentJson: string,
  canvas: DraftCanvasContract,
  options: { requireFinalHeight?: boolean } = {},
): string[] {
  let document: unknown
  try {
    document = JSON.parse(documentJson)
  } catch {
    return ['The native draft document is not valid JSON.']
  }
  if (!isRecord(document)) return ['The native draft document root is invalid.']
  const roots = draftRootNodes(document)
  if (roots.length !== canvas.rootCount) {
    return [`Canvas contract requires exactly one root frame; current root count is ${roots.length}.`]
  }
  const root = roots[0]
  if (!isRecord(root) || root.type !== canvas.rootType) {
    return ['Canvas contract requires the single root node to be a frame.']
  }
  if (root.width !== canvas.width) {
    return [`Canvas contract requires root width ${canvas.width}px; current root width is ${String(root.width)}.`]
  }
  if (canvas.fixedViewport && root.height !== canvas.seedHeight) {
    return [`Canvas contract requires the explicit root height ${canvas.seedHeight}px; current root height is ${String(root.height)}.`]
  }
  if (options.requireFinalHeight === true && !canvas.fixedViewport && root.height !== canvas.finalHeight) {
    return [`Canvas contract requires the completed root height ${String(canvas.finalHeight)}; current root height is ${String(root.height)}.`]
  }
  return []
}

function canvasContractRepairTargets(
  documentJson: string,
  canvas: DraftCanvasContract,
  options: { requireFinalHeight?: boolean } = {},
): GeneratedDesignQualityRepairTarget[] {
  let document: unknown
  try {
    document = JSON.parse(documentJson)
  } catch {
    return []
  }
  if (!isRecord(document)) return []
  const roots = draftRootNodes(document)
  if (roots.length !== 1 || !isRecord(roots[0]) || roots[0].type !== canvas.rootType) return []
  const root = roots[0]
  const nodeId = typeof root.id === 'string' && /^[A-Za-z0-9][A-Za-z0-9._:-]{0,127}$/.test(root.id)
    ? root.id
    : undefined
  if (nodeId === undefined) return []
  const patch: GeneratedDesignQualityRepairTarget['patch'] = {}
  if (root.width !== canvas.width) patch.width = canvas.width
  if (canvas.fixedViewport && root.height !== canvas.seedHeight) patch.height = canvas.seedHeight
  if (options.requireFinalHeight === true && !canvas.fixedViewport && root.height !== canvas.finalHeight) {
    patch.height = 'fit_content'
  }
  if (Object.keys(patch).length === 0) return []
  return [{ nodeId, operation: 'U', rule: 'canvas-contract', patch }]
}

function contextFingerprint(tool: ContextTool, args: Record<string, unknown>): string {
  const canonical = (value: unknown): string => {
    if (value === null || typeof value !== 'object') return JSON.stringify(value) ?? 'null'
    if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
    const record = value as Record<string, unknown>
    return `{${Object.keys(record).sort().map(key => `${JSON.stringify(key)}:${canonical(record[key])}`).join(',')}}`
  }
  return `${tool}:${canonical(args)}`
}

function publicCall(result: DesignDraftCallResult): Record<string, JsonValue> {
  return {
    tool: result.tool,
    value: asJson(result.value),
    version: result.version,
    changed: result.changed === true,
    hasImage: result.hasImage === true,
  }
}

function assertNoExternalArguments(value: unknown, location = 'arguments', depth = 0): void {
  if (depth > 20) throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: arguments are too deeply nested`)
  if (Array.isArray(value)) {
    value.forEach((item, index) => assertNoExternalArguments(item, `${location}[${index}]`, depth + 1))
    return
  }
  if (!isRecord(value)) return
  for (const [key, child] of Object.entries(value)) {
    if (/(?:^|_)(?:file|source|output|preset|svg|html|snapshot)?path$/i.test(key)
      || /(?:url|uri|directory|outputdir|export|import|spawn)/i.test(key)) {
      throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: ${location}.${key} is not allowed in an isolated design draft`)
    }
    assertNoExternalArguments(child, `${location}.${key}`, depth + 1)
  }
}

const ISSUE_FIELD = /(?:issues$|^advisories$|^diagnostics$|^errors$|^emptyShells$|^intentQuestions$|^imageSlots$)/i
const OBSERVATIONAL_QUALITY_FIELD = /^(?:emptyShells|imageSlots|intentQuestions)$/i
const NON_BLOCKING_LINT_WARNINGS = new Set([
  'invisible-container',
  'mixed-sibling-corner-radius',
  'mixed-sibling-padding',
  'sibling-inconsistency',
  'text-effect',
  'text-explicit-height',
])

function issueValues(value: unknown, options: { includeObservational?: boolean } = {}): string[] {
  const includeObservational = options.includeObservational ?? true
  const issues: string[] = []
  const seen = new Set<object>()
  const visit = (candidate: unknown, key = '', depth = 0): void => {
    if (depth > 8 || candidate === null || candidate === undefined) return
    if (typeof candidate !== 'object') return
    if (seen.has(candidate)) return
    seen.add(candidate)
    if (Array.isArray(candidate)) {
      if (ISSUE_FIELD.test(key)) {
        for (const entry of candidate) {
          if (typeof entry === 'string' && entry.trim().length > 0) issues.push(entry.slice(0, 300))
          else if (isRecord(entry)) issues.push(JSON.stringify(entry).slice(0, 300))
        }
      } else {
        for (const entry of candidate) visit(entry, key, depth + 1)
      }
      return
    }
    for (const [childKey, child] of Object.entries(candidate)) {
      if (ISSUE_FIELD.test(childKey)) {
        // Empty shells include intentional layout primitives such as spacers
        // and dividers. Image-slot hints are also observational: script mode
        // cannot call the G() helper, and a deliberate
        // product swatch or placeholder must not force a repair loop. Keep
        // both visible in explicit inspection while excluding them from the
        // publication gate without weakening hard rendering diagnostics.
        if (!includeObservational && OBSERVATIONAL_QUALITY_FIELD.test(childKey)) continue
        if (Array.isArray(child)) visit(child, childKey, depth + 1)
        else if (typeof child === 'string' && child.trim().length > 0) issues.push(child.slice(0, 300))
      } else {
        visit(child, childKey, depth + 1)
      }
    }
  }
  visit(value)
  return issues.slice(0, 30)
}

function blockingLintValue(value: unknown): unknown {
  if (!isRecord(value) || !Array.isArray(value.issues)) return value
  return {
    ...value,
    // OpenPencil's own fixer treats Info as observational. A narrow set of
    // visual-consistency and authored-text warnings are also non-fatal, but
    // other Warning categories (for example invisible paths, accessibility,
    // or pathological effects) still block publication.
    issues: value.issues.filter(issue => {
      if (!isRecord(issue)) return true
      const severity = typeof issue.severity === 'string'
        ? issue.severity.toLocaleLowerCase('en-US')
        : ''
      if (severity === 'info') return false
      const categoryValue = typeof issue.category === 'string' ? issue.category : issue.code
      const category = typeof categoryValue === 'string'
        ? categoryValue.toLocaleLowerCase('en-US')
        : ''
      return !(severity === 'warning' && NON_BLOCKING_LINT_WARNINGS.has(category))
    }),
  }
}

function presentationMeta(editorHost: EditorHostController, render: RenderAccessController) {
  return (_args: unknown, value: JsonValue): JsonValue => {
    if (isRecord(value) && value.published === false) {
      return projectScreenshotPresentation(value, render)
    }
    const result = value as unknown as PublishedDraft
    const editor = editorHost.grantFor(result.path, result.document?.sha256)
    return projectDocumentGrant(value, render, editor)
  }
}

function inspectionPreviewFilename(sha256: string): string {
  return `render-stage-${sha256}.png`
}

function projectScreenshotPresentation(value: JsonValue, render: RenderAccessController): JsonValue {
  if (!isRecord(value) || !isRecord(value.screenshot)) return value
  const artifact = value.screenshot
  if (
    typeof artifact.bytes !== 'number'
    || typeof artifact.sha256 !== 'string'
    || typeof artifact.width !== 'number'
    || typeof artifact.height !== 'number'
  ) return value
  return projectImageArtifactGrant(value, render, {
    filename: inspectionPreviewFilename(artifact.sha256),
    bytes: artifact.bytes,
    sha256: artifact.sha256,
    width: artifact.width,
    height: artifact.height,
    name: 'Live design preview',
    index: 0,
  })
}

function inspectionPresentationMeta(render: RenderAccessController) {
  return (_args: unknown, value: JsonValue): JsonValue => {
    if (!isRecord(value) || value.kind !== 'screenshot') return value
    return projectScreenshotPresentation(value, render)
  }
}

async function persistInspectionScreenshot(screenshot: DesignDraftScreenshot): Promise<{
  path: string
  filename: string
  mimeType: 'image/png'
  bytes: number
  sha256: string
  width?: number
  height?: number
}> {
  if (screenshot.mimeType !== 'image/png') throw new Error('OpenPencil draft screenshot must be a PNG')
  if (!Buffer.isBuffer(screenshot.bytes) || screenshot.bytes.length === 0) {
    throw new Error('OpenPencil draft screenshot was empty')
  }
  if (screenshot.bytes.length < PNG_SIGNATURE.length
    || !screenshot.bytes.subarray(0, PNG_SIGNATURE.length).equals(PNG_SIGNATURE)) {
    throw new Error('OpenPencil draft screenshot did not contain PNG bytes')
  }
  if (screenshot.bytes.length > MAX_SCREENSHOT_BYTES) {
    throw new Error('OpenPencil draft screenshot exceeded the safe cache limit')
  }
  const sha256 = createHash('sha256').update(screenshot.bytes).digest('hex')
  const filename = `${sha256}.png`
  const directory = join(stateRoot(), 'design-draft-inspections')
  const path = join(directory, filename)
  await mkdir(directory, { recursive: true, mode: 0o700 })
  try {
    await writeFile(path, screenshot.bytes, { flag: 'wx', mode: 0o600 })
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error
    const existing = await readFile(path)
    if (!existing.equals(screenshot.bytes)) {
      throw new Error('OpenPencil content-addressed draft screenshot was modified')
    }
  }
  const browserFilename = inspectionPreviewFilename(sha256)
  const browserDirectory = renderDir()
  const browserPath = join(browserDirectory, browserFilename)
  await mkdir(browserDirectory, { recursive: true, mode: 0o700 })
  try {
    await writeFile(browserPath, screenshot.bytes, { flag: 'wx', mode: 0o600 })
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code !== 'EEXIST') throw error
    const existing = await readFile(browserPath)
    if (!existing.equals(screenshot.bytes)) {
      throw new Error('OpenPencil content-addressed browser preview was modified')
    }
  }
  const verified = await verifyRenderOutput(browserPath)
  if (verified.bytes !== screenshot.bytes.length || verified.sha256 !== sha256) {
    throw new Error('OpenPencil browser preview did not match its exact PNG render artifact')
  }
  return {
    path,
    filename,
    mimeType: 'image/png',
    bytes: screenshot.bytes.length,
    sha256,
    width: verified.width,
    height: verified.height,
  }
}

/** Owns model tool definitions and the publication metadata kept outside the daemon. */
export class DesignDraftToolController {
  readonly #drafts: DesignDraftController
  readonly #pending = new Map<string, PendingPublication>()
  readonly #editorHost: EditorHostController
  readonly #services: DesignDraftToolServices
  readonly #styleStore = createStyleSelectionStore({ now: () => this.#styleNow() })
  readonly #resolveStyleCatalog = (): StyleCatalogSnapshot => {
    const catalog = this.#services.styleCatalog
    if (catalog === undefined) return loadInstalledStyleCatalog()
    return typeof catalog === 'function' ? catalog() : catalog
  }
  readonly #styleNow = (): number => this.#services.styleClock?.() ?? Date.now()
  readonly #styleHandoffEnabled = (): boolean => this.#services.styleHandoff?.enabled !== false
  #disposed = false
  #disposePromise: Promise<void> | undefined

  constructor(editorHost: EditorHostController, services: DesignDraftToolServices) {
    this.#editorHost = editorHost
    this.#services = services
    this.#drafts = editorHost.designDrafts
  }

  createTools() {
    return [
      this.#beginTool(),
      this.#contextTool(),
      this.#batchTool(),
      this.#agentRunTool(),
      this.#inspectTool(),
      this.#finishTool(),
      this.#abortTool(),
      ...createStylePlannerTools({
        store: this.#styleStore,
        resolveCatalog: this.#resolveStyleCatalog,
        ...(this.#services.styleProvider === undefined ? {} : { provider: this.#services.styleProvider }),
        now: this.#styleNow,
        beginStyleInputSupported: this.#styleHandoffEnabled,
      }),
    ] as const
  }

  /** Whether this DSH agent currently owns an unpublished pipeline draft. */
  hasActiveDraft(owner: string): boolean {
    for (const pending of this.#pending.values()) {
      if (pending.ownerSessionId === owner) return true
    }
    return false
  }

  async abortOwner(owner: string): Promise<void> {
    await this.#drafts.abortOwner(owner)
    for (const [draftId, pending] of this.#pending) {
      if (pending.ownerSessionId === owner) this.#pending.delete(draftId)
    }
  }

  dispose(): Promise<void> {
    this.#disposePromise ??= (async () => {
      this.#disposed = true
      this.#pending.clear()
      await this.#drafts.dispose()
    })()
    return this.#disposePromise
  }

  #requirePending(draftId: string, owner: string): PendingPublication {
    if (this.#disposed) throw new Error('OpenPencil design-draft tools are disposed')
    const pending = this.#pending.get(draftId)
    if (pending === undefined || pending.ownerSessionId !== owner) {
      throw new Error('OpenPencil design draft does not exist or belongs to another DSH agent')
    }
    return pending
  }

  #beginTool() {
    const drafts = this.#drafts
    const services = this.#services
    const editorHost = this.#editorHost
    return defineTool({
      name: OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
      description: 'Start a private OpenPencil draft, open its live canvas, and return the two-batch QuickJS contract. The .op target is created only after a successful finish.',
      parameters: {
        path: { type: 'string', description: 'Optional new workspace-relative or absolute .op target. Omit it unless the user explicitly named a file; the plugin creates a concrete collision-resistant filename. An explicit target must not exist and is preserved exactly.' },
        brief: { type: 'string', required: true, description: 'The user\'s design request. Preserve it; do not invent a mobile platform when none was requested.' },
        skip_visual_review: { type: 'boolean', description: 'Optional. True skips the single post-finalization visual-review round and publishes directly after a clean quality gate.' },
        engine: { type: 'string', enum: ['script', 'app-agent'], description: 'Optional. Default "script" (the ordinary two-batch flow). "app-agent" runs the OpenPencil App builtin design-agent loop on the daemon instead — use only when the user explicitly asks for the high-fidelity App-identical engine.' },
        ...(this.#styleHandoffEnabled() ? {
          style_selection: { type: 'string', description: 'Optional live style-selection handle from openpencil_style_confirm. Validated against the current catalog, this brief, platform, session, and its 15-minute expiry before any draft is created; a valid selection fixes the guide for the whole draft. Omit for the ordinary automatic flow.' },
        } : {}),
      },
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: renderJson,
        presentationMeta: (_args: unknown, value: JsonValue): JsonValue => {
          if (!isRecord(value) || typeof value.draftId !== 'string') return value
          const pending = this.#pending.get(value.draftId)
          if (pending === undefined || !isRecord(value.document)) return value
          const editor = editorHost.grantForDraft(value.draftId, pending.ownerSessionId)
          return projectDocumentGrant(value, services.render, editor)
        },
      },
      execute: async (args: { path?: string; brief: string; skip_visual_review?: boolean; engine?: string; style_selection?: string }, exec) => {
        const engine = parsePipelineEngine(args.engine, OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME)
        const brief = args.brief.trim()
        const requestedPath = args.path?.trim() || defaultDraftPath(brief)
        if (extname(requestedPath).toLowerCase() !== '.op') throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: path must end in .op`)
        if (brief.length === 0) throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: brief is required`)
        if (brief.length > MAX_BRIEF_LENGTH) throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: brief is too large`)
        const directUserText = this.#styleUserIntent(exec, args.style_selection) ?? latestDirectUserText(exec)
        const canvas = draftCanvasContract(brief, directUserText)
        const commerceIntent = COMMERCE_BRIEF.test(directUserText ?? brief)
        const briefText = directUserText ?? brief
        // Opt-in style handoff (spec §4): validate the confirmed selection
        // against the current catalog, unchanged brief, platform, principal
        // and expiry BEFORE any draft or editor exists, and never silently
        // downgrade a confirmed choice to the ordinary automatic flow.
        const selection = this.#resolveStyleSelection(exec, args, brief, canvas, commerceIntent, engine)
        // App-alignment: match the brief onto the vendored style-guide and
        // domain-skill corpora. Desktop commerce keeps its hardwired builtin
        // direction (the generation contract pins its exact CTA colors), so a
        // guide only overrides where the contract text is palette-neutral.
        // A confirmed catalog selection bypasses automatic picking entirely.
        const fallbackPalette = commerceIntent ? ECOMMERCE_PALETTE : DESIGN_PALETTE
        const styleGuide = selection !== undefined
          ? asSelectedStyleGuide(selection.guide, fallbackPalette)
          : commerceIntent && canvas.platform === 'web'
            ? undefined
            : selectStyleGuide(briefText, canvas.platform, fallbackPalette)
        const domainGuidance = selectDomainGuidance(briefText, canvas.platform)
        const owner = ownerSessionId(exec)
        // A duplicate begin is the classic recovery spiral: without the live
        // draftId in the error, the model starts spelunking for it. Hand the
        // exact continuation instead of a dead end.
        for (const [activeDraftId, active] of this.#pending) {
          if (active.ownerSessionId !== owner) continue
          const step = active.generationScriptCount === 0
            ? 'send batch 1'
            : active.generationScriptCount === 1
              ? 'send the second and final batch'
              : `call ${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}`
          throw new Error(
            `${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: this DSH session already has the active OpenPencil draft "${activeDraftId}" `
            + `for ${active.processPath} (generation scripts committed: ${active.generationScriptCount}/2). `
            + `Do not begin again: continue that exact draftId — next required call: ${step}.`,
          )
        }
        const policy = services.sandboxPolicy.resolve({ session: exec.agent?.session })
        if (services.fs.sandboxMode !== undefined && policy.mode === 'read-only') {
          throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: a design draft requires Workspace Write access`)
        }
        const pathInfo = await services.fs.lstat(requestedPath, { cwd: policy.workspaceRoot }, exec.signal)
        if (pathInfo !== undefined) throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: target already exists: ${requestedPath}`)
        const target = await services.fs.resolve(requestedPath, { cwd: policy.workspaceRoot, signal: exec.signal })
        const processPath = services.fs.processPath(target)
        if (!isAbsolute(processPath) || extname(processPath).toLowerCase() !== '.op') {
          throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: target must resolve to an absolute local .op path`)
        }
        let parentInfo
        try {
          parentInfo = await lstat(dirname(processPath))
        } catch {
          throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: target parent is not available to the local OpenPencil host`)
        }
        if (!parentInfo.isDirectory()) throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: target parent must be a local directory`)
        const resolvedInfo = await services.fs.stat(target, exec.signal)
        if (resolvedInfo !== undefined) {
          services.observe(target, { kind: 'present', version: resolvedInfo.version }, exec)
          throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: target already exists: ${requestedPath}`)
        }
        services.observe(target, { kind: 'absent' }, exec)

        const begun = await drafts.begin({
          ownerSessionId: owner,
          target: { id: String(target.targetKey), label: target.displayPath, kind: 'file' },
          signal: exec.signal,
        })
        try {
          const seeded = await drafts.call(begun.draftId, owner, 'batch_design', {
            script: seedCanvasScript(canvas, commerceIntent, styleGuide?.palette.page),
            postProcess: true,
            canvasWidth: canvas.width,
          }, { signal: exec.signal })
          if (seeded.changed !== true) {
            throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: native canvas seed did not create the root frame`)
          }
          const authoritative = await drafts.snapshot(begun.draftId, owner, { signal: exec.signal })
          const document = await (services.createDocumentSnapshot ?? createDocumentSnapshotFromText)(
            authoritative.documentJson,
          )
          const rootNodeId = draftRootNodeId(authoritative.documentJson)
          const canvasDiagnostics = canvasContractDiagnostics(authoritative.documentJson, canvas)
          if (rootNodeId === undefined || canvasDiagnostics.length > 0) {
            throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: native canvas seed did not produce the authoritative single root frame`)
          }
          this.#pending.set(begun.draftId, {
            ownerSessionId: owner,
            requestedPath,
            processPath,
            target,
            canvas,
            canvasValidated: true,
            contextCalls: new Set(),
            contextEnrichmentUsed: false,
            finalEnrichmentUsed: false,
            rootNodeId,
            commerceIntent,
            generationScriptCount: 0,
            generationCorrectionAttempted: new Set(),
            repairAuthorized: false,
            engine,
            brief,
            agentRunUsed: false,
            // The app-agent loop performs its own screenshot verification
            // turns, so the extra DSH visual-review round defaults OFF there
            // (an explicit skip_visual_review:false opts back in).
            skipVisualReview: engine === 'app-agent'
              ? args.skip_visual_review !== false
              : args.skip_visual_review === true,
            visualReviewDone: false,
            visualReviewAuthorized: false,
            repairAttemptCount: 0,
            ...(selection === undefined ? {} : { selectedStyle: asJson(selection.section) }),
          })
          if (selection !== undefined) this.#styleStore.consume(selection.handle)
          return {
            draftId: begun.draftId,
            path: processPath,
            version: authoritative.version,
            ...(begun.createdAt === undefined ? {} : { createdAt: begun.createdAt }),
            platform: canvas.platform,
            canvas: asJson(canvas),
            buildContract: {
              ...compactBuildContract(canvas, commerceIntent, domainGuidance) as Record<string, JsonValue>,
              ...(selection === undefined ? {} : { selectedStyle: asJson(selection.section) }),
            },
            rootNodeId,
            continuationStyle: designContinuationStyle(canvas, commerceIntent, styleGuide),
            editorState: {},
            styleGuideTags: styleGuide !== undefined
              ? { name: styleGuide.name, tags: asJson(styleGuide.tags.slice(0, 8)) }
              : commerceIntent
                ? { name: 'ecommerce-modern-light', tags: ['clean', 'light-mode', 'modern', 'rounded', 'warm-tones', 'landing-page'] }
                : { name: 'dsh-editorial-warm', tags: ['editorial', 'warm', 'product-ui'] },
            document: asJson(document),
            sourceTool: OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
            previewIntent: 'document',
            editable: true,
            autoOpenEditor: true,
            liveCanvas: true,
            published: false,
            next: engine === 'app-agent'
              ? `Without narration, call ${OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME} exactly once now with only this draftId (timeout_seconds/max_turns optional). It runs the App builtin design-agent loop on the daemon; when it returns, call ${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME} once. Never send generation batch scripts on this draft.`
              : `Without narration, send batch 1 now using script.wrapper and const root="${rootNodeId}": <=32 I/K for a complete header/nav + polished hero. Follow generation.first exactly; bind frame/group containers and add their children immediately. text_input is a leaf, so icon+hint search uses a named Search frame wrapper. The ${canvas.platform} platform is final for this draft; never abort/rebegin to evade a gate. Follow generation.second after success.`,
          } as Record<string, JsonValue>
        } catch (error) {
          await drafts.abort(begun.draftId, owner).catch(() => {})
          throw error
        }
      },
      presentCall: (args: { path?: string }) => args.path === undefined
        ? { card: 'generic', title: 'Begin OpenPencil pipeline', kind: 'execute' }
        : { card: 'generic', title: `Begin OpenPencil pipeline for ${args.path}`, kind: 'execute', locations: [{ path: args.path }] },
    })
  }

  #styleUserIntent(exec: ToolRunContext, handle: string | undefined): string | undefined {
    if (!handle?.trim()) return undefined
    const verdict = this.#styleStore.validate(handle.trim(), { sessionKey: ownerSessionId(exec), now: this.#styleNow() })
    if (!verdict.ok) return undefined
    const current = latestDirectUserText(exec)
    if (verdict.record.confirmedUserFingerprint !== undefined
      && (current === undefined || this.#styleStore.briefFingerprintOf(current) !== verdict.record.confirmedUserFingerprint)) {
      throw new Error(`${OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME}: selection_error: the user request changed after confirmation; replan or reconfirm before creating a draft`)
    }
    return verdict.record.userIntent
  }

  /**
   * Resolve the optional confirmed style selection (spec §4) before any draft
   * or editor exists: unsupported modes are explained, the handle is validated
   * against the current catalog, the unchanged brief, the platform, the
   * owning session and its 15-minute expiry, and a confirmed-choice session
   * can never masquerade as an ordinary call by omitting the handle. Returns
   * the selected guide plus its immutable contract section, or `undefined`
   * for the ordinary automatic flow.
   */
  #resolveStyleSelection(
    exec: ToolRunContext,
    args: { style_selection?: string },
    briefText: string,
    canvas: DraftCanvasContract,
    commerceIntent: boolean,
    engine: PipelineEngine,
  ): { guide: StyleCatalogGuide; section: SelectedStyleSection; handle: string } | undefined {
    const name = OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME
    const sessionKey = ownerSessionId(exec)
    const fingerprint = this.#styleStore.briefFingerprintOf(briefText)
    const handle = args.style_selection?.trim()
    if (handle === undefined || handle === '') {
      const live = this.#styleStore.liveConfirmedFor(sessionKey, fingerprint, this.#styleNow())
      if (live !== undefined) {
        throw new Error(
          `${name}: selection_error: this session has a confirmed style choice for this brief (${live.guideId}); `
          + 'pass its live handle in style_selection, or have the designer explicitly abandon the selection before requesting ordinary generation',
        )
      }
      return undefined
    }
    if (!this.#styleHandoffEnabled()) {
      throw new Error(`${name}: selection_error: selection_handoff_unsupported: this host does not expose the style-selection handoff`)
    }
    if (engine === 'app-agent') {
      throw new Error(`${name}: selection_error: app-agent handoffs are unsupported in v1; a confirmed selection applies only to the ordinary script engine`)
    }
    if (commerceIntent) {
      throw new Error(
        `${name}: selection_error: ${canvas.platform === 'web' ? 'web-commerce' : 'commerce'} handoffs are unsupported in v1; `
        + 'web commerce keeps its pinned ecommerce direction and no chosen catalog guide overrides it',
      )
    }
    const snapshot = this.#resolveStyleCatalog()
    const verdict = this.#styleStore.validate(handle, {
      sessionKey,
      briefFingerprint: fingerprint,
      platform: canvas.platform,
      catalogRevision: snapshot.revision,
      constraintRevision: this.#styleStore.session(sessionKey)?.constraintRevision,
      now: this.#styleNow(),
    })
    if (!verdict.ok) {
      const detail: Record<string, string> = {
        unknown: 'unknown selection handle',
        consumed: 'the selection handle was already used by a draft; reconfirm the choice for a new draft',
        expired: 'the selection handle expired (15-minute lifetime); revalidate and reconfirm the choice',
        session: 'the selection handle is bound to another DSH session or principal',
        brief: 'the brief changed after confirmation; reconfirm the choice against the unchanged brief',
        platform: 'the selection handle was confirmed for a different platform',
        constraints: 'the confirmed constraints changed after confirmation',
        catalog: 'the catalog changed after confirmation (catalog drift); reconfirm to bind the new catalog revision',
        guide: 'the selected guide no longer matches the handle',
      }
      throw new Error(`${name}: selection_error: ${detail[verdict.reason]}`)
    }
    const guide = snapshot.guides.find(candidate => candidate.id === verdict.record.guideId)
    if (guide === undefined) {
      throw new Error(`${name}: selection_error: the confirmed guide is no longer installed in the catalog; reconfirm against the current catalog`)
    }
    const catalogPlatform = canvas.platform === 'web' ? 'webapp' : 'mobile'
    if (guide.platform !== catalogPlatform) {
      throw new Error(`${name}: selection_error: the confirmed guide (${guide.platform}) does not support this draft's ${canvas.platform} platform`)
    }
    return {
      guide,
      handle,
      section: selectedStyleOf(guide, snapshot, commerceIntent ? ECOMMERCE_PALETTE : DESIGN_PALETTE),
    }
  }

  #contextTool() {
    const drafts = this.#drafts
    return defineTool({
      name: OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME,
      description: 'Read native design context for an unpublished OpenPencil pipeline draft. '
        + 'Use it only for context not already returned by pipeline_begin. The allowlist includes bounded design reads and draft-local design-system/theme configuration; '
        + 'filesystem paths, URLs, imports, exports, and spawned agents are forbidden.',
      parameters: {
        draftId: { type: 'string', required: true, description: 'Draft id returned by openpencil_pipeline_begin.' },
        tool: { type: 'string', required: true, enum: [...CONTEXT_TOOLS], description: 'Allowed native OpenPencil context tool.' },
        arguments: { type: 'object', additionalProperties: true, description: 'Native tool arguments. Do not pass file/path/URL/import/export/spawn fields.' },
      },
      output: { schema: { type: 'object', additionalProperties: true }, render: renderJson },
      execute: async (args: { draftId: string; tool: ContextTool; arguments?: Record<string, unknown> }, exec) => {
        const owner = ownerSessionId(exec)
        const pending = this.#requirePending(args.draftId, owner)
        if (!CONTEXT_TOOLS.includes(args.tool)) throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: tool is not allowed`)
        const nativeArgs = { ...(args.arguments ?? {}) }
        assertNoExternalArguments(nativeArgs)
        // The native tool hard-requires a topic; a bare call is the common
        // model slip, so default it from the draft platform instead of
        // surfacing a raw MCP argument error mid-transaction.
        if (args.tool === 'get_guidelines' && typeof nativeArgs.topic !== 'string') {
          nativeArgs.topic = pending.canvas.platform === 'mobile' ? 'mobile' : 'web-app'
        }
        if (args.tool === 'enrich_images') {
          for (const key of Object.keys(nativeArgs)) {
            if (key !== 'timeout_seconds' && key !== 'root_ids') {
              throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: enrich_images only accepts timeout_seconds and root_ids`)
            }
          }
          if (nativeArgs.root_ids !== undefined && (
            !Array.isArray(nativeArgs.root_ids)
            || !nativeArgs.root_ids.every(id => typeof id === 'string' && id.trim().length > 0)
          )) throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: enrich_images root_ids must be an array of node ids`)
        }
        const fingerprint = contextFingerprint(args.tool, nativeArgs)
        if (pending.contextCalls.has(fingerprint)) {
          throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: this exact native context request was already consumed; continue from the prior result`)
        }
        if (args.tool === 'enrich_images') {
          if (pending.contextEnrichmentUsed) {
            throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: enrich_images already ran for this draft`)
          }
        } else if (pending.contextCalls.size >= MAX_TARGETED_CONTEXT_CALLS) {
          throw new Error(`${OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME}: targeted context budget exhausted; continue with the compact begin contract and current draft`)
        }
        const result = await drafts.call(args.draftId, owner, args.tool, nativeArgs, { signal: exec.signal })
        pending.contextCalls.add(fingerprint)
        if (args.tool === 'enrich_images') pending.contextEnrichmentUsed = true
        if (result.changed) clearFinalizationCheckpoint(pending)
        return { draftId: args.draftId, ...publicCall(result) }
      },
      presentCall: (args: { tool: string }) => ({ card: 'generic', title: `Read OpenPencil draft context: ${args.tool}`, kind: 'read' }),
    })
  }

  #batchTool() {
    const drafts = this.#drafts
    return defineTool({
      name: OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
      description: 'Commit the next QuickJS step from begin.buildContract and return its live-canvas preview. Generation is exactly two I/K scripts; only a complete finish.repairTargets array may authorize one later U-only script.',
      parameters: {
        draftId: { type: 'string', required: true },
        script: { type: 'string', required: true, description: 'Standalone sandboxed QuickJS. In run_code declare String.raw`...`, call the fixed {draftId,script} object, and return r. Follow begin.buildContract and the latest next exactly.' },
      },
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: renderJson,
        presentationMeta: (_args: unknown, value: JsonValue): JsonValue => projectScreenshotPresentation(value, this.#services.render),
      },
      execute: async (args: { draftId: string; script?: string; pageId?: string; canvasWidth?: number }, exec) => {
        const owner = ownerSessionId(exec)
        const pending = this.#requirePending(args.draftId, owner)
        const script = args.script?.trim() ?? ''
        if (script === '') {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: provide one non-empty QuickJS script`)
        }
        if (pending.engine === 'app-agent' && pending.generationScriptCount < 2) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: this draft uses the app-agent engine; call ${OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME} once instead of generation scripts`)
        }
        const postGeneration = pending.generationScriptCount >= 2
        // A repair authorization (gate failures with exact targets) always
        // outranks the visual-review authorization; the two are set by
        // disjoint finish outcomes and never coexist on a healthy draft.
        const visualScript = postGeneration && !pending.repairAuthorized && pending.visualReviewAuthorized
        const repairScript = postGeneration && !visualScript
        if (!postGeneration && /\b[UCDMRG]\s*\(/u.test(script)) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: generation scripts may only create with I/K; finish must authorize a later U() repair script`)
        }
        if (repairScript && !pending.repairAuthorized) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: ordinary generation is limited to two direct QuickJS scripts; call finish before any repair script`)
        }
        if (repairScript && (!/\bU\s*\(/u.test(script) || /\b[IKCDMRG]\s*\(/u.test(script))) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: the authorized repair must be one bounded QuickJS script containing only U(nodeId, patch) mutations`)
        }
        if (visualScript) {
          const violation = visualReviewScriptViolation(script)
          if (violation !== undefined) {
            throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: ${violation}`)
          }
        }
        let effectiveScript = script
        if (!repairScript && /\bI\s*\(\s*null\s*,/u.test(effectiveScript)) {
          // The classic slip: the model creates its own page wrapper with
          // I(null, {...}) instead of using begin.rootNodeId. The canonical
          // single-wrapper form heals in place — the binding becomes the
          // existing root id and the redundant wrapper object is dropped —
          // so the very common mistake costs zero round trips. Any other
          // I(null, ...) still rejects with the exact rewrite.
          const healed = healRootWrapperScript(effectiveScript, pending.rootNodeId)
          if (healed !== undefined) effectiveScript = healed
          if (/\bI\s*\(\s*null\s*,/u.test(effectiveScript)) {
            throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: the page root already exists — replace const page = I(null, {...}) with const page = ${JSON.stringify(pending.rootNodeId)}; and attach every top-level region to it, then resend the corrected script once`)
          }
        }
        // A bare Inter face triggers the editor's missing-font prompt on
        // machines without a local Inter install; normalize it to the
        // portable stack the contract mandates.
        effectiveScript = effectiveScript.replace(
          /fontFamily\s*:\s*(["'])Inter\1/gu,
          'fontFamily:"Inter, system-ui, sans-serif"',
        )
        // The two classic wrapper mistakes produce opaque QuickJS syntax
        // errors ("return not in a function", "expecting ';'") that send the
        // model into blind retries. Reject them with the exact correction.
        if (/\btools\s*\.\s*openpencil_pipeline_batch\b/u.test(script)
          || /\bString\s*\.\s*raw\b/u.test(script)
          || /^\s*return\b/mu.test(script)) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: the script field is the QuickJS body itself — never nest the run_code wrapper (const draftId, String.raw, tools.${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}, return r) inside it. Start the script directly with const root="${pending.rootNodeId}"; followed by plain I(parent, node) / K(...) calls, and resend it once corrected.`)
        }
        if (/(?:\b(?:const|let|var)\s+[IK]\s*=|\bfunction\s+[IK]\s*\()/u.test(script)) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: I and K are host-provided creation helpers — never redefine them. Call I(parent, node) directly and resend the corrected script once.`)
        }
        if (!repairScript && hasMemberMutation(effectiveScript)) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: I/K bindings are opaque node-id strings and cannot be mutated; put properties inside the I/K object and never assign binding.x, binding.y, or any member`)
        }
        if (!repairScript && pending.generationScriptCount === 0 && (
          Buffer.byteLength(effectiveScript) > MAX_FIRST_GENERATION_BYTES
          || creationCallCount(effectiveScript) > MAX_FIRST_GENERATION_CALLS
        )) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: the first live-preview script is limited to ${MAX_FIRST_GENERATION_CALLS} I/K calls and ${MAX_FIRST_GENERATION_BYTES} bytes; keep only the first visible viewport named by begin.next`)
        }
        if (effectiveScript.length > MAX_BATCH_LENGTH) throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: batch source is too large`)
        if (args.canvasWidth !== undefined && (!Number.isFinite(args.canvasWidth) || args.canvasWidth <= 0 || args.canvasWidth > 16_384)) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: canvasWidth must be greater than 0 and at most 16384`)
        }
        if (args.canvasWidth !== undefined && args.canvasWidth !== pending.canvas.width) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: canvasWidth must match the ${pending.canvas.width}px begin canvas contract`)
        }
        const generationStage = pending.generationScriptCount
        const beforeGeneration = !postGeneration
          ? await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
          : undefined
        const callBatchDesign = () => drafts.call(args.draftId, owner, 'batch_design', {
          script: effectiveScript,
          postProcess: true,
          ...(args.pageId === undefined || args.pageId.trim() === '' ? {} : { pageId: args.pageId }),
          canvasWidth: pending.canvas.width,
        }, {
          signal: exec.signal,
          ...(beforeGeneration === undefined ? {} : { expectedVersion: beforeGeneration.version }),
        })
        let batch: Awaited<ReturnType<typeof callBatchDesign>>
        try {
          batch = await callBatchDesign()
        } catch (error) {
          exec.signal?.throwIfAborted()
          // A raw QuickJS failure ("script error: expecting ';'") gives the
          // model nothing to correct against; restate the exact body shape.
          if (error instanceof Error && /script error/iu.test(error.message)) {
            throw new Error(`${error.message}. The script field must be one standalone QuickJS body: start with const root="${pending.rootNodeId}"; use only plain I(parent, node) / K(...) calls with literal values, no imports, no return statements, and no nested run_code wrapper. Resend one complete corrected script.`)
          }
          throw error
        }
        if (repairScript) {
          pending.repairAuthorized = false
          pending.repairAttemptCount += 1
        }
        if (visualScript) pending.visualReviewAuthorized = false
        if (batch.changed) clearFinalizationCheckpoint(pending)
        let snapshot = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
        if (snapshot.version !== batch.version) {
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: 'blocked_generation_validation',
            version: snapshot.version,
            diagnostics: ['The live canvas changed while the generation batch was being validated.'],
            canContinue: false,
            next: 'Stop and report the concurrent live-canvas change once. Do not retry, guess a repair, abort, or rebuild another draft.',
          } as Record<string, JsonValue>
        }
        if (!repairScript && batch.changed === true && beforeGeneration !== undefined) {
          const structure = inspectGeneratedDraftStructureReport(
            snapshot.documentJson,
            generationStage === 0 && pending.commerceIntent && pending.canvas.width >= 900,
          )
          if (structure.diagnostics.length > 0) {
            let restored: DesignDraftSnapshot
            try {
              restored = await drafts.restoreSnapshot(
                args.draftId,
                owner,
                beforeGeneration,
                { signal: exec.signal, expectedVersion: snapshot.version },
              )
            } catch {
              return {
                draftId: args.draftId,
                path: pending.processPath,
                published: false,
                stage: 'blocked_generation_validation',
                version: snapshot.version,
                diagnostics: structure.diagnostics,
                canContinue: false,
                next: 'The invalid second batch could not be safely rolled back because the live canvas changed. Stop and report this once; do not retry or overwrite the editor.',
              } as Record<string, JsonValue>
            }
            clearFinalizationCheckpoint(pending)
            pending.canvasValidated = canvasContractDiagnostics(restored.documentJson, pending.canvas).length === 0
            const canCorrect = !pending.generationCorrectionAttempted.has(generationStage)
            pending.generationCorrectionAttempted.add(generationStage)
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: canCorrect ? 'needs_generation_correction' : 'blocked_generation_validation',
              version: restored.version,
              changed: false,
              rolledBack: true,
              generationScriptCount: pending.generationScriptCount,
              generationScriptLimit: 2,
              diagnostics: structure.diagnostics,
              canContinue: canCorrect,
              next: canCorrect
                ? generationStage === 0
                  ? 'Without narration, resend the complete corrected first script once: the rollback removed both Header/Nav and Hero, so recreate both. Follow begin.buildContract.generation.first exactly and use literal hex colors, no aliases. Bind the Header first (const header=I(root,{role:"navbar",height:64,padding:[0,160],justifyContent:"space_between"})) and insert Brand, Nav, and Actions through that binding, never as root siblings; an empty role container invalidates the batch again. Nav role nav-links, with every 44px role nav-link on a frame containing its text child, never on text; Header actions role toolbar with 44x44 role icon-button wrappers. CTA role button 160x48 #C2410C/#FFFFFF; insert its label through the CTA binding. Hero is width:"fill_container" horizontal padding:[64,160], containing copy width512 + gap64 + image width448; never combine width1120 with padding. Headline and subtitle each use width:"fill_container". Keep all non-brand copy in the user language. For generic commerce create the image directly under Hero exactly as I(hero,{type:"image",name:"Hero product image",width:448,height:360,imageSearchQuery:"gray loveseat isolated photo"}); keep that query distinct from every product card; never use a wrapper, image:{...}, or shapes. No blank field or overflow.'
                  : `Without narration, resend only the corrected second script once. ${SECOND_BATCH_SCOPE_INSTRUCTION} Use the nested 56x56 face exactly; keep every category and product card complete, and fix every reported rail height/width/overflow issue in this replacement batch. If a diagnostic reports duplicate image queries or assets, keep the Hero query untouched and give each flagged card a different concrete single-product English query.`
                : `Stop and report the repeated invalid ${generationStage === 0 ? 'first' : 'second'}-generation structure once. Do not retry, inspect, abort, or rebuild another draft.`,
            } as Record<string, JsonValue>
          }
        }
        if (batch.changed === true && !postGeneration) {
          pending.generationScriptCount += 1
          // The App fills authored image slots as soon as their section lands,
          // so its canvas becomes a real design rather than a wireframe. Keep
          // DSH's two-script speed, but automatically do the same after each
          // commerce generation batch commits. Failure is preview-only:
          // finish still owns one canonical post-final enrichment attempt.
          if (pending.commerceIntent && hasUnresolvedImageSearchQuery(snapshot.documentJson)) {
            try {
              const enriched = await drafts.call(
                args.draftId,
                owner,
                'enrich_images',
                { timeout_seconds: 8 },
                { signal: exec.signal, expectedVersion: snapshot.version },
              )
              pending.contextEnrichmentUsed = true
              if (enriched.changed) clearFinalizationCheckpoint(pending)
              const enrichedSnapshot = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
              if (enriched.version === enrichedSnapshot.version) snapshot = enrichedSnapshot
            } catch {
              // Preserve the committed live canvas. The finalizer may reshape
              // slots and finish will retry canonical enrichment once.
            }
          }
        }
        const version = snapshot.version
        const rootNodeId = draftRootNodeId(snapshot.documentJson)
        const canvasDiagnostics = canvasContractDiagnostics(snapshot.documentJson, pending.canvas)
        if (rootNodeId !== pending.rootNodeId) {
          throw new Error(`${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME}: authoritative root changed unexpectedly`)
        }
        pending.canvasValidated = canvasDiagnostics.length === 0
        // Batch receipts are a live-canvas checkpoint, not a repair phase.
        // Native layout/quality diagnostics are rerun and aggregated by finish,
        // where exact repair targets can be authorized atomically. Surfacing
        // them between generation batches makes models abandon the returned
        // `next` contract and start speculative repair loops.
        const diagnostics = canvasDiagnostics.slice(0, 30)
        let screenshot: InspectionScreenshotArtifact | undefined
        let previewUnavailable = false
        if (batch.changed === true) {
          try {
            const rendered = await drafts.screenshot(args.draftId, owner, { signal: exec.signal })
            const artifact = await persistInspectionScreenshot(rendered)
            pending.latestRootScreenshot = {
              version: rendered.version,
              documentSha256: rendered.documentSha256,
              finalized: false,
              ...artifact,
            }
            screenshot = artifact
          } catch {
            // The design transaction already committed. Report a recoverable
            // preview-only state so the model never repeats the creation
            // script and duplicates the page after a transient render error.
            previewUnavailable = true
          }
        }
        return {
          draftId: args.draftId,
          version,
          changed: batch.changed === true,
          generationScriptCount: pending.generationScriptCount,
          generationScriptLimit: 2,
          ...(rootNodeId === undefined ? {} : { rootNodeId }),
          batch: publicBatchReceipt(batch.value),
          ...(pending.selectedStyle === undefined ? {} : { selectedStyle: pending.selectedStyle }),
          canvas: asJson(pending.canvas),
          canvasCheck: {
            valid: pending.canvasValidated,
            diagnostics: canvasDiagnostics,
          },
          ...(screenshot === undefined ? {} : { screenshot: asJson(screenshot) }),
          ...(previewUnavailable ? { previewUnavailable: true } : {}),
          diagnostics,
          canContinue: true,
          next: diagnostics.length > 0
            ? pending.generationScriptCount < 2
              ? `The committed preview includes native diagnostics. Without narration or inspection, continue with ${pending.generationScriptCount === 0 ? 'the first bounded visible-viewport' : 'the second and final completion'} JS script. ${pending.generationScriptCount === 1 ? `${SECOND_BATCH_SCOPE_INSTRUCTION} ` : ''}Finish will validate the completed page.`
              : 'The complete composition preview includes native diagnostics. Call finish once so it can aggregate and authorize one exact-id repair transaction.'
            : previewUnavailable
              ? pending.generationScriptCount < 2
                ? `The committed JS is visible on the live canvas, but its inline PNG user preview was temporarily unavailable. Do not rerun it. Continue with ${pending.generationScriptCount === 0 ? 'the first bounded visible-viewport' : 'the second and final completion'} JS script. ${pending.generationScriptCount === 1 ? `${SECOND_BATCH_SCOPE_INSTRUCTION} ` : ''}Finish will render the final preview automatically.`
                : 'The completed JS composition is visible on the live canvas, but its inline PNG user preview was temporarily unavailable. Do not rerun it or call inspect. Call finish once; it will render the final preview automatically.'
              : pending.generationScriptCount === 0
                ? 'No visible generation change was committed. Send the bounded first-visible-viewport QuickJS script now.'
                : pending.generationScriptCount === 1
                  ? `Without narration, send the second and final QuickJS script now. ${SECOND_BATCH_SCOPE_INSTRUCTION} Each media frame gets one primary visual; outside commerce default to one image total unless the user explicitly requested more. Then call finish once.`
                  : 'Without narration, call finish exactly once now. It will validate, render the final user preview, and publish atomically.',
        }
      },
      presentCall: () => ({ card: 'generic', title: 'Build OpenPencil draft batch', kind: 'execute' }),
    })
  }


  #agentRunTool() {
    const drafts = this.#drafts
    return defineTool({
      name: OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME,
      description: 'Run the OpenPencil App builtin design-agent loop on the daemon for an app-agent pipeline draft. '
        + 'Exactly one call per draft, only when begin selected engine:"app-agent"; it blocks until the loop lands its atomic result, then finish validates and publishes as usual.',
      parameters: {
        draftId: { type: 'string', required: true, description: 'Draft id returned by openpencil_pipeline_begin with engine:"app-agent".' },
        timeout_seconds: { type: 'number', description: 'Optional loop wall clock (30-270, default 240). One blocking call must fit the plugin transport budget.' },
        max_turns: { type: 'number', description: 'Optional loop turn budget (4-28, default 12, sized to the tighter wall clock).' },
      },
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: renderJson,
        presentationMeta: (_args: unknown, value: JsonValue): JsonValue => projectScreenshotPresentation(value, this.#services.render),
      },
      execute: async (args: { draftId: string; timeout_seconds?: number; max_turns?: number }, exec) => {
        const owner = ownerSessionId(exec)
        const pending = this.#requirePending(args.draftId, owner)
        if (pending.engine !== 'app-agent') {
          throw new Error(`${OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME}: this draft uses the script engine; follow begin.next with generation batches instead`)
        }
        if (pending.agentRunUsed) {
          throw new Error(`${OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME}: the single agent run was already used; call ${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME} once now`)
        }
        const timeoutSeconds = validateAgentRunTimeoutSeconds(args.timeout_seconds, OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME)
        const maxTurns = validateAgentRunMaxTurns(args.max_turns, OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME)
        pending.agentRunUsed = true
        let run
        try {
          run = await drafts.call(args.draftId, owner, 'run_design_agent', {
            brief: pending.brief,
            timeout_seconds: timeoutSeconds,
            max_turns: maxTurns,
          }, { signal: exec.signal })
        } catch (error) {
          exec.signal?.throwIfAborted()
          // Provider/key configuration problems are terminal operator states,
          // never retry targets: surface the daemon message plus the fix path.
          if (error instanceof Error && isProviderConfigurationError(error.message)) {
            throw new Error(`${error.message}. ${AGENT_RUN_PROVIDER_GUIDANCE}`)
          }
          throw error
        }
        clearFinalizationCheckpoint(pending)
        const summary = summarizeAgentRun(run.value as JsonValue | undefined)
        const snapshot = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
        const rootNodeId = draftRootNodeId(snapshot.documentJson)
        pending.canvasValidated = canvasContractDiagnostics(snapshot.documentJson, pending.canvas).length === 0
        if (rootNodeId !== undefined) pending.rootNodeId = rootNodeId
        // The loop owns both generation slots: later scripts must route
        // through the ordinary finish-authorized repair path only.
        pending.generationScriptCount = 2
        if (run.changed !== true || summary.landedRoots === 0 || summary.loopError !== undefined) {
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: 'blocked_agent_run',
            version: snapshot.version,
            agentRun: asJson(summary as unknown as JsonValue),
            diagnostics: summary.loopError === undefined ? [] : [summary.loopError],
            canContinue: false,
            next: 'The daemon design-agent loop landed no usable result. Stop and report this once; do not retry, script, or rebuild another draft.',
          } as Record<string, JsonValue>
        }
        let screenshot: InspectionScreenshotArtifact | undefined
        let previewUnavailable = false
        try {
          const rendered = await drafts.screenshot(args.draftId, owner, { signal: exec.signal })
          const artifact = await persistInspectionScreenshot(rendered)
          pending.latestRootScreenshot = {
            version: rendered.version,
            documentSha256: rendered.documentSha256,
            finalized: false,
            ...artifact,
          }
          screenshot = artifact
        } catch {
          previewUnavailable = true
        }
        return {
          draftId: args.draftId,
          version: snapshot.version,
          changed: true,
          agentRun: asJson(summary as unknown as JsonValue),
          ...(screenshot === undefined ? {} : { screenshot: asJson(screenshot) }),
          ...(previewUnavailable ? { previewUnavailable: true } : {}),
          canContinue: true,
          next: `Without narration, call ${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME} exactly once now. It will validate, render the final user preview, and publish atomically.`,
        } as Record<string, JsonValue>
      },
      presentCall: () => ({ card: 'generic', title: 'Run OpenPencil App design agent', kind: 'execute' }),
    })
  }

  #inspectTool() {
    const drafts = this.#drafts
    return defineTool({
      name: OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
      description: 'Inspect an unpublished draft using native resolved layout, composite quality/lint, or an exact PNG screenshot. '
        + 'Layout inspection returns the resolved node array directly as tree (not a nested layout.layout envelope). '
        + 'Screenshot returns a bounded content-addressed DSH cache path for the user preview and records render-integrity metadata for that exact draft version. '
        + 'This tool is optional and user-directed; finish renders its own final root PNG and never requires model image inspection.',
      parameters: {
        draftId: { type: 'string', required: true },
        kind: { type: 'string', required: true, enum: ['layout', 'quality', 'screenshot'] },
        nodeId: { type: 'string', description: 'Optional node id for screenshot. Omit for the root design.' },
        maxDepth: {
          type: 'number',
          description: 'Optional layout-only depth, default 6 and max 12. Omit for quality and screenshot inspection.',
        },
      },
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: renderJson,
        presentationMeta: inspectionPresentationMeta(this.#services.render),
      },
      execute: async (args: { draftId: string; kind: 'layout' | 'quality' | 'screenshot'; nodeId?: string; maxDepth?: number }, exec) => {
        const owner = ownerSessionId(exec)
        this.#requirePending(args.draftId, owner)
        if (args.kind === 'layout') {
          const maxDepth = args.maxDepth ?? 6
          if (!Number.isSafeInteger(maxDepth) || maxDepth < 1 || maxDepth > 12) {
            throw new Error(`${OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME}: maxDepth must be an integer from 1 through 12`)
          }
          const layout = await drafts.call(args.draftId, owner, 'snapshot_layout', { maxDepth }, { signal: exec.signal })
          const rawLayout = layout.value
          const tree = isRecord(rawLayout) && 'layout' in rawLayout
            ? rawLayout.layout
            : rawLayout
          return {
            draftId: args.draftId,
            kind: args.kind,
            version: layout.version,
            tree: asJson(tree),
            diagnostics: issueValues(rawLayout),
          } as Record<string, JsonValue>
        }
        if (args.kind === 'quality') {
          const quality = await drafts.call(args.draftId, owner, 'get_design_quality', {}, { signal: exec.signal })
          const lint = await drafts.call(args.draftId, owner, 'lint_document', {}, { signal: exec.signal })
          return {
            draftId: args.draftId,
            kind: args.kind,
            version: lint.version,
            quality: asJson(quality.value),
            lint: asJson(lint.value),
            diagnostics: issueValues({ quality: quality.value, lint: lint.value }),
          } as Record<string, JsonValue>
        }
        const requestedNodeId = args.nodeId?.trim()
        const rootScreenshot = requestedNodeId === undefined || requestedNodeId === '' || requestedNodeId === 'root'
        const screenshot = await drafts.screenshot(args.draftId, owner, {
          ...(rootScreenshot ? {} : { nodeId: requestedNodeId }),
          signal: exec.signal,
        })
        const artifact = await persistInspectionScreenshot(screenshot)
        const pending = this.#requirePending(args.draftId, owner)
        const screenshotMatchesFinalization = rootScreenshot
          && pending.finalization?.version === screenshot.version
          && pending.finalization.documentSha256 === screenshot.documentSha256
        if (rootScreenshot) {
          pending.latestRootScreenshot = {
            version: screenshot.version,
            documentSha256: screenshot.documentSha256,
            finalized: screenshotMatchesFinalization,
            ...artifact,
          }
        }
        const completionInstruction = !rootScreenshot
          ? 'This is a child-node detail preview only; finish will still render its own finalized root preview before publication.'
          : screenshotMatchesFinalization
            ? 'This root user preview matches the finalized version and document SHA checkpoint; call finish exactly once to publish.'
            : 'This root user preview predates a matching finalized checkpoint; call finish once to finalize, render its own final preview, and publish.'
        return {
          draftId: args.draftId,
          kind: args.kind,
          version: screenshot.version,
          screenshot: artifact,
          next: `The exact user preview is ready at ${artifact.path}. No model image inspection is required. ${completionInstruction}`,
        } as Record<string, JsonValue>
      },
      presentCall: (args: { kind: string }) => ({ card: 'generic', title: `Inspect OpenPencil draft: ${args.kind}`, kind: 'read' }),
    })
  }

  #finishTool() {
    const drafts = this.#drafts
    const services = this.#services
    return defineTool({
      name: OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
      description: 'Validate, render the final user preview, and atomically publish a completed two-batch draft. A complete structured repairTargets array may authorize one U-only repair, and a clean gate yields one needs_visual_review round whose digest/checklist may authorize one bounded I/K/U correction batch; any unstructured validation or host failure is terminal and must not be retried.',
      parameters: {
        draftId: { type: 'string', required: true, description: 'Draft id returned by pipeline_begin.' },
      },
      output: {
        schema: { type: 'object', additionalProperties: true },
        render: renderJson,
        presentationMeta: presentationMeta(this.#editorHost, services.render),
      },
      execute: async (args: { draftId: string }, exec) => {
        const owner = ownerSessionId(exec)
        const pending = this.#requirePending(args.draftId, owner)
        if (pending.generationScriptCount < 2) {
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: 'needs_generation',
            generationScriptCount: pending.generationScriptCount,
            generationScriptLimit: 2,
            diagnostics: [],
            canContinue: true,
            next: pending.generationScriptCount === 0
              ? 'Send the bounded first-visible-viewport QuickJS script. Its preview will render automatically.'
              : `Without narration, send the second and final direct JS script now. ${SECOND_BATCH_SCOPE_INSTRUCTION} Its preview will render automatically, then call finish once.`,
          }
        }
        const beforeFinalize = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
        const beforeSha256 = documentSha256(beforeFinalize.documentJson)
        let checkpoint = pending.finalization
        let reusedFinalization = checkpoint !== undefined
          && checkpoint.version === beforeFinalize.version
          && checkpoint.documentSha256 === beforeSha256
        let finalizedSnapshot = beforeFinalize
        if (!reusedFinalization) {
          let finalized: DesignDraftCallResult
          try {
            finalized = await drafts.finalize(args.draftId, owner, { signal: exec.signal })
          } catch (error) {
            pending.repairAuthorized = false
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: 'blocked_host_failure',
              version: beforeFinalize.version,
              diagnostics: [error instanceof Error ? error.message.slice(0, 500) : 'OpenPencil native finalization failed.'],
              canContinue: false,
              next: 'Stop and report this host failure once. Do not retry finish, inspect/context-read, guess a repair, abort, or rebuild another draft.',
            }
          }
          const nativeFinalizedSnapshot = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
          if (finalized.version !== nativeFinalizedSnapshot.version) {
            throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: native finalization version did not match its authoritative snapshot`)
          }
          finalizedSnapshot = nativeFinalizedSnapshot
          let enrichment: JsonValue | undefined
          // Match the App/native contract: finalization may materialize or
          // reshape image slots, so stock enrichment must see the canonical
          // post-final tree. Running it first left finalizer-created slots empty
          // and published grey placeholders.
          if (!pending.finalEnrichmentUsed && hasUnresolvedImageSearchQuery(finalizedSnapshot.documentJson)) {
            const enriched = await drafts.call(
              args.draftId,
              owner,
              'enrich_images',
              { timeout_seconds: 20 },
              { signal: exec.signal },
            )
            pending.finalEnrichmentUsed = true
            enrichment = asJson(enriched.value)
            finalizedSnapshot = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
            if (enriched.version !== finalizedSnapshot.version) {
              throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: image enrichment version did not match its authoritative snapshot`)
            }
          }
          const finalizedSha256 = documentSha256(finalizedSnapshot.documentJson)
          const versionChanged = finalizedSnapshot.version !== beforeFinalize.version
          const documentChanged = finalizedSha256 !== beforeSha256
          if (documentChanged && !versionChanged) {
            throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: native finalization changed document bytes without advancing its version`)
          }
          checkpoint = {
            version: finalizedSnapshot.version,
            documentSha256: finalizedSha256,
            beforeVersion: beforeFinalize.version,
            versionChanged,
            documentChanged,
            result: enrichment === undefined
              ? asJson(finalized.value)
              : asJson({ finalize: finalized.value, enrichment }),
          }
          pending.finalization = checkpoint
          reusedFinalization = false
        }
        if (checkpoint === undefined) {
          throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: finalized checkpoint was not recorded`)
        }
        const finalizedRootNodeId = draftRootNodeId(finalizedSnapshot.documentJson)
        if (finalizedRootNodeId === undefined) {
          throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: native finalization did not preserve one valid page root`)
        }
        // Native finalization may transactionally replace the root while
        // preserving the one-root canvas contract. Subsequent authorized U()
        // repair scripts never create under that root, but the post-commit
        // invariant must follow the authoritative finalized id instead of
        // throwing after a successful repair mutation.
        pending.rootNodeId = finalizedRootNodeId
        let validation = checkpoint.validation
        if (validation === undefined) {
          const canvasDiagnostics = canvasContractDiagnostics(
            finalizedSnapshot.documentJson,
            pending.canvas,
            { requireFinalHeight: true },
          )
          const canvasRepairTargets = canvasContractRepairTargets(
            finalizedSnapshot.documentJson,
            pending.canvas,
            { requireFinalHeight: true },
          )
          const quality = await drafts.call(args.draftId, owner, 'get_design_quality', {}, { signal: exec.signal })
          const lint = await drafts.call(args.draftId, owner, 'lint_document', {}, { signal: exec.signal })
          const layout = await drafts.call(args.draftId, owner, 'snapshot_layout', { maxDepth: 8 }, { signal: exec.signal })
          const blockingLint = blockingLintValue(lint.value)
          const nativeBySource = {
            finalize: issueValues(checkpoint.result, { includeObservational: false }),
            quality: issueValues(quality.value, { includeObservational: false }),
            lint: issueValues(blockingLint, { includeObservational: false }),
            layout: issueValues(layout.value, { includeObservational: false }),
          }
          const nativeDiagnostics = [
            ...nativeBySource.finalize,
            ...nativeBySource.quality,
            ...nativeBySource.lint,
            ...nativeBySource.layout,
          ].slice(0, 30)
          validation = {
            canvasDiagnostics,
            canvasRepairTargets,
            nativeDiagnostics,
            nativeSources: {
              finalize: nativeBySource.finalize.length,
              quality: nativeBySource.quality.length,
              lint: nativeBySource.lint.length,
              layout: nativeBySource.layout.length,
            },
            dsh: inspectGeneratedDesignQualityReport(finalizedSnapshot.documentJson),
          }
          checkpoint.validation = validation
        }
        pending.canvasValidated = validation.canvasDiagnostics.length === 0
        const diagnostics = [
          ...validation.canvasDiagnostics,
          ...validation.nativeDiagnostics,
          ...validation.dsh.diagnostics,
        ].slice(0, MAX_CORRECTION_DIAGNOSTICS)
        if (diagnostics.length > 0) {
          const repairTargets = [...validation.canvasRepairTargets, ...validation.dsh.repairTargets]
          const omittedRepairTargets = validation.dsh.repairTargetSummary.omitted
          const canvasRepairIsComplete = validation.canvasDiagnostics.length === 0
            || validation.canvasRepairTargets.length > 0
          // Native findings do not veto the repair attempt: they routinely
          // co-report the same defect a DSH repair target fixes (a starved
          // mixed-width card both starves in native quality AND carries a
          // width patch here). Publication itself still requires a clean
          // native pass, so an unrelated native defect stays terminal on
          // the bounded retry.
          const repairIsComplete = canvasRepairIsComplete
            && validation.dsh.unrepairableDiagnosticCount === 0
            && repairTargets.length > 0
            && omittedRepairTargets === 0
            && pending.repairAttemptCount < MAX_FINISH_REPAIR_ROUNDS
          pending.repairAuthorized = repairIsComplete
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: repairIsComplete ? 'needs_correction' : 'blocked_host_validation',
            version: checkpoint.version,
            finalization: publicFinalizationCheckpoint(checkpoint, reusedFinalization),
            checks: asJson({
              canvas: {
                diagnosticCount: validation.canvasDiagnostics.length,
                repairTargetCount: validation.canvasRepairTargets.length,
              },
              native: {
                diagnosticCount: validation.nativeDiagnostics.length,
                sources: validation.nativeSources,
              },
              dsh: {
                  diagnosticCount: validation.dsh.diagnostics.length,
                  unrepairableDiagnosticCount: validation.dsh.unrepairableDiagnosticCount,
                  repairTargetSummary: validation.dsh.repairTargetSummary,
              },
            }),
            diagnostics,
            ...(repairIsComplete ? { repairTargets: asJson(repairTargets) } : {}),
            canContinue: repairIsComplete,
            next: repairIsComplete
              ? `Apply every repairTargets item in one ${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME} QuickJS script using only U(nodeId, patch), then call finish exactly once. Repair only when a result presents a complete repairTargets array; the host bounds repair rounds.`
              : 'Stop and report these host validation diagnostics once. They do not provide one complete actionable repair transaction, or the bounded repair rounds were already used. Do not guess node ids, inspect/context-read, retry finish, abort, or rebuild another draft.',
          }
        }
        pending.repairAuthorized = false

        if (
          pending.latestRootScreenshot === undefined
          || pending.latestRootScreenshot.version !== checkpoint.version
          || pending.latestRootScreenshot.documentSha256 !== checkpoint.documentSha256
          || !pending.latestRootScreenshot.finalized
        ) {
          let rendered: DesignDraftScreenshot
          let artifact: InspectionScreenshotArtifact
          try {
            rendered = await drafts.screenshot(args.draftId, owner, { signal: exec.signal })
            artifact = await persistInspectionScreenshot(rendered)
          } catch (error) {
            exec.signal?.throwIfAborted()
            pending.latestRootScreenshot = undefined
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: 'needs_preview',
              reason: 'preview_unavailable',
              version: checkpoint.version,
              finalization: publicFinalizationCheckpoint(checkpoint, reusedFinalization),
              diagnostics: [],
              canContinue: true,
              next: 'The finalized document is clean, but its final PNG user preview is temporarily unavailable. Call finish exactly once to retry only the preview render and publish; do not call inspect, rerun generation, or perform model image inspection.',
            }
          }
          if (
            rendered.version !== checkpoint.version
            || rendered.documentSha256 !== checkpoint.documentSha256
          ) {
            clearFinalizationCheckpoint(pending)
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: 'needs_refinalization',
              version: rendered.version,
              diagnostics: [],
              canContinue: true,
              next: 'The live canvas changed while the automatic final preview was rendered. Call finish once; the stale checkpoint and preview artifact were discarded.',
            }
          }
          pending.latestRootScreenshot = {
            version: rendered.version,
            documentSha256: rendered.documentSha256,
            finalized: true,
            ...artifact,
          }
        }

        if (!pending.skipVisualReview && !pending.visualReviewDone) {
          // Single see-then-fix round: the browser gets the exact post-final
          // PNG through the standard screenshot presentation channel, and the
          // model gets a deterministic layout digest of the same document
          // (the DeepSeek adapter rejects image blocks, so the digest is the
          // model-facing rendering). One bounded I/K/U correction batch is
          // authorized; calling finish again unchanged accepts and publishes.
          const reviewed = await drafts.snapshot(args.draftId, owner, { signal: exec.signal })
          if (
            reviewed.version !== checkpoint.version
            || documentSha256(reviewed.documentJson) !== checkpoint.documentSha256
          ) {
            clearFinalizationCheckpoint(pending)
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: 'needs_refinalization',
              version: reviewed.version,
              diagnostics: [],
              canContinue: true,
              next: 'The live canvas changed while the visual review was being prepared. Call finish once; the stale checkpoint was discarded.',
            }
          }
          pending.visualReviewDone = true
          pending.visualReviewAuthorized = true
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: 'needs_visual_review',
            version: checkpoint.version,
            finalization: publicFinalizationCheckpoint(checkpoint, reusedFinalization),
            ...(publicScreenshot(pending.latestRootScreenshot, checkpoint.version, checkpoint.documentSha256) === undefined
              ? {}
              : { screenshot: publicScreenshot(pending.latestRootScreenshot, checkpoint.version, checkpoint.documentSha256) as JsonValue }),
            digest: buildVisualReviewDigest(reviewed.documentJson),
            checklist: [...VISUAL_REVIEW_CHECKLIST],
            diagnostics: [],
            canContinue: true,
            next: 'Quality gates passed and the exact final preview was rendered. Review the digest (and preview) against every checklist item. If everything holds, call finish exactly once more with no batch in between to publish. Otherwise send exactly ONE bounded correction batch (I/K/U only, at most 16 calls and 6 KiB, never rebuilding Header or Hero), then call finish; the gates re-validate the corrected page.',
            ...(pending.selectedStyle === undefined ? {} : { selectedStyle: pending.selectedStyle }),
          } as Record<string, JsonValue>
        }
        pending.visualReviewAuthorized = false

        let published: PublishedDraft
        try {
          const finished = await drafts.finish(args.draftId, owner, {
            signal: exec.signal,
            expectedVersion: checkpoint.version,
            expectedDocumentSha256: checkpoint.documentSha256,
            publish: async (authoritative): Promise<PublishedDraft> => {
              // Recheck under the controller's serialized publication lock so
              // a concurrent editor mutation cannot bypass the preflight.
              const jsReport = inspectGeneratedDesignQualityReport(authoritative.documentJson)
              if (jsReport.diagnostics.length > 0) throw new DesignDraftJsQualityError(jsReport)
              const inspected = pending.latestRootScreenshot
              const authoritativeSha256 = documentSha256(authoritative.documentJson)
              if (
                inspected === undefined
                || inspected.version !== authoritative.version
                || inspected.documentSha256 !== authoritativeSha256
              ) {
                throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: current post-final preview artifact is unavailable`)
              }
              const policy = services.sandboxPolicy.resolve({ session: exec.agent?.session })
              if (services.fs.sandboxMode !== undefined && policy.mode === 'read-only') {
                throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: publishing requires Workspace Write access`)
              }
              const currentPathInfo = await services.fs.lstat(pending.requestedPath, { cwd: policy.workspaceRoot }, exec.signal)
              if (currentPathInfo !== undefined) throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: target now exists: ${pending.requestedPath}`)
              const currentTarget = await services.fs.resolve(pending.requestedPath, { cwd: policy.workspaceRoot, signal: exec.signal })
              const currentProcessPath = services.fs.processPath(currentTarget)
              if (currentProcessPath !== pending.processPath || String(currentTarget.targetKey) !== String(pending.target.targetKey)) {
                throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: target identity changed while the draft was open`)
              }
              const previewPath = await createRenderOutput()
              await copyFile(inspected.path, previewPath)
              let preview: RenderFrame
              try {
                const verified = await verifyRenderOutput(previewPath)
                if (verified.bytes !== inspected.bytes || verified.sha256 !== inspected.sha256) {
                  throw new Error(`${OPENPENCIL_PIPELINE_FINISH_TOOL_NAME}: post-final preview changed after its exact PNG integrity checkpoint`)
                }
                preview = {
                  path: previewPath,
                  filename: basename(previewPath),
                  mimeType: 'image/png',
                  bytes: verified.bytes,
                  width: verified.width,
                  height: verified.height,
                  sha256: verified.sha256,
                  index: 0,
                }
              } catch (error) {
                await rm(previewPath, { force: true }).catch(() => {})
                throw error
              }
              let document: DocumentSnapshot
              try {
                document = await (services.createDocumentSnapshot ?? createDocumentSnapshotFromText)(
                  authoritative.documentJson,
                )
              } catch (error) {
                await rm(previewPath, { force: true }).catch(() => {})
                throw error
              }
              let outcome
              try {
                outcome = await services.fs.writeText(
                  pending.target,
                  authoritative.documentJson,
                  { kind: 'createIfAbsent' },
                  exec.signal,
                  policy,
                )
              } catch (error) {
                await rm(previewPath, { force: true }).catch(() => {})
                throw error
              }
              // createIfAbsent writes the exact supplied UTF-8 text. All
              // fallible presentation artifacts were prepared first, so an
              // observer failure after commit must not turn success into an
              // unretryable half-published draft.
              try {
                services.observe(pending.target, { kind: 'present', version: outcome.version }, exec)
              } catch {
                // Best-effort post-commit notification only.
              }
              return {
                draftId: args.draftId,
                path: pending.processPath,
                filename: basename(pending.processPath),
                bytes: document.bytes,
                sha256: document.sha256,
                created: true,
                applied: true,
                saved: true,
                published: true,
                sourceTool: OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
                previewIntent: 'document',
                editable: true,
                autoOpenEditor: true,
                preview,
                document,
                note: `Published ${pending.processPath} atomically after native quality, lint, layout, render-integrity, and DSH quality gates. The exact final PNG user preview and live editor are already attached; stop now.`,
                ...(pending.selectedStyle === undefined ? {} : { selectedStyle: pending.selectedStyle }),
              }
            },
          })
          published = finished.published
        } catch (error) {
          if (error instanceof DesignDraftJsQualityError) {
            const repairTargets = error.report.repairTargets
            const omittedRepairTargets = error.report.repairTargetSummary.omitted
            const repairIsComplete = repairTargets.length > 0
              && omittedRepairTargets === 0
              && error.report.unrepairableDiagnosticCount === 0
              && pending.repairAttemptCount < MAX_FINISH_REPAIR_ROUNDS
            pending.repairAuthorized = repairIsComplete
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: repairIsComplete ? 'needs_correction' : 'blocked_host_validation',
              version: checkpoint.version,
              finalization: publicFinalizationCheckpoint(checkpoint, reusedFinalization),
              checks: asJson({
                canvas: { diagnosticCount: 0 },
                native: { diagnosticCount: 0, sources: { finalize: 0, quality: 0, lint: 0, layout: 0 } },
                dsh: {
                  diagnosticCount: error.report.diagnostics.length,
                  unrepairableDiagnosticCount: error.report.unrepairableDiagnosticCount,
                  repairTargetSummary: error.report.repairTargetSummary,
                  recheckedAtPublish: true,
                },
              }),
              diagnostics: error.report.diagnostics,
              ...(repairIsComplete ? { repairTargets: asJson(repairTargets) } : {}),
              canContinue: repairIsComplete,
              next: repairIsComplete
                ? `Apply every repairTargets item in one ${OPENPENCIL_PIPELINE_BATCH_TOOL_NAME} QuickJS script using only U(nodeId, patch), then call finish exactly once. Repair only when a result presents a complete repairTargets array; the host bounds repair rounds.`
                : 'Stop and report these host validation diagnostics once. They do not provide one complete actionable repair transaction, or the bounded repair rounds were already used. Do not guess node ids, inspect/context-read, retry finish, abort, or rebuild another draft.',
            }
          }
          const code = isRecord(error) && typeof error.code === 'string' ? error.code : undefined
          if (code === 'OPENPENCIL_DRAFT_CHECKPOINT_DRIFT') {
            const controllerVersion = isRecord(error)
              && typeof error.currentVersion === 'number'
              && Number.isSafeInteger(error.currentVersion)
              && error.currentVersion >= 0
              ? error.currentVersion
              : checkpoint.version
            clearFinalizationCheckpoint(pending)
            pending.repairAuthorized = false
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: 'needs_refinalization',
              version: controllerVersion,
              diagnostics: [],
              canContinue: true,
              next: `The authoritative document changed after version ${checkpoint.version} and SHA ${checkpoint.documentSha256} were validated. The stale finalization and root preview proof were discarded. Call finish exactly once; it will finalize, validate, automatically render the current root preview, and publish when clean.`,
            }
          }
          if (code === 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED') {
            const controllerVersion = isRecord(error)
              && typeof error.currentVersion === 'number'
              && Number.isSafeInteger(error.currentVersion)
              && error.currentVersion >= 0
              ? error.currentVersion
              : checkpoint.version
            const checkpointIsCurrent = controllerVersion === checkpoint.version
            if (!checkpointIsCurrent) clearFinalizationCheckpoint(pending)
            const screenshot = publicScreenshot(
              pending.latestRootScreenshot,
              controllerVersion,
              checkpointIsCurrent ? checkpoint.documentSha256 : undefined,
            )
            return {
              draftId: args.draftId,
              path: pending.processPath,
              published: false,
              stage: checkpointIsCurrent ? 'needs_preview' : 'needs_refinalization',
              ...(checkpointIsCurrent ? { reason: 'preview_unavailable' } : {}),
              version: controllerVersion,
              ...(screenshot === undefined ? {} : { screenshot }),
              diagnostics: [],
              ...(checkpointIsCurrent
                ? { finalization: publicFinalizationCheckpoint(checkpoint, reusedFinalization) }
                : {}),
              canContinue: true,
              next: checkpointIsCurrent
                ? `The finalized document is clean, but its final PNG user preview is not yet available. Call finish exactly once to retry only the preview proof and publish; finalization and diagnostics are checkpointed and will not rerun unless the draft changes. Do not call inspect or perform model image inspection.`
                : `The document changed concurrently to version ${controllerVersion}, so stale finalization and preview proof were discarded. Call finish once to finalize, validate, render the current root preview, and publish when clean.`,
            }
          }
          pending.repairAuthorized = false
          return {
            draftId: args.draftId,
            path: pending.processPath,
            published: false,
            stage: 'blocked_host_failure',
            version: checkpoint.version,
            diagnostics: [error instanceof Error ? error.message.slice(0, 500) : 'OpenPencil publication failed.'],
            canContinue: false,
            next: 'Stop and report this host failure once. Do not retry finish, inspect/context-read, guess a repair, abort, or rebuild another draft.',
          }
        }
        this.#pending.delete(args.draftId)
        return published as unknown as Record<string, JsonValue>
      },
      presentCall: () => ({ card: 'generic', title: 'Finalize OpenPencil design pipeline', kind: 'execute' }),
    })
  }

  #abortTool() {
    const drafts = this.#drafts
    return defineTool({
      name: OPENPENCIL_PIPELINE_ABORT_TOOL_NAME,
      description: 'Abort one unpublished OpenPencil design draft and remove its private native runtime. No target .op file is created.',
      parameters: { draftId: { type: 'string', required: true } },
      output: { schema: { type: 'object', additionalProperties: true }, render: renderJson },
      execute: async (args: { draftId: string }, exec) => {
        const owner = ownerSessionId(exec)
        const pending = this.#requirePending(args.draftId, owner)
        await drafts.abort(args.draftId, owner)
        this.#pending.delete(args.draftId)
        return { draftId: args.draftId, path: pending.processPath, aborted: true, published: false }
      },
      presentCall: () => ({ card: 'generic', title: 'Abort OpenPencil design draft', kind: 'execute' }),
    })
  }
}

export function createDesignDraftToolController(
  editorHost: EditorHostController,
  services: DesignDraftToolServices,
): DesignDraftToolController {
  return new DesignDraftToolController(editorHost, services)
}
