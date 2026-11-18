/** Plugin-owned OpenPencil workbench for DSH builds without Tool details. */

import { useCallback, useEffect, useId, useLayoutEffect, useRef, useState } from 'react'
import { createPortal } from 'react-dom'
import type { PresentationGrant } from './index.js'
import { confirmEditorClose, type EditorColorScheme, type EditorLocale } from './editor-bridge.js'
import {
  INITIAL_EDITOR_LIFECYCLE_STATE,
  ManagedOpenPencilEditor,
  type EditorLifecycleController,
  type EditorLifecycleState,
} from './editor-panel.js'
import { claimEditorWorkbenchDock, type EditorWorkbenchDockLease } from './editor-dock-layout.js'

export const EDITOR_WORKBENCH_FULLSCREEN_BREAKPOINT = 1480
export const EDITOR_WORKBENCH_MIN_WIDTH = 640
export const EDITOR_WORKBENCH_MAX_WIDTH = 960
export const EDITOR_WORKBENCH_LEFT_CLEARANCE = 840
export const EDITOR_WORKBENCH_RESIZE_STEP = 32
export const EDITOR_WORKBENCH_CLOSE_BUTTON_MIN_WIDTH = 72
export const OPENPENCIL_EDITOR_CLOSE_BUTTON_ATTRIBUTE = 'data-openpencil-editor-close'

let bodyScrollLockCount = 0
let bodyOverflowBeforeLock = ''

function lockBodyScroll(): () => void {
  if (bodyScrollLockCount === 0) {
    bodyOverflowBeforeLock = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
  bodyScrollLockCount += 1
  let released = false
  return () => {
    if (released) return
    released = true
    bodyScrollLockCount = Math.max(0, bodyScrollLockCount - 1)
    if (bodyScrollLockCount === 0) document.body.style.overflow = bodyOverflowBeforeLock
  }
}

interface EditorWorkbenchCopy {
  title: string
  close: string
  fullscreen: string
  restore: string
  resize: string
  discard: string
}

const EDITOR_WORKBENCH_COPY: Record<EditorLocale, EditorWorkbenchCopy> = {
  'zh-CN': {
    title: 'OpenPencil 编辑器',
    close: '关闭',
    fullscreen: '全屏',
    restore: '退出全屏',
    resize: '拖动调整编辑区宽度',
    discard: 'OpenPencil 中有未保存的更改，确定关闭并放弃吗？',
  },
  'en-US': {
    title: 'OpenPencil editor',
    close: 'Close',
    fullscreen: 'Full screen',
    restore: 'Exit full screen',
    resize: 'Drag to resize the editor',
    discard: 'OpenPencil has unsaved changes. Close and discard them?',
  },
}

export function editorModalCopy(locale: EditorLocale): EditorWorkbenchCopy {
  return EDITOR_WORKBENCH_COPY[locale]
}

export function editorWorkbenchUsesFullscreen(viewportWidth: number): boolean {
  return !Number.isFinite(viewportWidth) || viewportWidth < EDITOR_WORKBENCH_FULLSCREEN_BREAKPOINT
}

export interface EditorWorkbenchWidthBounds {
  min: number
  max: number
  initial: number
}

/** Keep useful DSH conversation space while allowing a large desktop canvas. */
export function editorWorkbenchWidthBounds(viewportWidth: number): EditorWorkbenchWidthBounds {
  const safeViewport = Number.isFinite(viewportWidth) ? Math.max(0, viewportWidth) : 0
  const available = Math.max(0, safeViewport - EDITOR_WORKBENCH_LEFT_CLEARANCE)
  const max = Math.min(EDITOR_WORKBENCH_MAX_WIDTH, Math.max(EDITOR_WORKBENCH_MIN_WIDTH, available))
  const min = Math.min(EDITOR_WORKBENCH_MIN_WIDTH, max)
  const preferred = 720
  return { min, max, initial: Math.min(max, Math.max(min, preferred)) }
}

export function clampEditorWorkbenchWidth(width: number, viewportWidth: number): number {
  const bounds = editorWorkbenchWidthBounds(viewportWidth)
  const safeWidth = Number.isFinite(width) ? width : bounds.initial
  return Math.min(bounds.max, Math.max(bounds.min, safeWidth))
}

/** A left-edge drag grows the right-docked workbench as the pointer moves left. */
export function resizedEditorWorkbenchWidth(
  startWidth: number,
  startClientX: number,
  clientX: number,
  viewportWidth: number,
): number {
  return clampEditorWorkbenchWidth(startWidth + startClientX - clientX, viewportWidth)
}

/** Key only the editor process; outer workbench geometry remains stable. */
export function editorWorkbenchEditorKey(grant: PresentationGrant, sessionId: string): string {
  return `${sessionId}\n${grant.editor?.launchUrl ?? ''}`
}

/**
 * Return the focus target used at a fullscreen Tab boundary.
 *
 * `activeIndex` is -1 when focus is outside the workbench. Returning -1 means
 * normal browser tab order should continue inside the workbench.
 */
export function editorWorkbenchFocusTargetIndex(
  focusableCount: number,
  activeIndex: number,
  backwards: boolean,
): number {
  if (!Number.isInteger(focusableCount) || focusableCount <= 0) return -1
  if (!Number.isInteger(activeIndex) || activeIndex < 0 || activeIndex >= focusableCount) {
    return backwards ? focusableCount - 1 : 0
  }
  if (backwards && activeIndex === 0) return focusableCount - 1
  if (!backwards && activeIndex === focusableCount - 1) return 0
  return -1
}

/** Side mode is non-modal, so Escape only belongs to it while focus is inside. */
export function editorWorkbenchShouldHandleEscape(fullscreen: boolean, targetInside: boolean): boolean {
  return fullscreen || targetInside
}

/** Read the editor's durable dirty marker before allowing the workbench to close. */
export function confirmEditorModalClose(
  root: Pick<ParentNode, 'querySelector'> | null,
  message: string,
  confirm = window.confirm,
): boolean {
  const dirty = (root?.querySelector('[data-tool-details-dirty="true"]') ?? null) !== null
  return confirmEditorClose(dirty, () => confirm(message))
}

const styles: Record<string, React.CSSProperties> = {
  surface: {
    position: 'fixed', top: 'var(--dsh-windows-titlebar-height, 0px)', right: 0, bottom: 0, zIndex: 1100,
    boxSizing: 'border-box', minWidth: 0, minHeight: 0, display: 'flex', flexDirection: 'column', overflow: 'hidden',
    borderLeft: '1px solid var(--dsw-alias-border-l2, rgba(128,128,128,0.5))',
    color: 'var(--dsw-alias-label-primary, #202124)',
    background: 'var(--dsw-alias-bg-base, #fff)',
  },
  fullscreen: { left: 0, width: 'auto', borderLeft: 0 },
  resizeHandle: {
    position: 'absolute', zIndex: 3, top: 0, bottom: 0, left: -6, width: 12,
    display: 'flex', alignItems: 'center', justifyContent: 'center',
    cursor: 'ew-resize', touchAction: 'none', background: 'transparent',
  },
  resizeGrip: {
    width: 3, height: 32, flex: 'none', borderRadius: 999,
    border: '1px solid var(--dsw-alias-border-l2, rgba(128,128,128,0.45))',
    background: 'var(--dsw-alias-button-floating-fill, var(--dsw-alias-bg-layer-2, #fff))',
    boxShadow: '0 1px 3px rgba(0,0,0,0.18)', pointerEvents: 'none',
  },
  button: {
    boxSizing: 'border-box', width: 28, height: 28, flex: 'none', display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    border: 0, borderRadius: 6, color: 'var(--dsw-alias-label-secondary, inherit)', background: 'transparent',
    padding: 0, cursor: 'pointer', fontFamily: 'inherit', fontSize: 12, fontWeight: 'inherit', lineHeight: 1,
  },
  closeButton: {
    boxSizing: 'border-box', minWidth: EDITOR_WORKBENCH_CLOSE_BUTTON_MIN_WIDTH, height: 30, flex: 'none',
    display: 'inline-flex', alignItems: 'center', justifyContent: 'center', gap: 6,
    border: '1px solid var(--dsw-alias-border-l2, rgba(128,128,128,0.5))', borderRadius: 6,
    color: 'var(--dsw-alias-label-primary, inherit)',
    background: 'var(--dsw-alias-button-floating-fill, var(--dsw-alias-bg-layer-2, #fff))',
    padding: '0 10px', cursor: 'pointer', fontFamily: 'inherit', fontSize: 12, fontWeight: 500, lineHeight: 1,
  },
  body: { flex: 1, minHeight: 0, overflow: 'hidden' },
  focusGuard: {
    position: 'fixed', width: 1, height: 1, padding: 0, margin: 0,
    overflow: 'hidden', opacity: 0, pointerEvents: 'none',
  },
}

const EDITOR_WORKBENCH_FOCUSABLE_SELECTOR = [
  'a[href]',
  'button:not([disabled])',
  'input:not([disabled])',
  'select:not([disabled])',
  'textarea:not([disabled])',
  'iframe:not([tabindex="-1"])',
  '[tabindex]:not([tabindex="-1"])',
].join(',')

function editorWorkbenchFocusableElements(surface: HTMLElement): HTMLElement[] {
  return Array.from(surface.querySelectorAll<HTMLElement>(EDITOR_WORKBENCH_FOCUSABLE_SELECTOR))
    .filter(element => {
      if (element.dataset.openpencilFocusGuard === 'true') return false
      if (element.hidden || element.getAttribute('aria-hidden') === 'true') return false
      if (element.closest('[hidden], [inert], [aria-hidden="true"]')) return false
      const style = window.getComputedStyle(element)
      return style.display !== 'none' && style.visibility !== 'hidden'
    })
}

function focusEditorWorkbenchBoundary(surface: HTMLElement, backwards: boolean): void {
  const focusable = editorWorkbenchFocusableElements(surface)
  const target = backwards ? focusable.at(-1) : focusable[0]
  ;(target ?? surface).focus()
}

function FullscreenIcon(): React.ReactElement {
  return <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
    <path d="M2.5 6V2.5H6M10 2.5h3.5V6M13.5 10v3.5H10M6 13.5H2.5V10" />
  </svg>
}

function RestoreIcon(): React.ReactElement {
  return <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
    <path d="M6 2.5V6H2.5M13.5 6H10V2.5M10 13.5V10h3.5M2.5 10H6v3.5" />
  </svg>
}

function CloseIcon(): React.ReactElement {
  return <svg aria-hidden="true" width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round">
    <path d="M3.5 3.5l9 9M12.5 3.5l-9 9" />
  </svg>
}

export function ManagedOpenPencilEditorModal({
  grant,
  colorScheme,
  locale,
  sessionId,
  ownerId,
  onLifecycleState,
  onLifecycleController,
  onClose,
  allowEditorTakeover = true,
}: {
  grant: PresentationGrant
  colorScheme: EditorColorScheme
  locale: EditorLocale
  sessionId: string
  ownerId?: string
  onLifecycleState?: (state: EditorLifecycleState) => void
  onLifecycleController?: (controller: EditorLifecycleController | undefined) => void
  onClose: () => void
  allowEditorTakeover?: boolean
}) {
  const bodyRef = useRef<HTMLDivElement>(null)
  const surfaceRef = useRef<HTMLElement>(null)
  const closeRef = useRef<HTMLButtonElement>(null)
  const openerRef = useRef<HTMLElement | null>(null)
  const resizeCleanupRef = useRef<() => void>()
  const dockLeaseRef = useRef<EditorWorkbenchDockLease>()
  const dockOwnerId = useId()
  const copy = editorModalCopy(locale)
  const [viewportWidth, setViewportWidth] = useState(() => window.innerWidth)
  const [requestedFullscreen, setRequestedFullscreen] = useState(false)
  const [dockUnavailable, setDockUnavailable] = useState(false)
  const [preferredWidth, setPreferredWidth] = useState(() => editorWorkbenchWidthBounds(window.innerWidth).initial)
  const [lifecycle, setLifecycle] = useState<EditorLifecycleState>(INITIAL_EDITOR_LIFECYCLE_STATE)
  const lifecycleRef = useRef<EditorLifecycleState>(INITIAL_EDITOR_LIFECYCLE_STATE)
  const lifecycleControllerRef = useRef<EditorLifecycleController>()
  const automaticFullscreen = editorWorkbenchUsesFullscreen(viewportWidth)
  const fullscreen = automaticFullscreen || requestedFullscreen || dockUnavailable
  const width = clampEditorWorkbenchWidth(preferredWidth, viewportWidth)

  useLayoutEffect(() => {
    if (fullscreen) return
    const root = document.getElementById('root')
    if (root === null) {
      setDockUnavailable(true)
      return
    }
    const computedMarginRight = Number.parseFloat(window.getComputedStyle(root).marginRight)
    const lease = claimEditorWorkbenchDock(root, dockOwnerId, width, computedMarginRight)
    if (lease === undefined) {
      setDockUnavailable(true)
      return
    }
    dockLeaseRef.current = lease
    return () => {
      if (dockLeaseRef.current === lease) dockLeaseRef.current = undefined
      lease.release()
    }
  }, [dockOwnerId, fullscreen])

  useLayoutEffect(() => {
    if (!fullscreen) dockLeaseRef.current?.update(width)
  }, [fullscreen, width])

  const closeWithoutPrompt = useCallback(() => { onClose() }, [onClose])
  const requestClose = useCallback(async () => {
    if (lifecycleRef.current.phase === 'saving') return
    if (!confirmEditorClose(lifecycleRef.current.dirty, () => window.confirm(copy.discard))) return
    if (lifecycleControllerRef.current !== undefined) {
      const closed = await lifecycleControllerRef.current.requestClose()
      if (!closed) return
    }
    closeWithoutPrompt()
  }, [closeWithoutPrompt, copy.discard])
  const requestTakeover = useCallback((_state: EditorLifecycleState): boolean => {
    // The page-wide coordinator is synchronous, while a guarded daemon DELETE
    // is asynchronous and may fail. Veto this shortcut: the stable workbench
    // host performs close/replace through lifecycleController.requestClose().
    return false
  }, [])
  const updateLifecycle = useCallback((next: EditorLifecycleState) => {
    lifecycleRef.current = next
    setLifecycle(next)
    onLifecycleState?.(next)
  }, [onLifecycleState])
  const updateLifecycleController = useCallback((next: EditorLifecycleController | undefined) => {
    lifecycleControllerRef.current = next
    onLifecycleController?.(next)
  }, [onLifecycleController])

  useEffect(() => {
    openerRef.current = document.activeElement instanceof HTMLElement ? document.activeElement : null
    const mountedSurface = surfaceRef.current
    closeRef.current?.focus()
    return () => {
      const opener = openerRef.current
      const activeElement = document.activeElement
      const focusStayedInWorkbench = activeElement === document.body
        || (activeElement !== null && mountedSurface?.contains(activeElement))
      if (focusStayedInWorkbench && opener?.isConnected === true) opener.focus()
    }
  }, [])

  useEffect(() => {
    const onResize = (): void => {
      setViewportWidth(window.innerWidth)
    }
    window.addEventListener('resize', onResize)
    return () => { window.removeEventListener('resize', onResize) }
  }, [])

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent): void => {
      if (event.key !== 'Escape') return
      const surface = surfaceRef.current
      const targetInside = event.target instanceof Node && surface?.contains(event.target) === true
      if (!editorWorkbenchShouldHandleEscape(fullscreen, targetInside)) return
      event.preventDefault()
      requestClose()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => { window.removeEventListener('keydown', onKeyDown) }
  }, [fullscreen, requestClose])

  useEffect(() => {
    if (!fullscreen) return
    const surface = surfaceRef.current
    if (!surface) return

    const containFocus = (event: FocusEvent): void => {
      if (event.target instanceof Node && surface.contains(event.target)) return
      focusEditorWorkbenchBoundary(surface, false)
    }
    const wrapTab = (event: KeyboardEvent): void => {
      if (event.key !== 'Tab') return
      const focusable = editorWorkbenchFocusableElements(surface)
      const activeIndex = focusable.indexOf(document.activeElement as HTMLElement)
      const targetIndex = editorWorkbenchFocusTargetIndex(focusable.length, activeIndex, event.shiftKey)
      if (targetIndex < 0) return
      event.preventDefault()
      event.stopPropagation()
      focusable[targetIndex]?.focus()
    }

    document.addEventListener('focusin', containFocus, true)
    document.addEventListener('keydown', wrapTab, true)
    if (!surface.contains(document.activeElement)) focusEditorWorkbenchBoundary(surface, false)
    return () => {
      document.removeEventListener('focusin', containFocus, true)
      document.removeEventListener('keydown', wrapTab, true)
    }
  }, [fullscreen])

  useEffect(() => {
    if (!fullscreen) return
    return lockBodyScroll()
  }, [fullscreen])

  useEffect(() => () => { resizeCleanupRef.current?.() }, [])
  useEffect(() => {
    if (fullscreen) resizeCleanupRef.current?.()
  }, [fullscreen])

  const startResize = useCallback((event: React.PointerEvent<HTMLDivElement>) => {
    if (fullscreen) return
    event.preventDefault()
    resizeCleanupRef.current?.()
    const handle = event.currentTarget
    const pointerId = event.pointerId
    const surface = surfaceRef.current
    const editorFrame = surface?.querySelector('iframe') ?? null
    const inlineSurfaceWidth = surface === null ? Number.NaN : Number.parseFloat(surface.style.width)
    let liveWidth = Number.isFinite(inlineSurfaceWidth) ? inlineSurfaceWidth : width
    let appliedClientX = event.clientX
    const previousCursor = document.body.style.cursor
    const previousUserSelect = document.body.style.userSelect
    const previousFrameStyle = editorFrame === null ? undefined : {
      position: editorFrame.style.position,
      top: editorFrame.style.top,
      right: editorFrame.style.right,
      bottom: editorFrame.style.bottom,
      left: editorFrame.style.left,
      width: editorFrame.style.width,
      height: editorFrame.style.height,
      maxWidth: editorFrame.style.maxWidth,
      pointerEvents: editorFrame.style.pointerEvents,
    }
    document.body.style.cursor = 'ew-resize'
    document.body.style.userSelect = 'none'
    if (editorFrame !== null) {
      const frameWidth = editorFrame.getBoundingClientRect().width
      Object.assign(editorFrame.style, {
        position: 'absolute', top: '0', right: '0', bottom: '0', left: 'auto',
        width: `${frameWidth}px`, height: '100%', maxWidth: 'none', pointerEvents: 'none',
      })
    }
    try { handle.setPointerCapture(pointerId) } catch { /* the window listeners remain as a fallback */ }
    let animationFrame: number | undefined
    let nextClientX = event.clientX
    let stopped = false

    const applyWidth = (clientX: number): number => {
      liveWidth = resizedEditorWorkbenchWidth(liveWidth, appliedClientX, clientX, window.innerWidth)
      appliedClientX = clientX
      if (surface !== null) surface.style.width = `${liveWidth}px`
      dockLeaseRef.current?.update(liveWidth)
      handle.setAttribute('aria-valuenow', String(Math.round(liveWidth)))
      return liveWidth
    }
    const flushWidth = (): number => {
      if (animationFrame !== undefined) {
        window.cancelAnimationFrame(animationFrame)
        animationFrame = undefined
      }
      return applyWidth(nextClientX)
    }
    const onMove = (moveEvent: PointerEvent): void => {
      if (moveEvent.pointerId !== pointerId) return
      const coalesced = moveEvent.getCoalescedEvents?.()
      nextClientX = coalesced?.at(-1)?.clientX ?? moveEvent.clientX
      if (animationFrame !== undefined) return
      animationFrame = window.requestAnimationFrame(() => {
        animationFrame = undefined
        applyWidth(nextClientX)
      })
    }
    const cleanup = (): void => {
      if (stopped) return
      stopped = true
      if (animationFrame !== undefined) window.cancelAnimationFrame(animationFrame)
      window.removeEventListener('pointermove', onMove, true)
      window.removeEventListener('pointerup', onPointerEnd, true)
      window.removeEventListener('pointercancel', onPointerCancel, true)
      window.removeEventListener('blur', onBlur)
      handle.removeEventListener('lostpointercapture', onLostPointerCapture)
      try {
        if (handle.hasPointerCapture(pointerId)) handle.releasePointerCapture(pointerId)
      } catch { /* capture may already have been released by the browser */ }
      document.body.style.cursor = previousCursor
      document.body.style.userSelect = previousUserSelect
      if (editorFrame !== null && previousFrameStyle !== undefined) Object.assign(editorFrame.style, previousFrameStyle)
      resizeCleanupRef.current = undefined
    }
    const finish = (): void => {
      if (stopped) return
      const finalWidth = flushWidth()
      cleanup()
      setPreferredWidth(finalWidth)
    }
    function onPointerEnd(endEvent: PointerEvent): void {
      if (endEvent.pointerId !== pointerId) return
      nextClientX = Number.isFinite(endEvent.clientX) ? endEvent.clientX : nextClientX
      finish()
    }
    function onPointerCancel(cancelEvent: PointerEvent): void {
      if (cancelEvent.pointerId === pointerId) finish()
    }
    function onBlur(): void { finish() }
    function onLostPointerCapture(lostEvent: PointerEvent): void {
      if (lostEvent.pointerId === pointerId) finish()
    }

    resizeCleanupRef.current = cleanup
    // Capture phase cannot be interrupted by the embedded editor or another
    // host listener. Pointer capture keeps the handle as the event target even
    // while the pointer crosses the editor iframe.
    window.addEventListener('pointermove', onMove, true)
    window.addEventListener('pointerup', onPointerEnd, true)
    window.addEventListener('pointercancel', onPointerCancel, true)
    window.addEventListener('blur', onBlur)
    handle.addEventListener('lostpointercapture', onLostPointerCapture)
  }, [fullscreen, width])

  const resizeWithKeyboard = useCallback((event: React.KeyboardEvent<HTMLDivElement>) => {
    if (fullscreen) return
    const bounds = editorWorkbenchWidthBounds(window.innerWidth)
    let next: number | undefined
    if (event.key === 'ArrowLeft') next = width + EDITOR_WORKBENCH_RESIZE_STEP
    if (event.key === 'ArrowRight') next = width - EDITOR_WORKBENCH_RESIZE_STEP
    if (event.key === 'Home') next = bounds.min
    if (event.key === 'End') next = bounds.max
    if (next === undefined) return
    event.preventDefault()
    setPreferredWidth(clampEditorWorkbenchWidth(next, window.innerWidth))
  }, [fullscreen, width])

  const surface = (
    <section
      ref={surfaceRef}
      style={{
        ...styles.surface,
        ...(fullscreen ? styles.fullscreen : { width }),
        ...(colorScheme === 'dark' ? {
          color: 'var(--dsw-alias-label-primary, #eee)',
          background: 'var(--dsw-alias-bg-base, #17171a)',
        } : {}),
      }}
      role={fullscreen ? 'dialog' : 'complementary'}
      aria-modal={fullscreen ? true : undefined}
      aria-label={copy.title}
      data-openpencil-editor-workbench="true"
      data-openpencil-editor-workbench-owner={ownerId}
      data-openpencil-editor-modal="true"
      data-openpencil-editor-mode={fullscreen ? 'fullscreen' : 'side'}
      tabIndex={-1}
    >
      {fullscreen ? (
        <span
          data-openpencil-focus-guard="true"
          style={styles.focusGuard}
          tabIndex={0}
          onFocus={() => {
            if (surfaceRef.current) focusEditorWorkbenchBoundary(surfaceRef.current, true)
          }}
        />
      ) : null}
      {!fullscreen ? (
        <div
          style={styles.resizeHandle}
          role="separator"
          aria-orientation="vertical"
          aria-label={copy.resize}
          aria-valuemin={editorWorkbenchWidthBounds(viewportWidth).min}
          aria-valuemax={editorWorkbenchWidthBounds(viewportWidth).max}
          aria-valuenow={Math.round(width)}
          tabIndex={0}
          title={copy.resize}
          onPointerDown={startResize}
          onKeyDown={resizeWithKeyboard}
          onDoubleClick={() => { setPreferredWidth(editorWorkbenchWidthBounds(window.innerWidth).initial) }}
        ><span style={styles.resizeGrip} aria-hidden="true" /></div>
      ) : null}
      <div ref={bodyRef} style={styles.body}>
        <ManagedOpenPencilEditor
          key={editorWorkbenchEditorKey(grant, sessionId)}
          grant={grant}
          colorScheme={colorScheme}
          locale={locale}
          sessionId={sessionId}
          onTakeoverRequest={requestTakeover}
          onLifecycleState={updateLifecycle}
          onLifecycleController={updateLifecycleController}
          allowTakeover={allowEditorTakeover}
          workbenchActions={<>
            {!automaticFullscreen && !dockUnavailable ? (
              <button
                type="button"
                style={styles.button}
                aria-label={fullscreen ? copy.restore : copy.fullscreen}
                title={fullscreen ? copy.restore : copy.fullscreen}
                onClick={() => { setRequestedFullscreen(current => !current) }}
              >
                {fullscreen ? <RestoreIcon /> : <FullscreenIcon />}
              </button>
            ) : null}
            <button
              ref={closeRef}
              type="button"
              {...{ [OPENPENCIL_EDITOR_CLOSE_BUTTON_ATTRIBUTE]: 'true' }}
              style={{ ...styles.closeButton, ...(lifecycle.phase === 'saving' ? { cursor: 'not-allowed', opacity: 0.55 } : {}) }}
              aria-label={copy.close}
              title={copy.close}
              disabled={lifecycle.phase === 'saving'}
              onClick={() => { void requestClose() }}
            >
              <CloseIcon />
              <span>{copy.close}</span>
            </button>
          </>}
        />
      </div>
      {fullscreen ? (
        <span
          data-openpencil-focus-guard="true"
          style={styles.focusGuard}
          tabIndex={0}
          onFocus={() => {
            if (surfaceRef.current) focusEditorWorkbenchBoundary(surfaceRef.current, false)
          }}
        />
      ) : null}
    </section>
  )

  return createPortal(surface, document.body)
}
