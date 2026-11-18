/** Shared lifecycle primitives for plugin-owned OpenPencil managed daemons. */

import { spawn, type ChildProcessWithoutNullStreams } from 'node:child_process'
import { managedEditorProcessOptions } from './managed-editor-process.js'
import { randomBytes, timingSafeEqual } from 'node:crypto'
import {
  createServer,
  request as requestHttp,
  type IncomingHttpHeaders,
  type IncomingMessage,
  type OutgoingHttpHeaders,
  type Server,
  type ServerResponse,
} from 'node:http'
import type { Socket } from 'node:net'
import { isAbsolute } from 'node:path'
import { readManagedDaemonDocument, type ManagedDaemonDocument } from './editor-recovery.js'
import type { EditorRuntime } from './editor-runtime.js'

const START_TIMEOUT_MS = 20_000
const READY_TIMEOUT_MS = 15_000
const STOP_TIMEOUT_MS = 3_000
const RESET_TIMEOUT_MS = 8_000
const MAX_HANDSHAKE_BYTES = 16 * 1024
const MAX_DIAGNOSTIC_BYTES = 64 * 1024
const MAX_RESET_RESPONSE_BYTES = 16 * 1024
const READ_ONLY_PROXY_TOKEN_BYTES = 32
const READ_ONLY_PROXY_ERROR_BYTES = 16 * 1024

const HOP_BY_HOP_HEADERS = new Set([
  'connection',
  'keep-alive',
  'proxy-authenticate',
  'proxy-authorization',
  'te',
  'trailer',
  'transfer-encoding',
  'upgrade',
])

const FORWARDED_REQUEST_HEADERS = new Set([
  'accept',
  'accept-encoding',
  'cache-control',
  'if-modified-since',
  'if-none-match',
  'range',
  'user-agent',
])

const READ_ONLY_PROXY_GET_PATHS = new Set([
  '/api/mcp/document',
  '/api/mcp/indicators',
  '/api/mcp/selection',
  '/api/mcp/server',
  '/api/mcp/version',
])
// `/api/mcp/events` is intentionally absent: EventSource cannot present the
// per-attach header. The current Web shell's authoritative 400 ms
// version/document polling remains available and is sufficient for Agent
// batch updates without opening a tokenless stream.

interface ManagedHandshake {
  port: number
  token: string
  version: string | number
}

export interface ManagedEditorDaemon {
  /** Internal child handle. Never include this object in a model-facing result. */
  readonly child: ChildProcessWithoutNullStreams
  readonly baseUrl: string
  /** Internal managed credential. Never include this object in a model-facing result. */
  readonly token: string
  readonly sourcePath: string
  readonly runtimeVersion: string
}

/** Revocable browser-only view of one managed daemon. */
export interface ManagedEditorReadOnlyProxy {
  readonly baseUrl: string
  /** Browser credential for this proxy only; never accepted by the daemon. */
  readonly token: string
  /** Synchronously revoke the browser endpoint and all open proxy sockets. */
  stop(): void
}

export interface StartManagedEditorDaemonOptions {
  sourcePath: string
  allowOrigin: string
  signal?: AbortSignal
  /** Lifecycle seam used by the controller to join children during disposal. */
  onSpawn?: (child: ChildProcessWithoutNullStreams) => void
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error)
}

function sanitizedDiagnostics(value: string, sourcePath?: string): string {
  const redacted = value
    .replace(/((?:authorization|bearer|token)[\s"'=:]+)[A-Za-z0-9._~-]{8,}/gi, '$1[redacted]')
  return (sourcePath === undefined ? redacted : redacted.split(sourcePath).join('[managed-document]')).trim()
}

function proxyJson(res: ServerResponse, status: number, value: unknown): void {
  if (res.headersSent || res.destroyed) return
  const body = Buffer.from(JSON.stringify(value))
  res.writeHead(status, {
    'cache-control': 'no-store',
    'content-type': 'application/json; charset=utf-8',
    'content-length': body.byteLength,
  })
  res.end(body)
}

function browserTokenMatches(req: IncomingMessage, token: string): boolean {
  const presented = req.headers['x-openpencil-token']
  if (typeof presented !== 'string') return false
  const expectedBytes = Buffer.from(token)
  const presentedBytes = Buffer.from(presented)
  return expectedBytes.byteLength === presentedBytes.byteLength
    && timingSafeEqual(expectedBytes, presentedBytes)
}

function requestHeadersForDaemon(req: IncomingMessage): OutgoingHttpHeaders {
  const headers: OutgoingHttpHeaders = {}
  for (const [name, value] of Object.entries(req.headers)) {
    if (!FORWARDED_REQUEST_HEADERS.has(name) || value === undefined) continue
    headers[name] = value
  }
  return headers
}

function responseHeadersForBrowser(headers: IncomingHttpHeaders): OutgoingHttpHeaders {
  const forwarded: OutgoingHttpHeaders = {}
  for (const [name, value] of Object.entries(headers)) {
    if (HOP_BY_HOP_HEADERS.has(name) || name === 'set-cookie' || value === undefined) continue
    forwarded[name] = value
  }
  return forwarded
}

function forwardReadOnlyRequest(
  req: IncomingMessage,
  res: ServerResponse,
  daemon: ManagedEditorDaemon,
  path: string,
): void {
  const upstream = requestHttp(new URL(path, daemon.baseUrl), {
    method: req.method,
    headers: requestHeadersForDaemon(req),
  }, upstreamResponse => {
    if (res.destroyed) {
      upstreamResponse.destroy()
      return
    }
    res.writeHead(
      upstreamResponse.statusCode ?? 502,
      responseHeadersForBrowser(upstreamResponse.headers),
    )
    upstreamResponse.pipe(res)
  })
  upstream.on('error', () => {
    if (!res.headersSent) proxyJson(res, 502, { ok: false, error: 'OpenPencil live canvas upstream is unavailable' })
    else res.destroy()
  })
  req.once('aborted', () => { upstream.destroy() })
  res.once('close', () => { upstream.destroy() })
  // Only GET/HEAD requests reach this helper, so browser-controlled bytes are
  // never relayed into the managed daemon.
  upstream.end()
}

async function currentDaemonVersion(daemon: ManagedEditorDaemon): Promise<number> {
  const response = await fetch(`${daemon.baseUrl}/api/mcp/version`, {
    headers: {
      accept: 'application/json',
    },
    signal: AbortSignal.timeout(2_000),
  })
  const bytes = Buffer.from(await response.arrayBuffer())
  if (!response.ok) throw new Error(`OpenPencil managed daemon version failed (${response.status})`)
  if (bytes.byteLength > READ_ONLY_PROXY_ERROR_BYTES) {
    throw new Error('OpenPencil managed daemon version response exceeded its size limit')
  }
  try {
    const value = JSON.parse(bytes.toString('utf8')) as Record<string, unknown>
    if (typeof value.version === 'number' && Number.isSafeInteger(value.version) && value.version >= 0) {
      return value.version
    }
    throw new Error('OpenPencil managed daemon version response was invalid')
  } catch {
    throw new Error('OpenPencil managed daemon version response was invalid')
  }
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
      const detail = diagnostics()
      finish(new Error(`OpenPencil editor host exited before startup (${String(code)})${detail === '' ? '' : `: ${detail}`}`))
    }
    const timer = setTimeout(() => {
      const detail = diagnostics()
      finish(new Error(`OpenPencil editor host did not start within ${START_TIMEOUT_MS} ms${detail === '' ? '' : `: ${detail}`}`))
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

/** Stop a child even if startup has not produced a daemon handshake yet. */
export function stopManagedEditorChild(child: ChildProcessWithoutNullStreams): Promise<void> {
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

/** Start a managed host bound to one already-created `.op` document. */
export async function startManagedEditorDaemon(
  runtime: EditorRuntime,
  options: StartManagedEditorDaemonOptions,
): Promise<ManagedEditorDaemon> {
  options.signal?.throwIfAborted()
  if (!isAbsolute(options.sourcePath) || !options.sourcePath.toLowerCase().endsWith('.op')) {
    throw new Error('OpenPencil managed daemon source must be an absolute .op path')
  }
  const origin = new URL(options.allowOrigin)
  if (origin.origin !== options.allowOrigin || (origin.protocol !== 'http:' && origin.protocol !== 'https:')) {
    throw new Error('OpenPencil managed daemon allow-origin must be an HTTP origin')
  }
  const launch = await managedEditorProcessOptions(runtime, options.sourcePath, options.allowOrigin)
  const child = spawn(runtime.binary, launch.args, { stdio: ['pipe', 'pipe', 'pipe'], env: launch.env })
  options.onSpawn?.(child)
  let diagnostics = ''
  child.stderr.on('data', (chunk: Buffer) => {
    if (diagnostics.length < MAX_DIAGNOSTIC_BYTES) {
      diagnostics += chunk.toString('utf8').slice(0, MAX_DIAGNOSTIC_BYTES - diagnostics.length)
    }
  })
  const onAbort = (): void => { void stopManagedEditorChild(child) }
  options.signal?.addEventListener('abort', onAbort, { once: true })
  try {
    const handshake = await waitForHandshake(
      child,
      () => sanitizedDiagnostics(diagnostics, options.sourcePath),
      options.signal,
    )
    const actualVersion = String(handshake.version)
    if (actualVersion !== runtime.openPencilVersion) {
      throw new Error(
        `OpenPencil editor runtime version mismatch: expected ${runtime.openPencilVersion}, received ${actualVersion}`,
      )
    }
    options.signal?.throwIfAborted()
    const baseUrl = `http://127.0.0.1:${handshake.port}`
    await waitForEditorReady(baseUrl, options.signal)
    options.signal?.throwIfAborted()
    return {
      child,
      baseUrl,
      token: handshake.token,
      sourcePath: options.sourcePath,
      runtimeVersion: actualVersion,
    }
  } catch (error) {
    await stopManagedEditorChild(child)
    throw error
  } finally {
    options.signal?.removeEventListener('abort', onAbort)
  }
}

/**
 * Expose a managed daemon to one live browser mount without delegating its
 * writer credential. Static resources are public on this ephemeral loopback
 * origin; daemon data requires a per-attach browser token and only GET/HEAD
 * requests are forwarded. The Web shell's mandatory bootstrap reset receives
 * a synthetic already-consumed acknowledgement, while every other mutation is
 * rejected before any bytes can reach the daemon.
 */
export async function startManagedEditorReadOnlyProxy(
  daemon: ManagedEditorDaemon,
  signal?: AbortSignal,
): Promise<ManagedEditorReadOnlyProxy> {
  signal?.throwIfAborted()
  const browserToken = randomBytes(READ_ONLY_PROXY_TOKEN_BYTES).toString('base64url')
  const sockets = new Set<Socket>()
  let stopped = false
  let expectedHost: string | undefined
  const server: Server = createServer((req, res) => {
    if (expectedHost !== undefined && req.headers.host !== expectedHost) {
      req.resume()
      proxyJson(res, 403, { ok: false, error: 'OpenPencil live canvas host is invalid' })
      return
    }
    const rawUrl = req.url ?? '/'
    if (rawUrl.startsWith('//') || /^[A-Za-z][A-Za-z0-9+.-]*:\/\//.test(rawUrl)) {
      proxyJson(res, 400, { ok: false, error: 'absolute OpenPencil live canvas URLs are forbidden' })
      return
    }
    let path: string
    let pathname: string
    try {
      const parsed = new URL(rawUrl, 'http://127.0.0.1')
      pathname = parsed.pathname
      path = `${parsed.pathname}${parsed.search}`
    } catch {
      proxyJson(res, 400, { ok: false, error: 'invalid OpenPencil live canvas request' })
      return
    }
    const method = (req.method ?? 'GET').toUpperCase()
    if ((method === 'GET' || method === 'HEAD') && (
      req.headers['transfer-encoding'] !== undefined
      || (req.headers['content-length'] !== undefined && req.headers['content-length'] !== '0')
    )) {
      req.resume()
      proxyJson(res, 400, { ok: false, error: 'OpenPencil live canvas reads cannot carry a request body' })
      return
    }
    const privileged = pathname === '/mcp'
      || pathname.startsWith('/mcp/')
      || pathname.startsWith('/api/')

    if (method === 'OPTIONS') {
      res.writeHead(204, {
        'access-control-allow-headers': 'X-OpenPencil-Token, Content-Type',
        'access-control-allow-methods': 'GET, HEAD, POST, OPTIONS',
        'cache-control': 'no-store',
      })
      res.end()
      return
    }
    if (privileged && !browserTokenMatches(req, browserToken)) {
      req.resume()
      proxyJson(res, 401, { ok: false, error: 'OpenPencil live canvas capability is required' })
      return
    }
    if (method === 'POST' && pathname === '/api/mcp/sync-reset') {
      // prepareLiveLaunch consumed and verified the daemon's one-shot reset.
      // Do not forward this browser POST: acknowledge the harmless skipped
      // state locally so a compromised iframe still cannot race that guard.
      req.resume()
      void currentDaemonVersion(daemon).then(version => {
        proxyJson(res, 200, { ok: true, skipped: true, version })
      }).catch(() => {
        proxyJson(res, 502, { ok: false, error: 'OpenPencil live canvas version is unavailable' })
      })
      return
    }
    if (method !== 'GET' && method !== 'HEAD') {
      req.resume()
      proxyJson(res, 403, {
        ok: false,
        error: 'OpenPencil live pipeline canvas is read-only',
        code: 'read-only-live-canvas',
      })
      return
    }
    if (privileged && !READ_ONLY_PROXY_GET_PATHS.has(pathname)) {
      proxyJson(res, 403, {
        ok: false,
        error: 'OpenPencil live pipeline canvas route is not available to a read-only browser',
        code: 'read-only-live-canvas',
      })
      return
    }
    forwardReadOnlyRequest(req, res, daemon, path)
  })
  server.on('connection', socket => {
    sockets.add(socket)
    socket.once('close', () => { sockets.delete(socket) })
  })
  server.on('clientError', (_error, socket) => {
    socket.end('HTTP/1.1 400 Bad Request\r\nConnection: close\r\n\r\n')
  })
  try {
    await new Promise<void>((resolveListening, rejectListening) => {
      const onError = (error: Error): void => {
        cleanup()
        rejectListening(error)
      }
      const onAbort = (): void => {
        cleanup()
        rejectListening(signal?.reason instanceof Error
          ? signal.reason
          : new Error('OpenPencil live canvas proxy startup was cancelled'))
      }
      const cleanup = (): void => {
        server.off('error', onError)
        signal?.removeEventListener('abort', onAbort)
      }
      server.once('error', onError)
      signal?.addEventListener('abort', onAbort, { once: true })
      server.listen(0, '127.0.0.1', () => {
        const address = server.address()
        if (address !== null && typeof address !== 'string') expectedHost = `127.0.0.1:${address.port}`
        cleanup()
        resolveListening()
      })
      if (signal?.aborted) onAbort()
    })
  } catch (error) {
    server.close()
    for (const socket of sockets) socket.destroy()
    throw error
  }
  const address = server.address()
  if (address === null || typeof address === 'string') {
    server.close()
    for (const socket of sockets) socket.destroy()
    throw new Error('OpenPencil live canvas proxy did not bind a loopback port')
  }
  server.unref()
  const stop = (): void => {
    if (stopped) return
    stopped = true
    server.close()
    for (const socket of sockets) socket.destroy()
  }
  // Retain an error listener after startup so a late listener fault cannot
  // escape as an uncaught process error. Revocation remains owned by stop().
  server.on('error', stop)
  return {
    baseUrl: `http://127.0.0.1:${address.port}`,
    token: browserToken,
    stop,
  }
}

export function stopManagedEditorDaemon(daemon: ManagedEditorDaemon): Promise<void> {
  return stopManagedEditorChild(daemon.child)
}

/**
 * Consume the daemon's one-shot sync reset after the controller has refreshed
 * its private backing file. The credential and backing path never enter a returned
 * value or diagnostic; a browser mount can therefore only receive the later
 * `skipped: true` reset response.
 */
export async function resetManagedEditorDaemonFromSource(
  daemon: ManagedEditorDaemon,
  signal?: AbortSignal,
): Promise<{ version: number; skipped: boolean }> {
  if (signal?.aborted) throw new Error('OpenPencil managed daemon reset was cancelled')
  const timeout = AbortSignal.timeout(RESET_TIMEOUT_MS)
  const requestSignal = signal === undefined ? timeout : AbortSignal.any([signal, timeout])
  let response: Response
  try {
    response = await fetch(`${daemon.baseUrl}/api/mcp/sync-reset`, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
      },
      body: '{}',
      signal: requestSignal,
    })
  } catch {
    if (signal?.aborted) throw new Error('OpenPencil managed daemon reset was cancelled')
    if (timeout.aborted) throw new Error('OpenPencil managed daemon reset timed out')
    throw new Error('OpenPencil managed daemon reset request failed')
  }
  let text: string
  try {
    text = await response.text()
  } catch {
    if (signal?.aborted) throw new Error('OpenPencil managed daemon reset was cancelled')
    if (timeout.aborted) throw new Error('OpenPencil managed daemon reset timed out')
    throw new Error('OpenPencil managed daemon reset response failed')
  }
  if (Buffer.byteLength(text) > MAX_RESET_RESPONSE_BYTES) {
    throw new Error('OpenPencil managed daemon reset response exceeded its size limit')
  }
  if (response.status !== 200) {
    throw new Error(`OpenPencil managed daemon reset failed (${response.status})`)
  }
  let value: unknown
  try {
    value = JSON.parse(text)
  } catch {
    throw new Error('OpenPencil managed daemon reset returned invalid JSON')
  }
  if (typeof value !== 'object' || value === null || Array.isArray(value)) {
    throw new Error('OpenPencil managed daemon reset returned an invalid response')
  }
  const result = value as Record<string, unknown>
  if (
    result.ok !== true
    || (result.skipped !== undefined && typeof result.skipped !== 'boolean')
    || typeof result.version !== 'number'
    || !Number.isSafeInteger(result.version)
    || result.version < 0
  ) {
    throw new Error('OpenPencil managed daemon did not accept its initial reset')
  }
  return { version: result.version, skipped: result.skipped === true }
}

/** Read an authoritative bounded snapshot without exposing daemon credentials. */
export function readManagedEditorDaemon(
  daemon: ManagedEditorDaemon,
  signal?: AbortSignal,
): Promise<ManagedDaemonDocument> {
  return readManagedDaemonDocument(daemon.baseUrl, daemon.token, fetch, signal)
}
