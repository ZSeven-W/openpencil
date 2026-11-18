/** Full OpenPencil editor shared by native Tool details and plugin-owned surfaces. */

import { useCallback, useEffect, useRef, useState } from 'react'
import type { PresentationGrant } from './index.js'
import {
  claimEditor,
  confirmEditorClose,
  editorControlUrl,
  editorIframeUrlWithTheme,
  editorIframeUrlWithLocale,
  editorMessageFrom,
  editorOrigin,
  encodeEditorOutbound,
  type EditorColorScheme,
  type EditorInboundMessage,
  type EditorLocale,
} from './editor-bridge.js'
import { editorGrantForBoot, rememberEditorSuccessor } from './editor-successor.js'
import {
  clearOpenPencilSelection,
  liveSelectionOf,
  publishOpenPencilSelection,
} from './selection-store.js'
import { startEditorSelectionPolling } from './selection-polling.js'
import {
  captureManagedEditorRecovery,
  editorRecoveryCopy,
  editorRecoverySummaryOf,
  restoreManagedEditorRecovery,
  type EditorRecoverySummary,
} from './editor-recovery.js'

export type { EditorRecoverySummary } from './editor-recovery.js'

export interface LaunchResponse {
  sessionId: string
  iframeUrl: string
  token: string
  saveUrl: string
  selectionUrl?: string
  recoveryUrl?: string
  recovery?: EditorRecoverySummary
  closeUrl: string
  docJson?: string
  /** The iframe is attached to a pipeline-owned daemon instead of a persisted source session. */
  liveDraft?: boolean
  /** Same-origin capability that acknowledges the live iframe after its bridge is ready. */
  readyUrl?: string
  /** Client-only marker: the persisted launch capability was renewed. */
  renewed?: true
}

/** Pipeline drafts publish through `pipeline_finish`; their canvas has no independent save edge. */
export function managedEditorAllowsSave(launch: Pick<LaunchResponse, 'liveDraft'> | undefined): boolean {
  return launch?.liveDraft !== true
}

const DEFAULT_REFRESH_URL = '/_dsh/dsh-openpencil/editor/refresh'
const MAX_RESPONSE_ERROR_LENGTH = 512

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function launchResponseOf(value: unknown): LaunchResponse {
  if (!isRecord(value)) throw new Error('OpenPencil editor launch returned an invalid response')
  const fields = ['sessionId', 'iframeUrl', 'token', 'saveUrl', 'closeUrl'] as const
  for (const field of fields) {
    if (typeof value[field] !== 'string' || value[field].length === 0) {
      throw new Error(`OpenPencil editor launch omitted ${field}`)
    }
  }
  const liveDraft = value.liveDraft === true
  if (value.liveDraft !== undefined && typeof value.liveDraft !== 'boolean') {
    throw new Error('OpenPencil editor launch returned an invalid liveDraft flag')
  }
  if (liveDraft && (typeof value.readyUrl !== 'string' || value.readyUrl.length === 0)) {
    throw new Error('OpenPencil live draft launch omitted readyUrl')
  }
  return {
    sessionId: value.sessionId as string,
    iframeUrl: value.iframeUrl as string,
    token: value.token as string,
    saveUrl: editorControlUrl(value.saveUrl as string),
    ...(typeof value.selectionUrl === 'string' && value.selectionUrl.length > 0
      ? { selectionUrl: editorControlUrl(value.selectionUrl) }
      : {}),
    ...(typeof value.recoveryUrl === 'string' && value.recoveryUrl.length > 0
      ? { recoveryUrl: editorControlUrl(value.recoveryUrl) }
      : {}),
    closeUrl: editorControlUrl(value.closeUrl as string),
    ...(editorRecoverySummaryOf(value.recovery) === undefined ? {} : { recovery: editorRecoverySummaryOf(value.recovery) }),
    ...(typeof value.docJson === 'string' ? { docJson: value.docJson } : {}),
    ...(value.liveDraft === undefined ? {} : { liveDraft }),
    ...(typeof value.readyUrl === 'string' && value.readyUrl.length > 0
      ? { readyUrl: editorControlUrl(value.readyUrl) }
      : {}),
  }
}

async function responseJson(response: Response, action: string): Promise<unknown> {
  if (!response.ok) {
    const fallback = `${action} failed (${response.status})`
    let body: unknown
    try {
      body = await response.json()
    } catch {
      throw new Error(fallback)
    }
    if (!isRecord(body) || typeof body.error !== 'string' || body.error.length > MAX_RESPONSE_ERROR_LENGTH) {
      throw new Error(fallback)
    }
    const detail = body.error.trim()
    if (detail.length === 0) throw new Error(fallback)
    throw new Error(`${fallback}: ${detail}`)
  }
  return response.json()
}

function refreshedLaunchUrlOf(value: unknown): string {
  if (!isRecord(value) || typeof value.launchUrl !== 'string' || value.launchUrl.length === 0) {
    throw new Error('OpenPencil editor refresh omitted launchUrl')
  }
  return editorControlUrl(value.launchUrl)
}

export interface EditorBootResult {
  launch: LaunchResponse
  documentJson: string
}

export interface EditorPanelCopy {
  save: string
  saving: string
  unsaved: string
  saved: string
  unavailable: string
  loading: string
  errorTitle: string
  pngFallback: string
  editorTitle: (title: string) => string
  editorTimeout: string
  editorBusy: string
  discard: string
  saveConflict: (serverVersion: number) => string
  syncConflict: (serverVersion: number) => string
}

const EDITOR_PANEL_COPY: Record<EditorLocale, EditorPanelCopy> = {
  'zh-CN': {
    save: '保存',
    saving: '保存中…',
    unsaved: '未保存',
    saved: '已保存',
    unavailable: '当前结果无法使用可编辑的 OpenPencil 画布。',
    loading: '正在加载可编辑的 OpenPencil 画布…',
    errorTitle: 'OpenPencil 编辑器不可用',
    pngFallback: '打开 PNG 预览',
    editorTitle: title => `OpenPencil 编辑器：${title}`,
    editorTimeout: 'OpenPencil 编辑器未能及时就绪',
    editorBusy: '另一个 OpenPencil 编辑器仍有未保存的更改。',
    discard: 'OpenPencil 中有未保存的更改，确定关闭并放弃吗？',
    saveConflict: serverVersion => `OpenPencil 保存冲突（服务器版本 ${serverVersion}）`,
    syncConflict: serverVersion => `源文件已在编辑器外部更改（服务器版本 ${serverVersion}），已停止保存。`,
  },
  'en-US': {
    save: 'Save',
    saving: 'Saving…',
    unsaved: 'Unsaved',
    saved: 'Saved',
    unavailable: 'Editable OpenPencil canvas is not available for this result.',
    loading: 'Loading editable OpenPencil canvas…',
    errorTitle: 'OpenPencil editor unavailable',
    pngFallback: 'Open PNG fallback',
    editorTitle: title => `OpenPencil editor: ${title}`,
    editorTimeout: 'OpenPencil editor did not become ready',
    editorBusy: 'Another OpenPencil editor still has unsaved changes.',
    discard: 'OpenPencil has unsaved changes. Close and discard them?',
    saveConflict: serverVersion => `OpenPencil save conflict (server v${serverVersion})`,
    syncConflict: serverVersion => `The source changed outside this editor (server v${serverVersion}). Save was stopped.`,
  },
}

/** Chrome copy for the locale already resolved by the DSH host. */
export function editorPanelCopy(locale: EditorLocale): EditorPanelCopy {
  return EDITOR_PANEL_COPY[locale]
}

interface EditorBootOptions {
  signal?: AbortSignal
  fetcher?: typeof fetch
  sessionId?: string
}

interface EditorCloseOptions {
  fetcher?: typeof fetch
  dirty?: boolean
  keepalive?: boolean
}

function launchRequest(fetcher: typeof fetch, url: string, signal?: AbortSignal, sessionId?: string): Promise<Response> {
  return fetcher(editorControlUrl(url), {
    method: 'POST',
    credentials: 'same-origin',
    ...(sessionId === undefined ? {} : {
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ sessionId }),
    }),
    ...(signal === undefined ? {} : { signal }),
  })
}

/**
 * Launch one editor, renewing exactly once when a replayed launch capability
 * has expired. A refreshed capability is never persisted back into the Tool
 * block, and only same-origin control routes can receive document metadata.
 */
export async function launchManagedEditor(
  editor: NonNullable<PresentationGrant['editor']>,
  document: NonNullable<PresentationGrant['document']>,
  options: EditorBootOptions = {},
): Promise<LaunchResponse> {
  const fetcher = options.fetcher ?? fetch
  let launchUrl = editor.launchUrl
  let renewed = false
  let response = await launchRequest(fetcher, launchUrl, options.signal, options.sessionId)
  if (response.status === 410) {
    if (document.path === undefined) {
      throw new Error('OpenPencil editor launch expired and cannot be refreshed without a source path')
    }
    const refreshUrl = editorControlUrl(editor.refreshUrl ?? DEFAULT_REFRESH_URL)
    const refreshResponse = await fetcher(refreshUrl, {
      method: 'POST',
      credentials: 'same-origin',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify({
        launchUrl: editor.launchUrl,
        sourcePath: document.path,
        documentUrl: document.url,
      }),
      ...(options.signal === undefined ? {} : { signal: options.signal }),
    })
    launchUrl = refreshedLaunchUrlOf(await responseJson(refreshResponse, 'OpenPencil editor refresh'))
    renewed = true
    // One bounded retry only. A second 410 is surfaced by responseJson and
    // never loops back through refresh.
    response = await launchRequest(fetcher, launchUrl, options.signal, options.sessionId)
  }
  const launch = launchResponseOf(await responseJson(response, 'OpenPencil editor launch'))
  return renewed ? { ...launch, renewed: true } : launch
}

/** Prefer the daemon's current source; fetch the immutable snapshot only for old hosts. */
export async function prepareManagedEditor(
  editor: NonNullable<PresentationGrant['editor']>,
  document: NonNullable<PresentationGrant['document']>,
  options: EditorBootOptions = {},
): Promise<EditorBootResult> {
  const fetcher = options.fetcher ?? fetch
  const launch = await launchManagedEditor(editor, document, { ...options, fetcher })
  try {
    // The pipeline daemon already owns the authoritative in-memory document.
    // Fetching (and later opening) a publication snapshot would both delay the
    // sidebar and risk replacing edits made after the begin result settled.
    if (launch.liveDraft === true) return { launch, documentJson: '' }
    if (launch.docJson !== undefined) return { launch, documentJson: launch.docJson }
    if (editor.refreshUrl !== undefined || launch.renewed === true) {
      throw new Error('OpenPencil editor launch omitted current docJson')
    }
    const documentResponse = await fetcher(editorControlUrl(document.url), {
      credentials: 'same-origin',
      ...(options.signal === undefined ? {} : { signal: options.signal }),
    })
    if (!documentResponse.ok) throw new Error(`OpenPencil document request failed (${documentResponse.status})`)
    return { launch, documentJson: await documentResponse.text() }
  } catch (error) {
    // The daemon already exists once launchManagedEditor returns. A cancelled
    // fallback document fetch or contract error must not strand that process.
    await closeManagedEditorLaunch(launch, { fetcher, keepalive: true }).catch(() => {})
    throw error
  }
}

/**
 * Complete the bridge-ready edge without confusing a live pipeline daemon
 * with a persisted editor session. Normal sessions receive their exact boot
 * document; live drafts acknowledge attachment through a same-origin host
 * capability and keep the daemon's current document untouched.
 */
export async function completeManagedEditorBridgeReady(
  launch: LaunchResponse,
  documentJson: string,
  post: (message: Parameters<typeof encodeEditorOutbound>[0]) => void,
  options: { fetcher?: typeof fetch; signal?: AbortSignal } = {},
): Promise<'document-open-requested' | 'live-draft-attached'> {
  if (launch.liveDraft !== true) {
    post({ type: 'op-bridge/open-document', json: documentJson })
    return 'document-open-requested'
  }
  if (launch.readyUrl === undefined) throw new Error('OpenPencil live draft launch omitted readyUrl')
  const response = await (options.fetcher ?? fetch)(launch.readyUrl, {
    method: 'POST',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: launch.sessionId }),
    ...(options.signal === undefined ? {} : { signal: options.signal }),
  })
  if (!response.ok) throw new Error(`OpenPencil live draft attach failed (${response.status})`)
  return 'live-draft-attached'
}

/** Close exactly the managed session returned by one launch response. */
export async function closeManagedEditorLaunch(
  launch: LaunchResponse,
  options: EditorCloseOptions = {},
): Promise<void> {
  const fetcher = options.fetcher ?? fetch
  const response = await fetcher(launch.closeUrl, {
    method: 'DELETE',
    credentials: 'same-origin',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ sessionId: launch.sessionId, dirty: options.dirty ?? false }),
    ...(options.keepalive === undefined ? {} : { keepalive: options.keepalive }),
  })
  if (!response.ok) throw new Error(`OpenPencil editor close failed (${response.status})`)
}

/**
 * Mount-aware boot boundary. React may cancel an effect after its launch POST
 * has committed; release that precise returned session before ignoring it.
 */
export async function prepareManagedEditorForMount(
  editor: NonNullable<PresentationGrant['editor']>,
  document: NonNullable<PresentationGrant['document']>,
  accept: () => boolean,
  options: EditorBootOptions = {},
): Promise<EditorBootResult | undefined> {
  const prepared = await prepareManagedEditor(editor, document, options)
  if (accept()) return prepared
  await closeManagedEditorLaunch(prepared.launch, {
    ...(options.fetcher === undefined ? {} : { fetcher: options.fetcher }),
    keepalive: true,
  })
  return undefined
}

const panelStyles: Record<string, React.CSSProperties> = {
  root: {
    height: '100%', minHeight: 0, display: 'flex', flexDirection: 'column',
    color: 'var(--dsw-alias-label-primary)', background: 'var(--dsw-alias-bg-base)',
  },
  toolbar: {
    minHeight: 48, display: 'flex', alignItems: 'center', gap: 8, padding: '8px 10px 8px 12px',
    borderBottom: '1px solid var(--dsw-alias-border-l2)',
  },
  title: { minWidth: 0, marginRight: 'auto', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap', fontSize: 14, lineHeight: '20px', fontWeight: 500 },
  status: { fontSize: 11, color: 'var(--dsw-alias-label-secondary)', whiteSpace: 'nowrap' },
  button: {
    border: '1px solid var(--dsw-alias-border-l2)', borderRadius: 6,
    color: 'var(--dsw-alias-label-primary)', background: 'var(--dsw-alias-bg-layer-1)',
    minHeight: 28, padding: '4px 8px', cursor: 'pointer', fontFamily: 'inherit', fontSize: 12, fontWeight: 'inherit', lineHeight: 1,
  },
  stage: { position: 'relative', flex: 1, minHeight: 0, overflow: 'hidden', background: 'var(--dsw-alias-bg-base)' },
  iframe: { display: 'block', width: '100%', height: '100%', border: 0, background: 'var(--dsw-alias-bg-base)' },
  overlay: {
    position: 'absolute', inset: 0, display: 'flex', alignItems: 'center', justifyContent: 'center',
    flexDirection: 'column', gap: 10, padding: 24, textAlign: 'center',
    color: 'var(--dsw-alias-label-primary)', background: 'var(--dsw-alias-bg-base)', fontSize: 12,
  },
  error: { color: 'var(--dsw-alias-state-error-primary)', maxWidth: 420, overflowWrap: 'anywhere' },
}

export type EditorLifecyclePhase = 'launching' | 'loading' | 'ready' | 'saving' | 'error'

export interface EditorLifecycleState {
  dirty: boolean
  phase: EditorLifecyclePhase
}

export interface EditorLifecycleController {
  /** Close through the guarded server capability while the React owner remains mounted. */
  requestClose: () => Promise<boolean>
  /** Ask the host to persist an opaque, source-scoped daemon recovery snapshot. */
  captureRecovery: () => Promise<boolean>
  /** Join a save the user already started; never starts a new source write. */
  awaitExistingSave: () => Promise<boolean>
  /** HMR-only escape hatch: leave the daemon for the server lifecycle when capture failed. */
  retainServerSessionOnUnmount: () => void
}

export const INITIAL_EDITOR_LIFECYCLE_STATE: EditorLifecycleState = {
  dirty: false,
  phase: 'launching',
}

export type EditorUnmountDisposition = 'closed' | 'retained'

export interface EditorUnmountState {
  /** Explicit retention armed after a failed workbench recovery capture. */
  retainServerSession: boolean
  /** Latest client dirty signal. */
  dirty: boolean
  /** False after a successful explicit close cleared the exact launch. */
  hasLiveLaunch: boolean
  /** Pipeline-owned daemons are only detached by this panel and never retained for recovery. */
  liveDraft?: boolean
}

export interface EditorInitRetryTimer<T> {
  schedule: (callback: () => void, delayMs: number) => T
  cancel: (handle: T) => void
}

export interface EditorInitRetryController {
  /** Stop retrying and disarm the timeout budget. Called once `op-bridge/ready` arrives. */
  stop: () => void
  /**
   * The editor announced its message listener (`op-bridge/listening`): deliver
   * init immediately and stop the periodic retries, because this init is now
   * guaranteed to arrive. The timeout stays armed so a listening editor that
   * never becomes ready still surfaces the bounded timeout error.
   */
  acknowledgeListening: () => void
}

/**
 * Start the managed-editor init handshake immediately, then retry on a bounded
 * interval until the caller stops it after `op-bridge/ready` or the timeout
 * budget is exhausted.
 *
 * This deliberately starts before the iframe `load` event. OpenPencil waits
 * briefly for this token before starting daemon-backed services (including the
 * account-status request), while a browser may delay `load` until module/Wasm
 * startup is already complete. Waiting for `load` can therefore make the first
 * account request run unauthenticated and hide the login button until the
 * editor's later health refresh.
 *
 * postMessage drops messages sent before the iframe registered a listener, so
 * the budget must outlive the editor's first Wasm download (~24 MB on a cold
 * start). Newer editors announce that moment with `op-bridge/listening`;
 * `acknowledgeListening` then delivers init immediately and stops the now
 * redundant periodic retries. Older editors never announce it and simply keep
 * receiving retries until `ready` or the timeout.
 */
export function beginEditorInitRetry<T>(
  send: () => void,
  onExhausted: () => void,
  timer: EditorInitRetryTimer<T>,
  options: { intervalMs?: number; timeoutMs?: number } = {},
): EditorInitRetryController {
  const intervalMs = options.intervalMs ?? 500
  const timeoutMs = options.timeoutMs ?? 60_000
  let intervalHandle: T | undefined
  let intervalGeneration = 0
  let deadlineHandle: T | undefined
  let deadlineGeneration = 0
  let listeningAcknowledged = false
  let stopped = false

  const cancelInterval = (): void => {
    intervalGeneration += 1
    if (intervalHandle !== undefined) {
      timer.cancel(intervalHandle)
      intervalHandle = undefined
    }
  }
  const cancelDeadline = (): void => {
    deadlineGeneration += 1
    if (deadlineHandle !== undefined) {
      timer.cancel(deadlineHandle)
      deadlineHandle = undefined
    }
  }
  const stop = (): void => {
    if (stopped) return
    stopped = true
    cancelInterval()
    cancelDeadline()
  }
  const scheduleNext = (): void => {
    if (stopped) return
    const generation = ++intervalGeneration
    intervalHandle = timer.schedule(() => {
      if (stopped || generation !== intervalGeneration) return
      attempt()
    }, intervalMs)
  }
  const attempt = (): void => {
    if (stopped) return
    send()
    scheduleNext()
  }
  const scheduleDeadline = (): void => {
    if (stopped) return
    const generation = ++deadlineGeneration
    deadlineHandle = timer.schedule(() => {
      if (stopped || generation !== deadlineGeneration) return
      stop()
      onExhausted()
    }, timeoutMs)
  }
  const acknowledgeListening = (): void => {
    if (stopped) return
    // The editor's listener is registered: this init is guaranteed to arrive.
    send()
    if (!listeningAcknowledged) {
      listeningAcknowledged = true
      cancelInterval()
      cancelDeadline()
      scheduleDeadline()
    }
  }

  attempt()
  scheduleDeadline()
  return { stop, acknowledgeListening }
}

export namespace beginEditorInitRetry {
  /** Accept the first ready edge only; duplicate readiness must not reopen a document. */
  export function takeReady(latch: { current: boolean }): boolean {
    if (latch.current) return false
    latch.current = true
    return true
  }
}

/**
 * Apply the unmount policy without letting React cleanup accidentally issue
 * DELETE for a dirty persisted editor. Pipeline-owned live drafts are the
 * inverse: DELETE only detaches this panel, so they are always released.
 */
export function applyManagedEditorUnmountPolicy(
  state: EditorUnmountState,
  closeDaemon: () => void,
): EditorUnmountDisposition {
  if (state.liveDraft === true) {
    closeDaemon()
    return 'closed'
  }
  if (state.retainServerSession || (state.dirty && state.hasLiveLaunch)) return 'retained'
  closeDaemon()
  return 'closed'
}

/** Editable panel. The daemon is created lazily only while this component is mounted. */
export function ManagedOpenPencilEditor({
  grant,
  colorScheme,
  locale,
  sessionId,
  onTakeoverRequest,
  onLifecycleState,
  onLifecycleController,
  workbenchActions,
  allowTakeover = true,
}: {
  grant: PresentationGrant
  colorScheme: EditorColorScheme
  locale: EditorLocale
  sessionId: string
  onTakeoverRequest?: (state: EditorLifecycleState) => boolean
  onLifecycleState?: (state: EditorLifecycleState) => void
  onLifecycleController?: (controller: EditorLifecycleController | undefined) => void
  workbenchActions?: React.ReactNode
  allowTakeover?: boolean
}) {
  const iframeRef = useRef<HTMLIFrameElement>(null)
  const launchRef = useRef<LaunchResponse>()
  const iframeSrcRef = useRef('')
  const originRef = useRef('')
  const docJsonRef = useRef('')
  const colorSchemeRef = useRef(colorScheme)
  colorSchemeRef.current = colorScheme
  const localeRef = useRef(locale)
  localeRef.current = locale
  const takeoverRequestRef = useRef(onTakeoverRequest)
  takeoverRequestRef.current = onTakeoverRequest
  const lifecycleStateRef = useRef(onLifecycleState)
  lifecycleStateRef.current = onLifecycleState
  const lifecycleControllerRef = useRef(onLifecycleController)
  lifecycleControllerRef.current = onLifecycleController
  const stopInitLoopRef = useRef<EditorInitRetryController>()
  const bridgeReadyRef = useRef(false)
  const liveReadyAbortRef = useRef<AbortController>()
  const selectionPollStopRef = useRef<() => void>()
  const requestCounterRef = useRef(0)
  const saveWaitersRef = useRef(new Map<string, { resolve: (message: Extract<EditorInboundMessage, { type: 'op-bridge/snapshot-result' }>) => void; reject: (error: Error) => void }>())
  const [phase, setPhase] = useState<EditorLifecyclePhase>('launching')
  const phaseRef = useRef<EditorLifecyclePhase>('launching')
  const [failure, setFailure] = useState('')
  const [dirty, setDirty] = useState(false)
  const dirtyRef = useRef(false)
  const saveInFlightRef = useRef<Promise<boolean>>()
  const closeDaemonRef = useRef<(dirty?: boolean) => Promise<void>>()
  const retainServerSessionOnUnmountRef = useRef(false)
  const restoredRecoveryRef = useRef(false)
  const documentGrant = grant.document!
  const editorGrant = grant.editor!

  const publishLifecycle = useCallback(() => {
    lifecycleStateRef.current?.({ dirty: dirtyRef.current, phase: phaseRef.current })
  }, [])
  const updatePhase = useCallback((next: EditorLifecyclePhase) => {
    phaseRef.current = next
    setPhase(next)
    publishLifecycle()
  }, [publishLifecycle])
  const updateDirty = useCallback((next: boolean) => {
    dirtyRef.current = next
    setDirty(next)
    publishLifecycle()
  }, [publishLifecycle])

  const post = useCallback((message: Parameters<typeof encodeEditorOutbound>[0]) => {
    const frame = iframeRef.current?.contentWindow
    if (frame === null || frame === undefined || originRef.current === '') return
    frame.postMessage(encodeEditorOutbound(message), originRef.current)
  }, [])

  const save = useCallback(async (): Promise<boolean> => {
    if (saveInFlightRef.current !== undefined) return saveInFlightRef.current
    const launch = launchRef.current
    if (launch === undefined || phaseRef.current === 'launching' || phaseRef.current === 'loading') return false
    if (!managedEditorAllowsSave(launch)) return true
    if (!dirtyRef.current) return true
    const operation = (async (): Promise<boolean> => {
      updatePhase('saving')
      setFailure('')
      const requestId = `dsh-save-${++requestCounterRef.current}`
      let snapshotTimer: ReturnType<typeof setTimeout> | undefined
      try {
        const snapshot = await Promise.race([
          new Promise<Extract<EditorInboundMessage, { type: 'op-bridge/snapshot-result' }>>((resolve, reject) => {
            saveWaitersRef.current.set(requestId, { resolve, reject })
            post({ type: 'op-bridge/snapshot', purpose: 'save', requestId })
          }),
          new Promise<never>((_, reject) => {
            snapshotTimer = setTimeout(() => { reject(new Error('OpenPencil snapshot timed out')) }, 8_000)
          }),
        ])
        const response = await fetch(launch.saveUrl, {
          method: 'POST', credentials: 'same-origin',
          headers: { 'content-type': 'application/json' },
          body: JSON.stringify({
            sessionId: launch.sessionId,
            docJson: snapshot.docJson,
            generation: snapshot.generation,
            revision: snapshot.revision,
          }),
        })
        const saveResponse = await responseJson(response, 'OpenPencil save')
        rememberEditorSuccessor(editorGrant.launchUrl, saveResponse)
        post({ type: 'op-bridge/save-committed', generation: snapshot.generation, revision: snapshot.revision })
        restoredRecoveryRef.current = false
        updateDirty(false)
        updatePhase('ready')
        return true
      } catch (error) {
        setFailure(error instanceof Error ? error.message : String(error))
        updatePhase('error')
        return false
      } finally {
        if (snapshotTimer !== undefined) clearTimeout(snapshotTimer)
        saveWaitersRef.current.delete(requestId)
      }
    })()
    saveInFlightRef.current = operation
    try {
      return await operation
    } finally {
      if (saveInFlightRef.current === operation) saveInFlightRef.current = undefined
    }
  }, [editorGrant.launchUrl, post, updateDirty, updatePhase])

  useEffect(() => {
    publishLifecycle()
    const controller: EditorLifecycleController = {
      requestClose: async () => {
        if (phaseRef.current === 'saving') return false
        const closeDaemon = closeDaemonRef.current
        if (closeDaemon === undefined) return false
        try {
          await closeDaemon(dirtyRef.current)
          return true
        } catch (error) {
          setFailure(error instanceof Error ? error.message : String(error))
          updatePhase('error')
          return false
        }
      },
      captureRecovery: async () => {
        const launch = launchRef.current
        if (!managedEditorAllowsSave(launch) || launch?.recoveryUrl === undefined) return false
        try {
          await captureManagedEditorRecovery(launch)
          return true
        } catch {
          return false
        }
      },
      awaitExistingSave: async () => {
        const pending = saveInFlightRef.current
        return pending === undefined ? true : pending
      },
      retainServerSessionOnUnmount: () => {
        retainServerSessionOnUnmountRef.current = true
      },
    }
    lifecycleControllerRef.current?.(controller)
    return () => { lifecycleControllerRef.current?.(undefined) }
  }, [publishLifecycle, updatePhase])

  useEffect(() => {
    let cancelled = false
    const abort = new AbortController()
    const coordinatorToken = Symbol('openpencil-editor')
    const closeDaemon = async (dirtyAtClose = dirtyRef.current): Promise<void> => {
      selectionPollStopRef.current?.()
      selectionPollStopRef.current = undefined
      clearOpenPencilSelection(sessionId, documentGrant.path)
      const launch = launchRef.current
      if (launch === undefined) return
      await closeManagedEditorLaunch(launch, {
        dirty: managedEditorAllowsSave(launch) ? dirtyAtClose : false,
        keepalive: true,
      })
      if (launchRef.current === launch) launchRef.current = undefined
    }
    closeDaemonRef.current = closeDaemon
    const releaseEditor = claimEditor(coordinatorToken, () => {
      const takeoverRequest = takeoverRequestRef.current
      const lifecycle = { dirty: dirtyRef.current, phase: phaseRef.current }
      if (takeoverRequest !== undefined && !takeoverRequest(lifecycle)) return false
      if (phaseRef.current === 'saving') return false
      if (takeoverRequest === undefined && !confirmEditorClose(
        dirtyRef.current,
        () => window.confirm(editorPanelCopy(localeRef.current).discard),
      )) return false
      abort.abort()
      void closeDaemon(dirtyRef.current)
      return true
    }, { replace: allowTakeover })
    if (releaseEditor === undefined) {
      setFailure(editorPanelCopy(localeRef.current).editorBusy)
      updatePhase('error')
      return () => { abort.abort() }
    }
    const boot = async (): Promise<void> => {
      try {
        bridgeReadyRef.current = false
        const bootGrant = editorGrantForBoot(editorGrant)
        const prepared = await prepareManagedEditorForMount(bootGrant, documentGrant, () => (
          !cancelled && !abort.signal.aborted
        ), {
          sessionId,
        })
        if (prepared === undefined) return
        const { launch } = prepared
        // Publish immediately: recovery restore performs another await and a
        // failed restore must remain reachable by cleanup instead of leaking
        // the already-created daemon.
        launchRef.current = launch
        let { documentJson } = prepared
        if (managedEditorAllowsSave(launch) && launch.recovery !== undefined) {
          const recoveryCopy = editorRecoveryCopy(localeRef.current)
          const message = launch.recovery.sourceChangedSinceCapture
            ? recoveryCopy.conflict(launch.recovery.sourceName)
            : recoveryCopy.available(launch.recovery.sourceName)
          if (window.confirm(message)) {
            documentJson = await restoreManagedEditorRecovery(launch, launch.recovery)
            restoredRecoveryRef.current = true
          }
        }
        const origin = editorOrigin(launch.iframeUrl)
        // Capture first-navigation host presentation exactly once. Later host
        // theme/locale changes travel over the bridge and never reload the iframe.
        iframeSrcRef.current = editorIframeUrlWithLocale(
          editorIframeUrlWithTheme(launch.iframeUrl, colorSchemeRef.current),
          localeRef.current,
        )
        docJsonRef.current = documentJson
        originRef.current = origin
        updatePhase('loading')
      } catch (error) {
        // A recovery request may fail after launch has already created a
        // daemon. Close that exact session now; waiting for component unmount
        // would strand it behind an error surface.
        await closeDaemon(dirtyRef.current).catch(() => {})
        if (cancelled || abort.signal.aborted) return
        setFailure(error instanceof Error ? error.message : String(error))
        updatePhase('error')
      }
    }
    void boot()
    return () => {
      cancelled = true
      abort.abort()
      releaseEditor()
      stopInitLoopRef.current?.stop()
      stopInitLoopRef.current = undefined
      liveReadyAbortRef.current?.abort()
      liveReadyAbortRef.current = undefined
      const disposed = new Error('OpenPencil editor closed')
      for (const waiter of saveWaitersRef.current.values()) waiter.reject(disposed)
      saveWaitersRef.current.clear()
      if (closeDaemonRef.current === closeDaemon) closeDaemonRef.current = undefined
      const disposition = applyManagedEditorUnmountPolicy({
        retainServerSession: retainServerSessionOnUnmountRef.current,
        dirty: dirtyRef.current,
        hasLiveLaunch: launchRef.current !== undefined,
        liveDraft: launchRef.current?.liveDraft === true,
      }, () => {
        void closeDaemon().catch(() => {})
      })
      if (disposition === 'retained') {
        selectionPollStopRef.current?.()
        selectionPollStopRef.current = undefined
        clearOpenPencilSelection(sessionId, documentGrant.path)
      }
    }
  }, [allowTakeover, documentGrant.path, documentGrant.url, editorGrant.launchUrl, editorGrant.refreshUrl, updatePhase])

  const startInitLoop = useCallback((): void => {
    const launch = launchRef.current
    if (launch === undefined || bridgeReadyRef.current) return
    stopInitLoopRef.current?.stop()
    stopInitLoopRef.current = beginEditorInitRetry(
      () => {
        post({ type: 'op-bridge/init', token: launch.token })
        post({ type: 'op-bridge/theme', colorScheme: colorSchemeRef.current })
        post({ type: 'op-bridge/locale', locale: localeRef.current })
      },
      () => {
        stopInitLoopRef.current = undefined
        setFailure(editorPanelCopy(localeRef.current).editorTimeout)
        updatePhase('error')
      },
      {
        schedule: (callback, delayMs) => window.setTimeout(callback, delayMs),
        cancel: handle => { window.clearTimeout(handle) },
      },
    )
  }, [post, updatePhase])

  // Begin posting as soon as React commits the iframe. Tying this to `load`
  // is too late for OpenPencil's managed bootstrap: its Wasm shell may have
  // already fallen back to an unauthenticated first status request by then.
  useEffect(() => {
    if (phase !== 'loading' || launchRef.current === undefined || iframeRef.current === null) return
    startInitLoop()
    return () => {
      stopInitLoopRef.current?.stop()
      stopInitLoopRef.current = undefined
    }
  }, [phase, startInitLoop])

  useEffect(() => {
    if (phase !== 'ready' && phase !== 'saving') return
    const selectionUrl = launchRef.current?.selectionUrl
    if (selectionUrl === undefined) return
    const stop = startEditorSelectionPolling({
      url: selectionUrl,
      onValue: value => {
        if (!isRecord(value)) return
        const selection = liveSelectionOf(value.selection)
        if (selection !== undefined) publishOpenPencilSelection(sessionId, selection)
      },
      onStop: () => { clearOpenPencilSelection(sessionId, documentGrant.path) },
    })
    selectionPollStopRef.current = stop
    return () => {
      if (selectionPollStopRef.current === stop) selectionPollStopRef.current = undefined
      stop()
    }
  }, [documentGrant.path, phase, sessionId])

  useEffect(() => {
    const listener = (event: MessageEvent): void => {
      const message = editorMessageFrom(event, iframeRef.current?.contentWindow ?? null, originRef.current)
      if (message === undefined) return
      switch (message.type) {
        case 'op-bridge/listening':
          // The page installs this early listener only after the main Wasm
          // module has instantiated. If an exceptionally slow load already
          // exhausted the budget, recover with one fresh controller; then the
          // listening acknowledgement sends init into the live listener and
          // stops periodic retries while retaining a bounded ready deadline.
          if (bridgeReadyRef.current || launchRef.current === undefined) break
          if (phaseRef.current === 'error' || stopInitLoopRef.current === undefined) {
            setFailure('')
            updatePhase('loading')
            startInitLoop()
          }
          stopInitLoopRef.current?.acknowledgeListening()
          break
        case 'op-bridge/ready':
          // A rebuilt iframe may announce readiness more than once. Opening
          // the authoritative boot document twice could overwrite live edits.
          if (!beginEditorInitRetry.takeReady(bridgeReadyRef)) break
          stopInitLoopRef.current?.stop()
          stopInitLoopRef.current = undefined
          post({ type: 'op-bridge/theme', colorScheme: colorSchemeRef.current })
          post({ type: 'op-bridge/locale', locale: localeRef.current })
          {
            const launch = launchRef.current
            if (launch === undefined) break
            const controller = new AbortController()
            liveReadyAbortRef.current?.abort()
            liveReadyAbortRef.current = controller
            void completeManagedEditorBridgeReady(launch, docJsonRef.current, post, {
              signal: controller.signal,
            }).then(result => {
              if (controller.signal.aborted) return
              if (liveReadyAbortRef.current === controller) liveReadyAbortRef.current = undefined
              if (result === 'live-draft-attached') updatePhase('ready')
            }).catch(error => {
              if (controller.signal.aborted) return
              if (liveReadyAbortRef.current === controller) liveReadyAbortRef.current = undefined
              setFailure(error instanceof Error ? error.message : String(error))
              updatePhase('error')
            })
          }
          break
        case 'op-bridge/opened':
          if (launchRef.current?.liveDraft === true) break
          updatePhase('ready')
          if (restoredRecoveryRef.current) updateDirty(true)
          break
        case 'op-bridge/dirty-changed':
          if (managedEditorAllowsSave(launchRef.current)) {
            updateDirty(restoredRecoveryRef.current || message.dirty)
          }
          break
        case 'op-bridge/snapshot-result':
          saveWaitersRef.current.get(message.requestId)?.resolve(message)
          break
        case 'op-bridge/snapshot-conflict':
          saveWaitersRef.current.get(message.requestId)?.reject(
            new Error(editorPanelCopy(localeRef.current).saveConflict(message.serverVersion)),
          )
          break
        case 'op-bridge/sync-conflict':
          setFailure(editorPanelCopy(localeRef.current).syncConflict(message.serverVersion))
          updatePhase('error')
          break
        case 'op-shell/save':
          if (managedEditorAllowsSave(launchRef.current)) void save()
          break
        case 'op-shell/copy':
          void navigator.clipboard?.writeText(message.text).catch(() => {})
          break
        case 'op-bridge/conflict-resolved':
          break
      }
    }
    window.addEventListener('message', listener)
    return () => { window.removeEventListener('message', listener) }
  }, [post, save, startInitLoop, updateDirty, updatePhase])

  useEffect(() => {
    post({ type: 'op-bridge/theme', colorScheme })
  }, [colorScheme, post])

  useEffect(() => {
    post({ type: 'op-bridge/locale', locale })
  }, [locale, post])

  useEffect(() => {
    const beforeUnload = (event: BeforeUnloadEvent): void => {
      if (!dirtyRef.current) return
      event.preventDefault()
      event.returnValue = ''
    }
    window.addEventListener('beforeunload', beforeUnload)
    return () => { window.removeEventListener('beforeunload', beforeUnload) }
  }, [])

  const title = documentGrant.path?.replaceAll('\\', '/').split('/').at(-1) ?? 'OpenPencil'
  const copy = editorPanelCopy(locale)
  const liveDraft = launchRef.current?.liveDraft === true
  const saveEnabled = launchRef.current !== undefined && managedEditorAllowsSave(launchRef.current)
  return (
    <section
      style={panelStyles.root}
      data-tool-details-fill="true"
      data-tool-details-dirty={dirty || undefined}
      data-openpencil-editor-panel="true"
      data-openpencil-live-draft={liveDraft || undefined}
      aria-busy={phase === 'launching' || phase === 'loading' || phase === 'saving' || undefined}
    >
      <div style={panelStyles.toolbar}>
        <strong style={panelStyles.title} title={documentGrant.path}>{title}</strong>
        {saveEnabled ? <span style={panelStyles.status}>{phase === 'saving' ? copy.saving : dirty ? copy.unsaved : phase === 'ready' ? copy.saved : ''}</span> : null}
        {saveEnabled ? <button type="button" style={panelStyles.button} disabled={!dirty || phase === 'saving'} onClick={() => { void save() }}>{copy.save}</button> : null}
        {workbenchActions}
      </div>
      <div style={panelStyles.stage}>
        {launchRef.current !== undefined ? (
          <iframe
            ref={iframeRef}
            style={liveDraft ? { ...panelStyles.iframe, pointerEvents: 'none' } : panelStyles.iframe}
            src={iframeSrcRef.current}
            title={copy.editorTitle(title)}
            allow="clipboard-read; clipboard-write; local-fonts"
            aria-disabled={liveDraft || undefined}
            tabIndex={!liveDraft && (phase === 'ready' || phase === 'saving') ? 0 : -1}
          />
        ) : null}
        {phase === 'launching' || phase === 'loading' ? <div style={panelStyles.overlay} role="status">{copy.loading}</div> : null}
        {phase === 'error' ? (
          <div style={panelStyles.overlay} role="alert">
            <strong>{copy.errorTitle}</strong>
            <span style={panelStyles.error}>{failure}</span>
            {grant.image !== undefined ? <a href={grant.image.previewUrl} target="_blank" rel="noreferrer">{copy.pngFallback}</a> : null}
          </div>
        ) : null}
      </div>
    </section>
  )
}
