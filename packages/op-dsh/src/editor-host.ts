/** Lazy managed OpenPencil editor sessions for the DSH details panel. */

import {
  createCipheriv,
  createDecipheriv,
  createHash,
  hkdfSync,
  randomBytes,
  randomUUID,
} from 'node:crypto'
import { constants as fsConstants } from 'node:fs'
import { lstat, mkdtemp, open, rename, rm, writeFile } from 'node:fs/promises'
import type { IncomingMessage, ServerResponse } from 'node:http'
import { TLSSocket } from 'node:tls'
import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { managedEditorProcessOptions } from './managed-editor-process.js'
import { tmpdir } from 'node:os'
import { basename, dirname, isAbsolute, join, resolve } from 'node:path'
import {
  callOpenPencilMcp,
  getOpenPencilMcpVersion,
  selectionSnapshotFromMcp,
  type OpenPencilMcpResult,
  type OpenPencilSelectionSnapshot,
} from './mcp-client.js'
import {
  EditorRecoveryStore,
  readManagedDaemonDocument,
  restoreManagedDaemonDocument,
  type EditorRecoveryReason,
} from './editor-recovery.js'
import {
  resolveEditorRuntime,
  type EditorRuntime,
} from './editor-runtime.js'
import { DesignDraftController } from './design-draft-controller.js'
import {
  readManagedEditorDaemon,
  startManagedEditorDaemon,
  stopManagedEditorDaemon,
  type ManagedEditorDaemon,
} from './managed-editor-daemon.js'

export const EDITOR_ROUTE_PREFIX = '/_dsh/dsh-openpencil/editor'

const CAPABILITY_TTL_MS = 2 * 60 * 60 * 1000
const CAPABILITY_REFRESH_TTL_MS = 24 * 60 * 60 * 1000
const SESSION_IDLE_MS = 4 * 60 * 60 * 1000
const START_TIMEOUT_MS = 20_000
const READY_TIMEOUT_MS = 15_000
const STOP_TIMEOUT_MS = 3_000
const OPERATION_RETIRE_TIMEOUT_MS = 25_000
const MAX_DOCUMENT_BYTES = 64 * 1024 * 1024
const MAX_HANDSHAKE_BYTES = 16 * 1024
const MAX_DIAGNOSTIC_BYTES = 64 * 1024
const TOKEN_PATTERN = /^[A-Za-z0-9_-]{24,128}$/
const EDITOR_CAPABILITY_AAD = Buffer.from('dsh-openpencil/editor-capability/v1')
const EDITOR_CAPABILITY_PREFIX = 'v1.'
const EDITOR_CAPABILITY_MAX_LENGTH = 16 * 1024
const EMPTY_DOCUMENT_JSON = '{\n  "version": "1.0.0",\n  "children": []\n}\n'

export interface EditorGrant {
  enabled: true
  launchUrl: string
  refreshUrl: string
}

interface CapabilityLifetime {
  v: 1
  issuedAt: number
  launchExpiresAt: number
  refreshExpiresAt: number
}

interface EditCapability extends CapabilityLifetime {
  scope: 'edit-source'
  sourcePath: string
  sourceSha256: string
}

interface LiveDraftCapability extends CapabilityLifetime {
  scope: 'live-draft'
  draftId: string
  ownerSessionId: string
}

type EditorCapability = EditCapability | LiveDraftCapability

function isSha256(value: unknown): value is string {
  return typeof value === 'string' && /^[a-f0-9]{64}$/.test(value)
}

function deriveEditorKey(masterKey: Buffer): Buffer {
  if (masterKey.length !== 32) throw new TypeError('editor master key must be 32 bytes')
  return Buffer.from(hkdfSync(
    'sha256',
    masterKey,
    Buffer.from('dsh-openpencil/editor-key/v1'),
    Buffer.from('aes-256-gcm'),
    32,
  ))
}

function capabilityFrom(value: unknown): EditorCapability | undefined {
  if (typeof value !== 'object' || value === null || Array.isArray(value)) return undefined
  const payload = value as Record<string, unknown>
  if (
    payload.v !== 1
    || typeof payload.issuedAt !== 'number'
    || !Number.isSafeInteger(payload.issuedAt)
    || typeof payload.launchExpiresAt !== 'number'
    || !Number.isSafeInteger(payload.launchExpiresAt)
    || typeof payload.refreshExpiresAt !== 'number'
    || !Number.isSafeInteger(payload.refreshExpiresAt)
    || payload.launchExpiresAt <= payload.issuedAt
    || payload.launchExpiresAt - payload.issuedAt > CAPABILITY_TTL_MS
    || payload.refreshExpiresAt < payload.launchExpiresAt
    || payload.refreshExpiresAt - payload.issuedAt > CAPABILITY_REFRESH_TTL_MS
  ) return undefined
  if (payload.scope === 'edit-source') {
    if (
      typeof payload.sourcePath !== 'string'
      || !isAbsolute(payload.sourcePath)
      || !payload.sourcePath.toLowerCase().endsWith('.op')
      || !isSha256(payload.sourceSha256)
    ) return undefined
    return payload as unknown as EditCapability
  }
  if (payload.scope === 'live-draft') {
    if (
      typeof payload.draftId !== 'string'
      || !/^[A-Za-z0-9_-]{32}$/.test(payload.draftId)
      || typeof payload.ownerSessionId !== 'string'
      || payload.ownerSessionId.length === 0
      || payload.ownerSessionId.length > 256
      || payload.ownerSessionId.includes('\0')
    ) return undefined
    return payload as unknown as LiveDraftCapability
  }
  return undefined
}

interface ManagedHandshake {
  port: number
  token: string
  version: string | number
}

interface EditorSession {
  id: string
  sourcePath: string
  ownerSessionId?: string
  baselineSha256: string
  child: ChildProcessWithoutNullStreams
  iframeUrl: string
  daemonToken: string
  refreshExpiresAt: number
  createdAt: number
  closed: boolean
  saving: boolean
  mutating: boolean
  activeOperation?: Promise<void>
  recoveryCaptureRequested: boolean
}

interface LiveDraftEditorSession {
  id: string
  draftId: string
  ownerSessionId: string
  attachId: string
  iframeUrl: string
  daemonToken: string
  createdAt: number
  closed: boolean
}

export type OpenPencilLiveTool = 'get_selection' | 'update_node' | 'batch_design'

export interface ActiveMcpCallOptions {
  /** Refuse to drive a different transcript card's live editor. */
  sourcePath?: string
  ownerSessionId?: string
  signal?: AbortSignal
}

export interface CreateDocumentBatchOptions {
  script: string
  canvasWidth?: number
  signal: AbortSignal
}

export interface CreateDocumentBatchResult {
  documentJson: string
  result: unknown
}

function json(res: ServerResponse, status: number, value: unknown): void {
  const body = JSON.stringify(value)
  res.statusCode = status
  res.setHeader('content-type', 'application/json; charset=utf-8')
  res.setHeader('cache-control', 'no-store')
  res.setHeader('x-content-type-options', 'nosniff')
  res.end(body)
}

async function waitForResponseFinish(res: ServerResponse): Promise<boolean> {
  if (res.writableFinished) return true
  await new Promise<void>(resolveFinished => {
    const finish = (): void => {
      res.off('finish', finish)
      res.off('close', finish)
      resolveFinished()
    }
    res.once('finish', finish)
    res.once('close', finish)
  })
  return res.writableFinished
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function sha256(bytes: Buffer | string): string {
  return createHash('sha256').update(bytes).digest('hex')
}

async function readSourceDocument(path: string): Promise<Buffer> {
  const before = await lstat(path)
  if (before.isSymbolicLink() || !before.isFile()) throw new HttpError(409, 'OpenPencil source is no longer a regular file')
  if (before.size <= 0 || before.size > MAX_DOCUMENT_BYTES) throw new HttpError(413, 'OpenPencil source size is invalid')
  const noFollow = typeof fsConstants.O_NOFOLLOW === 'number' ? fsConstants.O_NOFOLLOW : 0
  const handle = await open(path, fsConstants.O_RDONLY | noFollow)
  try {
    const opened = await handle.stat()
    if (!opened.isFile() || opened.size <= 0 || opened.size > MAX_DOCUMENT_BYTES) {
      throw new HttpError(413, 'OpenPencil source size is invalid')
    }
    const bytes = await handle.readFile()
    const after = await lstat(path)
    if (
      after.isSymbolicLink()
      || !after.isFile()
      || (opened.dev !== 0 && opened.ino !== 0 && (opened.dev !== after.dev || opened.ino !== after.ino))
    ) throw new HttpError(409, 'OpenPencil source changed while it was opened')
    return bytes
  } finally {
    await handle.close().catch(() => {})
  }
}

/** Locate the plugin-owned GUI-free managed host without probing a desktop app. */
export function findEditorHostBinary(): string | undefined {
  try {
    return resolveEditorRuntime().binary
  } catch {
    return undefined
  }
}

/** Host-owned authentication for Desktop requests whose proxy omits Origin. */
export type EditorRequestAuthentication = (req: IncomingMessage) => boolean

function requestOrigin(req: IncomingMessage, authenticate?: EditorRequestAuthentication): string {
  const raw = req.headers.origin
  if (raw === undefined) {
    const site = req.headers['sec-fetch-site']
    if (site !== undefined && site !== 'same-origin' && site !== 'none') {
      throw new HttpError(403, 'editor request must be same-origin with DSH')
    }
    let authenticated = false
    try {
      authenticated = authenticate !== undefined && authenticate(req) === true
    } catch {
      // An unavailable or failing Host authentication service grants no access.
    }
    if (!authenticated) throw new HttpError(403, 'editor launch requires a browser Origin header or authenticated DSH request')
    const host = req.headers.host
    const protocol = req.socket instanceof TLSSocket && req.socket.encrypted ? 'https:' : 'http:'
    const defaultPort = protocol === 'https:' ? 443 : 80
    let url: URL
    try {
      url = new URL(`${protocol}//${host ?? ''}`)
    } catch {
      throw new HttpError(403, 'editor request Host is invalid')
    }
    const canonicalHost = url.host === host || (url.port === '' && `${url.hostname}:${defaultPort}` === host)
    if (!canonicalHost || url.username !== '' || url.password !== '' || url.pathname !== '/' || url.search !== '' || url.hash !== '') {
      throw new HttpError(403, 'editor request Host is invalid')
    }
    if (url.hostname !== '127.0.0.1' && url.hostname !== 'localhost' && url.hostname !== '[::1]') {
      throw new HttpError(403, 'editable OpenPencil sessions are restricted to loopback DSH')
    }
    if (req.socket.localPort === undefined || Number(url.port || defaultPort) !== req.socket.localPort) {
      throw new HttpError(403, 'editor request Host must match the DSH listening port')
    }
    return url.origin
  }
  if (typeof raw !== 'string') throw new HttpError(403, 'editor launch requires a browser Origin header')
  let url: URL
  try {
    url = new URL(raw)
  } catch {
    throw new HttpError(403, 'editor launch origin is invalid')
  }
  const host = req.headers.host
  if (host === undefined || url.host !== host || (url.protocol !== 'http:' && url.protocol !== 'https:')) {
    throw new HttpError(403, 'editor launch must be same-origin with DSH')
  }
  const loopback = url.hostname === '127.0.0.1' || url.hostname === 'localhost' || url.hostname === '[::1]'
  if (!loopback) throw new HttpError(403, 'editable OpenPencil sessions are restricted to loopback DSH')
  return url.origin
}

function isIpv4LoopbackAddress(address: string): boolean {
  const parts = address.split('.')
  return parts.length === 4
    && parts[0] === '127'
    && parts.every(part => /^\d{1,3}$/.test(part) && Number(part) <= 255)
}

/**
 * Trust the transport peer, never forwarded or caller-controlled host data.
 * Node may expose an IPv4 peer either directly or as an IPv4-mapped IPv6
 * address, including the compact hexadecimal form used by some platforms.
 */
export function isLoopbackRemoteAddress(address: string | undefined): boolean {
  if (address === undefined) return false
  const normalized = address.toLowerCase().split('%', 1)[0]!
  if (normalized === '::1' || isIpv4LoopbackAddress(normalized)) return true
  if (!normalized.startsWith('::ffff:')) return false
  const mapped = normalized.slice('::ffff:'.length)
  if (isIpv4LoopbackAddress(mapped)) return true
  const hexadecimal = /^([a-f0-9]{1,4}):([a-f0-9]{1,4})$/.exec(mapped)
  if (hexadecimal === null) return false
  return (Number.parseInt(hexadecimal[1]!, 16) >>> 8) === 127
}

function requireLoopbackPeer(req: IncomingMessage): void {
  if (!isLoopbackRemoteAddress(req.socket?.remoteAddress)) {
    throw new HttpError(403, 'editable OpenPencil sessions require a loopback network peer')
  }
}

class HttpError extends Error {
  constructor(readonly status: number, message: string) {
    super(message)
  }
}

async function readRequestBody(req: IncomingMessage): Promise<Buffer> {
  const chunks: Buffer[] = []
  let size = 0
  for await (const chunk of req) {
    const bytes = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)
    size += bytes.length
    if (size > MAX_DOCUMENT_BYTES + 1024 * 1024) throw new HttpError(413, 'editor request is too large')
    chunks.push(bytes)
  }
  return Buffer.concat(chunks)
}

function parseHandshake(line: string): ManagedHandshake {
  let value: unknown
  try {
    value = JSON.parse(line)
  } catch {
    throw new Error('OpenPencil editor host returned an invalid handshake')
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('OpenPencil editor host returned an invalid handshake')
  }
  const record = value as Record<string, unknown>
  if (
    record.ok !== true
    || typeof record.port !== 'number' || !Number.isSafeInteger(record.port) || record.port < 1 || record.port > 65535
    || typeof record.token !== 'string' || record.token.length < 16
    || !(
      (typeof record.version === 'number' && Number.isSafeInteger(record.version) && record.version >= 0)
      || (typeof record.version === 'string' && record.version.length > 0 && record.version.length <= 64)
    )
  ) {
    throw new Error('OpenPencil editor host returned an incomplete handshake')
  }
  return { port: record.port, token: record.token, version: record.version }
}

async function waitForHandshake(
  child: ChildProcessWithoutNullStreams,
  diagnostics: () => string,
  signal?: AbortSignal,
): Promise<ManagedHandshake> {
  signal?.throwIfAborted()
  return new Promise((resolveHandshake, rejectHandshake) => {
    let settled = false
    let stdout = ''
    const finish = (error?: Error, handshake?: ManagedHandshake): void => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      child.stdout.off('data', onData)
      child.off('error', onError)
      child.off('close', onClose)
      signal?.removeEventListener('abort', onAbort)
      if (error !== undefined) rejectHandshake(error)
      else resolveHandshake(handshake!)
    }
    const onData = (chunk: Buffer): void => {
      stdout += chunk.toString('utf8')
      if (stdout.length > MAX_HANDSHAKE_BYTES) {
        finish(new Error('OpenPencil editor host handshake exceeded its size limit'))
        return
      }
      const newline = stdout.indexOf('\n')
      if (newline < 0) return
      try {
        finish(undefined, parseHandshake(stdout.slice(0, newline).trim()))
      } catch (error) {
        finish(error instanceof Error ? error : new Error(String(error)))
      }
    }
    const onError = (error: Error): void => { finish(error) }
    const onAbort = (): void => {
      const reason = signal?.reason
      finish(reason instanceof Error ? reason : new Error('OpenPencil editor startup was cancelled'))
    }
    const onClose = (code: number | null): void => {
      finish(new Error(`OpenPencil editor host exited before startup (${String(code)})${diagnostics() === '' ? '' : `: ${diagnostics()}`}`))
    }
    const timer = setTimeout(() => {
      finish(new Error(`OpenPencil editor host did not start within ${START_TIMEOUT_MS} ms${diagnostics() === '' ? '' : `: ${diagnostics()}`}`))
    }, START_TIMEOUT_MS)
    child.stdout.on('data', onData)
    child.once('error', onError)
    child.once('close', onClose)
    signal?.addEventListener('abort', onAbort, { once: true })
    if (signal?.aborted) onAbort()
  })
}

async function waitForEditorReady(baseUrl: string, signal?: AbortSignal): Promise<void> {
  const deadline = Date.now() + READY_TIMEOUT_MS
  let last = ''
  while (Date.now() < deadline) {
    signal?.throwIfAborted()
    try {
      const requestSignal = signal === undefined
        ? AbortSignal.timeout(2_000)
        : AbortSignal.any([signal, AbortSignal.timeout(2_000)])
      const [root, glue] = await Promise.all([
        fetch(`${baseUrl}/`, { signal: requestSignal }),
        fetch(`${baseUrl}/pkg/op_host_web.js`, { signal: requestSignal }),
      ])
      await Promise.all([root.arrayBuffer().catch(() => undefined), glue.arrayBuffer().catch(() => undefined)])
      if (root.status === 200 && glue.status === 200) return
      last = `root=${root.status}, bundle=${glue.status}`
    } catch (error) {
      signal?.throwIfAborted()
      last = errorMessage(error)
    }
    await new Promise(resolveDelay => setTimeout(resolveDelay, 150))
  }
  throw new Error(`OpenPencil editor web bundle was not ready${last === '' ? '' : `: ${last}`}`)
}

const stoppingChildren = new WeakMap<ChildProcessWithoutNullStreams, Promise<void>>()

function waitForChildClose(child: ChildProcessWithoutNullStreams, timeoutMs: number): Promise<boolean> {
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve(true)
  return new Promise<boolean>(resolveClosed => {
    let settled = false
    const finish = (value: boolean): void => {
      if (settled) return
      settled = true
      clearTimeout(timer)
      child.off('close', onClose)
      resolveClosed(value)
    }
    const onClose = (): void => { finish(true) }
    const timer = setTimeout(() => { finish(false) }, timeoutMs)
    child.once('close', onClose)
  })
}

function stopChild(child: ChildProcessWithoutNullStreams): Promise<void> {
  const current = stoppingChildren.get(child)
  if (current !== undefined) return current
  if (child.exitCode !== null || child.signalCode !== null) return Promise.resolve()
  const stopping = (async (): Promise<void> => {
    if (!child.stdin.writableEnded) child.stdin.end()
    const closed = await waitForChildClose(child, STOP_TIMEOUT_MS)
    if (!closed && child.exitCode === null && child.signalCode === null) {
      child.kill('SIGKILL')
      await waitForChildClose(child, STOP_TIMEOUT_MS)
    }
  })()
  stoppingChildren.set(child, stopping)
  void stopping.finally(() => {
    if (stoppingChildren.get(child) === stopping) stoppingChildren.delete(child)
  })
  return stopping
}

async function atomicWriteDocument(path: string, text: string): Promise<string> {
  const bytes = Buffer.from(text)
  if (bytes.length === 0 || bytes.length > MAX_DOCUMENT_BYTES) throw new HttpError(413, 'OpenPencil document size is invalid')
  let parsed: unknown
  try {
    parsed = JSON.parse(text)
  } catch {
    throw new HttpError(400, 'OpenPencil save payload is not valid JSON')
  }
  if (typeof parsed !== 'object' || parsed === null || Array.isArray(parsed)) {
    throw new HttpError(400, 'OpenPencil save payload must be a document object')
  }
  const info = await lstat(path)
  if (info.isSymbolicLink() || !info.isFile()) throw new HttpError(409, 'OpenPencil source is no longer a regular file')
  const temp = join(dirname(path), `.${basename(path)}.dsh-save-${randomUUID()}.tmp`)
  let handle
  try {
    handle = await open(temp, 'wx', info.mode & 0o777)
    await handle.writeFile(bytes)
    await handle.sync()
    await handle.close()
    handle = undefined
    await rename(temp, path)
  } finally {
    await handle?.close().catch(() => {})
    await rm(temp, { force: true }).catch(() => {})
  }
  return sha256(bytes)
}

/** Owns opaque launch capabilities and all live managed editor children. */
export class EditorHostController {
  readonly runtime: EditorRuntime | undefined
  readonly runtimeError: Error | undefined
  readonly binary: string | undefined
  readonly designDrafts: DesignDraftController
  #routeRefs = 0
  #routeGeneration = 0
  readonly #editorKey: Buffer
  readonly #recoveryStore: EditorRecoveryStore
  #sessions = new Map<string, EditorSession>()
  #liveDraftSessions = new Map<string, LiveDraftEditorSession>()
  #pendingChildren = new Set<ChildProcessWithoutNullStreams>()
  #launchQueue: Promise<void> = Promise.resolve()
  #disposePromise: Promise<void> | undefined
  #requestAuthentication: EditorRequestAuthentication | undefined

  constructor(masterKey: Buffer, runtime?: EditorRuntime) {
    this.#editorKey = deriveEditorKey(masterKey)
    this.#recoveryStore = new EditorRecoveryStore(masterKey)
    let resolved = runtime
    let runtimeError: Error | undefined
    if (resolved === undefined) {
      try {
        resolved = resolveEditorRuntime()
      } catch (error) {
        runtimeError = error instanceof Error ? error : new Error(String(error))
      }
    }
    this.runtime = resolved
    this.runtimeError = runtimeError
    this.binary = resolved?.binary
    const detail = runtimeError?.message.trim()
    this.designDrafts = new DesignDraftController(resolved, {
      runtimeUnavailableMessage: detail === undefined || detail.length === 0
        ? 'OpenPencil editor runtime is unavailable'
        : `OpenPencil editor runtime is unavailable: ${detail}`,
      onDraftEnded: (draftId, ownerSessionId) => {
        this.#revokeLiveDraftSessions(draftId, ownerSessionId)
      },
    })
  }

  get available(): boolean { return this.runtime !== undefined }
  get routeAvailable(): boolean { return this.#routeRefs > 0 }

  /** Bind optional Host authentication until its providing service is disposed. */
  attachRequestAuthentication(authenticate: EditorRequestAuthentication): () => void {
    const attached: EditorRequestAuthentication = req => authenticate(req)
    this.#requestAuthentication = attached
    return () => {
      if (this.#requestAuthentication === attached) this.#requestAuthentication = undefined
    }
  }

  attachRoute(): () => void {
    this.#routeRefs += 1
    let attached = true
    return () => {
      if (!attached) return
      attached = false
      this.#routeRefs = Math.max(0, this.#routeRefs - 1)
    }
  }

  /** Mint an opaque, runtime-only launch URL; no source path enters metadata. */
  grantFor(sourcePath: string | undefined, sourceSha256: string | undefined): EditorGrant | undefined {
    if (!this.available || !this.routeAvailable || sourcePath === undefined || sourceSha256 === undefined) return undefined
    if (!isAbsolute(sourcePath) || !sourcePath.toLowerCase().endsWith('.op') || !isSha256(sourceSha256)) return undefined
    const now = Date.now()
    const token = this.#sealCapability({
      v: 1,
      scope: 'edit-source',
      sourcePath: resolve(sourcePath),
      sourceSha256,
      issuedAt: now,
      launchExpiresAt: now + CAPABILITY_TTL_MS,
      refreshExpiresAt: now + CAPABILITY_REFRESH_TTL_MS,
    })
    return {
      enabled: true,
      launchUrl: `${EDITOR_ROUTE_PREFIX}/${token}/launch`,
      refreshUrl: `${EDITOR_ROUTE_PREFIX}/${token}/refresh`,
    }
  }

  /** Mint an owner-bound capability for the exact unpublished draft daemon. */
  grantForDraft(draftId: string, ownerSessionId: string): EditorGrant | undefined {
    if (!this.available || !this.routeAvailable) return undefined
    if (!/^[A-Za-z0-9_-]{32}$/.test(draftId)) return undefined
    if (ownerSessionId.length === 0 || ownerSessionId.length > 256 || ownerSessionId.includes('\0')) return undefined
    const now = Date.now()
    const token = this.#sealCapability({
      v: 1,
      scope: 'live-draft',
      draftId,
      ownerSessionId,
      issuedAt: now,
      launchExpiresAt: now + CAPABILITY_TTL_MS,
      refreshExpiresAt: now + CAPABILITY_REFRESH_TTL_MS,
    })
    return {
      enabled: true,
      launchUrl: `${EDITOR_ROUTE_PREFIX}/${token}/launch`,
      refreshUrl: `${EDITOR_ROUTE_PREFIX}/${token}/refresh`,
    }
  }

  async handle(req: IncomingMessage, res: ServerResponse): Promise<void> {
    try {
      this.#prune()
      // Host and Origin are caller-controlled HTTP data. Apply the transport
      // fence before resolving any bearer capability or session identifier so
      // a LAN client cannot spoof localhost to launch, refresh, read, mutate,
      // recover, or close a managed editor session.
      requireLoopbackPeer(req)
      const url = new URL(req.url ?? '/', 'http://dsh.invalid')
      const launch = new RegExp(`^${EDITOR_ROUTE_PREFIX}/([A-Za-z0-9_.-]+)/launch$`).exec(url.pathname)
      const refresh = new RegExp(`^${EDITOR_ROUTE_PREFIX}/([A-Za-z0-9_.-]+)/refresh$`).exec(url.pathname)
      const legacyRefresh = url.pathname === `${EDITOR_ROUTE_PREFIX}/refresh`
      const save = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)/save$`).exec(url.pathname)
      const selection = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)/selection$`).exec(url.pathname)
      const recovery = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)/recovery$`).exec(url.pathname)
      const recoveryItem = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)/recovery/([A-Za-z0-9_-]+)$`).exec(url.pathname)
      const ready = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)/ready$`).exec(url.pathname)
      const close = new RegExp(`^${EDITOR_ROUTE_PREFIX}/session/([A-Za-z0-9_-]+)$`).exec(url.pathname)
      if (launch !== null && req.method === 'POST') {
        const body = await readRequestBody(req)
        let ownerSessionId: string | undefined
        if (body.length > 0) {
          try {
            const value: unknown = JSON.parse(body.toString('utf8'))
            if (typeof value === 'object' && value !== null && !Array.isArray(value)) {
              const candidate = (value as Record<string, unknown>).sessionId
              if (typeof candidate === 'string' && candidate.length > 0 && candidate.length <= 256) ownerSessionId = candidate
            }
          } catch {
            throw new HttpError(400, 'editor launch request is not valid JSON')
          }
        }
        const routeGeneration = this.#routeGeneration
        await this.#serializeLaunch(() => this.#launch(
          launch[1]!, requestOrigin(req, this.#requestAuthentication), res, routeGeneration, ownerSessionId,
        ))
        return
      }
      if (refresh !== null && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        await this.#refresh(refresh[1]!, res)
        return
      }
      if (legacyRefresh && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        throw new HttpError(410, 'This editor card predates restart-safe editing; open the .op file in a new editable OpenPencil card')
      }
      if (save !== null && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        await this.#save(save[1]!, req, res)
        return
      }
      if (ready !== null && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        const body = await readRequestBody(req)
        let sessionId: unknown
        try {
          const value: unknown = JSON.parse(body.toString('utf8'))
          sessionId = typeof value === 'object' && value !== null && !Array.isArray(value)
            ? (value as Record<string, unknown>).sessionId
            : undefined
        } catch {
          throw new HttpError(400, 'live editor ready request is not valid JSON')
        }
        if (sessionId !== ready[1]) throw new HttpError(400, 'live editor ready request is incomplete')
        const attached = this.#liveDraftReady(ready[1]!)
        json(res, 200, { ok: true, attached })
        return
      }
      if (selection !== null && req.method === 'GET') {
        const snapshot = await this.#selectionForSession(selection[1]!)
        json(res, 200, { ok: true, selection: snapshot })
        return
      }
      if (recovery !== null && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        const snapshot = await this.#captureRecoveryForSession(recovery[1]!, 'client-dispose')
        json(res, 200, { ok: true, recovery: snapshot ?? null })
        return
      }
      if (recoveryItem !== null && req.method === 'POST') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        const restored = await this.#restoreRecoveryForSession(recoveryItem[1]!, recoveryItem[2]!)
        json(res, 200, { ok: true, ...restored })
        return
      }
      if (recoveryItem !== null && req.method === 'DELETE') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        const discarded = await this.#discardRecoveryForSession(recoveryItem[1]!, recoveryItem[2]!)
        json(res, 200, { ok: true, discarded })
        return
      }
      if (close !== null && req.method === 'DELETE') {
        requestOrigin(req, this.#requestAuthentication)
        await readRequestBody(req).catch(() => Buffer.alloc(0))
        await this.#close(close[1]!)
        json(res, 200, { ok: true })
        return
      }
      json(res, 404, { ok: false, error: 'editor capability not found' })
    } catch (error) {
      const status = error instanceof HttpError ? error.status : 500
      json(res, status, { ok: false, error: errorMessage(error) })
    }
  }

  dispose(): Promise<void> {
    if (this.#disposePromise !== undefined) return this.#disposePromise
    this.#disposePromise = (async () => {
      // Invalidate work accepted by the route before it was detached. Pending
      // launches check this generation around every asynchronous startup phase,
      // and their child is stopped here so a handshake wait cannot leak.
      this.#routeGeneration += 1
      const sessions = [...this.#sessions.values()]
      this.#sessions.clear()
      const liveDraftSessions = [...this.#liveDraftSessions.values()]
      this.#liveDraftSessions.clear()
      const pending = [...this.#pendingChildren]
      await Promise.all([
        this.designDrafts.dispose(),
        ...sessions.map(session => this.#captureThenDispose(session, 'plugin-dispose')),
        ...liveDraftSessions.map(session => this.#detachLiveDraftSession(session)),
        ...pending.map(child => stopChild(child)),
      ])
      // A launch can already own a retired session after removing it from the
      // public session map. Wait for that serialized retirement too, so plugin
      // disposal does not return while its previous editor is still alive.
      await this.#launchQueue
    })()
    return this.#disposePromise
  }

  /**
   * Build one brand-new document without requiring a browser-owned editor.
   * The managed daemon is transient and never enters the visible-session map,
   * so this operation neither depends on nor retires an existing workbench.
   * Callers publish the returned authoritative JSON through DSH's filesystem
   * capability only after the whole batch succeeds.
   */
  async createDocumentBatch(options: CreateDocumentBatchOptions): Promise<CreateDocumentBatchResult> {
    const runtime = this.runtime
    if (runtime === undefined) throw new Error(this.#runtimeUnavailableMessage())
    options.signal.throwIfAborted()
    if (this.#disposePromise !== undefined) throw new Error('OpenPencil editor host is shutting down')

    return this.#serializeLaunch(async () => {
      options.signal.throwIfAborted()
      if (this.#disposePromise !== undefined) throw new Error('OpenPencil editor host is shutting down')
      return this.#createDocumentBatch(runtime, options)
    })
  }

  async #createDocumentBatch(
    runtime: EditorRuntime,
    options: CreateDocumentBatchOptions,
  ): Promise<CreateDocumentBatchResult> {
    const tempRoot = await mkdtemp(join(tmpdir(), 'dsh-openpencil-new-'))
    const sourcePath = join(tempRoot, 'starter.op')
    let daemon: ManagedEditorDaemon | undefined
    let spawnedChild: ChildProcessWithoutNullStreams | undefined
    try {
      await writeFile(sourcePath, EMPTY_DOCUMENT_JSON, { flag: 'wx', mode: 0o600 })
      options.signal.throwIfAborted()
      if (this.#disposePromise !== undefined) throw new Error('OpenPencil editor host is shutting down')

      daemon = await startManagedEditorDaemon(runtime, {
        sourcePath,
        allowOrigin: 'http://127.0.0.1',
        signal: options.signal,
        onSpawn: child => {
          spawnedChild = child
          this.#pendingChildren.add(child)
        },
      })
      const onAbort = (): void => { if (daemon !== undefined) void stopManagedEditorDaemon(daemon) }
      options.signal.addEventListener('abort', onAbort, { once: true })
      try {
        options.signal.throwIfAborted()
        const baseUrl = daemon.baseUrl
        const beforeVersion = await getOpenPencilMcpVersion({
          baseUrl,
          token: daemon.token,
          signal: options.signal,
        })
        const build = await callOpenPencilMcp({
          baseUrl,
          token: daemon.token,
          tool: 'batch_design',
          arguments: {
            script: options.script,
            postProcess: true,
            ...(options.canvasWidth === undefined ? {} : { canvasWidth: options.canvasWidth }),
          },
          signal: options.signal,
        })
        const buildVersion = await getOpenPencilMcpVersion({
          baseUrl,
          token: daemon.token,
          signal: options.signal,
        })
        if (buildVersion <= beforeVersion) {
          throw new Error('OpenPencil MCP batch_design reported success but did not create a document change')
        }
        options.signal.throwIfAborted()
        const finalize = await callOpenPencilMcp({
          baseUrl,
          token: daemon.token,
          tool: 'finalize_design',
          arguments: {},
          signal: options.signal,
        })
        const finalVersion = await getOpenPencilMcpVersion({
          baseUrl,
          token: daemon.token,
          signal: options.signal,
        })
        // Finalization is intentionally idempotent. A clean generated tree can
        // therefore keep the build version; only version regression is invalid.
        if (finalVersion < buildVersion) {
          throw new Error('OpenPencil MCP document version regressed during design finalization')
        }
        options.signal.throwIfAborted()
        const authoritative = await readManagedEditorDaemon(daemon, options.signal)
        options.signal.throwIfAborted()
        if (authoritative.version < finalVersion) {
          throw new Error('OpenPencil managed document snapshot is older than the finalized design')
        }
        return {
          documentJson: authoritative.documentJson,
          result: {
            pipeline: {
              mode: 'script',
              postProcessed: true,
              finalized: true,
            },
            build: build.value,
            finalize: finalize.value,
          },
        }
      } finally {
        options.signal.removeEventListener('abort', onAbort)
      }
    } finally {
      if (spawnedChild !== undefined) this.#pendingChildren.delete(spawnedChild)
      if (daemon !== undefined) await stopManagedEditorDaemon(daemon)
      await rm(tempRoot, { recursive: true, force: true }).catch(() => {})
    }
  }

  /** Current live editor selection, suitable for Agent context and UI chips. */
  async getActiveSelection(options: ActiveMcpCallOptions = {}): Promise<OpenPencilSelectionSnapshot> {
    const session = this.#activeSession(options.sourcePath, options.ownerSessionId)
    return this.#selectionFor(session, options.signal)
  }

  /**
   * Drive one allowlisted first-party MCP tool on the currently visible
   * editor. The managed daemon token never crosses this controller boundary.
   */
  async callActiveMcp(
    tool: OpenPencilLiveTool,
    args: Record<string, unknown>,
    options: ActiveMcpCallOptions = {},
  ): Promise<OpenPencilMcpResult> {
    const session = this.#activeSession(options.sourcePath, options.ownerSessionId)
    const mutating = tool === 'update_node' || tool === 'batch_design'
    if ('filePath' in args || 'sourceFilePath' in args || 'source_file_path' in args) {
      throw new Error('OpenPencil live tools cannot target a filesystem path through MCP arguments')
    }
    const execute = async (): Promise<OpenPencilMcpResult> => {
      let beforeVersion: number | undefined
      if (mutating) {
        const current = await readSourceDocument(session.sourcePath)
        if (sha256(current) !== session.baselineSha256) {
          throw new Error('OpenPencil source changed outside the active editor; rerender before applying Agent changes')
        }
        beforeVersion = await getOpenPencilMcpVersion({
          baseUrl: new URL(session.iframeUrl).origin,
          token: session.daemonToken,
          signal: options.signal,
        })
      }
      const result = await callOpenPencilMcp({
        baseUrl: new URL(session.iframeUrl).origin,
        token: session.daemonToken,
        tool,
        arguments: args,
        signal: options.signal,
      })
      if (beforeVersion !== undefined) {
        const afterVersion = await getOpenPencilMcpVersion({
          baseUrl: new URL(session.iframeUrl).origin,
          token: session.daemonToken,
          signal: options.signal,
        })
        if (afterVersion <= beforeVersion) {
          throw new Error(`OpenPencil MCP ${tool} reported success but did not apply a document change`)
        }
      }
      session.createdAt = Date.now()
      return result
    }
    if (!mutating) return execute()
    return this.#enqueueSessionOperation(session, async () => {
      session.mutating = true
      try {
        return await execute()
      } finally {
        session.mutating = false
      }
    })
  }

  async #serializeLaunch<T>(task: () => Promise<T>): Promise<T> {
    const run = this.#launchQueue.then(task, task)
    // A failed launch must not poison the lifecycle queue for later requests.
    this.#launchQueue = run.then(() => undefined, () => undefined)
    return await run
  }

  #runtimeUnavailableMessage(): string {
    const detail = this.runtimeError?.message.trim()
    return detail === undefined || detail.length === 0
      ? 'OpenPencil editor runtime is unavailable'
      : `OpenPencil editor runtime is unavailable: ${detail}`
  }

  #assertRuntimeVersion(handshake: ManagedHandshake, runtime: EditorRuntime): void {
    const actual = String(handshake.version)
    if (actual !== runtime.openPencilVersion) {
      throw new Error(
        `OpenPencil editor runtime version mismatch: expected ${runtime.openPencilVersion}, received ${actual}`,
      )
    }
  }

  #assertLaunchLifecycle(routeGeneration: number): void {
    if (routeGeneration !== this.#routeGeneration || !this.routeAvailable) {
      throw new HttpError(410, 'editor route was unloaded during launch')
    }
  }

  async #launch(
    token: string,
    origin: string,
    res: ServerResponse,
    routeGeneration: number,
    ownerSessionId?: string,
  ): Promise<void> {
    let disconnected = res.destroyed
    let launchChild: ChildProcessWithoutNullStreams | undefined
    const onResponseClose = (): void => {
      if (res.writableFinished) return
      disconnected = true
      if (launchChild !== undefined) void stopChild(launchChild)
    }
    res.once('close', onResponseClose)
    const assertConnected = (): void => {
      this.#assertLaunchLifecycle(routeGeneration)
      if (disconnected || res.destroyed) throw new HttpError(499, 'editor launch client disconnected')
    }
    try {
      assertConnected()
      const capability = this.#openCapability(token)
      if (Date.now() > capability.launchExpiresAt) throw new HttpError(410, 'editor capability expired; open the .op file in a new editable OpenPencil card')
      if (capability.scope === 'live-draft') {
        if (ownerSessionId === undefined || ownerSessionId !== capability.ownerSessionId) {
          throw new HttpError(403, 'live OpenPencil draft belongs to a different DSH session')
        }
        await this.#launchLiveDraft(capability, res, routeGeneration, assertConnected)
        return
      }
      const current = await readSourceDocument(capability.sourcePath)
      assertConnected()
      if (sha256(current) !== capability.sourceSha256) {
        throw new HttpError(409, 'source changed since this card was created; open the .op file in a new editable OpenPencil card')
      }
      const runtime = this.runtime
      if (runtime === undefined) throw new HttpError(503, this.#runtimeUnavailableMessage())

      // The details panel hosts one editor. Retire an earlier daemon before a
      // successor starts so stale transcript cards cannot retain authority.
      const old = [...this.#sessions.values()]
      this.#sessions.clear()
      const oldLive = [...this.#liveDraftSessions.values()]
      this.#liveDraftSessions.clear()
      await Promise.all([
        ...old.map(session => this.#captureThenDispose(session, 'plugin-dispose')),
        ...oldLive.map(session => this.#detachLiveDraftSession(session)),
      ])
      assertConnected()

      const launch = await managedEditorProcessOptions(runtime, capability.sourcePath, origin)
      const child = spawn(runtime.binary, launch.args, { stdio: ['pipe', 'pipe', 'pipe'], env: launch.env })
      launchChild = child
      this.#pendingChildren.add(child)
      let diagnostics = ''
      child.stderr.on('data', (chunk: Buffer) => {
        if (diagnostics.length < MAX_DIAGNOSTIC_BYTES) {
          diagnostics += chunk.toString('utf8').slice(0, MAX_DIAGNOSTIC_BYTES - diagnostics.length)
        }
      })
      try {
        const handshake = await waitForHandshake(child, () => diagnostics.trim())
        this.#assertRuntimeVersion(handshake, runtime)
        assertConnected()
        const baseUrl = `http://127.0.0.1:${handshake.port}`
        await waitForEditorReady(baseUrl)
        assertConnected()
        const id = randomBytes(24).toString('base64url')
        const session: EditorSession = {
          id,
          sourcePath: capability.sourcePath,
          ...(ownerSessionId === undefined ? {} : { ownerSessionId }),
          baselineSha256: capability.sourceSha256,
          child,
          iframeUrl: `${baseUrl}/?embed=vscode`,
          daemonToken: handshake.token,
          refreshExpiresAt: capability.refreshExpiresAt,
          createdAt: Date.now(),
          closed: false,
          saving: false,
          mutating: false,
          recoveryCaptureRequested: false,
        }
        this.#sessions.set(id, session)
        this.#pendingChildren.delete(child)
        child.once('close', () => {
          if (this.#sessions.get(id) === session) this.#sessions.delete(id)
          session.closed = true
        })
        let recovery
        try {
          recovery = await this.#recoveryStore.find(
            session.sourcePath,
            session.baselineSha256,
            current.toString('utf8'),
          )
        } catch {
          // Recovery is additive. A damaged/unwritable cache must never block
          // the primary managed editor from opening.
        }
        json(res, 200, {
          sessionId: id,
          iframeUrl: session.iframeUrl,
          token: session.daemonToken,
          saveUrl: `${EDITOR_ROUTE_PREFIX}/session/${id}/save`,
          selectionUrl: `${EDITOR_ROUTE_PREFIX}/session/${id}/selection`,
          closeUrl: `${EDITOR_ROUTE_PREFIX}/session/${id}`,
          recoveryUrl: `${EDITOR_ROUTE_PREFIX}/session/${id}/recovery`,
          ...(recovery === undefined ? {} : { recovery }),
          docJson: current.toString('utf8'),
        })
        if (!await waitForResponseFinish(res)) {
          throw new HttpError(499, 'editor launch client disconnected')
        }
        launchChild = undefined
      } catch (error) {
        this.#pendingChildren.delete(child)
        await stopChild(child)
        throw error
      }
    } finally {
      res.off('close', onResponseClose)
    }
  }

  async #launchLiveDraft(
    capability: LiveDraftCapability,
    res: ServerResponse,
    routeGeneration: number,
    assertConnected: () => void,
  ): Promise<void> {
    this.#assertLaunchLifecycle(routeGeneration)

    // The details pane owns one canvas. Detach any prior draft canvas and
    // retire persisted editors before binding the exact Agent draft daemon.
    const old = [...this.#sessions.values()]
    this.#sessions.clear()
    const oldLive = [...this.#liveDraftSessions.values()]
    this.#liveDraftSessions.clear()
    await Promise.all([
      ...old.map(session => this.#captureThenDispose(session, 'plugin-dispose')),
      ...oldLive.map(session => this.#detachLiveDraftSession(session)),
    ])
    assertConnected()

    let session: LiveDraftEditorSession | undefined
    try {
      const launch = await this.designDrafts.prepareLiveLaunch(
        capability.draftId,
        capability.ownerSessionId,
      )
      session = {
        id: randomBytes(24).toString('base64url'),
        draftId: capability.draftId,
        ownerSessionId: capability.ownerSessionId,
        attachId: launch.attachId,
        iframeUrl: `${launch.baseUrl}/?embed=vscode`,
        daemonToken: launch.token,
        createdAt: Date.now(),
        closed: false,
      }
      assertConnected()
      this.#liveDraftSessions.set(session.id, session)
      json(res, 200, {
        sessionId: session.id,
        iframeUrl: session.iframeUrl,
        token: session.daemonToken,
        // Kept for the shared launch response contract. The live client hides
        // save and this route rejects writes to an unpublished pipeline draft.
        saveUrl: `${EDITOR_ROUTE_PREFIX}/session/${session.id}/save`,
        closeUrl: `${EDITOR_ROUTE_PREFIX}/session/${session.id}`,
        readyUrl: `${EDITOR_ROUTE_PREFIX}/session/${session.id}/ready`,
        docJson: launch.documentJson,
        liveDraft: true,
      })
      if (!await waitForResponseFinish(res)) throw new HttpError(499, 'editor launch client disconnected')
    } catch (error) {
      if (session !== undefined) {
        if (this.#liveDraftSessions.get(session.id) === session) this.#liveDraftSessions.delete(session.id)
        await this.#detachLiveDraftSession(session)
      }
      throw error
    }
  }

  async #refresh(token: string, res: ServerResponse): Promise<void> {
    const capability = this.#openCapability(token)
    const now = Date.now()
    if (now > capability.refreshExpiresAt) throw new HttpError(410, 'editor capability expired; open the .op file in a new editable OpenPencil card')
    if (capability.scope === 'live-draft') {
      const next = this.#sealCapability({
        ...capability,
        issuedAt: now,
        launchExpiresAt: Math.min(now + CAPABILITY_TTL_MS, capability.refreshExpiresAt),
      })
      json(res, 200, { launchUrl: `${EDITOR_ROUTE_PREFIX}/${next}/launch` })
      return
    }
    const current = await readSourceDocument(capability.sourcePath)
    if (sha256(current) !== capability.sourceSha256) {
      throw new HttpError(409, 'source changed since this card was created; open the .op file in a new editable OpenPencil card')
    }
    const next = this.#sealCapability({
      ...capability,
      issuedAt: now,
      launchExpiresAt: Math.min(now + CAPABILITY_TTL_MS, capability.refreshExpiresAt),
    })
    json(res, 200, { launchUrl: `${EDITOR_ROUTE_PREFIX}/${next}/launch` })
  }

  async #save(id: string, req: IncomingMessage, res: ServerResponse): Promise<void> {
    if (!TOKEN_PATTERN.test(id)) throw new HttpError(404, 'editor session not found')
    if (this.#liveDraftSessions.has(id)) {
      await readRequestBody(req).catch(() => Buffer.alloc(0))
      throw new HttpError(409, 'live pipeline drafts are saved only by openpencil_pipeline_finish')
    }
    const session = this.#sessions.get(id)
    if (session === undefined || session.closed) throw new HttpError(410, 'editor session has ended')
    await this.#enqueueSessionOperation(session, async () => {
      session.saving = true
      try {
        const bytes = await readRequestBody(req)
        if (session.closed) throw new HttpError(410, 'editor session ended before save completed')
        let value: unknown
        try {
          value = JSON.parse(bytes.toString('utf8'))
        } catch {
          throw new HttpError(400, 'editor save request is not valid JSON')
        }
        if (typeof value !== 'object' || value === null || Array.isArray(value)) throw new HttpError(400, 'editor save request is invalid')
        const record = value as Record<string, unknown>
        if (record.sessionId !== id || typeof record.docJson !== 'string') throw new HttpError(400, 'editor save request is incomplete')
        if (
          typeof record.generation !== 'number' || !Number.isSafeInteger(record.generation) || record.generation < 0
          || typeof record.revision !== 'number' || !Number.isSafeInteger(record.revision) || record.revision < 0
        ) throw new HttpError(400, 'editor save revision is invalid')
        const current = await readSourceDocument(session.sourcePath)
        if (sha256(current) !== session.baselineSha256) {
          throw new HttpError(409, 'source changed outside this editor; save was stopped')
        }
        const nextHash = await atomicWriteDocument(session.sourcePath, record.docJson)
        await this.#recoveryStore.discardFor(session.sourcePath).catch(() => false)
        session.baselineSha256 = nextHash
        const now = Date.now()
        session.createdAt = now
        const successor = session.refreshExpiresAt > now
          ? this.#sealCapability({
              v: 1,
              scope: 'edit-source',
              sourcePath: session.sourcePath,
              sourceSha256: nextHash,
              issuedAt: now,
              launchExpiresAt: Math.min(now + CAPABILITY_TTL_MS, session.refreshExpiresAt),
              refreshExpiresAt: session.refreshExpiresAt,
            })
          : undefined
        json(res, 200, {
          ok: true,
          sha256: nextHash,
          ...(successor === undefined ? {} : {
            editor: {
              enabled: true,
              launchUrl: `${EDITOR_ROUTE_PREFIX}/${successor}/launch`,
              refreshUrl: `${EDITOR_ROUTE_PREFIX}/${successor}/refresh`,
            },
          }),
        })
      } finally {
        session.saving = false
      }
    })
  }

  async #close(id: string): Promise<void> {
    const liveDraft = this.#liveDraftSessions.get(id)
    if (liveDraft !== undefined) {
      this.#liveDraftSessions.delete(id)
      await this.#detachLiveDraftSession(liveDraft)
      return
    }
    const session = this.#sessions.get(id)
    if (session === undefined) return
    // An ordinary user close remains guarded while a write is active. Once a
    // recovery capture has been requested, however, close becomes the commit
    // edge of that retirement transaction: remove authority for new work,
    // await all work accepted after/before capture, recapture the final daemon
    // document, then stop the child.
    if (!session.recoveryCaptureRequested && session.activeOperation !== undefined) {
      throw new HttpError(409, 'OpenPencil editor is still saving or applying a change')
    }
    this.#sessions.delete(id)
    if (session.recoveryCaptureRequested) {
      await this.#captureThenDispose(session, 'client-dispose')
    } else {
      await this.#disposeSession(session)
    }
  }

  #liveDraftReady(id: string): boolean {
    if (!TOKEN_PATTERN.test(id)) throw new HttpError(404, 'editor session not found')
    const session = this.#liveDraftSessions.get(id)
    if (session === undefined || session.closed) throw new HttpError(410, 'editor session has ended')
    session.createdAt = Date.now()
    return this.designDrafts.markLiveReady(session.draftId, session.ownerSessionId, session.attachId)
  }

  async #detachLiveDraftSession(session: LiveDraftEditorSession): Promise<void> {
    if (session.closed) return
    session.closed = true
    this.designDrafts.detachLive(session.draftId, session.ownerSessionId, session.attachId)
  }

  #revokeLiveDraftSessions(draftId: string, ownerSessionId: string): void {
    for (const [id, session] of this.#liveDraftSessions) {
      if (session.draftId !== draftId || session.ownerSessionId !== ownerSessionId) continue
      this.#liveDraftSessions.delete(id)
      session.closed = true
    }
  }

  #sessionForControl(id: string): EditorSession {
    if (!TOKEN_PATTERN.test(id)) throw new HttpError(404, 'editor session not found')
    const session = this.#sessions.get(id)
    if (session === undefined || session.closed) throw new HttpError(410, 'editor session has ended')
    return session
  }

  async #captureRecoveryForSession(id: string, reason: EditorRecoveryReason) {
    const session = this.#sessionForControl(id)
    // Set this before queueing so a close arriving immediately after this
    // request joins the atomic capture/write/close retirement path.
    session.recoveryCaptureRequested = true
    return this.#enqueueSessionOperation(session, () => this.#captureRecovery(session, reason))
  }

  async #captureRecovery(session: EditorSession, reason: EditorRecoveryReason) {
    if (session.closed) return undefined
    const [source, daemonDocument] = await Promise.all([
      readSourceDocument(session.sourcePath),
      readManagedDaemonDocument(new URL(session.iframeUrl).origin, session.daemonToken),
    ])
    return this.#recoveryStore.capture({
      sourcePath: session.sourcePath,
      // Bind the recovery to the source revision this editor was launched
      // from. The current disk bytes are used only for the clean comparison.
      // If the source changed externally from A to B while the daemon became
      // C, reopening B must surface that the C recovery was based on A.
      sourceSha256: session.baselineSha256,
      sourceDocumentJson: source.toString('utf8'),
      daemonDocument,
      reason,
    })
  }

  async #restoreRecoveryForSession(id: string, recoveryId: string): Promise<{ version: number; docJson: string }> {
    const session = this.#sessionForControl(id)
    return this.#enqueueSessionOperation(session, async () => {
      session.mutating = true
      try {
        const recovery = await this.#recoveryStore.read(session.sourcePath, recoveryId)
        if (recovery === undefined) throw new HttpError(404, 'OpenPencil recovery snapshot not found')
        const current = await readManagedDaemonDocument(new URL(session.iframeUrl).origin, session.daemonToken)
        const version = await restoreManagedDaemonDocument(
          new URL(session.iframeUrl).origin,
          session.daemonToken,
          { documentJson: recovery.documentJson, version: current.version },
        )
        session.createdAt = Date.now()
        return { version, docJson: recovery.documentJson }
      } finally {
        session.mutating = false
      }
    })
  }

  async #discardRecoveryForSession(id: string, recoveryId: string): Promise<boolean> {
    const session = this.#sessionForControl(id)
    return this.#recoveryStore.discard(session.sourcePath, recoveryId)
  }

  #activeSession(expectedSourcePath?: string, ownerSessionId?: string): EditorSession {
    this.#prune()
    const sessions = [...this.#sessions.values()].filter(session => !session.closed)
    if (sessions.length !== 1) {
      throw new Error('No active OpenPencil editor. Render with editable=true and open “Edit in sidebar” first.')
    }
    const session = sessions[0]!
    if (ownerSessionId !== undefined && session.ownerSessionId !== ownerSessionId) {
      throw new Error('The active OpenPencil editor belongs to a different DSH session')
    }
    if (expectedSourcePath !== undefined && resolve(expectedSourcePath) !== session.sourcePath) {
      throw new Error(`The active OpenPencil editor is ${session.sourcePath}, not ${resolve(expectedSourcePath)}`)
    }
    return session
  }

  async #selectionForSession(id: string): Promise<OpenPencilSelectionSnapshot> {
    if (!TOKEN_PATTERN.test(id)) throw new HttpError(404, 'editor session not found')
    const session = this.#sessions.get(id)
    if (session === undefined || session.closed) throw new HttpError(410, 'editor session has ended')
    return this.#selectionFor(session)
  }

  async #selectionFor(session: EditorSession, signal?: AbortSignal): Promise<OpenPencilSelectionSnapshot> {
    const result = await callOpenPencilMcp({
      baseUrl: new URL(session.iframeUrl).origin,
      token: session.daemonToken,
      tool: 'get_selection',
      arguments: { readDepth: 0 },
      signal,
    })
    session.createdAt = Date.now()
    return selectionSnapshotFromMcp(session.sourcePath, result.value)
  }

  async #disposeSession(session: EditorSession): Promise<void> {
    if (session.closed) return
    session.closed = true
    await stopChild(session.child)
  }

  #enqueueSessionOperation<Result>(session: EditorSession, task: () => Promise<Result>): Promise<Result> {
    const previous = session.activeOperation ?? Promise.resolve()
    const result = previous.then(async () => {
      if (session.closed) throw new HttpError(410, 'editor session ended before operation completed')
      return task()
    })
    const settled = result.then(() => {}, () => {})
    session.activeOperation = settled
    void settled.then(() => {
      if (session.activeOperation === settled) delete session.activeOperation
    })
    return result
  }

  async #waitForSessionOperation(session: EditorSession): Promise<boolean> {
    const operation = session.activeOperation
    if (operation === undefined) return true
    let timer: NodeJS.Timeout | undefined
    try {
      return await Promise.race([
        operation.then(() => true),
        new Promise<false>(resolveTimeout => {
          timer = setTimeout(() => resolveTimeout(false), OPERATION_RETIRE_TIMEOUT_MS)
          timer.unref()
        }),
      ])
    } finally {
      if (timer !== undefined) clearTimeout(timer)
    }
  }

  async #captureThenDispose(session: EditorSession, reason: EditorRecoveryReason): Promise<void> {
    try {
      // A mutation updates the daemon immediately before its promise settles.
      // Waiting here guarantees recovery observes that final document instead
      // of an earlier version. A bounded failure skips capture rather than
      // persisting a known-racy snapshot, then still reaps the child below.
      if (await this.#waitForSessionOperation(session)) {
        await this.#captureRecovery(session, reason)
      }
    } catch {
      // Recovery is best-effort, but disposal is mandatory. In particular an
      // unavailable daemon or cache must never leave an idle/HMR child alive.
    } finally {
      await this.#disposeSession(session)
    }
  }

  #prune(): void {
    const now = Date.now()
    for (const [id, session] of this.#sessions) {
      if (now - session.createdAt > SESSION_IDLE_MS) {
        this.#sessions.delete(id)
        void this.#captureThenDispose(session, 'plugin-dispose')
      }
    }
    for (const [id, session] of this.#liveDraftSessions) {
      if (now - session.createdAt > SESSION_IDLE_MS) {
        this.#liveDraftSessions.delete(id)
        void this.#detachLiveDraftSession(session)
      }
    }
  }

  #sealCapability(capability: EditorCapability): string {
    const nonce = randomBytes(12)
    const cipher = createCipheriv('aes-256-gcm', this.#editorKey, nonce)
    cipher.setAAD(EDITOR_CAPABILITY_AAD)
    const ciphertext = Buffer.concat([cipher.update(JSON.stringify(capability), 'utf8'), cipher.final()])
    return `${EDITOR_CAPABILITY_PREFIX}${Buffer.concat([nonce, cipher.getAuthTag(), ciphertext]).toString('base64url')}`
  }

  #openCapability(token: string): EditorCapability {
    if (!token.startsWith(EDITOR_CAPABILITY_PREFIX)) {
      // Compatibility boundary for pre-fix transcript cards: their random
      // in-memory token cannot safely be recreated after a plugin reload.
      throw new HttpError(410, 'editor capability expired; open the .op file in a new editable OpenPencil card')
    }
    if (token.length > EDITOR_CAPABILITY_MAX_LENGTH) throw new HttpError(404, 'editor capability not found')
    try {
      const packed = Buffer.from(token.slice(EDITOR_CAPABILITY_PREFIX.length), 'base64url')
      if (packed.length < 29) throw new Error('short token')
      const decipher = createDecipheriv('aes-256-gcm', this.#editorKey, packed.subarray(0, 12))
      decipher.setAAD(EDITOR_CAPABILITY_AAD)
      decipher.setAuthTag(packed.subarray(12, 28))
      const plain = Buffer.concat([decipher.update(packed.subarray(28)), decipher.final()])
      const capability = capabilityFrom(JSON.parse(plain.toString('utf8')))
      if (capability === undefined) throw new Error('invalid payload')
      return capability
    } catch {
      throw new HttpError(404, 'editor capability not found')
    }
  }
}
