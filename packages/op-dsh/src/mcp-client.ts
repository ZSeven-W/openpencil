/** Strict loopback JSON-RPC client for one managed OpenPencil editor. */

const MCP_TIMEOUT_MS = 20_000
const MAX_MCP_TIMEOUT_MS = 5 * 60 * 1000
// get_screenshot currently carries the PNG once as ImageContent and once in
// its compatibility metadata. Leave room for two base64 encodings of the
// bounded 16 MiB image, then strip the metadata copy during parsing.
const MAX_MCP_RESPONSE_BYTES = 48 * 1024 * 1024
const MAX_MCP_IMAGE_BYTES = 16 * 1024 * 1024
const MAX_MCP_IMAGES = 4

export interface OpenPencilNodeSummary {
  id: string
  type?: string
  name?: string
  x?: number
  y?: number
  width?: number
  height?: number
}

export interface OpenPencilSelectionSnapshot {
  sourcePath: string
  activePageId: string
  selectedIds: string[]
  nodes: OpenPencilNodeSummary[]
  updatedAt: number
}

export interface OpenPencilMcpResult {
  tool: string
  value: unknown
  text: string
  images: OpenPencilMcpImage[]
}

export interface OpenPencilMcpImage {
  mimeType: string
  bytes: Buffer
}

export interface McpCallOptions {
  baseUrl: string
  token: string
  tool: string
  arguments?: Record<string, unknown>
  signal?: AbortSignal
  fetcher?: typeof fetch
  /** Enrichment can legitimately run longer than ordinary MCP tools. */
  timeoutMs?: number
}

export interface McpVersionOptions {
  baseUrl: string
  token: string
  signal?: AbortSignal
  fetcher?: typeof fetch
  /** Test seam; production uses the same bounded timeout as MCP calls. */
  timeoutMs?: number
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function finite(value: unknown): number | undefined {
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function optionalString(value: unknown): string | undefined {
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function textFromContent(value: unknown): string {
  if (!Array.isArray(value)) return ''
  const parts: string[] = []
  for (const item of value) {
    if (isRecord(item) && item.type === 'text' && typeof item.text === 'string') parts.push(item.text)
  }
  return parts.join('\n')
}

function imagesFromContent(value: unknown): OpenPencilMcpImage[] {
  if (!Array.isArray(value)) return []
  const images: OpenPencilMcpImage[] = []
  for (const item of value) {
    if (!isRecord(item) || item.type !== 'image') continue
    if (images.length >= MAX_MCP_IMAGES) throw new Error('OpenPencil MCP returned too many images')
    if (typeof item.data !== 'string' || typeof item.mimeType !== 'string') {
      throw new Error('OpenPencil MCP returned an invalid image block')
    }
    if (!/^image\/(?:png|jpeg|webp)$/.test(item.mimeType)) {
      throw new Error('OpenPencil MCP returned an unsupported image type')
    }
    if (item.data.length === 0 || item.data.length % 4 !== 0 || !/^[A-Za-z0-9+/]*={0,2}$/.test(item.data)) {
      throw new Error('OpenPencil MCP returned invalid image data')
    }
    const bytes = Buffer.from(item.data, 'base64')
    if (bytes.length === 0 || bytes.length > MAX_MCP_IMAGE_BYTES) {
      throw new Error('OpenPencil MCP returned an image outside the size limit')
    }
    const canonical = bytes.toString('base64').replace(/=+$/, '')
    if (canonical !== item.data.replace(/=+$/, '')) throw new Error('OpenPencil MCP returned invalid image data')
    images.push({ mimeType: item.mimeType, bytes })
  }
  return images
}

function stripEmbeddedBinary(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(stripEmbeddedBinary)
  if (!isRecord(value)) return value
  const clean: Record<string, unknown> = {}
  for (const [key, item] of Object.entries(value)) {
    if (key === 'image_base64' || key === 'bytes_base64') continue
    clean[key] = stripEmbeddedBinary(item)
  }
  return clean
}

function redactSecret(value: string, secret: string): string {
  return secret.length === 0 ? value : value.split(secret).join('[redacted]')
}

function redactResultSecrets(value: unknown, secret: string): unknown {
  if (typeof value === 'string') return redactSecret(value, secret)
  if (Array.isArray(value)) return value.map(item => redactResultSecrets(item, secret))
  if (!isRecord(value)) return value
  const clean: Record<string, unknown> = {}
  for (const [key, item] of Object.entries(value)) {
    clean[key] = /(?:^|_)(?:token|authorization|bearer)(?:$|_)/i.test(key)
      ? '[redacted]'
      : redactResultSecrets(item, secret)
  }
  return clean
}

async function boundedResponseBytes(response: Response, tool: string): Promise<Buffer> {
  const declared = Number(response.headers.get('content-length'))
  if (Number.isFinite(declared) && declared > MAX_MCP_RESPONSE_BYTES) {
    throw new OpenPencilMcpTransportError(`OpenPencil MCP ${tool} response is too large`)
  }
  if (response.body === null) return Buffer.alloc(0)
  const reader = response.body.getReader()
  const chunks: Buffer[] = []
  let size = 0
  try {
    while (true) {
      const next = await reader.read()
      if (next.done) break
      const chunk = Buffer.from(next.value)
      size += chunk.length
      if (size > MAX_MCP_RESPONSE_BYTES) {
        await reader.cancel().catch(() => {})
        throw new OpenPencilMcpTransportError(`OpenPencil MCP ${tool} response is too large`)
      }
      chunks.push(chunk)
    }
  } finally {
    reader.releaseLock()
  }
  return Buffer.concat(chunks, size)
}

export class OpenPencilMcpTransportError extends Error {
  readonly uncertain = true
}

/** Decode a JSON-RPC tools/call envelope, including MCP's HTTP-200 errors. */
export function parseOpenPencilMcpResponse(tool: string, value: unknown): OpenPencilMcpResult {
  if (!isRecord(value)) throw new Error(`OpenPencil MCP ${tool} returned an invalid JSON-RPC response`)
  if (isRecord(value.error)) {
    const message = optionalString(value.error.message) ?? JSON.stringify(value.error)
    throw new Error(`OpenPencil MCP ${tool} failed: ${message}`)
  }
  if (!isRecord(value.result)) throw new Error(`OpenPencil MCP ${tool} omitted its result`)
  const rawText = textFromContent(value.result.content)
  const images = imagesFromContent(value.result.content)
  if (value.result.isError === true) {
    throw new Error(`OpenPencil MCP ${tool} failed${rawText === '' ? '' : `: ${rawText}`}`)
  }
  let decoded: unknown = rawText
  let text = rawText
  if (rawText !== '') {
    try {
      const parsed: unknown = JSON.parse(rawText)
      decoded = stripEmbeddedBinary(parsed)
      if (JSON.stringify(decoded) !== JSON.stringify(parsed)) text = JSON.stringify(decoded)
    } catch {
      // Some first-party tools intentionally return human-readable text.
    }
  }
  if (isRecord(decoded) && decoded.applied === false) {
    const details = Array.isArray(decoded.errors)
      ? decoded.errors.map(error => typeof error === 'string' ? error : JSON.stringify(error)).join('; ')
      : text
    throw new Error(`OpenPencil MCP ${tool} did not apply${details === '' ? '' : `: ${details}`}`)
  }
  return { tool, value: decoded, text, images }
}

/** Call a first-party tool on a managed editor without exposing its daemon token. */
export async function callOpenPencilMcp(options: McpCallOptions): Promise<OpenPencilMcpResult> {
  const origin = new URL(options.baseUrl)
  const loopback = origin.hostname === '127.0.0.1' || origin.hostname === 'localhost' || origin.hostname === '::1'
  if (!loopback || (origin.protocol !== 'http:' && origin.protocol !== 'https:')) {
    throw new Error('OpenPencil MCP endpoint must use an HTTP loopback origin')
  }
  if (origin.pathname !== '/' || origin.search !== '' || origin.hash !== '') {
    throw new Error('OpenPencil MCP base URL must be an origin')
  }
  const fetcher = options.fetcher ?? fetch
  const timeoutMs = options.timeoutMs ?? MCP_TIMEOUT_MS
  if (!Number.isSafeInteger(timeoutMs) || timeoutMs < 1 || timeoutMs > MAX_MCP_TIMEOUT_MS) {
    throw new Error(`OpenPencil MCP ${options.tool} timeout is invalid`)
  }
  const timeout = AbortSignal.timeout(timeoutMs)
  const signal = options.signal === undefined ? timeout : AbortSignal.any([options.signal, timeout])
  try {
    const response = await fetcher(new URL('/mcp', origin).href, {
      method: 'POST',
      headers: {
        'content-type': 'application/json',
        accept: 'application/json',
      },
      body: JSON.stringify({
        jsonrpc: '2.0',
        id: `dsh-${Date.now().toString(36)}`,
        method: 'tools/call',
        params: { name: options.tool, arguments: options.arguments ?? {} },
      }),
      signal,
    })
    const bytes = await boundedResponseBytes(response, options.tool)
    if (!response.ok) {
      throw new OpenPencilMcpTransportError(`OpenPencil MCP ${options.tool} request failed (${response.status})`)
    }
    let value: unknown
    try {
      value = JSON.parse(bytes.toString('utf8'))
    } catch {
      throw new OpenPencilMcpTransportError(`OpenPencil MCP ${options.tool} returned invalid JSON`)
    }
    const parsed = parseOpenPencilMcpResponse(options.tool, value)
    return {
      ...parsed,
      value: redactResultSecrets(parsed.value, options.token),
      text: redactSecret(parsed.text, options.token),
    }
  } catch (error) {
    if (error instanceof OpenPencilMcpTransportError) throw error
    if (
      error instanceof Error
      && (
        error.message.startsWith(`OpenPencil MCP ${options.tool} failed`)
        || error.message.startsWith(`OpenPencil MCP ${options.tool} did not apply`)
      )
    ) {
      throw new Error(redactSecret(error.message, options.token))
    }
    const message = redactSecret(error instanceof Error ? error.message : String(error), options.token)
    throw new OpenPencilMcpTransportError(`OpenPencil MCP ${options.tool} request did not complete: ${message}`)
  }
}

/** Probe the daemon document version before and after a write. */
export async function getOpenPencilMcpVersion(options: McpVersionOptions): Promise<number> {
  const origin = new URL(options.baseUrl)
  const loopback = origin.hostname === '127.0.0.1' || origin.hostname === 'localhost' || origin.hostname === '::1'
  if (!loopback || (origin.protocol !== 'http:' && origin.protocol !== 'https:')) {
    throw new Error('OpenPencil MCP endpoint must use an HTTP loopback origin')
  }
  const timeout = AbortSignal.timeout(options.timeoutMs ?? MCP_TIMEOUT_MS)
  const signal = options.signal === undefined ? timeout : AbortSignal.any([options.signal, timeout])
  const response = await (options.fetcher ?? fetch)(new URL('/api/mcp/version', origin).href, {
    headers: { accept: 'application/json' },
    signal,
  })
  if (!response.ok) throw new Error(`OpenPencil MCP version probe failed (${response.status})`)
  const value: unknown = await response.json()
  if (!isRecord(value) || typeof value.version !== 'number' || !Number.isSafeInteger(value.version) || value.version < 0) {
    throw new Error('OpenPencil MCP version probe returned an invalid result')
  }
  return value.version
}

function nodeSummary(value: unknown): OpenPencilNodeSummary | undefined {
  if (!isRecord(value) || typeof value.id !== 'string' || value.id.length === 0) return undefined
  return {
    id: value.id,
    ...(optionalString(value.type) === undefined ? {} : { type: optionalString(value.type) }),
    ...(optionalString(value.name) === undefined ? {} : { name: optionalString(value.name) }),
    ...(finite(value.x) === undefined ? {} : { x: finite(value.x) }),
    ...(finite(value.y) === undefined ? {} : { y: finite(value.y) }),
    ...(finite(value.width) === undefined ? {} : { width: finite(value.width) }),
    ...(finite(value.height) === undefined ? {} : { height: finite(value.height) }),
  }
}

/** Project get_selection into the bounded shape shared with the DSH client. */
export function selectionSnapshotFromMcp(
  sourcePath: string,
  value: unknown,
  updatedAt = Date.now(),
): OpenPencilSelectionSnapshot {
  if (!isRecord(value)) throw new Error('OpenPencil get_selection returned an invalid result')
  const selectedIds = Array.isArray(value.selectedIds)
    ? value.selectedIds.filter((id): id is string => typeof id === 'string' && id.length > 0)
    : []
  const nodes = Array.isArray(value.nodes)
    ? value.nodes.map(nodeSummary).filter((node): node is OpenPencilNodeSummary => node !== undefined)
    : []
  return {
    sourcePath,
    activePageId: optionalString(value.activePageId) ?? '',
    selectedIds,
    nodes,
    updatedAt,
  }
}
