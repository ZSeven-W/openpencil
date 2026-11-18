/**
 * App-agent engine support for the OpenPencil pipeline.
 *
 * `openpencil_pipeline_begin({engine:"app-agent"})` routes generation through
 * the daemon's built-in design-agent loop (`run_design_agent`): the same
 * skills-assembled system prompt, 15-tool loop, screenshot verification, and
 * finalize-on-exit the OpenPencil App uses. DSH keeps ownership of the
 * transaction shell — draft lifecycle, quality gates, enrichment, and atomic
 * publication stay exactly the script-engine path.
 *
 * The single hard budget is the plugin's MCP HTTP layer (5-minute request
 * cap): one blocking loop call must finish inside it, so the defaults here
 * are deliberately tighter than the daemon's own 600s/28-turn defaults.
 * @module dsh-openpencil/design-agent-run
 */

import type { JsonValue } from './json-value.js'

/** Engine selector accepted by `openpencil_pipeline_begin`. */
export type PipelineEngine = 'script' | 'app-agent'

/** Loop-call wall clock (seconds) forwarded to `run_design_agent`. */
export const DEFAULT_AGENT_RUN_TIMEOUT_SECONDS = 240
export const MIN_AGENT_RUN_TIMEOUT_SECONDS = 30
export const MAX_AGENT_RUN_TIMEOUT_SECONDS = 270

/**
 * Turn budget forwarded to the loop. The daemon default (28) is sized for a
 * 600s wall clock; inside the plugin's tighter clock an overshoot times out
 * atomically and loses the whole run, so the default halves.
 */
export const DEFAULT_AGENT_RUN_MAX_TURNS = 12
export const MIN_AGENT_RUN_MAX_TURNS = 4
export const MAX_AGENT_RUN_MAX_TURNS = 28

export function parsePipelineEngine(value: unknown, toolName: string): PipelineEngine {
  if (value === undefined || value === 'script') return 'script'
  if (value === 'app-agent') return 'app-agent'
  throw new Error(`${toolName}: engine must be "script" or "app-agent"`)
}

export function validateAgentRunTimeoutSeconds(value: unknown, toolName: string): number {
  if (value === undefined) return DEFAULT_AGENT_RUN_TIMEOUT_SECONDS
  if (
    typeof value !== 'number'
    || !Number.isFinite(value)
    || value < MIN_AGENT_RUN_TIMEOUT_SECONDS
    || value > MAX_AGENT_RUN_TIMEOUT_SECONDS
  ) {
    throw new Error(
      `${toolName}: timeout_seconds must be a number between ${MIN_AGENT_RUN_TIMEOUT_SECONDS} and ${MAX_AGENT_RUN_TIMEOUT_SECONDS}`,
    )
  }
  return Math.ceil(value)
}

export function validateAgentRunMaxTurns(value: unknown, toolName: string): number {
  if (value === undefined) return DEFAULT_AGENT_RUN_MAX_TURNS
  if (
    typeof value !== 'number'
    || !Number.isSafeInteger(value)
    || value < MIN_AGENT_RUN_MAX_TURNS
    || value > MAX_AGENT_RUN_MAX_TURNS
  ) {
    throw new Error(
      `${toolName}: max_turns must be an integer between ${MIN_AGENT_RUN_MAX_TURNS} and ${MAX_AGENT_RUN_MAX_TURNS}`,
    )
  }
  return value
}

/** Public, bounded projection of the daemon loop report. */
export interface AgentRunSummary {
  toolCalls: number
  stopReason: string
  landedRoots: number
  finalize?: {
    committedScreens: number
    unfilledScreens: number
    qualityChecks: number
    qualityRepairs: number
    qualityNotes: number
  }
  loopError?: string
}

function boundedCount(value: unknown): number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
    ? Math.min(value, 1_000_000)
    : 0
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

/**
 * Reduce the raw `run_design_agent` JSON to the fixed public summary. The
 * daemon result is trusted infrastructure but still bounded here so the tool
 * result (which reaches the model verbatim) cannot balloon.
 */
export function summarizeAgentRun(value: JsonValue | undefined): AgentRunSummary {
  const raw = isRecord(value) ? value : {}
  const finalize = isRecord(raw.finalize) ? raw.finalize : undefined
  const loopError = typeof raw.loopError === 'string' && raw.loopError.trim().length > 0
    ? raw.loopError.slice(0, 400)
    : undefined
  return {
    toolCalls: boundedCount(raw.toolCalls),
    stopReason: typeof raw.stopReason === 'string' ? raw.stopReason.slice(0, 80) : 'unknown',
    landedRoots: boundedCount(raw.landedRoots),
    ...(finalize === undefined ? {} : {
      finalize: {
        committedScreens: boundedCount(finalize.committedScreens),
        unfilledScreens: boundedCount(finalize.unfilledScreens),
        qualityChecks: boundedCount(finalize.qualityChecks),
        qualityRepairs: boundedCount(finalize.qualityRepairs),
        qualityNotes: boundedCount(finalize.qualityNotes),
      },
    }),
    ...(loopError === undefined ? {} : { loopError }),
  }
}

/**
 * Provider/key problems come back as structured daemon errors. They are
 * terminal configuration states, never retry targets, so the pipeline
 * message routes the operator to the daemon's agent settings.
 */
export function isProviderConfigurationError(message: string): boolean {
  return /provider|api[ _-]?key|credential|not configured|no builtin agent/i.test(message)
}

export const AGENT_RUN_PROVIDER_GUIDANCE =
  'The daemon has no ready builtin design-agent provider. Configure one in the '
  + 'OpenPencil agent settings (an OpenAI-compatible endpoint or Anthropic key) '
  + 'and start a fresh pipeline; do not retry this draft.'
