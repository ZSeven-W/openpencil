/** Persistent, owner-isolated design drafts backed by managed OpenPencil daemons. */

import { createHash, randomBytes } from 'node:crypto'
import { mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { join } from 'node:path'
import {
  callOpenPencilMcp,
  getOpenPencilMcpVersion,
  OpenPencilMcpTransportError,
  type OpenPencilMcpResult,
} from './mcp-client.js'
import { restoreManagedDaemonDocument } from './editor-recovery.js'
import {
  readManagedEditorDaemon,
  resetManagedEditorDaemonFromSource,
  startManagedEditorDaemon,
  startManagedEditorReadOnlyProxy,
  stopManagedEditorDaemon,
  type ManagedEditorDaemon,
  type ManagedEditorReadOnlyProxy,
} from './managed-editor-daemon.js'
import type { EditorRuntime } from './editor-runtime.js'

const DEFAULT_IDLE_MS = 30 * 60 * 1000
const DEFAULT_ABSOLUTE_MS = 2 * 60 * 60 * 1000
const DEFAULT_MAX_DRAFTS = 8
const DEFAULT_ENRICH_TIMEOUT_MS = 130_000
const MAX_ENRICH_TIMEOUT_MS = 150_000
// The daemon design-agent loop is one long blocking MCP call. Its ceiling
// must stay under the MCP client's hard 5-minute request cap (with margin
// for the daemon to settle its own timeout before the HTTP layer fires).
const MAX_AGENT_RUN_TIMEOUT_MS = 285_000
const DEFAULT_AGENT_RUN_TIMEOUT_MS = 255_000
const MAX_ORDINARY_TIMEOUT_MS = 30_000
const MAX_ARGUMENT_BYTES = 512 * 1024
const EMPTY_DOCUMENT_JSON = '{\n  "version": "1.0.0",\n  "children": []\n}\n'
const DRAFT_ID_PATTERN = /^[A-Za-z0-9_-]{32}$/

const READ_TOOLS = new Set([
  'get_design_agent_prompt',
  'list_ui_kits',
  'get_design_quality',
  'get_editor_state',
  'get_variables',
  'get_active_theme',
  'get_guidelines',
  'get_style_guide_tags',
  'get_style_guide',
  'list_style_guides',
  'get_design_prompt',
  'get_design_md',
  'batch_get',
  'read_nodes',
  'snapshot_layout',
  'get_screenshot',
  'find_empty_space',
  'get_canvas_bounds',
  'find_node_by_name',
  'get_node',
  'get_node_parent',
  'get_node_children',
  'count_nodes',
  'list_node_kinds',
  'get_document_info',
  'list_pages',
  'list_components',
  'get_component',
  'lint_document',
])
const MUST_CHANGE_TOOLS = new Set([
  'apply_design_system',
  'batch_design',
  'set_active_axis_value',
  'set_themes',
  'set_variables',
  'design_skeleton',
  'design_content',
  'design_refine',
  'update_node',
])
const MAY_NOOP_TOOLS = new Set(['finalize_design', 'enrich_images', 'run_design_agent'])
const FORBIDDEN_ARGUMENT_NAME = /(?:path|url|export|import|spawn)/i

export type DesignDraftToolMode = 'read' | 'must-change' | 'may-noop'

export function designDraftToolMode(tool: string): DesignDraftToolMode | undefined {
  if (READ_TOOLS.has(tool)) return 'read'
  if (MUST_CHANGE_TOOLS.has(tool)) return 'must-change'
  if (MAY_NOOP_TOOLS.has(tool)) return 'may-noop'
  return undefined
}

export interface DesignDraftTarget {
  id: string
  label?: string
  kind?: string
}

export interface DesignDraftMetadata {
  draftId: string
  target: DesignDraftTarget
  createdAt: number
  idleExpiresAt: number
  absoluteExpiresAt: number
  version: number
}

export interface DesignDraftBeginOptions {
  ownerSessionId: string
  target: DesignDraftTarget
  signal: AbortSignal
}

export interface DesignDraftCallOptions {
  signal?: AbortSignal
  timeoutMs?: number
  /** Optional optimistic guard for a controller-owned compound operation. */
  expectedVersion?: number
}

export interface DesignDraftCallResult {
  draftId: string
  target: DesignDraftTarget
  tool: string
  value: unknown
  text: string
  version: number
  changed: boolean
  hasImage: boolean
}

export interface DesignDraftSnapshot {
  draftId: string
  target: DesignDraftTarget
  version: number
  documentJson: string
}

export interface DesignDraftScreenshot {
  draftId: string
  target: DesignDraftTarget
  version: number
  documentSha256: string
  bytes: Buffer
  mimeType: string
  metadata: unknown
}

/** Browser attachment to the exact daemon that owns an unpublished draft. */
export interface DesignDraftLiveLaunch {
  draftId: string
  target: DesignDraftTarget
  attachId: string
  baseUrl: string
  token: string
  sourcePath: string
  documentJson: string
  version: number
}

export interface DesignDraftFinishOptions<Published> {
  signal?: AbortSignal
  requireCurrentScreenshot?: boolean
  /** Optional finalized checkpoint expected by the caller. Both fields must be supplied together. */
  expectedVersion?: number
  expectedDocumentSha256?: string
  /** Runs while the draft's serialized operation lock is still held. */
  publish: (snapshot: DesignDraftSnapshot) => Promise<Published>
}

export interface DesignDraftFinishResult<Published> {
  draftId: string
  target: DesignDraftTarget
  version: number
  published: Published
}

export interface DesignDraftControllerOptions {
  maxDrafts?: number
  idleMs?: number
  absoluteMs?: number
  now?: () => number
  runtimeUnavailableMessage?: string
  onDraftEnded?: (draftId: string, ownerSessionId: string) => void
}

interface DesignDraftSession {
  id: string
  ownerSessionId: string
  target: DesignDraftTarget
  daemon: ManagedEditorDaemon
  tempRoot: string
  createdAt: number
  touchedAt: number
  closed: boolean
  retirement: AbortController
  timer?: NodeJS.Timeout
  activeOperation?: Promise<void>
  lastScreenshotVersion?: number
  lastScreenshotDocumentSha256?: string
  lastScreenshotFinalizedVersion?: number
  lastScreenshotFinalizedDocumentSha256?: string
  finalizedVersion?: number
  finalizedDocumentSha256?: string
  publishing: boolean
  bootstrapResetConsumed: boolean
  liveAttach?: {
    id: string
    ready: boolean
    proxy: ManagedEditorReadOnlyProxy
  }
}

export class DesignDraftPreviewRequiredError extends Error {
  readonly code = 'OPENPENCIL_DRAFT_PREVIEW_REQUIRED'
  readonly currentVersion: number
  readonly finalizedVersion?: number
  readonly screenshotVersion?: number
  readonly screenshotDocumentSha256?: string
  readonly screenshotFinalizedVersion?: number
  readonly screenshotFinalizedDocumentSha256?: string

  constructor(state: {
    currentVersion: number
    finalizedVersion?: number
    screenshotVersion?: number
    screenshotDocumentSha256?: string
    screenshotFinalizedVersion?: number
    screenshotFinalizedDocumentSha256?: string
  }) {
    super('OpenPencil draft must have an exact root preview artifact from its current finalized document before publishing')
    this.currentVersion = state.currentVersion
    this.finalizedVersion = state.finalizedVersion
    this.screenshotVersion = state.screenshotVersion
    this.screenshotDocumentSha256 = state.screenshotDocumentSha256
    this.screenshotFinalizedVersion = state.screenshotFinalizedVersion
    this.screenshotFinalizedDocumentSha256 = state.screenshotFinalizedDocumentSha256
  }
}

export class DesignDraftCheckpointDriftError extends Error {
  readonly code = 'OPENPENCIL_DRAFT_CHECKPOINT_DRIFT'

  constructor(
    readonly expectedVersion: number,
    readonly expectedDocumentSha256: string,
    readonly currentVersion: number,
    readonly currentDocumentSha256: string,
  ) {
    super('OpenPencil draft changed after its finalized checkpoint was validated')
  }
}

function documentSha256(documentJson: string): string {
  return createHash('sha256').update(documentJson).digest('hex')
}

function clearPublicationProof(session: DesignDraftSession): void {
  delete session.lastScreenshotVersion
  delete session.lastScreenshotDocumentSha256
  delete session.lastScreenshotFinalizedVersion
  delete session.lastScreenshotFinalizedDocumentSha256
  delete session.finalizedVersion
  delete session.finalizedDocumentSha256
}

function copyTarget(target: DesignDraftTarget): DesignDraftTarget {
  if (typeof target.id !== 'string' || target.id.length === 0 || target.id.length > 512 || target.id.includes('\0')) {
    throw new Error('OpenPencil draft target id is invalid')
  }
  if (target.label !== undefined && (target.label.length === 0 || target.label.length > 512 || target.label.includes('\0'))) {
    throw new Error('OpenPencil draft target label is invalid')
  }
  if (target.kind !== undefined && (target.kind.length === 0 || target.kind.length > 128 || target.kind.includes('\0'))) {
    throw new Error('OpenPencil draft target kind is invalid')
  }
  return {
    id: target.id,
    ...(target.label === undefined ? {} : { label: target.label }),
    ...(target.kind === undefined ? {} : { kind: target.kind }),
  }
}

function validOwner(ownerSessionId: string): void {
  if (typeof ownerSessionId !== 'string' || ownerSessionId.length === 0 || ownerSessionId.length > 256 || ownerSessionId.includes('\0')) {
    throw new Error('OpenPencil draft owner session is invalid')
  }
}

function assertSafeArguments(
  value: unknown,
  location = 'arguments',
  depth = 0,
  seen: WeakSet<object> = new WeakSet(),
): void {
  if (depth > 20) throw new Error('OpenPencil draft MCP arguments are too deeply nested')
  if (Array.isArray(value)) {
    if (seen.has(value)) throw new Error('OpenPencil draft MCP arguments must not contain cycles')
    seen.add(value)
    value.forEach((item, index) => assertSafeArguments(item, `${location}[${index}]`, depth + 1, seen))
    return
  }
  if (typeof value !== 'object' || value === null) return
  if (seen.has(value)) throw new Error('OpenPencil draft MCP arguments must not contain cycles')
  seen.add(value)
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    if (FORBIDDEN_ARGUMENT_NAME.test(key)) {
      throw new Error(`OpenPencil draft MCP arguments cannot include ${key}`)
    }
    assertSafeArguments(child, `${location}.${key}`, depth + 1, seen)
  }
}

function replaceSecret(value: string, secret: string, replacement: string): string {
  return secret.length === 0 ? value : value.split(secret).join(replacement)
}

function sanitizeInternalValue(value: unknown, session: DesignDraftSession): unknown {
  if (typeof value === 'string') {
    return replaceSecret(
      replaceSecret(replaceSecret(value, session.daemon.token, '[redacted]'), session.tempRoot, '[managed-draft]'),
      session.daemon.sourcePath,
      '[managed-draft].op',
    )
  }
  if (Array.isArray(value)) return value.map(item => sanitizeInternalValue(item, session))
  if (typeof value !== 'object' || value === null) return value
  const sanitized: Record<string, unknown> = {}
  for (const [key, item] of Object.entries(value as Record<string, unknown>)) {
    sanitized[key] = /(?:^|_)(?:token|authorization|bearer)(?:$|_)/i.test(key)
      ? '[redacted]'
      : sanitizeInternalValue(item, session)
  }
  return sanitized
}

function sanitizeInternalError(error: unknown, session: DesignDraftSession): Error {
  const original = error instanceof Error ? error : new Error(String(error))
  const message = sanitizeInternalValue(original.message, session) as string
  if (message === original.message) return original
  const sanitized = new Error(message)
  sanitized.name = original.name
  const code = (original as Error & { code?: unknown }).code
  if (typeof code === 'string') (sanitized as Error & { code: string }).code = code
  return sanitized
}

function signalFor(session: DesignDraftSession, signal?: AbortSignal): AbortSignal {
  return signal === undefined
    ? session.retirement.signal
    : AbortSignal.any([signal, session.retirement.signal])
}

function isUncertain(error: unknown, signal: AbortSignal): boolean {
  return signal.aborted
    || error instanceof OpenPencilMcpTransportError
    || (error instanceof Error && (error.name === 'AbortError' || error.name === 'TimeoutError'))
}

/** A separate draft map; browser-visible EditorSession authority is never reused. */
export class DesignDraftController {
  readonly #runtime: EditorRuntime | undefined
  readonly #maxDrafts: number
  readonly #idleMs: number
  readonly #absoluteMs: number
  readonly #now: () => number
  readonly #runtimeUnavailableMessage: string
  readonly #onDraftEnded: ((draftId: string, ownerSessionId: string) => void) | undefined
  readonly #drafts = new Map<string, DesignDraftSession>()
  readonly #ownerDraft = new Map<string, string>()
  readonly #pendingBegins = new Map<string, AbortController>()
  readonly #controllerAbort = new AbortController()
  #lifecycleQueue: Promise<void> = Promise.resolve()
  #disposePromise: Promise<void> | undefined
  #disposed = false

  constructor(runtime: EditorRuntime | undefined, options: DesignDraftControllerOptions = {}) {
    this.#runtime = runtime
    this.#maxDrafts = options.maxDrafts ?? DEFAULT_MAX_DRAFTS
    this.#idleMs = options.idleMs ?? DEFAULT_IDLE_MS
    this.#absoluteMs = options.absoluteMs ?? DEFAULT_ABSOLUTE_MS
    this.#now = options.now ?? Date.now
    this.#runtimeUnavailableMessage = options.runtimeUnavailableMessage ?? 'OpenPencil editor runtime is unavailable'
    this.#onDraftEnded = options.onDraftEnded
    if (!Number.isSafeInteger(this.#maxDrafts) || this.#maxDrafts < 1 || this.#maxDrafts > 64) {
      throw new TypeError('OpenPencil draft global limit is invalid')
    }
    if (!Number.isSafeInteger(this.#idleMs) || this.#idleMs < 1) throw new TypeError('OpenPencil draft idle timeout is invalid')
    if (!Number.isSafeInteger(this.#absoluteMs) || this.#absoluteMs < this.#idleMs) {
      throw new TypeError('OpenPencil draft absolute timeout is invalid')
    }
  }

  async begin(options: DesignDraftBeginOptions): Promise<DesignDraftMetadata> {
    validOwner(options.ownerSessionId)
    const target = copyTarget(options.target)
    options.signal.throwIfAborted()
    if (this.#disposed) throw new Error('OpenPencil draft controller is shutting down')
    this.#pruneExpired()
    if (this.#ownerDraft.has(options.ownerSessionId) || this.#pendingBegins.has(options.ownerSessionId)) {
      throw new Error('This DSH session already has an active OpenPencil draft')
    }
    if (this.#drafts.size + this.#pendingBegins.size >= this.#maxDrafts) {
      throw new Error('OpenPencil draft capacity is full; finish or abort an existing draft first')
    }
    const pendingAbort = new AbortController()
    this.#pendingBegins.set(options.ownerSessionId, pendingAbort)
    try {
      return await this.#serializeLifecycle(async () => {
        const startupSignal = AbortSignal.any([
          options.signal,
          this.#controllerAbort.signal,
          pendingAbort.signal,
        ])
        startupSignal.throwIfAborted()
        if (this.#disposed) throw new Error('OpenPencil draft controller is shutting down')
        if (this.#ownerDraft.has(options.ownerSessionId)) {
          throw new Error('This DSH session already has an active OpenPencil draft')
        }
        const runtime = this.#runtime
        if (runtime === undefined) throw new Error(this.#runtimeUnavailableMessage)
        const tempRoot = await mkdtemp(join(tmpdir(), 'dsh-openpencil-draft-'))
        const sourcePath = join(tempRoot, 'draft.op')
        let daemon: ManagedEditorDaemon | undefined
        let registered = false
        try {
          await writeFile(sourcePath, EMPTY_DOCUMENT_JSON, { flag: 'wx', mode: 0o600 })
          startupSignal.throwIfAborted()
          daemon = await startManagedEditorDaemon(runtime, {
            sourcePath,
            allowOrigin: 'http://127.0.0.1',
            signal: startupSignal,
          })
          startupSignal.throwIfAborted()
          const version = await getOpenPencilMcpVersion({
            baseUrl: daemon.baseUrl,
            token: daemon.token,
            signal: startupSignal,
          })
          startupSignal.throwIfAborted()
          const now = this.#now()
          const session: DesignDraftSession = {
            id: randomBytes(24).toString('base64url'),
            ownerSessionId: options.ownerSessionId,
            target,
            daemon,
            tempRoot,
            createdAt: now,
            touchedAt: now,
            closed: false,
            retirement: new AbortController(),
            publishing: false,
            bootstrapResetConsumed: false,
          }
          this.#drafts.set(session.id, session)
          this.#ownerDraft.set(session.ownerSessionId, session.id)
          daemon.child.once('close', () => {
            if (!session.closed) void this.#retire(session, false)
          })
          this.#scheduleExpiry(session)
          registered = true
          daemon = undefined
          return this.#metadata(session, version)
        } finally {
          if (daemon !== undefined) await stopManagedEditorDaemon(daemon)
          if (!registered) {
            await rm(tempRoot, { recursive: true, force: true }).catch(() => {})
          }
        }
      })
    } finally {
      if (this.#pendingBegins.get(options.ownerSessionId) === pendingAbort) {
        this.#pendingBegins.delete(options.ownerSessionId)
      }
    }
  }

  async call(
    draftId: string,
    ownerSessionId: string,
    tool: string,
    args: Record<string, unknown>,
    options: DesignDraftCallOptions = {},
  ): Promise<DesignDraftCallResult> {
    const session = this.#session(draftId, ownerSessionId)
    return this.#enqueue(session, () => this.#callLocked(session, tool, args, options))
  }

  async snapshot(
    draftId: string,
    ownerSessionId: string,
    options: Pick<DesignDraftCallOptions, 'signal'> = {},
  ): Promise<DesignDraftSnapshot> {
    const session = this.#session(draftId, ownerSessionId)
    return this.#enqueue(session, async () => {
      const signal = signalFor(session, options.signal)
      signal.throwIfAborted()
      const document = await readManagedEditorDaemon(session.daemon, signal)
      signal.throwIfAborted()
      this.#touch(session)
      return this.#snapshot(session, document.version, document.documentJson)
    })
  }

  /**
   * Atomically restore a controller-owned snapshot after a rejected batch.
   * The current daemon version is used as the optimistic base so a concurrent
   * live-canvas edit conflicts instead of being overwritten.
   */
  async restoreSnapshot(
    draftId: string,
    ownerSessionId: string,
    snapshot: DesignDraftSnapshot,
    options: Pick<DesignDraftCallOptions, 'signal' | 'expectedVersion'> = {},
  ): Promise<DesignDraftSnapshot> {
    const session = this.#session(draftId, ownerSessionId)
    if (snapshot.draftId !== draftId || snapshot.target.id !== session.target.id) {
      throw new Error('OpenPencil draft restore snapshot does not belong to this draft')
    }
    return this.#enqueue(session, async () => {
      const signal = signalFor(session, options.signal)
      signal.throwIfAborted()
      try {
        const current = await readManagedEditorDaemon(session.daemon, signal)
        if (options.expectedVersion !== undefined && current.version !== options.expectedVersion) {
          throw new Error('OpenPencil draft changed before its rejected batch could be restored')
        }
        signal.throwIfAborted()
        const restoredVersion = await restoreManagedDaemonDocument(
          session.daemon.baseUrl,
          session.daemon.token,
          { documentJson: snapshot.documentJson, version: current.version },
        )
        signal.throwIfAborted()
        const confirmed = await readManagedEditorDaemon(session.daemon, signal)
        if (confirmed.version !== restoredVersion || confirmed.documentJson !== snapshot.documentJson) {
          throw new Error('OpenPencil draft restore could not confirm the authoritative document')
        }
        clearPublicationProof(session)
        this.#touch(session)
        return this.#snapshot(session, confirmed.version, confirmed.documentJson)
      } catch (error) {
        throw sanitizeInternalError(error, session)
      }
    })
  }

  finalize(
    draftId: string,
    ownerSessionId: string,
    options: DesignDraftCallOptions = {},
  ): Promise<DesignDraftCallResult> {
    return this.call(draftId, ownerSessionId, 'finalize_design', {}, options)
  }

  async screenshot(
    draftId: string,
    ownerSessionId: string,
    options: Pick<DesignDraftCallOptions, 'signal' | 'timeoutMs'> & { nodeId?: string } = {},
  ): Promise<DesignDraftScreenshot> {
    const session = this.#session(draftId, ownerSessionId)
    return this.#enqueue(session, async () => {
      if (options.timeoutMs !== undefined && (
        !Number.isSafeInteger(options.timeoutMs) || options.timeoutMs < 1 || options.timeoutMs > 60_000
      )) throw new Error('OpenPencil screenshot timeout is invalid')
      const signal = signalFor(session, options.signal)
      signal.throwIfAborted()
      const requestedNodeId = options.nodeId?.trim()
      const rootScreenshot = requestedNodeId === undefined || requestedNodeId === '' || requestedNodeId === 'root'
      const beforeDocument = await readManagedEditorDaemon(session.daemon, signal)
      const beforeDocumentSha256 = documentSha256(beforeDocument.documentJson)
      let result: OpenPencilMcpResult
      try {
        result = await callOpenPencilMcp({
          baseUrl: session.daemon.baseUrl,
          token: session.daemon.token,
          tool: 'get_screenshot',
          arguments: { nodeId: rootScreenshot ? 'root' : requestedNodeId },
          signal,
          ...(options.timeoutMs === undefined ? {} : { timeoutMs: options.timeoutMs }),
        })
      } catch (error) {
        throw sanitizeInternalError(error, session)
      }
      const afterDocument = await readManagedEditorDaemon(session.daemon, signal)
      const afterDocumentSha256 = documentSha256(afterDocument.documentJson)
      if (
        afterDocument.version !== beforeDocument.version
        || afterDocumentSha256 !== beforeDocumentSha256
      ) {
        throw new Error('OpenPencil document changed while its exact root preview artifact was rendered')
      }
      if (result.images.length !== 1) throw new Error('OpenPencil screenshot did not return exactly one bounded image')
      if (rootScreenshot) {
        session.lastScreenshotVersion = afterDocument.version
        session.lastScreenshotDocumentSha256 = afterDocumentSha256
        const matchesFinalization = session.finalizedVersion === afterDocument.version
          && session.finalizedDocumentSha256 === afterDocumentSha256
        session.lastScreenshotFinalizedVersion = matchesFinalization
          ? afterDocument.version
          : undefined
        session.lastScreenshotFinalizedDocumentSha256 = matchesFinalization
          ? afterDocumentSha256
          : undefined
      }
      this.#touch(session)
      const image = result.images[0]!
      return {
        draftId: session.id,
        target: { ...session.target },
        version: afterDocument.version,
        documentSha256: afterDocumentSha256,
        bytes: Buffer.from(image.bytes),
        mimeType: image.mimeType,
        metadata: sanitizeInternalValue(result.value, session),
      }
    })
  }

  async finish<Published>(
    draftId: string,
    ownerSessionId: string,
    options: DesignDraftFinishOptions<Published>,
  ): Promise<DesignDraftFinishResult<Published>> {
    const session = this.#session(draftId, ownerSessionId)
    return this.#enqueue(session, async () => {
      const signal = signalFor(session, options.signal)
      signal.throwIfAborted()
      const document = await readManagedEditorDaemon(session.daemon, signal)
      signal.throwIfAborted()
      const authoritativeSha256 = documentSha256(document.documentJson)
      const hasExpectedVersion = options.expectedVersion !== undefined
      const hasExpectedSha256 = options.expectedDocumentSha256 !== undefined
      if (hasExpectedVersion !== hasExpectedSha256) {
        throw new Error('OpenPencil draft finish expectedVersion and expectedDocumentSha256 must be provided together')
      }
      if (hasExpectedVersion && (
        !Number.isSafeInteger(options.expectedVersion)
        || options.expectedVersion! < 0
        || !/^[a-f0-9]{64}$/u.test(options.expectedDocumentSha256!)
      )) throw new Error('OpenPencil draft finish checkpoint is invalid')
      if (hasExpectedVersion && (
        options.expectedVersion !== document.version
        || options.expectedDocumentSha256 !== authoritativeSha256
      )) {
        clearPublicationProof(session)
        throw new DesignDraftCheckpointDriftError(
          options.expectedVersion!,
          options.expectedDocumentSha256!,
          document.version,
          authoritativeSha256,
        )
      }
      if ((options.requireCurrentScreenshot ?? true) && (
        session.lastScreenshotVersion !== document.version
        || session.lastScreenshotDocumentSha256 !== authoritativeSha256
        || session.lastScreenshotFinalizedVersion !== document.version
        || session.lastScreenshotFinalizedDocumentSha256 !== authoritativeSha256
        || session.finalizedVersion !== document.version
        || session.finalizedDocumentSha256 !== authoritativeSha256
      )) {
        throw new DesignDraftPreviewRequiredError({
          currentVersion: document.version,
          ...(session.finalizedVersion === undefined ? {} : { finalizedVersion: session.finalizedVersion }),
          ...(session.lastScreenshotVersion === undefined ? {} : { screenshotVersion: session.lastScreenshotVersion }),
          ...(session.lastScreenshotDocumentSha256 === undefined
            ? {}
            : { screenshotDocumentSha256: session.lastScreenshotDocumentSha256 }),
          ...(session.lastScreenshotFinalizedVersion === undefined
            ? {}
            : { screenshotFinalizedVersion: session.lastScreenshotFinalizedVersion }),
          ...(session.lastScreenshotFinalizedDocumentSha256 === undefined
            ? {}
            : { screenshotFinalizedDocumentSha256: session.lastScreenshotFinalizedDocumentSha256 }),
        })
      }
      const snapshot = this.#snapshot(session, document.version, document.documentJson)
      // Publication deliberately stays under the same serialized draft lock.
      // A failed callback leaves the draft active and retryable.
      session.publishing = true
      let published: Published
      try {
        published = await options.publish(snapshot)
      } finally {
        session.publishing = false
      }
      session.closed = true
      this.#removeAuthority(session)
      await this.#cleanup(session)
      return {
        draftId: session.id,
        target: { ...session.target },
        version: document.version,
        published,
      }
    })
  }

  async abort(draftId: string, ownerSessionId: string): Promise<boolean> {
    if (!DRAFT_ID_PATTERN.test(draftId)) return false
    validOwner(ownerSessionId)
    const session = this.#drafts.get(draftId)
    if (session === undefined) return false
    if (session.ownerSessionId !== ownerSessionId) throw new Error('OpenPencil draft belongs to a different DSH session')
    await this.#retire(session, true)
    return true
  }

  /**
   * Freeze the current daemon document into its private starter file and
   * consume the daemon's one-shot sync reset before exposing the iframe. The
   * browser's later bootstrap reset is therefore an idempotent skipped call,
   * so subsequent Agent writes never wait on (or race) Web/Wasm startup.
   */
  async prepareLiveLaunch(
    draftId: string,
    ownerSessionId: string,
    signal?: AbortSignal,
  ): Promise<DesignDraftLiveLaunch> {
    const session = this.#session(draftId, ownerSessionId)
    return this.#enqueue(session, async () => {
      const operationSignal = signalFor(session, signal)
      operationSignal.throwIfAborted()
      this.#clearLiveAttach(session)
      const document = await readManagedEditorDaemon(session.daemon, operationSignal)
      operationSignal.throwIfAborted()
      // The path is a controller-owned 0700 temp directory and never accepts
      // caller input. The Web shell's bootstrap reset reads this exact file.
      await writeFile(session.daemon.sourcePath, document.documentJson, { mode: 0o600 })
      operationSignal.throwIfAborted()
      let version = document.version
      if (!session.bootstrapResetConsumed) {
        let reset: { version: number; skipped: boolean } | undefined
        let resetFailure: unknown
        try {
          reset = await resetManagedEditorDaemonFromSource(session.daemon, operationSignal)
        } catch (error) {
          resetFailure = error
        }

        let confirmed
        try {
          confirmed = await readManagedEditorDaemon(session.daemon, operationSignal)
        } catch (error) {
          await this.#retire(session, false)
          throw sanitizeInternalError(resetFailure ?? error, session)
        }
        const exactDocument = confirmed.documentJson === document.documentJson
        const confirmedCommit = reset === undefined
          ? confirmed.version > document.version
          : reset.skipped
            ? true
            : reset.version > document.version && confirmed.version >= reset.version
        if (!exactDocument || !confirmedCommit) {
          await this.#retire(session, false)
          throw sanitizeInternalError(
            resetFailure ?? new Error('OpenPencil managed daemon initial reset could not be confirmed'),
            session,
          )
        }
        session.bootstrapResetConsumed = true
        version = confirmed.version
      }
      operationSignal.throwIfAborted()

      const attachId = randomBytes(24).toString('base64url')
      let proxy: ManagedEditorReadOnlyProxy | undefined
      try {
        proxy = await startManagedEditorReadOnlyProxy(session.daemon, operationSignal)
        operationSignal.throwIfAborted()
        session.liveAttach = { id: attachId, ready: false, proxy }
        this.#touch(session)
        return {
          draftId: session.id,
          target: { ...session.target },
          attachId,
          baseUrl: proxy.baseUrl,
          token: proxy.token,
          sourcePath: session.daemon.sourcePath,
          documentJson: document.documentJson,
          version,
        }
      } catch (error) {
        proxy?.stop()
        throw sanitizeInternalError(error, session)
      }
    })
  }

  /** Record that the browser bridge mounted; data safety no longer depends on this edge. */
  markLiveReady(draftId: string, ownerSessionId: string, attachId: string): boolean {
    const session = this.#session(draftId, ownerSessionId)
    const pending = session.liveAttach
    if (pending === undefined || pending.id !== attachId || pending.ready) return false
    pending.ready = true
    this.#touch(session)
    return true
  }

  /** Detach a sidebar without retiring the Agent-owned draft daemon. */
  detachLive(draftId: string, ownerSessionId: string, attachId: string): boolean {
    if (!DRAFT_ID_PATTERN.test(draftId)) return false
    validOwner(ownerSessionId)
    const session = this.#drafts.get(draftId)
    if (session === undefined || session.closed || session.ownerSessionId !== ownerSessionId) return false
    if (session.liveAttach?.id !== attachId) return false
    this.#clearLiveAttach(session)
    this.#touch(session)
    return true
  }

  async abortOwner(ownerSessionId: string): Promise<number> {
    validOwner(ownerSessionId)
    let aborted = 0
    const pending = this.#pendingBegins.get(ownerSessionId)
    if (pending !== undefined) {
      pending.abort(new Error('OpenPencil draft owner session ended during startup'))
      aborted = 1
    }
    const id = this.#ownerDraft.get(ownerSessionId)
    if (id === undefined) return aborted
    const session = this.#drafts.get(id)
    if (session === undefined) {
      this.#ownerDraft.delete(ownerSessionId)
      return aborted
    }
    await this.#retire(session, true)
    return 1
  }

  dispose(): Promise<void> {
    if (this.#disposePromise !== undefined) return this.#disposePromise
    this.#disposed = true
    this.#controllerAbort.abort(new Error('OpenPencil draft controller is shutting down'))
    this.#disposePromise = this.#serializeLifecycle(async () => {
      const sessions = [...this.#drafts.values()]
      await Promise.all(sessions.map(session => this.#retire(session, true)))
    })
    return this.#disposePromise
  }

  async #callLocked(
    session: DesignDraftSession,
    tool: string,
    args: Record<string, unknown>,
    options: DesignDraftCallOptions,
  ): Promise<DesignDraftCallResult> {
    const mode = designDraftToolMode(tool)
    if (mode === undefined) throw new Error(`OpenPencil MCP tool ${tool} is not allowed in a managed design draft`)
    assertSafeArguments(args)
    if (Buffer.byteLength(JSON.stringify(args)) > MAX_ARGUMENT_BYTES) {
      throw new Error(`OpenPencil MCP ${tool} arguments are too large`)
    }
    let timeoutMs = options.timeoutMs
    if (tool === 'run_design_agent') {
      const requestedSeconds = args.timeout_seconds
      if (requestedSeconds !== undefined && (
        typeof requestedSeconds !== 'number'
        || !Number.isFinite(requestedSeconds)
        || requestedSeconds < 30
        || requestedSeconds > 270
      )) throw new Error('OpenPencil run_design_agent timeout_seconds is invalid')
      timeoutMs ??= requestedSeconds === undefined
        ? DEFAULT_AGENT_RUN_TIMEOUT_MS
        : Math.min(MAX_AGENT_RUN_TIMEOUT_MS, Math.ceil(requestedSeconds * 1_000) + 15_000)
      if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > MAX_AGENT_RUN_TIMEOUT_MS) {
        throw new Error('OpenPencil run_design_agent timeout is invalid')
      }
    } else if (tool === 'enrich_images') {
      const requestedSeconds = args.timeout_seconds
      if (requestedSeconds !== undefined && (
        typeof requestedSeconds !== 'number'
        || !Number.isFinite(requestedSeconds)
        || requestedSeconds < 1
        || requestedSeconds > 120
      )) throw new Error('OpenPencil enrich_images timeout_seconds is invalid')
      timeoutMs ??= requestedSeconds === undefined
        ? DEFAULT_ENRICH_TIMEOUT_MS
        : Math.min(MAX_ENRICH_TIMEOUT_MS, Math.ceil(requestedSeconds * 1_000) + 10_000)
      if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > MAX_ENRICH_TIMEOUT_MS) {
        throw new Error('OpenPencil enrich_images timeout is invalid')
      }
    } else if (timeoutMs !== undefined && (
      !Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > MAX_ORDINARY_TIMEOUT_MS
    )) throw new Error(`OpenPencil MCP ${tool} timeout is invalid`)

    const signal = signalFor(session, options.signal)
    signal.throwIfAborted()
    let beforeVersion: number
    try {
      beforeVersion = await getOpenPencilMcpVersion({
        baseUrl: session.daemon.baseUrl,
        token: session.daemon.token,
        signal,
      })
    } catch (error) {
      if (isUncertain(error, signal)) await this.#retire(session, false)
      throw sanitizeInternalError(error, session)
    }
    if (options.expectedVersion !== undefined) {
      if (!Number.isSafeInteger(options.expectedVersion) || options.expectedVersion < 0) {
        throw new Error('OpenPencil guarded operation version is invalid')
      }
      if (beforeVersion !== options.expectedVersion) {
        throw new Error('OpenPencil draft changed before its guarded operation started')
      }
    }

    let result: OpenPencilMcpResult
    try {
      result = await callOpenPencilMcp({
        baseUrl: session.daemon.baseUrl,
        token: session.daemon.token,
        tool,
        arguments: args,
        signal,
        ...(timeoutMs === undefined ? {} : { timeoutMs }),
      })
    } catch (error) {
      if (mode !== 'read' && isUncertain(error, signal)) await this.#retire(session, false)
      throw sanitizeInternalError(error, session)
    }

    let afterVersion: number
    try {
      afterVersion = await getOpenPencilMcpVersion({
        baseUrl: session.daemon.baseUrl,
        token: session.daemon.token,
        signal,
      })
    } catch (error) {
      if (mode !== 'read') await this.#retire(session, false)
      throw sanitizeInternalError(error, session)
    }
    if (afterVersion < beforeVersion) {
      if (mode !== 'read') await this.#retire(session, false)
      throw new Error(`OpenPencil MCP ${tool} regressed the managed document version`)
    }
    if (mode === 'read' && afterVersion !== beforeVersion) {
      await this.#retire(session, false)
      throw new Error(`OpenPencil MCP read tool ${tool} unexpectedly changed the managed document`)
    }
    if (mode === 'must-change' && afterVersion <= beforeVersion) {
      throw new Error(`OpenPencil MCP ${tool} reported success but did not apply a document change`)
    }
    let finalizedDocumentSha256: string | undefined
    if (tool === 'finalize_design') {
      try {
        const authoritative = await readManagedEditorDaemon(session.daemon, signal)
        if (authoritative.version !== afterVersion) {
          await this.#retire(session, false)
          throw new Error('OpenPencil finalize_design version did not match its authoritative document')
        }
        finalizedDocumentSha256 = documentSha256(authoritative.documentJson)
      } catch (error) {
        if (!session.closed) await this.#retire(session, false)
        throw sanitizeInternalError(error, session)
      }
    }
    const changed = afterVersion > beforeVersion
    if (changed) clearPublicationProof(session)
    if (tool === 'finalize_design') {
      session.finalizedVersion = afterVersion
      session.finalizedDocumentSha256 = finalizedDocumentSha256
    }
    this.#touch(session)
    return {
      draftId: session.id,
      target: { ...session.target },
      tool,
      value: sanitizeInternalValue(result.value, session),
      text: sanitizeInternalValue(result.text, session) as string,
      version: afterVersion,
      changed,
      hasImage: result.images.length > 0,
    }
  }

  #session(draftId: string, ownerSessionId: string): DesignDraftSession {
    if (!DRAFT_ID_PATTERN.test(draftId)) throw new Error('OpenPencil draft was not found')
    validOwner(ownerSessionId)
    this.#pruneExpired()
    const session = this.#drafts.get(draftId)
    if (session === undefined || session.closed) throw new Error('OpenPencil draft has ended')
    if (session.ownerSessionId !== ownerSessionId) throw new Error('OpenPencil draft belongs to a different DSH session')
    return session
  }

  #enqueue<Result>(session: DesignDraftSession, task: () => Promise<Result>): Promise<Result> {
    const previous = session.activeOperation ?? Promise.resolve()
    const result = previous.then(async () => {
      if (session.closed) throw new Error('OpenPencil draft ended before its operation started')
      return task()
    })
    const settled = result.then(() => {}, () => {})
    session.activeOperation = settled
    void settled.then(() => {
      if (session.activeOperation === settled) delete session.activeOperation
    })
    return result
  }

  async #retire(session: DesignDraftSession, joinOperation: boolean): Promise<void> {
    // Publication is the commit edge. Once its callback starts, abort/dispose
    // joins that serialized transaction instead of racing a createIfAbsent
    // write. A failed publication leaves the session active, then retirement
    // proceeds; a successful one has already cleaned the session itself.
    const publishingOperation = joinOperation && session.publishing ? session.activeOperation : undefined
    if (publishingOperation !== undefined) {
      await publishingOperation.catch(() => {})
      if (session.closed) return
    }
    if (!session.closed) {
      this.#clearLiveAttach(session)
      session.closed = true
      this.#removeAuthority(session)
      session.retirement.abort(new Error('OpenPencil draft was retired'))
    }
    const operation = joinOperation ? session.activeOperation : undefined
    await this.#cleanup(session)
    await operation?.catch(() => {})
  }

  #removeAuthority(session: DesignDraftSession): void {
    const removed = this.#drafts.get(session.id) === session
    if (removed) this.#drafts.delete(session.id)
    if (this.#ownerDraft.get(session.ownerSessionId) === session.id) this.#ownerDraft.delete(session.ownerSessionId)
    if (session.timer !== undefined) {
      clearTimeout(session.timer)
      delete session.timer
    }
    if (removed) {
      try {
        this.#onDraftEnded?.(session.id, session.ownerSessionId)
      } catch {
        // Ending the daemon authority is primary; host-side UI revocation is
        // additive and must not turn a committed publication into a failure.
      }
    }
  }

  async #cleanup(session: DesignDraftSession): Promise<void> {
    this.#clearLiveAttach(session)
    await stopManagedEditorDaemon(session.daemon).catch(() => {})
    await rm(session.tempRoot, { recursive: true, force: true }).catch(() => {})
  }

  #touch(session: DesignDraftSession): void {
    session.touchedAt = this.#now()
    this.#scheduleExpiry(session)
  }

  #scheduleExpiry(session: DesignDraftSession): void {
    if (session.timer !== undefined) clearTimeout(session.timer)
    const now = this.#now()
    const deadline = Math.min(session.touchedAt + this.#idleMs, session.createdAt + this.#absoluteMs)
    const delay = Math.max(1, deadline - now)
    session.timer = setTimeout(() => { void this.#expireIfNeeded(session) }, delay)
    session.timer.unref()
  }

  async #expireIfNeeded(session: DesignDraftSession): Promise<void> {
    if (session.closed) return
    const now = this.#now()
    if (now - session.touchedAt >= this.#idleMs || now - session.createdAt >= this.#absoluteMs) {
      await this.#retire(session, true)
    } else {
      this.#scheduleExpiry(session)
    }
  }

  #pruneExpired(): void {
    const now = this.#now()
    for (const session of this.#drafts.values()) {
      if (now - session.touchedAt >= this.#idleMs || now - session.createdAt >= this.#absoluteMs) {
        void this.#retire(session, true)
      }
    }
  }

  #metadata(session: DesignDraftSession, version: number): DesignDraftMetadata {
    return {
      draftId: session.id,
      target: { ...session.target },
      createdAt: session.createdAt,
      idleExpiresAt: session.touchedAt + this.#idleMs,
      absoluteExpiresAt: session.createdAt + this.#absoluteMs,
      version,
    }
  }

  #snapshot(session: DesignDraftSession, version: number, documentJson: string): DesignDraftSnapshot {
    return {
      draftId: session.id,
      target: { ...session.target },
      version,
      documentJson,
    }
  }

  #clearLiveAttach(session: DesignDraftSession): void {
    if (session.liveAttach === undefined) return
    session.liveAttach.proxy.stop()
    delete session.liveAttach
  }

  async #serializeLifecycle<Result>(task: () => Promise<Result>): Promise<Result> {
    const run = this.#lifecycleQueue.then(task, task)
    this.#lifecycleQueue = run.then(() => undefined, () => undefined)
    return await run
  }
}
