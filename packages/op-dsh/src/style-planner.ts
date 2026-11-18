/**
 * Opt-in advisory style planner for the DSH OpenPencil pipeline
 * (spec 1599ef92 §3B–§3D, §5, §6).
 *
 * The planner sits strictly before generation: it browses the installed
 * catalog through the single catalog adapter, treats hard platform/theme
 * constraints as filters (never preferences), optionally asks one bounded
 * advisory provider for a distribution over the exact eligible set plus an
 * internal abstain sentinel, and turns a designer's explicit confirmation
 * into a 15-minute, session-bound selection handle. It never creates a
 * canvas, never writes files, and never logs raw brief text: briefs are
 * correlated through a keyed fingerprint owned by the selection store.
 *
 * @module dsh-openpencil/style-planner
 */

import { createHash, randomBytes } from 'node:crypto'
import { defineTool, type ToolRunContext } from '@deepseek-ai/dsh-tools'
import { COMMERCE_BRIEF, draftCanvasContract, hasExplicitCanvasIntent, latestDirectUserText } from './design-draft-tools.js'
import type { JsonValue } from './json-value.js'
import {
  OPENPENCIL_STYLE_CONFIRM_TOOL_NAME,
  OPENPENCIL_STYLE_PLAN_TOOL_NAME,
} from './tool-names.js'
import {
  eligibleStyleGuides,
  guideSupportsHandoff,
  loadInstalledStyleCatalog,
  searchStyleGuides,
  styleGuideTheme,
  supportedFontMappingOf,
  type StyleCatalogGuide,
  type StyleCatalogSnapshot,
} from './style-catalog.js'

/** Internal abstain option riding every provider distribution. */
export const ABSTAIN_SENTINEL = '__abstain__'

/** §5 bounds. */
export const MAX_BRIEF_BYTES = 16_000
export const MAX_ELIGIBLE_GUIDES = 254
export const MAX_OUTBOUND_BYTES = 128 * 1024
export const MAX_RESPONSE_BYTES = 32 * 1024
export const PROVIDER_DEADLINE_MS = 5_000
export const HANDLE_TTL_MS = 15 * 60 * 1000
/** Proposed picker distribution tolerance; not a claim about provider tolerance. */
const DISTRIBUTION_SUM_TOLERANCE = 1e-6

const NATIVE_FIG_PEN_BRIEF = /\.(fig|pen)\b|\bnative\s+openpencil\s+app\b/iu

function renderJson(_args: unknown, value: unknown): [{ type: 'text'; text: string }] {
  return [{ type: 'text', text: JSON.stringify(value, null, 2) }]
}

function asResult(value: object): Record<string, JsonValue> {
  return value as unknown as Record<string, JsonValue>
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/** Outbound selection input: brief text plus catalog descriptors, nothing else. */
export interface StyleRecommendationInput {
  brief: string
  platform: 'web' | 'mobile'
  catalogRevision: string
  options: string[]
}

/**
 * Bounded advisory provider. Off by default; fixtures inject a fake. One
 * `recommend` call per explicit recommendation action — the planner never
 * retries or falls back.
 */
export interface StyleRecommendationProvider {
  readonly name: string
  recommend(input: StyleRecommendationInput): Promise<unknown>
}

export type DistributionVerdict =
  | { valid: true; choice: string; probabilities: Record<string, number>; confidence: number }
  | { valid: false; reason: string }

/**
 * Validate a one-call choice distribution over the exact option set (eligible
 * IDs plus the abstain sentinel): finite probabilities in [0,1], complete
 * coverage, sum within 1e-6 of one, and a choice that is a maximum-probability
 * option within that tolerance. Contradictions are rejected; scores are never
 * manufactured.
 */
export function validateStyleDistribution(raw: unknown, optionIds: string[]): DistributionVerdict {
  if (!isRecord(raw)) return { valid: false, reason: 'response is not an object' }
  const { choice, probabilities, confidence } = raw
  if (typeof choice !== 'string') return { valid: false, reason: 'choice must be an option id' }
  if (!isRecord(probabilities)) return { valid: false, reason: 'probabilities must be an object' }
  if (typeof confidence !== 'number' || !Number.isFinite(confidence)) {
    return { valid: false, reason: 'confidence must be a finite number' }
  }
  const expected = new Set(optionIds)
  const seen = new Set<string>()
  let sum = 0
  let maxProbability = Number.NEGATIVE_INFINITY
  let maxId: string | undefined
  for (const [id, value] of Object.entries(probabilities)) {
    if (typeof value !== 'number' || !Number.isFinite(value) || value < 0 || value > 1) {
      return { valid: false, reason: `probability for ${id} must be finite within [0,1]` }
    }
    if (!expected.has(id)) return { valid: false, reason: `unknown option id: ${id}` }
    if (seen.has(id)) return { valid: false, reason: `duplicate option id: ${id}` }
    seen.add(id)
    sum += value
    if (value > maxProbability) {
      maxProbability = value
      maxId = id
    }
  }
  for (const id of optionIds) {
    if (!seen.has(id)) return { valid: false, reason: `incomplete distribution: missing ${id}` }
  }
  if (Math.abs(sum - 1) > DISTRIBUTION_SUM_TOLERANCE) {
    return { valid: false, reason: `probabilities sum to ${sum}, outside the 1e-6 tolerance of 1` }
  }
  if (!expected.has(choice)) return { valid: false, reason: `choice is not an option id: ${choice}` }
  if (choice !== maxId && (probabilities[choice] as number) < maxProbability - DISTRIBUTION_SUM_TOLERANCE) {
    return { valid: false, reason: 'choice is not a maximum-probability option' }
  }
  return {
    valid: true,
    choice,
    probabilities: probabilities as Record<string, number>,
    confidence,
  }
}

// --- evaluation accounting (§6) -------------------------------------------

export type StyleEvaluationEvent =
  | { kind: 'plan-request' }
  | { kind: 'provider-submitted' }
  | { kind: 'provider-response'; valid: boolean }
  | { kind: 'provider-failure'; reason: string }
  | { kind: 'abstain' }
  | { kind: 'recommendation-displayed'; top: string[] }
  | { kind: 'override'; from: string; to: string }
  | { kind: 'catalog-alternative-selected'; guideId: string }
  | { kind: 'manual-selected'; guideId: string }

export interface StyleEvaluationReport {
  denominators: {
    requests: number
    submissions: number
    validResponses: number
    failures: number
    abstentions: number
    overrides: number
    catalogAlternatives: number
    manualSelections: number
  }
  rates: { override: number | null; abstain: number | null; validResponseCoverage: number | null }
  provenance: { recommendations: 'model'; catalogAlternatives: 'catalog-alternative' }
  claims: string[]
  text: string
}

/**
 * Deterministic selection-accounting report over planner events: correct
 * denominators, explicit provenance (catalog alternatives are never scored as
 * model top-k predictions), honest small-sample labeling, and no
 * rendered-quality claims — selection quality only.
 */
export function evaluationReport(events: StyleEvaluationEvent[]): StyleEvaluationReport {
  const count = (kind: StyleEvaluationEvent['kind']) => events.filter(event => event.kind === kind).length
  const submissions = count('provider-submitted')
  const validResponses = events.filter(event => event.kind === 'provider-response' && event.valid).length
  const overrides = count('override')
  const abstentions = count('abstain')
  const requests = count('plan-request')
  const catalogAlternatives = count('catalog-alternative-selected')
  const manualSelections = count('manual-selected')
  const rate = (numerator: number) => submissions === 0 ? null : numerator / submissions
  const format = (value: number | null) => value === null ? 'n/a (no submissions)' : String(Math.round(value * 1000) / 1000)
  const overrideRate = rate(overrides)
  const abstainRate = rate(abstentions)
  const coverage = rate(validResponses)
  const lines = [
    `selection accounting: n=${requests} requests, ${submissions} submissions`,
    requests < 10
      ? 'sample: exploratory (small sample; counts, not production accuracy)'
      : 'sample sizes printed; no production-accuracy claim',
    `valid-response coverage: ${validResponses}/${submissions}`,
    `failures: ${count('provider-failure')}`,
    `abstentions: ${abstentions}`,
    `override rate: ${format(overrideRate)}`,
    `abstain rate: ${format(abstainRate)}`,
    `catalog alternatives selected: ${catalogAlternatives} (provenance: catalog-alternative; never scored as model top-k predictions)`,
    `manual selections: ${manualSelections}`,
    'Selection quality only: rendered layout quality is out of scope and no rendered-quality claim is made.',
  ]
  return {
    denominators: {
      requests,
      submissions,
      validResponses,
      failures: count('provider-failure'),
      abstentions,
      overrides,
      catalogAlternatives,
      manualSelections,
    },
    rates: { override: overrideRate, abstain: abstainRate, validResponseCoverage: coverage },
    provenance: { recommendations: 'model', catalogAlternatives: 'catalog-alternative' },
    claims: ['denominators', 'provenance', 'coverage', 'counts-only'],
    text: lines.join('\n'),
  }
}

// --- selection store (§3D) --------------------------------------------------

export interface StyleHandleRecord {
  handle: string
  sessionKey: string
  guideId: string
  briefFingerprint: string
  platform: 'web' | 'mobile'
  constraintRevision: number
  catalogRevision: string
  confirmedAt: number
  expiresAt: number
  consumed: boolean
  userIntent?: string
  confirmedUserFingerprint?: string
}

export interface StyleHandleValidationContext {
  sessionKey: string
  guideId?: string
  briefFingerprint?: string
  platform?: 'web' | 'mobile'
  constraintRevision?: number
  catalogRevision?: string
  now: number
}

export type StyleHandleVerdict =
  | { ok: true; record: StyleHandleRecord }
  | { ok: false; reason: 'unknown' | 'consumed' | 'expired' | 'session' | 'guide' | 'brief' | 'platform' | 'constraints' | 'catalog' }

/** One selection session's planner state. Ephemeral; never persisted. */
export interface PlannerSessionState {
  sessionKey: string
  brief: string
  userIntent?: string
  platform: 'web' | 'mobile'
  theme?: 'dark' | 'light'
  commerce: boolean
  revision: number
  requestId: string
  briefFingerprint: string
  constraintRevision: number
  catalogRevision: string
  lastRecommendation?: { valid: boolean; top: string[]; abstain: boolean }
  lastConfirmed?: { handle: string; guideId: string; catalogRevision: string }
}

/** Ephemeral, session-scoped selection state. Nothing here persists. */
export interface StyleSelectionStore {
  /** Keyed brief fingerprint; the shared secret never leaves the store. */
  briefFingerprintOf(brief: string): string
  confirm(input: {
    sessionKey: string
    guideId: string
    briefFingerprint: string
    platform: 'web' | 'mobile'
    constraintRevision: number
    catalogRevision: string
    ttlMs: number
    userIntent?: string
    confirmedUserFingerprint?: string
  }): string
  validate(handle: string, context: StyleHandleValidationContext): StyleHandleVerdict
  consume(handle: string): void
  session(sessionKey: string): PlannerSessionState | undefined
  recordSession(state: PlannerSessionState): void
  liveConfirmedFor(sessionKey: string, briefFingerprint: string, now: number): StyleHandleRecord | undefined
  events: StyleEvaluationEvent[]
}

export function createStyleSelectionStore(options: { now: () => number }): StyleSelectionStore {
  const handles = new Map<string, StyleHandleRecord>()
  const sessions = new Map<string, PlannerSessionState>()
  const events: StyleEvaluationEvent[] = []
  const secret = randomBytes(32).toString('hex')
  const now = options.now
  return {
    briefFingerprintOf(brief) {
      return createHash('sha256').update(`${secret}\u0000${brief}`).digest('hex')
    },
    confirm(input) {
      const at = now()
      const handle = `sel_${randomBytes(12).toString('hex')}`
      handles.set(handle, {
        handle,
        sessionKey: input.sessionKey,
        guideId: input.guideId,
        briefFingerprint: input.briefFingerprint,
        platform: input.platform,
        constraintRevision: input.constraintRevision,
        catalogRevision: input.catalogRevision,
        confirmedAt: at,
        expiresAt: at + input.ttlMs,
        consumed: false,
        ...(input.userIntent === undefined ? {} : { userIntent: input.userIntent }),
        ...(input.confirmedUserFingerprint === undefined ? {} : { confirmedUserFingerprint: input.confirmedUserFingerprint }),
      })
      return handle
    },
    validate(handle, context) {
      const record = handles.get(handle)
      if (record === undefined) return { ok: false, reason: 'unknown' }
      if (record.consumed) return { ok: false, reason: 'consumed' }
      if (context.now > record.expiresAt) {
        delete record.userIntent
        return { ok: false, reason: 'expired' }
      }
      if (record.sessionKey !== context.sessionKey) return { ok: false, reason: 'session' }
      if (context.guideId !== undefined && record.guideId !== context.guideId) return { ok: false, reason: 'guide' }
      if (context.briefFingerprint !== undefined && record.briefFingerprint !== context.briefFingerprint) return { ok: false, reason: 'brief' }
      if (context.platform !== undefined && record.platform !== context.platform) return { ok: false, reason: 'platform' }
      if (context.constraintRevision !== undefined && record.constraintRevision !== context.constraintRevision) return { ok: false, reason: 'constraints' }
      if (context.catalogRevision !== undefined && record.catalogRevision !== context.catalogRevision) return { ok: false, reason: 'catalog' }
      return { ok: true, record }
    },
    consume(handle) {
      const record = handles.get(handle)
      if (record !== undefined) record.consumed = true
    },
    session(sessionKey) {
      return sessions.get(sessionKey)
    },
    recordSession(state) {
      sessions.set(state.sessionKey, state)
    },
    liveConfirmedFor(sessionKey, briefFingerprint, at) {
      for (const record of handles.values()) {
        if (record.sessionKey === sessionKey
          && record.briefFingerprint === briefFingerprint
          && !record.consumed
          && at <= record.expiresAt) return record
      }
      return undefined
    },
    events,
  }
}

// --- planner tools ----------------------------------------------------------

export interface StylePlannerServices {
  store: StyleSelectionStore
  resolveCatalog(): StyleCatalogSnapshot
  provider?: StyleRecommendationProvider
  now(): number
  beginStyleInputSupported(): boolean
}

function sessionKeyOf(exec: ToolRunContext): string {
  if (exec.agent === undefined) {
    throw new Error(`${OPENPENCIL_STYLE_PLAN_TOOL_NAME}: style planning requires an agent-owned DSH execution`)
  }
  return String(exec.agent.session.id)
}

function guideView(guide: StyleCatalogGuide): Record<string, JsonValue> {
  return {
    id: guide.id,
    platform: guide.platform,
    tags: guide.tags.slice(0, 8),
    summary: guide.summary,
  }
}

/**
 * The two opt-in planner tools: `openpencil_style_plan` (browse, constrain,
 * optionally one bounded advisory recommendation) and
 * `openpencil_style_confirm` (explicit designer confirmation issuing the
 * 15-minute session-bound handle the begin boundary validates).
 */
export function createStylePlannerTools(services: StylePlannerServices) {
  const store = services.store

  const planTool = defineTool({
    name: OPENPENCIL_STYLE_PLAN_TOOL_NAME,
    description: 'Optional pre-generation style planning: browse the installed OpenPencil style-guide catalog deterministically, '
      + 'apply hard platform/theme constraints, optionally request one bounded advisory model recommendation (disabled unless explicitly enabled), '
      + 'and prepare an explicit guide choice for confirmation. Planning never creates a canvas, .op file, or native interaction.',
    parameters: {
      brief: { type: 'string', required: true, description: 'The user\'s design request, verbatim. Used for platform resolution and (only with recommend:true and an enabled provider) the advisory call.' },
      platform: { type: 'string', enum: ['web', 'mobile'], description: 'Optional explicit platform. Omit to keep the pipeline\'s default resolution from the brief.' },
      guide: { type: 'string', description: 'Optional exact canonical guide id from the catalog browse. An exact valid guide bypasses any model call.' },
      theme: { type: 'string', enum: ['dark', 'light'], description: 'Optional confirmed hard theme. Applied only where catalog theme metadata exists.' },
      query: { type: 'string', description: 'Optional deterministic local search over ids, tags, and summaries.' },
      recommend: { type: 'boolean', description: 'Optional. True requests exactly one advisory provider recommendation for this brief. Default false (manual mode).' },
    },
    output: { schema: { type: 'object', additionalProperties: true }, render: renderJson },
    execute: async (args: {
      brief: string
      platform?: string
      guide?: string
      theme?: string
      query?: string
      recommend?: boolean
    }, exec) => {
      const name = OPENPENCIL_STYLE_PLAN_TOOL_NAME
      const brief = args.brief.trim()
      if (Buffer.byteLength(brief, 'utf8') > MAX_BRIEF_BYTES) {
        throw new Error(`${name}: brief exceeds the 16000 UTF-8 byte limit; shorten it instead of truncating`)
      }
      if (NATIVE_FIG_PEN_BRIEF.test(brief)) {
        return asResult({
          supported: false,
          workflow: 'native-fig-pen',
          reason: 'Native .fig/.pen workflows are outside the DSH OpenPencil integration. The style picker identifies the request as unsupported and performs no native app or file operation.',
        })
      }
      const sessionKey = sessionKeyOf(exec)
      const previous = store.session(sessionKey)
      const userIntent = previous?.brief === brief ? previous.userIntent : latestDirectUserText(exec)
      const platform = args.platform === 'mobile' || args.platform === 'web'
        ? args.platform
        : draftCanvasContract(brief, userIntent).platform
      const theme = args.theme === 'dark' || args.theme === 'light' ? args.theme : undefined
      const commerce = COMMERCE_BRIEF.test(brief)
      const snapshot = services.resolveCatalog()
      const briefFingerprint = store.briefFingerprintOf(brief)

      const unchanged = previous !== undefined
        && previous.brief === brief
        && previous.platform === platform
        && previous.theme === theme
        && previous.catalogRevision === snapshot.revision
      const revision = unchanged ? previous.revision : (previous?.revision ?? 0) + 1
      const requestId = unchanged ? previous.requestId : `req_${randomBytes(6).toString('hex')}`
      const sessionState: PlannerSessionState = {
        sessionKey,
        brief,
        ...(userIntent === undefined ? {} : { userIntent }),
        platform,
        theme,
        commerce,
        revision,
        requestId,
        briefFingerprint,
        constraintRevision: (previous?.constraintRevision ?? 1)
          + (previous !== undefined && (previous.platform !== platform || previous.theme !== theme) ? 1 : 0),
        catalogRevision: snapshot.revision,
        ...(unchanged ? { lastRecommendation: previous?.lastRecommendation } : {}),
        lastConfirmed: previous?.lastConfirmed,
      }
      store.recordSession(sessionState)
      store.events.push({ kind: 'plan-request' })

      const handoff = services.beginStyleInputSupported()
        ? commerce
          ? { mode: 'unsupported', reason: platform === 'web' ? 'web-commerce' : 'commerce', beforeProvider: true }
          : { mode: 'supported' }
        : { mode: 'unsupported', reason: 'selection_handoff_unsupported', beforeProvider: true }

      if (args.guide !== undefined) {
        const guide = snapshot.guides.find(candidate => candidate.id === args.guide)
        if (guide === undefined) {
          throw new Error(`${name}: unknown catalog guide id "${args.guide}"; browse the eligible catalog first and use the exact canonical id`)
        }
        const catalogPlatform = platform === 'web' ? 'webapp' : 'mobile'
        const conflicts: { type: string; detail: string }[] = []
        if (guide.platform !== catalogPlatform) {
          conflicts.push({ type: 'platform', detail: `guide platform ${guide.platform} does not support the ${platform} handoff platform` })
        }
        const guideTheme = styleGuideTheme(guide)
        if (theme !== undefined && guideTheme !== undefined && guideTheme !== theme) {
          conflicts.push({ type: 'theme', detail: `confirmed ${theme} theme conflicts with the catalog's verified ${guideTheme}-mode metadata` })
        }
        const handoffCapable = handoff.mode === 'supported' && guideSupportsHandoff(guide)
        const guideHandoff = guideSupportsHandoff(guide)
          ? handoff
          : { mode: 'unsupported', reason: 'slides-card-reference-only', beforeProvider: true }
        const previousChoice = sessionState.lastConfirmed !== undefined
          && sessionState.lastConfirmed.guideId === guide.id
          && sessionState.lastConfirmed.catalogRevision !== snapshot.revision
          ? { status: 'provisional', detail: 'the catalog changed after this choice was confirmed; a fresh confirmation is required before a new handle issues' }
          : undefined
        return asResult({
          supported: true,
          mode: 'selection',
          platform,
          guide: guideView(guide),
          conflicts,
          confirmable: conflicts.length === 0 && handoffCapable,
          fonts: supportedFontMappingOf(guide),
          catalogRevision: snapshot.revision,
          handoff: guideHandoff,
          ...(previousChoice === undefined ? {} : { previousChoice }),
          request: { id: requestId, revision, briefFingerprint },
        })
      }

      const eligible = eligibleStyleGuides(snapshot, platform)
      const listed = searchStyleGuides(eligible, args.query ?? '')
      if (eligible.length === 0) {
        return asResult({
          supported: true,
          mode: 'no-match',
          platform,
          eligibleCount: 0,
          guides: [],
          quarantined: snapshot.quarantined,
          catalogRevision: snapshot.revision,
          submitted: false,
          handoff,
          request: { id: requestId, revision, briefFingerprint },
          reason: 'No eligible guide remains under the confirmed constraints; revise a constraint or choose manually.',
        })
      }

      let recommendation: Record<string, JsonValue> | undefined
      let submitted = false
      let limit: Record<string, JsonValue> | undefined
      if (args.recommend === true) {
        if (services.provider === undefined) {
          recommendation = { status: 'unavailable', reason: 'egress_not_authorized' }
        } else if (eligible.length > MAX_ELIGIBLE_GUIDES) {
          limit = { eligibleCount: eligible.length, maxEligible: MAX_ELIGIBLE_GUIDES, submitted: false }
        } else if (sessionState.lastRecommendation !== undefined) {
          const cached = sessionState.lastRecommendation
          recommendation = {
            valid: cached.valid,
            provenance: 'model',
            provider: services.provider.name,
            top: cached.top.map(id => ({ id })),
            abstain: cached.abstain,
            ...(cached.abstain ? { display: 'No model recommendation' } : {}),
          }
        } else {
          const options = [...eligible.map(guide => guide.id), ABSTAIN_SENTINEL]
          const input: StyleRecommendationInput = { brief, platform, catalogRevision: snapshot.revision, options }
          if (Buffer.byteLength(JSON.stringify(input), 'utf8') > MAX_OUTBOUND_BYTES) {
            recommendation = { valid: false, reason: 'serialized selection input exceeds the 128 KiB bound; staying in manual mode' }
          } else {
            submitted = true
            store.events.push({ kind: 'provider-submitted' })
            const startedAt = services.now()
            let raw: unknown
            try {
              raw = await Promise.race([
                services.provider.recommend(input),
                new Promise<never>((_, reject) => {
                  const timer = setTimeout(() => reject(new Error('provider deadline exceeded (5s)')), PROVIDER_DEADLINE_MS)
                  timer.unref?.()
                }),
              ])
            } catch (error) {
              const message = error instanceof Error ? error.message.slice(0, 200) : 'unknown error'
              store.events.push({ kind: 'provider-failure', reason: message })
              recommendation = {
                valid: false,
                reason: /deadline/iu.test(message) ? message : `provider failure: ${message}`,
              }
            }
            if (raw !== undefined) {
              if (Buffer.byteLength(JSON.stringify(raw), 'utf8') > MAX_RESPONSE_BYTES) {
                store.events.push({ kind: 'provider-response', valid: false })
                recommendation = { valid: false, reason: 'provider response exceeds the 32 KiB bound' }
              } else {
                const verdict = validateStyleDistribution(raw, options)
                store.events.push({ kind: 'provider-response', valid: verdict.valid })
                if (verdict.valid) {
                  const abstain = verdict.choice === ABSTAIN_SENTINEL
                  const top = Object.entries(verdict.probabilities)
                    .filter(([id]) => id !== ABSTAIN_SENTINEL)
                    .sort((first, second) => (second[1] - first[1]) || (first[0] < second[0] ? -1 : first[0] > second[0] ? 1 : 0))
                    .slice(0, 3)
                    .map(([id, probability]) => ({ id, probability }))
                  if (abstain) store.events.push({ kind: 'abstain' })
                  store.events.push({ kind: 'recommendation-displayed', top: top.map(entry => entry.id) })
                  sessionState.lastRecommendation = { valid: true, top: top.map(entry => entry.id), abstain }
                  store.recordSession(sessionState)
                  recommendation = {
                    valid: true,
                    provenance: 'model',
                    provider: services.provider.name,
                    latencyMs: services.now() - startedAt,
                    top,
                    abstain,
                    ...(abstain ? { display: 'No model recommendation' } : {}),
                  }
                } else {
                  recommendation = { valid: false, reason: verdict.reason, provenance: 'model', provider: services.provider.name }
                }
              }
            }
          }
        }
      }

      const mode = recommendation !== undefined && submitted ? 'recommendation' : 'manual'
      return asResult({
        supported: true,
        mode,
        platform,
        eligibleCount: eligible.length,
        guides: listed.map(guideView),
        quarantined: snapshot.quarantined,
        catalogRevision: snapshot.revision,
        submitted,
        ...(limit === undefined ? {} : { limit }),
        ...(recommendation === undefined ? {} : { recommendation }),
        ...(recommendation === undefined ? {} : {
          manualAlternatives: { label: 'catalog-alternatives', provenance: 'catalog', guides: listed.map(guideView) },
        }),
        handoff,
        request: { id: requestId, revision, briefFingerprint },
      })
    },
    presentCall: () => ({ card: 'generic', title: 'Plan OpenPencil style selection', kind: 'read' }),
  })

  const confirmTool = defineTool({
    name: OPENPENCIL_STYLE_CONFIRM_TOOL_NAME,
    description: 'Confirm the designer\'s explicit style-guide choice from openpencil_style_plan and issue the 15-minute, '
      + 'session-bound selection handle that openpencil_pipeline_begin validates before creating any draft. '
      + 'The handle grants no permission to generate; the brief passed to begin stays the user\'s original brief.',
    parameters: {
      guide: { type: 'string', required: true, description: 'Exact canonical guide id chosen by the designer (any eligible guide, not only model recommendations).' },
    },
    output: { schema: { type: 'object', additionalProperties: true }, render: renderJson },
    execute: async (args: { guide: string }, exec) => {
      const name = OPENPENCIL_STYLE_CONFIRM_TOOL_NAME
      const sessionKey = sessionKeyOf(exec)
      const session = store.session(sessionKey)
      if (session === undefined) {
        throw new Error(`${name}: call ${OPENPENCIL_STYLE_PLAN_TOOL_NAME} for this brief first; confirmation needs the session's current plan revision`)
      }
      const currentUserText = latestDirectUserText(exec)
      if (currentUserText !== undefined && currentUserText !== session.userIntent) {
        const original = draftCanvasContract(session.brief, session.userIntent)
        const current = draftCanvasContract(session.brief, currentUserText)
        if ((hasExplicitCanvasIntent(currentUserText)
          && (original.platform !== current.platform || original.width !== current.width
            || original.seedHeight !== current.seedHeight || original.finalHeight !== current.finalHeight
            || original.fixedViewport !== current.fixedViewport))
          || (!session.commerce && COMMERCE_BRIEF.test(currentUserText))) {
          return asResult({ status: 'conflict', reason: 'design_intent_changed', next: 'Replan the updated design brief before confirming a style.' })
        }
      }
      if (!services.beginStyleInputSupported()) {
        return asResult({ status: 'unsupported', reason: 'selection_handoff_unsupported' })
      }
      if (session.commerce) {
        return asResult({ status: 'unsupported', reason: session.platform === 'web' ? 'web-commerce' : 'commerce' })
      }
      const snapshot = services.resolveCatalog()
      const guide = snapshot.guides.find(candidate => candidate.id === args.guide)
      if (guide === undefined) {
        throw new Error(`${name}: unknown catalog guide id "${args.guide}"; use the exact canonical id from the browse result`)
      }
      if (!guideSupportsHandoff(guide)) {
        return asResult({ status: 'unsupported', reason: 'slides-card-reference-only' })
      }
      const catalogPlatform = session.platform === 'web' ? 'webapp' : 'mobile'
      if (guide.platform !== catalogPlatform) {
        return asResult({ status: 'conflict', reason: `guide platform ${guide.platform} does not support the ${session.platform} handoff platform` })
      }
      const guideTheme = styleGuideTheme(guide)
      if (session.theme !== undefined && guideTheme !== undefined && guideTheme !== session.theme) {
        return asResult({ status: 'conflict', reason: `confirmed ${session.theme} theme conflicts with the catalog's verified ${guideTheme}-mode metadata` })
      }
      const now = services.now()
      const fonts = supportedFontMappingOf(guide)
      const existing = session.lastConfirmed !== undefined
        ? store.validate(session.lastConfirmed.handle, {
          sessionKey,
          guideId: guide.id,
          briefFingerprint: session.briefFingerprint,
          platform: session.platform,
          catalogRevision: snapshot.revision,
          constraintRevision: session.constraintRevision,
          now,
        })
        : undefined
      const retained = existing?.ok === true
      const handle = retained
        ? existing.record.handle
        : store.confirm({
          sessionKey,
          guideId: guide.id,
          briefFingerprint: session.briefFingerprint,
          platform: session.platform,
          constraintRevision: session.constraintRevision,
          catalogRevision: snapshot.revision,
          ttlMs: HANDLE_TTL_MS,
          ...(session.userIntent === undefined ? {} : { userIntent: session.userIntent }),
          ...(currentUserText === undefined ? {} : { confirmedUserFingerprint: store.briefFingerprintOf(currentUserText) }),
        })
      if (retained && currentUserText !== undefined) existing.record.confirmedUserFingerprint = store.briefFingerprintOf(currentUserText)
      session.lastConfirmed = { handle, guideId: guide.id, catalogRevision: snapshot.revision }
      store.recordSession(session)
      const recommendation = session.lastRecommendation
      if (recommendation?.valid === true && recommendation.top.length > 0 && recommendation.top[0] !== guide.id) {
        store.events.push({ kind: 'override', from: recommendation.top[0], to: guide.id })
      } else if (recommendation?.valid === true && recommendation.top.includes(guide.id) && recommendation.top[0] !== guide.id) {
        store.events.push({ kind: 'catalog-alternative-selected', guideId: guide.id })
      } else if (recommendation?.valid !== true) {
        store.events.push({ kind: 'manual-selected', guideId: guide.id })
      }
      return asResult({
        status: 'confirmed',
        handle,
        guideId: guide.id,
        catalogRevision: snapshot.revision,
        expiresAt: now + HANDLE_TTL_MS,
        fonts,
        ...(retained ? { retained: true } : {}),
        next: `Call openpencil_pipeline_begin with the user's original brief unchanged and style_selection set to this handle within 15 minutes. The handle is bound to this session, brief, platform, constraints, and catalog revision.`,
      })
    },
    presentCall: () => ({ card: 'generic', title: 'Confirm OpenPencil style selection', kind: 'execute' }),
  })

  return [planTool, confirmTool]
}

/** Default catalog resolution: the installed snapshot shipped with the build. */
export function defaultCatalogResolver(
  catalog?: StyleCatalogSnapshot | (() => StyleCatalogSnapshot),
): () => StyleCatalogSnapshot {
  return () => {
    if (catalog === undefined) return loadInstalledStyleCatalog()
    return typeof catalog === 'function' ? catalog() : catalog
  }
}
