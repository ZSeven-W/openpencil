/** Page-stable owner for the plugin fallback editor workbench. */

import { useSyncExternalStore } from 'react'
import { createRoot, type Root } from 'react-dom/client'
import type { PresentationGrant, PresentationLocale } from './index.js'
import { editorLocaleFromDsh, hasActiveEditor, type EditorColorScheme } from './editor-bridge.js'
import {
  editorModalCopy,
  editorWorkbenchEditorKey,
  editorWorkbenchUsesFullscreen,
  ManagedOpenPencilEditorModal,
} from './editor-modal.js'
import { OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE } from './editor-dock-layout.js'
import {
  INITIAL_EDITOR_LIFECYCLE_STATE,
  type EditorLifecycleController,
  type EditorLifecycleState,
} from './editor-panel.js'

export interface EditorWorkbenchRequest {
  grant: PresentationGrant
  sessionId: string
  automatic?: boolean
}

type Listener = () => void

export interface EditorWorkbenchStore {
  getSnapshot: () => EditorWorkbenchRequest | undefined
  subscribe: (listener: Listener) => () => void
  open: (request: EditorWorkbenchRequest) => boolean
  close: () => void
}

function requestIdentity(request: EditorWorkbenchRequest): string {
  return editorWorkbenchEditorKey(request.grant, request.sessionId)
}

/** The sole automatic replacement allowed for an occupied workbench. */
export function isPublishedSuccessorOfLiveDraft(
  current: EditorWorkbenchRequest,
  next: EditorWorkbenchRequest,
): boolean {
  return current.sessionId === next.sessionId
    && current.grant.liveDraft === true
    && next.grant.liveDraft === false
    && typeof current.grant.draftId === 'string'
    && current.grant.draftId === next.grant.draftId
}

/** Close the exact live attachment before its published successor is mounted. */
export async function closeLiveDraftForPublishedSuccessor(
  current: EditorWorkbenchRequest,
  next: EditorWorkbenchRequest,
  controller: EditorLifecycleController | undefined,
  waitForController?: () => Promise<EditorLifecycleController | undefined>,
): Promise<boolean> {
  if (!isPublishedSuccessorOfLiveDraft(current, next)) return false
  const resolved = controller ?? await waitForController?.()
  if (resolved === undefined) return false
  return resolved.requestClose().catch(() => false)
}

/**
 * Small external store that is deliberately not owned by a Tool card. The
 * replacement gate lets the mounted host retain a dirty editor when another
 * historical card asks to open a different document.
 */
export function createEditorWorkbenchStore(
  canReplace: (current: EditorWorkbenchRequest) => boolean = () => true,
  onRepeat: () => void = () => {},
): EditorWorkbenchStore {
  let current: EditorWorkbenchRequest | undefined
  const listeners = new Set<Listener>()
  const emit = (): void => { for (const listener of listeners) listener() }
  return {
    getSnapshot: () => current,
    subscribe(listener) {
      listeners.add(listener)
      return () => { listeners.delete(listener) }
    },
    open(request) {
      if (current !== undefined && requestIdentity(current) === requestIdentity(request)) {
        onRepeat()
        return true
      }
      if (current !== undefined && !canReplace(current)) return false
      current = request
      emit()
      return true
    },
    close() {
      if (current === undefined) return
      current = undefined
      emit()
    },
  }
}

interface EditorWorkbenchHostOptions {
  subscribeTheme: (listener: Listener) => () => unknown
  getColorScheme: () => EditorColorScheme
  subscribeLocale: (listener: Listener) => () => unknown
  getLocale: () => PresentationLocale
  document?: Document
}

export interface EditorWorkbenchHost {
  open: (request: EditorWorkbenchRequest) => Promise<boolean>
  openIfIdle: (request: EditorWorkbenchRequest) => Promise<boolean>
  dispose: () => Promise<void>
}

export type AutomaticEditorWorkbenchLayout = 'fullscreen' | 'dock'

/** Narrow viewports are still valid automatic targets; they use the modal. */
export function automaticEditorWorkbenchLayout(viewportWidth: number): AutomaticEditorWorkbenchLayout {
  return editorWorkbenchUsesFullscreen(viewportWidth) ? 'fullscreen' : 'dock'
}

export type EditorDisposePreservation = 'clean' | 'saved' | 'recovered' | 'unrecovered'

/**
 * Preserve dirty state without inventing a save the user did not request.
 * An already-running save may finish; an idle dirty editor is recovery-only.
 */
export async function preserveEditorBeforeWorkbenchDispose(
  state: EditorLifecycleState,
  controller: EditorLifecycleController,
): Promise<EditorDisposePreservation> {
  const unrecovered = (): EditorDisposePreservation => {
    // React/HMR may still remove the client tree. Suppress its DELETE so the
    // server controller remains authoritative and can capture/recover later.
    controller.retainServerSessionOnUnmount()
    return 'unrecovered'
  }
  if (state.phase === 'saving') {
    const saved = await controller.awaitExistingSave().catch(() => false)
    if (saved) return 'saved'
    return await controller.captureRecovery().catch(() => false) ? 'recovered' : unrecovered()
  }
  if (!state.dirty) return 'clean'
  return await controller.captureRecovery().catch(() => false) ? 'recovered' : unrecovered()
}

interface HostViewProps {
  store: EditorWorkbenchStore
  subscribeTheme: EditorWorkbenchHostOptions['subscribeTheme']
  getColorScheme: EditorWorkbenchHostOptions['getColorScheme']
  subscribeLocale: EditorWorkbenchHostOptions['subscribeLocale']
  getLocale: EditorWorkbenchHostOptions['getLocale']
  ownerId: string
  onLifecycleState: (state: EditorLifecycleState) => void
  onLifecycleController: (controller: EditorLifecycleController | undefined) => void
  close: () => void
}

function EditorWorkbenchHostView({
  store,
  subscribeTheme,
  getColorScheme,
  subscribeLocale,
  getLocale,
  ownerId,
  onLifecycleState,
  onLifecycleController,
  close,
}: HostViewProps) {
  const request = useSyncExternalStore(store.subscribe, store.getSnapshot, store.getSnapshot)
  const colorScheme = useSyncExternalStore(subscribeTheme, getColorScheme, getColorScheme)
  const locale = useSyncExternalStore(subscribeLocale, getLocale, getLocale)
  if (request === undefined) return null
  return (
    <ManagedOpenPencilEditorModal
      grant={request.grant}
      colorScheme={colorScheme}
      locale={editorLocaleFromDsh(locale)}
      sessionId={request.sessionId}
      ownerId={ownerId}
      onLifecycleState={onLifecycleState}
      onLifecycleController={onLifecycleController}
      onClose={close}
      allowEditorTakeover={request.automatic !== true}
    />
  )
}

let nextHostId = 0

/** Mount one imperative React root for the whole plugin fiber. */
export function mountEditorWorkbenchHost(options: EditorWorkbenchHostOptions): EditorWorkbenchHost {
  const ownerDocument = options.document ?? document
  const hostId = `dsh-openpencil-workbench-${++nextHostId}`
  const container = ownerDocument.createElement('div')
  container.dataset.openpencilWorkbenchHost = hostId
  ownerDocument.body.append(container)
  let root: Root | undefined = createRoot(container)
  let destroyed = false
  let disposing = false
  let disposePromise: Promise<void> | undefined
  let openQueue = Promise.resolve()
  let lifecycle: EditorLifecycleState = INITIAL_EDITOR_LIFECYCLE_STATE
  let lifecycleController: EditorLifecycleController | undefined
  const lifecycleControllerWaiters = new Set<(controller: EditorLifecycleController | undefined) => void>()
  const publishLifecycleController = (next: EditorLifecycleController | undefined): void => {
    lifecycleController = next
    if (next === undefined) return
    for (const waiter of lifecycleControllerWaiters) waiter(next)
    lifecycleControllerWaiters.clear()
  }
  const waitForLifecycleController = (): Promise<EditorLifecycleController | undefined> => {
    if (lifecycleController !== undefined) return Promise.resolve(lifecycleController)
    return new Promise(resolveController => {
      let settled = false
      const finish = (next: EditorLifecycleController | undefined): void => {
        if (settled) return
        settled = true
        clearTimeout(timer)
        lifecycleControllerWaiters.delete(finish)
        resolveController(next)
      }
      const timer = setTimeout(() => { finish(undefined) }, 60_000)
      lifecycleControllerWaiters.add(finish)
    })
  }
  const cancelLifecycleControllerWaiters = (): void => {
    for (const waiter of lifecycleControllerWaiters) waiter(undefined)
    lifecycleControllerWaiters.clear()
  }

  const focusSurface = (): void => {
    const target = ownerDocument.querySelector<HTMLElement>(
      `[data-openpencil-editor-workbench-owner="${hostId}"] button, `
      + `[data-openpencil-editor-workbench-owner="${hostId}"] [tabindex="0"]`,
    )
    target?.focus()
  }
  const canDiscard = (): boolean => {
    if (lifecycle.phase === 'saving') return false
    return !lifecycle.dirty || window.confirm(
      editorModalCopy(editorLocaleFromDsh(options.getLocale())).discard,
    )
  }
  const store = createEditorWorkbenchStore(() => true, focusSurface)
  const close = (): void => {
    lifecycle = INITIAL_EDITOR_LIFECYCLE_STATE
    lifecycleController = undefined
    cancelLifecycleControllerWaiters()
    store.close()
  }

  const destroy = (): void => {
    if (destroyed) return
    destroyed = true
    cancelLifecycleControllerWaiters()
    root?.unmount()
    root = undefined
    container.remove()
  }

  root.render(
    <EditorWorkbenchHostView
      store={store}
      subscribeTheme={options.subscribeTheme}
      getColorScheme={options.getColorScheme}
      subscribeLocale={options.subscribeLocale}
      getLocale={options.getLocale}
      ownerId={hostId}
      onLifecycleState={next => { lifecycle = next }}
      onLifecycleController={publishLifecycleController}
      close={close}
    />,
  )

  return {
    open(request) {
      if (destroyed || disposing) return Promise.resolve(false)
      const operation = openQueue.then(async (): Promise<boolean> => {
        if (destroyed || disposing) return false
        const previous = store.getSnapshot()
        if (previous !== undefined && requestIdentity(previous) === requestIdentity(request)) {
          store.open(request)
          queueMicrotask(focusSurface)
          return true
        }
        if (previous !== undefined && !canDiscard()) return false
        if (previous !== undefined && lifecycleController !== undefined) {
          const closed = await lifecycleController.requestClose()
          if (!closed || destroyed || disposing) return false
        }
        const accepted = store.open(request)
        if (accepted) {
          lifecycle = INITIAL_EDITOR_LIFECYCLE_STATE
          lifecycleController = undefined
          queueMicrotask(focusSurface)
        }
        return accepted
      })
      openQueue = operation.then(() => {}, () => {})
      return operation
    },
    openIfIdle(request) {
      if (destroyed || disposing) return Promise.resolve(false)
      const operation = openQueue.then(async (): Promise<boolean> => {
        if (destroyed || disposing) return false
        const previous = store.getSnapshot()
        if (previous !== undefined) {
          const closed = await closeLiveDraftForPublishedSuccessor(
            previous,
            request,
            lifecycleController,
            waitForLifecycleController,
          )
          if (!closed || destroyed || disposing || store.getSnapshot() !== previous) return false
          const accepted = store.open({ ...request, automatic: true })
          if (accepted) {
            lifecycle = INITIAL_EDITOR_LIFECYCLE_STATE
            lifecycleController = undefined
          }
          return accepted
        }
        if (hasActiveEditor()) return false
        const viewportWidth = ownerDocument.defaultView?.innerWidth ?? window.innerWidth
        if (automaticEditorWorkbenchLayout(viewportWidth) === 'dock') {
          const dshRoot = ownerDocument.getElementById('root')
          if (dshRoot === null) return false
          if (dshRoot.dataset[OPENPENCIL_WORKBENCH_DOCK_ATTRIBUTE] !== undefined) return false
          const computedMarginRight = Number.parseFloat(ownerDocument.defaultView?.getComputedStyle(dshRoot).marginRight ?? '0')
          if (dshRoot.style.marginRight.trim() !== '' || (Number.isFinite(computedMarginRight) && computedMarginRight > 0.5)) return false
        }
        const accepted = store.open({ ...request, automatic: true })
        if (accepted) {
          lifecycle = INITIAL_EDITOR_LIFECYCLE_STATE
          lifecycleController = undefined
        }
        return accepted
      })
      openQueue = operation.then(() => {}, () => {})
      return operation
    },
    async dispose() {
      if (destroyed) return
      if (disposePromise !== undefined) return disposePromise
      disposing = true
      disposePromise = (async () => {
        await openQueue
        // Never turn unload into an implicit source write. Join only a save
        // the user already started; otherwise capture a recovery draft.
        if ((lifecycle.dirty || lifecycle.phase === 'saving') && lifecycleController !== undefined) {
          await preserveEditorBeforeWorkbenchDispose(lifecycle, lifecycleController)
        }
        close()
        destroy()
      })()
      return disposePromise
    },
  }
}
