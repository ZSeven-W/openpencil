/** Browser-side protocol helpers for the managed OpenPencil editor iframe. */

export type EditorInboundMessage =
  | { type: 'op-bridge/listening' }
  | { type: 'op-bridge/ready'; generation: number; revision: number }
  | { type: 'op-bridge/opened'; generation: number }
  | { type: 'op-bridge/dirty-changed'; generation: number; revision: number; dirty: boolean }
  | { type: 'op-bridge/snapshot-result'; requestId: string; docJson: string; generation: number; revision: number }
  | { type: 'op-bridge/snapshot-conflict'; requestId: string; serverVersion: number }
  | { type: 'op-bridge/sync-conflict'; generation: number; revision: number; serverVersion: number }
  | { type: 'op-bridge/conflict-resolved'; requestId: string }
  | { type: 'op-shell/save' }
  | { type: 'op-shell/copy'; text: string }

export type EditorOutboundMessage =
  | { type: 'op-bridge/init'; token: string; mcpUrl?: string }
  | { type: 'op-bridge/theme'; colorScheme: EditorColorScheme }
  | { type: 'op-bridge/locale'; locale: EditorLocale }
  | { type: 'op-bridge/open-document'; json: string }
  | { type: 'op-bridge/snapshot'; purpose: 'save'; requestId: string }
  | { type: 'op-bridge/save-committed'; generation: number; revision: number }

export type EditorColorScheme = 'light' | 'dark'
export type EditorLocale = 'zh-CN' | 'en-US'

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function safeInteger(value: unknown): value is number {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0
}

function string(value: unknown): value is string {
  return typeof value === 'string'
}

/** Parse only the editor/host messages DSH implements. Unknown traffic is ignored. */
export function parseEditorInbound(raw: unknown): EditorInboundMessage | undefined {
  if (typeof raw !== 'string') return undefined
  let value: unknown
  try {
    value = JSON.parse(raw)
  } catch {
    return undefined
  }
  if (!isRecord(value) || typeof value.type !== 'string') return undefined
  switch (value.type) {
    case 'op-bridge/listening':
      // Editor-side announcement that its message listener just registered.
      // The contract payload carries no other fields; extra keys are ignored
      // exactly like every other inbound message in this parser.
      return { type: value.type }
    case 'op-bridge/ready':
      return safeInteger(value.generation) && safeInteger(value.revision)
        ? { type: value.type, generation: value.generation, revision: value.revision }
        : undefined
    case 'op-bridge/opened':
      return safeInteger(value.generation) ? { type: value.type, generation: value.generation } : undefined
    case 'op-bridge/dirty-changed':
      return safeInteger(value.generation) && safeInteger(value.revision) && typeof value.dirty === 'boolean'
        ? { type: value.type, generation: value.generation, revision: value.revision, dirty: value.dirty }
        : undefined
    case 'op-bridge/snapshot-result':
      return string(value.requestId) && string(value.docJson)
        && safeInteger(value.generation) && safeInteger(value.revision)
        ? {
            type: value.type,
            requestId: value.requestId,
            docJson: value.docJson,
            generation: value.generation,
            revision: value.revision,
          }
        : undefined
    case 'op-bridge/snapshot-conflict':
      return string(value.requestId) && safeInteger(value.serverVersion)
        ? { type: value.type, requestId: value.requestId, serverVersion: value.serverVersion }
        : undefined
    case 'op-bridge/sync-conflict':
      return safeInteger(value.generation) && safeInteger(value.revision) && safeInteger(value.serverVersion)
        ? { type: value.type, generation: value.generation, revision: value.revision, serverVersion: value.serverVersion }
        : undefined
    case 'op-bridge/conflict-resolved':
      return string(value.requestId) ? { type: value.type, requestId: value.requestId } : undefined
    case 'op-shell/save':
      return { type: value.type }
    case 'op-shell/copy':
      return string(value.text) ? { type: value.type, text: value.text } : undefined
    default:
      return undefined
  }
}

export function encodeEditorOutbound(message: EditorOutboundMessage): string {
  return JSON.stringify(message)
}

/** Require an absolute loopback editor URL and derive its exact target origin. */
export function editorOrigin(iframeUrl: string): string {
  const url = new URL(iframeUrl)
  const loopback = url.hostname === '127.0.0.1' || url.hostname === 'localhost' || url.hostname === '::1'
  if (!loopback || (url.protocol !== 'http:' && url.protocol !== 'https:')) {
    throw new Error('OpenPencil editor URL must use an HTTP loopback origin')
  }
  return url.origin
}

/** Pin the host's resolved theme into the editor's first navigation. */
export function editorIframeUrlWithTheme(iframeUrl: string, colorScheme: EditorColorScheme): string {
  const url = new URL(iframeUrl)
  url.searchParams.set('theme', colorScheme)
  return url.href
}

/** Pin the host's resolved locale into the editor's first navigation. */
export function editorIframeUrlWithLocale(iframeUrl: string, locale: EditorLocale): string {
  const url = new URL(iframeUrl)
  url.searchParams.set('locale', locale)
  return url.href
}

/** Translate DSH's compact locale id to the editor's BCP 47 contract. */
export function editorLocaleFromDsh(locale: 'zh' | 'en'): EditorLocale {
  return locale === 'zh' ? 'zh-CN' : 'en-US'
}

/** Resolve a launch/save/close capability and reject cross-origin control routes. */
export function editorControlUrl(raw: string, base = window.location.href): string {
  const page = new URL(base)
  const url = new URL(raw, page)
  if (url.origin !== page.origin) throw new Error('OpenPencil editor control URL must be same-origin')
  return url.href
}

/** Validate source and exact origin before parsing any iframe message. */
export function editorMessageFrom(
  event: Pick<MessageEvent, 'source' | 'origin' | 'data'>,
  frameWindow: Window | null,
  origin: string,
): EditorInboundMessage | undefined {
  if (frameWindow === null || event.source !== frameWindow || event.origin !== origin) return undefined
  return parseEditorInbound(event.data)
}

interface ActiveEditor {
  readonly token: symbol
  readonly close: () => boolean | void
}

let activeEditor: ActiveEditor | undefined

/** Read-only gate for background auto-open flows; never asks an owner to close. */
export function hasActiveEditor(): boolean {
  return activeEditor !== undefined
}

/** Page-wide single-editor coordinator. An existing dirty editor may veto takeover. */
export function claimEditor(
  token: symbol,
  close: () => boolean | void,
  options: { replace?: boolean } = {},
): (() => void) | undefined {
  const previous = activeEditor
  if (previous !== undefined && previous.token !== token) {
    if (options.replace === false || previous.close() === false) return undefined
  }
  activeEditor = { token, close }
  return () => {
    if (activeEditor?.token === token) activeEditor = undefined
  }
}

/** Confirm before a user-driven panel close would discard unsaved canvas edits. */
export function confirmEditorClose(dirty: boolean, confirm = window.confirm): boolean {
  return !dirty || confirm('OpenPencil has unsaved changes. Close the editor and discard them?')
}
