/**
 * Browser presentation for OpenPencil render/new/pipeline cards and historical
 * `design_render` conversation cards.
 *
 * PNG remains the replay-safe default. When the host also grants access to
 * the source `.op`, the user can opt into one shared, read-only Web SDK
 * canvas. The SDK and document are fetched only after that explicit action.
 */

import {
  useCallback,
  useEffect,
  useRef,
  useState,
  useSyncExternalStore,
} from 'react'
import type { PointerEvent as ReactPointerEvent } from 'react'
import type { ToolCallViewProps } from '@deepseek-ai/dsh-client-ui-tool/client'
// DSH 0.1.5 removed @deepseek-ai/dsh-client-runtime; the client context is
// just the cordis Context. The empty type imports below exist for their module
// augmentations — dsh-client-ui-renderer declares ctx.slots and
// dsh-client-ui-session declares the session props the dock reads.
import type { Context as ClientContext } from '@deepseek-ai/cordis'
import type {} from '@deepseek-ai/dsh-client-ui-renderer/client'
import type {} from '@deepseek-ai/dsh-client-ui-session/client'
import type {} from '@deepseek-ai/dsh-client-locale/client'
import type {} from '@deepseek-ai/dsh-client-ui-conversation/client'
import type {} from '@deepseek-ai/dsh-client-ui-theme/client'
import { editorPanelCopy, ManagedOpenPencilEditor } from './editor-panel.js'
import type { EditorColorScheme, EditorLocale } from './editor-bridge.js'
import {
  requestOpenPencilEditor,
  type CompatibleToolCallViewProps,
  type CompatibleToolDetailsViewProps,
} from './details-compat.js'
import type { EditorWorkbenchRequest } from './editor-workbench-host.js'
import { mountEditorWorkbenchHost } from './editor-workbench-host.js'
import { editorWorkbenchEditorKey } from './editor-modal.js'
import { FrameGallery, normalizeFrameIndex as normalizedFrameIndex } from './frame-gallery.js'
import type { GalleryFrame, GalleryLocale } from './frame-gallery.js'
import { OpenPencilSelectionDock } from './selection-dock.js'
import {
  canonicalResultObjectFromText,
  presentationHydrationRequestOf,
  requestPresentationGrantWithRetry,
} from './presentation-hydration.js'
import {
  LEGACY_DESIGN_RENDER_TOOL_NAME,
  OPENPENCIL_NEW_TOOL_NAME,
  OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
  OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
  OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
  OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
  OPENPENCIL_RENDER_TOOL_NAME,
} from '../tool-names.js'

export {
  LEGACY_DESIGN_RENDER_TOOL_NAME,
  OPENPENCIL_NEW_TOOL_NAME,
  OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
  OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
  OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
  OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
  OPENPENCIL_RENDER_TOOL_NAME,
} from '../tool-names.js'

export {
  calculateGalleryFitViewZoom,
  clampGalleryZoom,
  frameLabel,
  frameGalleryCopy,
  galleryZoomCommandTarget,
  galleryViewportMaxHeight,
  galleryZoomPercent,
  galleryZoomShortcut,
  GALLERY_COMPACT_MAX_HEIGHT,
  GALLERY_TOOLBAR_CONTROL_CONTENT_LAYOUT,
  GALLERY_TOOLBAR_CONTROL_HEIGHT,
  GALLERY_TOOLBAR_CONTROL_LAYOUT,
  GALLERY_ZOOM_MAX,
  GALLERY_ZOOM_MIN,
  GALLERY_ZOOM_STEP,
  nextGalleryZoom,
  normalizeFrameIndex,
} from './frame-gallery.js'
export {
  applyManagedEditorUnmountPolicy,
  beginEditorInitRetry,
  closeManagedEditorLaunch,
  completeManagedEditorBridgeReady,
  editorPanelCopy,
  launchManagedEditor,
  managedEditorAllowsSave,
  prepareManagedEditor,
  prepareManagedEditorForMount,
} from './editor-panel.js'
export {
  requestOpenPencilEditor,
} from './details-compat.js'
export {
  clampEditorWorkbenchWidth,
  confirmEditorModalClose,
  editorModalCopy,
  editorWorkbenchEditorKey,
  editorWorkbenchFocusTargetIndex,
  editorWorkbenchShouldHandleEscape,
  editorWorkbenchUsesFullscreen,
  editorWorkbenchWidthBounds,
  EDITOR_WORKBENCH_CLOSE_BUTTON_MIN_WIDTH,
  EDITOR_WORKBENCH_FULLSCREEN_BREAKPOINT,
  EDITOR_WORKBENCH_LEFT_CLEARANCE,
  EDITOR_WORKBENCH_MAX_WIDTH,
  EDITOR_WORKBENCH_MIN_WIDTH,
  EDITOR_WORKBENCH_RESIZE_STEP,
  OPENPENCIL_EDITOR_CLOSE_BUTTON_ATTRIBUTE,
  resizedEditorWorkbenchWidth,
} from './editor-modal.js'
export {
  claimEditorWorkbenchDock,
  OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE,
} from './editor-dock-layout.js'
export {
  automaticEditorWorkbenchLayout,
  closeLiveDraftForPublishedSuccessor,
  createEditorWorkbenchStore,
  isPublishedSuccessorOfLiveDraft,
  mountEditorWorkbenchHost,
  preserveEditorBeforeWorkbenchDispose,
} from './editor-workbench-host.js'
export {
  editorGrantForBoot,
  editorSuccessorFromSave,
  editorSuccessorStorageKey,
  rememberEditorSuccessor,
} from './editor-successor.js'
export {
  captureManagedEditorRecovery,
  discardManagedEditorRecovery,
  editorRecoveryCopy,
  editorRecoveryItemUrl,
  editorRecoverySummaryOf,
  restoreManagedEditorRecovery,
} from './editor-recovery.js'

export {
  claimEditor,
  confirmEditorClose,
  editorControlUrl,
  editorIframeUrlWithLocale,
  editorIframeUrlWithTheme,
  editorLocaleFromDsh,
  editorMessageFrom,
  editorOrigin,
  encodeEditorOutbound,
  parseEditorInbound,
} from './editor-bridge.js'
export {
  clearOpenPencilSelection,
  getOpenPencilSelectionSnapshot,
  liveSelectionOf,
  publishOpenPencilSelection,
  subscribeOpenPencilSelection,
} from './selection-store.js'
export {
  isTerminalEditorSelectionStatus,
  startEditorSelectionPolling,
} from './selection-polling.js'
export {
  hasOpenPencilSelection,
  OPENPENCIL_SELECTION_DOCK_LAYOUT,
  selectionNodeDetail,
  selectionNodeLabel,
} from './selection-dock.js'
export {
  documentSha256FromCanonicalResult,
  PRESENTATION_HYDRATION_ENDPOINT,
  presentationHydrationRequestOf,
  requestPresentationGrant,
  requestPresentationGrantWithRetry,
} from './presentation-hydration.js'

/** Presentation metadata key the host half projects into `block.meta`. */
export const PRESENTATION_META_KEY = '$dshOpenPencil'

const LIVE_AUTO_OPEN_TTL_MS = 15 * 60 * 1000
const LIVE_AUTO_OPEN_MAX = 256
const liveAutoOpenActivatedAt = Date.now()
interface LiveAutoOpenRecord {
  state: 'armed' | 'opening' | 'accepted'
  expiresAt: number
}

const liveAutoOpenCalls = new Map<string, LiveAutoOpenRecord>()

function liveAutoOpenKey(sessionId: string, callId: string): string {
  return `${sessionId.length}:${sessionId}${callId}`
}

function pruneLiveAutoOpenCalls(now = Date.now()): void {
  for (const [key, record] of liveAutoOpenCalls) {
    if (record.expiresAt <= now) liveAutoOpenCalls.delete(key)
  }
  while (liveAutoOpenCalls.size > LIVE_AUTO_OPEN_MAX) {
    const oldest = liveAutoOpenCalls.keys().next().value as string | undefined
    if (oldest === undefined) break
    liveAutoOpenCalls.delete(oldest)
  }
}

export function rememberLiveAutoOpenCall(key: string): void {
  pruneLiveAutoOpenCalls()
  const existing = liveAutoOpenCalls.get(key)
  if (existing?.state === 'opening' || existing?.state === 'accepted') return
  liveAutoOpenCalls.delete(key)
  liveAutoOpenCalls.set(key, { state: 'armed', expiresAt: Date.now() + LIVE_AUTO_OPEN_TTL_MS })
  pruneLiveAutoOpenCalls()
}

/**
 * Legacy synchronous acceptance used by pure callers. The automatic React
 * path below waits for the host's actual answer before accepting the call.
 */
export function takeLiveAutoOpenCall(key: string): boolean {
  pruneLiveAutoOpenCalls()
  const record = liveAutoOpenCalls.get(key)
  if (record?.state !== 'armed') return false
  liveAutoOpenCalls.delete(key)
  liveAutoOpenCalls.set(key, { state: 'accepted', expiresAt: Date.now() + LIVE_AUTO_OPEN_TTL_MS })
  return true
}

/**
 * Attempt one live auto-open without consuming it until the host accepts.
 * A busy/unmounted host or a rejected promise re-arms the call for a later
 * card projection, while `opening` prevents duplicate concurrent requests.
 */
export async function attemptLiveAutoOpenCall(
  key: string,
  open: () => boolean | Promise<boolean>,
): Promise<boolean> {
  pruneLiveAutoOpenCalls()
  const record = liveAutoOpenCalls.get(key)
  if (record?.state !== 'armed') return false
  liveAutoOpenCalls.delete(key)
  liveAutoOpenCalls.set(key, { state: 'opening', expiresAt: Date.now() + LIVE_AUTO_OPEN_TTL_MS })

  let accepted = false
  try {
    accepted = await open() === true
  } catch {
    accepted = false
  }

  const current = liveAutoOpenCalls.get(key)
  if (current?.state === 'opening') {
    liveAutoOpenCalls.delete(key)
    liveAutoOpenCalls.set(key, {
      state: accepted ? 'accepted' : 'armed',
      expiresAt: Date.now() + LIVE_AUTO_OPEN_TTL_MS,
    })
  }
  return accepted
}

function forgetLiveAutoOpenCall(key: string): void {
  liveAutoOpenCalls.delete(key)
}

export type PresentationLocale = GalleryLocale

const DESIGN_RENDER_COPY = {
  en: {
    designRender: 'OpenPencil render',
    designNew: 'OpenPencil design',
    error: 'error',
    rendering: 'rendering…',
    done: 'done',
    renderingDocument: 'Rendering the design document…',
    creatingDocument: 'Creating the design document…',
    renderFailed: 'The render failed.',
    frames: 'frames',
    openInteractiveCanvas: 'Open interactive canvas',
    editCanvas: 'Edit canvas',
    editInSidebar: 'Edit in sidebar',
    openRenderedPng: 'Open rendered PNG',
    downloadPng: 'Download PNG',
    editSource: 'Edit source .op',
    downloadSource: 'Download source .op',
    inspectToolCall: 'Inspect tool call',
    recoveringPreview: 'Recovering the OpenPencil preview…',
    pendingPreview: 'This pipeline stage is waiting for its final preview.',
    noPreview: 'This result does not include a preview artifact.',
    canvas: 'OpenPencil canvas',
    zoomOut: 'Zoom out',
    zoomIn: 'Zoom in',
    fit: 'Fit',
    close: 'Close',
    readonlyCanvas: 'Read-only OpenPencil design canvas',
    loadingCanvas: 'Loading interactive canvas…',
    pngRemains: 'PNG preview remains available underneath the dialog.',
    canvasUnavailable: 'Interactive canvas unavailable',
    openPngFallback: 'Open PNG fallback',
    panHint: 'Drag to pan · scroll to pan · Ctrl/⌘ + scroll to zoom',
    snapshot: 'snapshot',
    editorUnavailable: 'Editable OpenPencil canvas is not available for this result.',
  },
  zh: {
    designRender: 'OpenPencil 渲染',
    designNew: 'OpenPencil 设计',
    error: '错误',
    rendering: '渲染中…',
    done: '完成',
    renderingDocument: '正在渲染设计文档…',
    creatingDocument: '正在创建设计文档…',
    renderFailed: '渲染失败。',
    frames: '页',
    openInteractiveCanvas: '打开交互画布',
    editCanvas: '编辑画布',
    editInSidebar: '在侧边栏编辑',
    openRenderedPng: '打开渲染 PNG',
    downloadPng: '下载 PNG',
    editSource: '编辑源文件 .op',
    downloadSource: '下载源文件 .op',
    inspectToolCall: '检查工具调用',
    recoveringPreview: '正在恢复 OpenPencil 预览…',
    pendingPreview: '当前流水线阶段正在等待最终预览。',
    noPreview: '此结果未包含可显示的预览产物。',
    canvas: 'OpenPencil 画布',
    zoomOut: '缩小',
    zoomIn: '放大',
    fit: '适应窗口',
    close: '关闭',
    readonlyCanvas: '只读 OpenPencil 设计画布',
    loadingCanvas: '正在加载交互画布…',
    pngRemains: '对话框下方仍保留 PNG 预览。',
    canvasUnavailable: '交互画布不可用',
    openPngFallback: '打开 PNG 预览',
    panHint: '拖动平移 · 滚动平移 · Ctrl/⌘ + 滚动缩放',
    snapshot: '快照',
    editorUnavailable: '此渲染结果没有可用的 OpenPencil 编辑画布。',
  },
} as const

export function designRenderCopy(locale: PresentationLocale) {
  return DESIGN_RENDER_COPY[locale]
}

/** Keep canonical create cards visually distinct while reusing one workbench. */
export function openPencilPresentationTitle(toolName: string, locale: PresentationLocale): string {
  const copy = designRenderCopy(locale)
  return toolName === OPENPENCIL_NEW_TOOL_NAME
    || toolName === OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME
    || toolName === OPENPENCIL_PIPELINE_FINISH_TOOL_NAME
    ? copy.designNew
    : copy.designRender
}

/**
 * Arm live auto-open even when a very fast tool settles before its running
 * card is committed. Replayed calls predate this client-bundle activation.
 */
export function shouldArmLiveAutoOpen(
  error: boolean,
  blockTime: number,
  activatedAt = liveAutoOpenActivatedAt,
): boolean {
  return !error && Number.isFinite(blockTime) && blockTime >= activatedAt
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

export interface ImageGrant extends GalleryFrame {}

export interface DocumentGrant {
  path?: string
  url: string
  downloadUrl?: string
  bytes?: number
  sha256?: string
  mimeType?: string
}

export interface ViewerGrant {
  sdkUrl: string
  wasmUrl: string
  canvasKitBaseUrl: string
}

export interface EditorGrant {
  enabled: true
  launchUrl: string
  refreshUrl?: string
}

export interface PresentationGrant {
  schemaVersion: 1 | 2
  image?: ImageGrant
  frames?: ImageGrant[]
  document?: DocumentGrant
  viewer?: ViewerGrant
  editor?: EditorGrant
  renderer?: string
  rendererBinary?: string
  fidelity?: string
  warnings?: string[]
  autoOpenEditor?: boolean
  draftId?: string
  liveDraft?: boolean
}

function optionalString(record: Record<string, unknown>, key: string): string | undefined {
  const value = record[key]
  return typeof value === 'string' && value.length > 0 ? value : undefined
}

function optionalFiniteNumber(record: Record<string, unknown>, key: string): number | undefined {
  const value = record[key]
  return typeof value === 'number' && Number.isFinite(value) ? value : undefined
}

function optionalStrings(record: Record<string, unknown>, key: string): string[] | undefined {
  const value = record[key]
  if (!Array.isArray(value)) return undefined
  const strings = value.filter((item): item is string => typeof item === 'string' && item.length > 0)
  return strings.length === 0 ? undefined : strings
}

function imageGrantOf(value: unknown): ImageGrant | undefined {
  if (!isRecord(value)) return undefined
  const path = optionalString(value, 'path')
  const previewUrl = optionalString(value, 'previewUrl')
  const downloadUrl = optionalString(value, 'downloadUrl')
  if (path === undefined || previewUrl === undefined || downloadUrl === undefined) return undefined
  const id = optionalString(value, 'id')
  const name = optionalString(value, 'name')
  const index = optionalFiniteNumber(value, 'index')
  return {
    path,
    previewUrl,
    downloadUrl,
    width: optionalFiniteNumber(value, 'width'),
    height: optionalFiniteNumber(value, 'height'),
    ...(id === undefined ? {} : { id }),
    ...(name === undefined ? {} : { name }),
    ...(index === undefined || !Number.isSafeInteger(index) || index < 0 ? {} : { index }),
  }
}

function imageGrantsOf(value: unknown): ImageGrant[] | undefined {
  if (!Array.isArray(value)) return undefined
  const frames = value.map(imageGrantOf).filter((frame): frame is ImageGrant => frame !== undefined)
  return frames.length === 0 ? undefined : frames
}

function documentGrantOf(envelope: Record<string, unknown>, image: unknown): DocumentGrant | undefined {
  const raw = isRecord(envelope.document) ? envelope.document : undefined
  const legacyImage = isRecord(image) ? image : undefined
  const url = raw === undefined
    ? optionalString(envelope, 'documentUrl')
      ?? optionalString(envelope, 'sourceUrl')
      ?? (legacyImage === undefined ? undefined : optionalString(legacyImage, 'documentUrl')
        ?? optionalString(legacyImage, 'sourceUrl')
        ?? optionalString(legacyImage, 'opUrl'))
    : optionalString(raw, 'url') ?? optionalString(raw, 'documentUrl')
  if (url === undefined) return undefined
  return {
    url,
    path: raw === undefined ? optionalString(envelope, 'sourcePath') : optionalString(raw, 'path'),
    downloadUrl: raw === undefined
      ? optionalString(envelope, 'documentDownloadUrl')
      : optionalString(raw, 'downloadUrl'),
    bytes: raw === undefined ? undefined : optionalFiniteNumber(raw, 'bytes'),
    sha256: raw === undefined ? undefined : optionalString(raw, 'sha256'),
    mimeType: raw === undefined ? undefined : optionalString(raw, 'mimeType'),
  }
}

function viewerGrantOf(value: unknown): ViewerGrant | undefined {
  if (!isRecord(value)) return undefined
  const sdkUrl = optionalString(value, 'sdkUrl')
  const wasmUrl = optionalString(value, 'wasmUrl')
  const canvasKitBaseUrl = optionalString(value, 'canvasKitBaseUrl')
    ?? optionalString(value, 'assetBaseUrl')
  if (sdkUrl === undefined || wasmUrl === undefined || canvasKitBaseUrl === undefined) return undefined
  return { sdkUrl, wasmUrl, canvasKitBaseUrl }
}

function editorGrantOf(value: unknown): EditorGrant | undefined {
  if (!isRecord(value) || value.enabled !== true) return undefined
  const launchUrl = optionalString(value, 'launchUrl')
  if (launchUrl === undefined) return undefined
  const refreshUrl = optionalString(value, 'refreshUrl')
  return { enabled: true, launchUrl, ...(refreshUrl === undefined ? {} : { refreshUrl }) }
}

/** Parse both the established v1 envelope and the additive v2 shape. */
export function presentationGrantOfMeta(metaValue: unknown): PresentationGrant | undefined {
  const meta = isRecord(metaValue) ? metaValue : undefined
  const envelope = meta?.[PRESENTATION_META_KEY]
  if (!isRecord(envelope) || (envelope.schemaVersion !== 1 && envelope.schemaVersion !== 2)) return undefined
  const frames = imageGrantsOf(envelope.frames)
  const image = imageGrantOf(envelope.image) ?? frames?.[0]
  const document = documentGrantOf(envelope, envelope.image)
  if (image === undefined && document === undefined) return undefined
  const draftId = optionalString(envelope, 'draftId')
  const liveDraft = typeof envelope.liveDraft === 'boolean' ? envelope.liveDraft : undefined
  return {
    schemaVersion: envelope.schemaVersion,
    image,
    frames: frames ?? (image === undefined ? undefined : [image]),
    document,
    viewer: viewerGrantOf(envelope.viewer),
    editor: editorGrantOf(envelope.editor),
    renderer: optionalString(envelope, 'renderer'),
    rendererBinary: optionalString(envelope, 'rendererBinary'),
    fidelity: optionalString(envelope, 'fidelity'),
    warnings: optionalStrings(envelope, 'warnings'),
    ...(envelope.autoOpenEditor === true ? { autoOpenEditor: true } : {}),
    ...(draftId === undefined || !/^[A-Za-z0-9_-]{32}$/.test(draftId) ? {} : { draftId }),
    ...(liveDraft === undefined ? {} : { liveDraft }),
  }
}

export function grantOf(block: ToolCallViewProps['block']): PresentationGrant | undefined {
  if (!('kind' in block) || block.isError) return undefined
  return presentationGrantOfMeta(block.meta)
}

/** Flatten the durable result text for the fallback disclosure. */
function resultText(block: ToolCallViewProps['block']): string | null {
  if (!('kind' in block)) return null
  const parts: string[] = []
  for (const item of block.content) {
    parts.push(item.type === 'text' ? item.text : JSON.stringify(item, null, 2))
  }
  if (parts.length === 0 && block.error !== undefined) {
    parts.push(`${block.error.name}: ${block.error.code}`)
  }
  return parts.join('\n') || null
}

/** Distinguish an intentional two-phase pipeline wait from a missing artifact. */
export function pipelineResultAwaitsPreview(toolName: string, text: string | null): boolean {
  if (toolName !== OPENPENCIL_PIPELINE_FINISH_TOOL_NAME || text === null || text.length > 1024 * 1024) return false
  const result = canonicalResultObjectFromText(text)
  return result !== undefined
    && result.published === false
    && (
      result.stage === 'needs_preview'
      // Preserve presentation recovery for pre-upgrade session history.
      || result.stage === 'needs_visual_preview'
      || result.stage === 'needs_visual_inspection'
    )
}

/** Hide settled successes that have no preview, editor, or pending hydration. */
export function shouldHidePresentationCard(state: {
  settled: boolean
  error: boolean
  hasGrant: boolean
  hydrationPending: boolean
  awaitingPreview: boolean
}): boolean {
  return state.settled
    && !state.error
    && !state.hasGrant
    && !state.hydrationPending
}

/** A preview-retry state has its own concise status and never dumps JSON as a fake preview. */
export function fallbackPresentationText(text: string | null, awaitingPreview: boolean): string | null {
  return awaitingPreview ? null : text
}

interface Viewport {
  panX: number
  panY: number
  zoom: number
}

interface OpViewer {
  readonly viewport: Viewport
  setZoom(zoom: number): void
  panTo(panX: number, panY: number): void
  zoomToFit(width: number, height: number): void
  on(event: 'viewportchange', callback: () => void): () => void
  destroy(): void
}

interface OpenPencilSdk {
  createViewer(options: {
    canvas: HTMLCanvasElement
    doc: string | Uint8Array
    wasmUrl?: string
    canvasKitBaseUrl?: string
  }): Promise<OpViewer>
}

const sdkLoads = new Map<string, Promise<OpenPencilSdk>>()

/** Load the host-served ESM core SDK without coupling the client bundle to React 19. */
export function loadOpenPencilSdk(url: string): Promise<OpenPencilSdk> {
  const absoluteUrl = new URL(url, window.location.href).href
  let pending = sdkLoads.get(absoluteUrl)
  if (pending === undefined) {
    pending = import(/* @vite-ignore */ absoluteUrl).then((module: unknown) => {
      if (!isRecord(module) || typeof module.createViewer !== 'function') {
        throw new Error('OpenPencil viewer SDK did not export createViewer')
      }
      return module as unknown as OpenPencilSdk
    })
    sdkLoads.set(absoluteUrl, pending)
    pending.catch(() => { sdkLoads.delete(absoluteUrl) })
  }
  return pending
}

interface ActiveCanvas {
  token: symbol
  close: () => void
}

let activeCanvas: ActiveCanvas | undefined

/** @internal Claim the page-wide SDK singleton; opening another canvas closes this one. */
export function claimCanvas(token: symbol, close: () => void): () => void {
  const previous = activeCanvas
  activeCanvas = { token, close }
  if (previous !== undefined && previous.token !== token) previous.close()
  return () => {
    if (activeCanvas?.token === token) activeCanvas = undefined
  }
}

const styles: Record<string, React.CSSProperties> = {
  card: {
    border: '1px solid var(--ui-border, rgba(128,128,128,0.35))',
    borderRadius: 8,
    overflow: 'hidden',
    background: 'var(--ui-card-bg, transparent)',
    fontFamily: 'inherit',
  },
  head: {
    display: 'flex', alignItems: 'center', gap: 8, padding: '8px 12px',
    fontSize: 13, fontWeight: 600,
    borderBottom: '1px solid var(--ui-border, rgba(128,128,128,0.2))',
  },
  badge: {
    fontSize: 11, padding: '1px 8px', borderRadius: 99,
    textTransform: 'uppercase', letterSpacing: 0.4,
  },
  badgeOk: { background: 'rgba(34,197,94,0.15)', color: '#16a34a' },
  badgeError: { background: 'rgba(239,68,68,0.15)', color: '#dc2626' },
  badgeRunning: { background: 'rgba(100,116,139,0.15)', color: '#64748b' },
  body: { padding: 12 },
  imageViewport: {
    maxHeight: 560,
    overflow: 'auto',
    overscrollBehavior: 'contain',
    borderRadius: 4,
    border: '1px solid rgba(128,128,128,0.25)',
    background: 'rgba(128,128,128,0.06)',
  },
  img: {
    display: 'block', width: 'auto', maxWidth: '100%', height: 'auto', margin: '0 auto',
  },
  meta: {
    display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 10,
    marginTop: 10, fontSize: 12, color: 'var(--ui-text-muted, #888)',
  },
  link: { color: 'var(--ui-accent, #0ea5e9)', textDecoration: 'none' },
  button: {
    color: 'var(--ui-accent, #0ea5e9)', background: 'none', border: 'none',
    cursor: 'pointer', padding: 0, font: 'inherit', fontSize: 12,
  },
  primaryButton: {
    border: '1px solid var(--ui-accent, #0ea5e9)', borderRadius: 6,
    color: 'var(--ui-accent, #0ea5e9)', background: 'transparent',
    padding: '4px 9px', cursor: 'pointer', font: 'inherit', fontSize: 12,
  },
  pre: { whiteSpace: 'pre-wrap', wordBreak: 'break-all', fontSize: 12, margin: 0, maxHeight: '24em', overflow: 'auto' },
  muted: { fontSize: 12, color: 'var(--ui-text-muted, #888)' },
  warning: {
    margin: '10px 0 0', padding: '7px 9px', borderRadius: 6,
    color: '#b45309', background: 'rgba(245,158,11,0.13)', fontSize: 12,
  },
  backdrop: {
    position: 'fixed', inset: 0, zIndex: 2147483000,
    display: 'flex', alignItems: 'center', justifyContent: 'center',
    padding: 20, background: 'rgba(0,0,0,0.72)',
  },
  dialog: {
    width: 'min(1120px, 94vw)', height: 'min(820px, 92vh)',
    display: 'flex', flexDirection: 'column', overflow: 'hidden',
    border: '1px solid var(--ui-border, rgba(128,128,128,0.5))', borderRadius: 10,
    background: 'var(--ui-card-bg, #17171a)', color: 'var(--ui-text, #eee)',
    boxShadow: '0 24px 80px rgba(0,0,0,0.45)',
  },
  toolbar: {
    display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 8,
    minHeight: 44, padding: '7px 10px',
    borderBottom: '1px solid var(--ui-border, rgba(128,128,128,0.3))',
  },
  canvasWrap: { position: 'relative', flex: 1, minHeight: 0, overflow: 'hidden', background: '#202124' },
  canvas: { display: 'block', width: '100%', height: '100%', cursor: 'grab', touchAction: 'none' },
  overlay: {
    position: 'absolute', inset: 0, display: 'flex', alignItems: 'center',
    justifyContent: 'center', flexDirection: 'column', gap: 10,
    padding: 24, textAlign: 'center', background: 'rgba(25,25,28,0.92)',
  },
}

function baseName(path: string): string {
  const normalized = path.replaceAll('\\', '/')
  return normalized.slice(normalized.lastIndexOf('/') + 1) || path
}

/** Size the canvas backing store to its CSS box before CanvasKit attaches. */
export function sizeCanvasForDisplay(
  canvas: Pick<HTMLCanvasElement, 'clientWidth' | 'clientHeight' | 'width' | 'height'>,
  devicePixelRatio = window.devicePixelRatio,
): { cssWidth: number; cssHeight: number; dpr: number } {
  const cssWidth = Math.max(1, Math.round(canvas.clientWidth))
  const cssHeight = Math.max(1, Math.round(canvas.clientHeight))
  const dpr = Number.isFinite(devicePixelRatio) && devicePixelRatio > 0 ? devicePixelRatio : 1
  canvas.width = Math.max(1, Math.round(cssWidth * dpr))
  canvas.height = Math.max(1, Math.round(cssHeight * dpr))
  return { cssWidth, cssHeight, dpr }
}

function CanvasModal({ grant, onClose, locale }: {
  grant: PresentationGrant
  onClose: () => void
  locale: PresentationLocale
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null)
  const viewerRef = useRef<OpViewer>()
  const dragRef = useRef<{ id: number; x: number; y: number; panX: number; panY: number }>()
  const [phase, setPhase] = useState<'loading' | 'ready' | 'error'>('loading')
  const [failure, setFailure] = useState('')
  const [viewport, setViewport] = useState<Viewport>({ panX: 0, panY: 0, zoom: 1 })
  const documentGrant = grant.document
  const viewerGrant = grant.viewer
  const copy = designRenderCopy(locale)

  const fit = useCallback(() => {
    const viewer = viewerRef.current
    const canvas = canvasRef.current
    if (viewer === undefined || canvas === null) return
    viewer.zoomToFit(Math.max(1, canvas.clientWidth), Math.max(1, canvas.clientHeight))
    setViewport(viewer.viewport)
  }, [])

  const zoomBy = useCallback((factor: number) => {
    const viewer = viewerRef.current
    if (viewer === undefined) return
    viewer.setZoom(Math.min(16, Math.max(0.05, viewer.viewport.zoom * factor)))
    setViewport(viewer.viewport)
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => { window.removeEventListener('keydown', onKeyDown) }
  }, [onClose])

  useEffect(() => {
    const canvas = canvasRef.current
    if (canvas === null || documentGrant === undefined || viewerGrant === undefined) return
    sizeCanvasForDisplay(canvas)
    const abort = new AbortController()
    let cancelled = false
    let created: OpViewer | undefined
    setPhase('loading')
    setFailure('')

    const load = async (): Promise<void> => {
      try {
        const [sdk, response] = await Promise.all([
          loadOpenPencilSdk(viewerGrant.sdkUrl),
          fetch(documentGrant.url, { signal: abort.signal, credentials: 'same-origin' }),
        ])
        if (!response.ok) throw new Error(`OpenPencil document request failed (${response.status})`)
        const source = await response.text()
        if (cancelled) return
        created = await sdk.createViewer({
          canvas,
          doc: source,
          wasmUrl: viewerGrant.wasmUrl,
          canvasKitBaseUrl: viewerGrant.canvasKitBaseUrl,
        })
        if (cancelled) {
          created.destroy()
          return
        }
        viewerRef.current = created
        const syncViewport = (): void => { if (!cancelled && created !== undefined) setViewport(created.viewport) }
        created.on('viewportchange', syncViewport)
        setPhase('ready')
        requestAnimationFrame(() => { if (!cancelled) fit() })
      } catch (error) {
        if (cancelled || abort.signal.aborted) return
        setFailure(error instanceof Error ? error.message : String(error))
        setPhase('error')
      }
    }
    void load()
    return () => {
      cancelled = true
      abort.abort()
      viewerRef.current = undefined
      created?.destroy()
    }
  }, [documentGrant?.url, fit, viewerGrant?.canvasKitBaseUrl, viewerGrant?.sdkUrl, viewerGrant?.wasmUrl])

  const pointerDown = (event: ReactPointerEvent<HTMLCanvasElement>): void => {
    const viewer = viewerRef.current
    if (viewer === undefined) return
    event.currentTarget.setPointerCapture(event.pointerId)
    const current = viewer.viewport
    dragRef.current = { id: event.pointerId, x: event.clientX, y: event.clientY, panX: current.panX, panY: current.panY }
  }
  const pointerMove = (event: ReactPointerEvent<HTMLCanvasElement>): void => {
    const drag = dragRef.current
    const viewer = viewerRef.current
    if (drag === undefined || drag.id !== event.pointerId || viewer === undefined) return
    viewer.panTo(drag.panX + event.clientX - drag.x, drag.panY + event.clientY - drag.y)
    setViewport(viewer.viewport)
  }
  const pointerUp = (event: ReactPointerEvent<HTMLCanvasElement>): void => {
    if (dragRef.current?.id === event.pointerId) dragRef.current = undefined
  }

  const title = documentGrant?.path === undefined ? copy.canvas : baseName(documentGrant.path)
  return (
    <div
      style={styles.backdrop}
      role="presentation"
      data-openpencil-canvas-modal="true"
      onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}
    >
      <div style={styles.dialog} role="dialog" aria-modal="true" aria-label={`${copy.canvas}: ${title}`}>
        <div style={styles.toolbar}>
          <strong style={{ marginRight: 'auto', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{title}</strong>
          <button type="button" style={styles.primaryButton} disabled={phase !== 'ready'} onClick={() => { zoomBy(0.8) }} aria-label={copy.zoomOut}>−</button>
          <span style={styles.muted}>{Math.round(viewport.zoom * 100)}%</span>
          <button type="button" style={styles.primaryButton} disabled={phase !== 'ready'} onClick={() => { zoomBy(1.25) }} aria-label={copy.zoomIn}>+</button>
          <button type="button" style={styles.primaryButton} disabled={phase !== 'ready'} onClick={fit}>{copy.fit}</button>
          <button type="button" style={styles.primaryButton} onClick={onClose}>{copy.close}</button>
        </div>
        <div style={styles.canvasWrap}>
          <canvas
            ref={canvasRef}
            style={styles.canvas}
            onPointerDown={pointerDown}
            onPointerMove={pointerMove}
            onPointerUp={pointerUp}
            onPointerCancel={pointerUp}
            aria-label={copy.readonlyCanvas}
          />
          {phase === 'loading' ? (
            <div style={styles.overlay} role="status"><strong>{copy.loadingCanvas}</strong><span style={styles.muted}>{copy.pngRemains}</span></div>
          ) : null}
          {phase === 'error' ? (
            <div style={styles.overlay} role="alert">
              <strong>{copy.canvasUnavailable}</strong>
              <span style={styles.muted}>{failure}</span>
              {grant.image !== undefined ? <a style={styles.link} href={grant.image.previewUrl} target="_blank" rel="noreferrer">{copy.openPngFallback}</a> : null}
            </div>
          ) : null}
        </div>
        <div style={{ ...styles.meta, margin: 0, padding: '7px 10px' }}>
          <span>{copy.panHint}</span>
          {documentGrant?.sha256 !== undefined ? <span title={documentGrant.sha256}>{copy.snapshot} {documentGrant.sha256.slice(0, 10)}</span> : null}
        </div>
      </div>
    </div>
  )
}

/** Render one OpenPencil render tool call as a PNG-first card. */
export function DesignRenderView({
  block,
  callId,
  toolName,
  openDetails,
  openFile,
  inspect,
  locale = 'en',
  sessionId,
  openEditorWorkbench,
  autoOpenEditorWorkbench,
}: CompatibleToolCallViewProps & {
  locale?: PresentationLocale
  openEditorWorkbench?: (request: EditorWorkbenchRequest) => boolean | Promise<boolean>
  autoOpenEditorWorkbench?: (request: EditorWorkbenchRequest) => boolean | Promise<boolean>
}) {
  const settled = 'kind' in block
  const error = settled && block.isError
  const running = !settled
  const embeddedGrant = grantOf(block)
  const hydrationRequest = !running && !error
    ? presentationHydrationRequestOf({
      block,
      toolName,
      sessionId: String(sessionId),
      callId,
      embeddedGrant,
    })
    : undefined
  const hydrationKey = hydrationRequest === undefined
    ? undefined
    : `${hydrationRequest.sessionId}\n${hydrationRequest.callId}\n${hydrationRequest.documentSha256}`
  const [hydrated, setHydrated] = useState<{ key: string; grant: PresentationGrant }>()
  const [hydrationFailedKey, setHydrationFailedKey] = useState<string>()
  const grant = embeddedGrant
    ?? (hydrated !== undefined && hydrated.key === hydrationKey ? hydrated.grant : undefined)
  const hydrationPending = hydrationKey !== undefined && hydrationFailedKey !== hydrationKey
  const copy = designRenderCopy(locale)
  const creating = toolName === OPENPENCIL_NEW_TOOL_NAME
    || toolName === OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME
    || toolName === OPENPENCIL_PIPELINE_FINISH_TOOL_NAME
  const text = resultText(block)
  const awaitingPreview = pipelineResultAwaitsPreview(toolName, text)
  const fallbackText = fallbackPresentationText(text, awaitingPreview)
  const frames = grant?.frames ?? []
  const [selectedFrameIndex, setSelectedFrameIndex] = useState(0)
  const currentFrameIndex = normalizedFrameIndex(selectedFrameIndex, frames.length)
  const selectedFrame = frames[currentFrameIndex] ?? grant?.image
  const [modalToken, setModalToken] = useState<symbol>()
  const releaseRef = useRef<() => void>()
  const liveAutoOpenCallKey = liveAutoOpenKey(String(sessionId), callId)

  useEffect(() => {
    if (hydrationKey === undefined || hydrationRequest === undefined) return
    const controller = new AbortController()
    void requestPresentationGrantWithRetry(hydrationRequest, presentationGrantOfMeta, { signal: controller.signal }).then(nextGrant => {
      if (nextGrant !== undefined && !controller.signal.aborted) {
        setHydrated({ key: hydrationKey, grant: nextGrant })
      } else if (!controller.signal.aborted) {
        setHydrationFailedKey(hydrationKey)
      }
    })
    return () => { controller.abort() }
    // The semantic key contains every request field. Depending on `block`
    // itself would restart a local exchange whenever DSH reprojects a snapshot.
  }, [hydrationKey])

  const closeCanvas = useCallback(() => {
    releaseRef.current?.()
    releaseRef.current = undefined
    setModalToken(undefined)
  }, [])

  const openCanvas = useCallback(() => {
    const token = Symbol('openpencil-canvas')
    releaseRef.current?.()
    releaseRef.current = claimCanvas(token, () => {
      setModalToken((current) => current === token ? undefined : current)
    })
    setModalToken(token)
  }, [])

  const openEditor = useCallback(() => {
    // A nested Code-mode result has no durable `block.meta`, so DSH's native
    // details owner would receive the unhydrated block. Keep that recovered
    // grant in our page-owned workbench instead. Embedded metadata continues
    // to prefer the native details surface.
    requestOpenPencilEditor(embeddedGrant === undefined ? undefined : openDetails, () => {
      if (grant === undefined) return
      void openEditorWorkbench?.({ grant, sessionId: String(sessionId) })
    })
  }, [embeddedGrant, grant, openDetails, openEditorWorkbench, sessionId])

  useEffect(() => {
    if (shouldArmLiveAutoOpen(error, block.time)) rememberLiveAutoOpenCall(liveAutoOpenCallKey)
    else if (error) forgetLiveAutoOpenCall(liveAutoOpenCallKey)
  }, [error, liveAutoOpenCallKey, running])

  useEffect(() => {
    if (
      running
      || error
      || grant?.autoOpenEditor !== true
      || grant.editor?.enabled !== true
      || autoOpenEditorWorkbench === undefined
    ) return
    void attemptLiveAutoOpenCall(liveAutoOpenCallKey, () => autoOpenEditorWorkbench({
      grant,
      sessionId: String(sessionId),
    }))
  }, [autoOpenEditorWorkbench, error, grant, liveAutoOpenCallKey, running, sessionId])

  useEffect(() => () => { releaseRef.current?.() }, [])
  useEffect(() => { setSelectedFrameIndex(0) }, [frames.map(frame => frame.previewUrl).join('\n')])

  if (shouldHidePresentationCard({
    settled,
    error,
    hasGrant: grant !== undefined,
    hydrationPending,
    awaitingPreview,
  })) return null

  const badge = error
    ? <span style={{ ...styles.badge, ...styles.badgeError }}>{copy.error}</span>
    : running
      ? <span style={{ ...styles.badge, ...styles.badgeRunning }}>{copy.rendering}</span>
      : <span style={{ ...styles.badge, ...styles.badgeOk }}>{copy.done}</span>

  return (
    <section style={styles.card} data-tool={toolName} data-state={error ? 'error' : running ? 'running' : 'success'}>
      <div style={styles.head}><span>{openPencilPresentationTitle(toolName, locale)}</span>{badge}</div>
      <div style={styles.body}>
        {running ? <p style={styles.muted}>{creating ? copy.creatingDocument : copy.renderingDocument}</p> : null}
        {error ? <p style={styles.muted}>{text ?? copy.renderFailed}</p> : null}
        {!running && !error && frames.length > 0 ? (
          <FrameGallery frames={frames} selectedIndex={currentFrameIndex} onSelect={setSelectedFrameIndex} locale={locale} />
        ) : null}
        {!running && !error && grant?.warnings !== undefined ? (
          <div style={styles.warning} role="status">{grant.warnings.join(' ')}</div>
        ) : null}
        {!running && !error && grant !== undefined ? (
          <div style={styles.meta}>
            {selectedFrame !== undefined ? <span>{selectedFrame.name ?? baseName(selectedFrame.path)}</span> : null}
            {frames.length > 1 ? <span>{frames.length} {copy.frames}</span> : null}
            {grant.renderer !== undefined ? (
              <span title={grant.rendererBinary}>{grant.renderer}{grant.fidelity === undefined ? '' : ` · ${grant.fidelity}`}</span>
            ) : null}
            {grant.document !== undefined && grant.viewer !== undefined ? <button type="button" style={styles.primaryButton} onClick={openCanvas}>{copy.openInteractiveCanvas}</button> : null}
            {grant.document !== undefined && grant.editor?.enabled === true ? (
              <button type="button" style={styles.primaryButton} onClick={openEditor}>
                {openDetails === undefined || embeddedGrant === undefined ? copy.editCanvas : copy.editInSidebar}
              </button>
            ) : null}
            {selectedFrame !== undefined && openFile !== undefined ? (
              <button type="button" style={styles.button} onClick={() => { openFile(selectedFrame.path) }}>{copy.openRenderedPng}</button>
            ) : null}
            {selectedFrame !== undefined ? <a style={styles.link} href={selectedFrame.downloadUrl} download>{copy.downloadPng}</a> : null}
            {grant.document?.path !== undefined && openFile !== undefined ? (
              <button type="button" style={styles.button} onClick={() => { openFile(grant.document?.path ?? '') }}>{copy.editSource}</button>
            ) : null}
            {grant.document?.downloadUrl !== undefined ? <a style={styles.link} href={grant.document.downloadUrl} download>{copy.downloadSource}</a> : null}
            {inspect !== undefined ? <button type="button" style={styles.button} onClick={inspect}>{copy.inspectToolCall}</button> : null}
          </div>
        ) : null}
        {!running && !error && grant === undefined && hydrationPending ? (
          <p style={styles.muted} role="status">{copy.recoveringPreview}</p>
        ) : null}
        {!running && !error && grant === undefined && !hydrationPending ? (
          <>
            <p style={styles.muted}>{awaitingPreview ? copy.pendingPreview : copy.noPreview}</p>
            {fallbackText !== null ? <pre style={{ ...styles.pre, marginTop: 8 }}>{fallbackText}</pre> : null}
          </>
        ) : null}
      </div>
      {modalToken !== undefined && grant?.document !== undefined && grant.viewer !== undefined ? <CanvasModal grant={grant} onClose={closeCanvas} locale={locale} /> : null}
    </section>
  )
}

/** Render the selected editable design inside DSH's resident details column. */
export function OpenPencilEditorPanel({ block, colorScheme, locale, sessionId }: CompatibleToolDetailsViewProps & {
  colorScheme: EditorColorScheme
  locale: EditorLocale
}) {
  const grant = grantOf(block)
  if (grant?.editor === undefined || grant.document === undefined) {
    return <div style={styles.overlay}>{editorPanelCopy(locale).unavailable}</div>
  }
  return <ManagedOpenPencilEditor
    key={editorWorkbenchEditorKey(grant, String(sessionId))}
    grant={grant}
    colorScheme={colorScheme}
    locale={locale}
    sessionId={String(sessionId)}
  />
}

/** Required client services. */
export const inject = ['slots', 'theme', 'locale']

/** Register canonical views plus a presentation-only alias for replaying historical cards. */
export function apply(ctx: ClientContext): void {
  const subscribeTheme = (notify: () => void): (() => boolean) => ctx.on('theme/change', notify)
  const getColorScheme = (): EditorColorScheme => ctx.theme.getTheme().active.colorScheme
  const subscribeLocale = (notify: () => void): (() => boolean) => ctx.on('locale/change', notify)
  // DSH 0.1.5's locale service reports `active` as a plain string, so narrow
  // it here rather than asserting: an unrecognised locale falls back to 'en',
  // which is what every copy table in this plugin already defaults to.
  const getLocale = (): PresentationLocale => (ctx.locale.getLocale().active === 'zh' ? 'zh' : 'en')
  let editorWorkbenchHost: ReturnType<typeof mountEditorWorkbenchHost> | undefined
  if (typeof document !== 'undefined') {
    ctx.effect(() => {
      const host = mountEditorWorkbenchHost({
        subscribeTheme,
        getColorScheme,
        subscribeLocale,
        getLocale,
      })
      editorWorkbenchHost = host
      return () => {
        if (editorWorkbenchHost === host) editorWorkbenchHost = undefined
        return host.dispose()
      }
    }, 'dsh-openpencil: fallback editor workbench host')
  }
  const HostSyncedDesignRenderView = (props: ToolCallViewProps): React.JSX.Element => {
    const locale = useSyncExternalStore(subscribeLocale, getLocale, getLocale)
    return (
      <DesignRenderView
        {...props}
        locale={locale}
        openEditorWorkbench={(request: EditorWorkbenchRequest) => editorWorkbenchHost?.open(request) ?? false}
        autoOpenEditorWorkbench={(request: EditorWorkbenchRequest) => editorWorkbenchHost?.openIfIdle(request) ?? false}
      />
    )
  }
  const HostSyncedOpenPencilSelectionDock = (props: Omit<React.ComponentProps<typeof OpenPencilSelectionDock>, 'locale'>): React.JSX.Element => {
    const locale = useSyncExternalStore(subscribeLocale, getLocale, getLocale)
    return <OpenPencilSelectionDock {...props} locale={locale} />
  }
  for (const toolName of [
    OPENPENCIL_RENDER_TOOL_NAME,
    OPENPENCIL_NEW_TOOL_NAME,
    OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
    OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
    LEGACY_DESIGN_RENDER_TOOL_NAME,
  ]) {
    ctx.slots.inject('tool.call.toolview', () => ctx.slots.register(
      { name: 'tool.call.toolview', key: toolName },
      HostSyncedDesignRenderView,
    ))
  }
  ctx.slots.inject('tool.call.toolview', () => ctx.slots.register(
    { name: 'tool.call.toolview', key: OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME },
    HostSyncedDesignRenderView,
  ))
  ctx.slots.inject('tool.call.toolview', () => ctx.slots.register(
    { name: 'tool.call.toolview', key: OPENPENCIL_PIPELINE_BATCH_TOOL_NAME },
    HostSyncedDesignRenderView,
  ))
  ctx.slots.inject('conversation.input.dock', () => ctx.slots.register(
    { name: 'conversation.input.dock', id: 'openpencil-selection', order: 30 },
    HostSyncedOpenPencilSelectionDock,
  ))
}
