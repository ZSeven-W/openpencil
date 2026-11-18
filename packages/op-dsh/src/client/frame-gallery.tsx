import { useCallback, useEffect, useRef, useState } from 'react'
import type { KeyboardEvent as ReactKeyboardEvent } from 'react'

/** Browser capability for one exact top-level OpenPencil frame. */
export interface GalleryFrame {
  path: string
  previewUrl: string
  downloadUrl: string
  width?: number
  height?: number
  id?: string
  name?: string
  index?: number
}

export type GalleryLocale = 'zh' | 'en'

const FRAME_GALLERY_COPY = {
  en: {
    frame: 'Frame',
    carousel: 'carousel',
    gallery: 'OpenPencil frames',
    toolbar: 'Preview zoom and card size controls',
    zoomOut: 'Zoom out preview',
    zoomOutTitle: 'Zoom out by 25% (Ctrl/Cmd −)',
    zoomIn: 'Zoom in preview',
    zoomInTitle: 'Zoom in by 25% (Ctrl/Cmd +)',
    previewZoom: 'Preview zoom',
    reset: 'Reset',
    resetAria: 'Reset preview zoom to 100%',
    resetTitle: 'Reset zoom to 100% (Ctrl/Cmd 0)',
    fitFrame: 'Fit frame',
    fitFrameAria: 'Fit entire frame inside the current card',
    fitFrameTitle: 'Fit the entire frame without changing the card size',
    fitContent: 'Fit content',
    fitContentAria: 'Fit card height to the entire frame',
    fitContentTitle: 'Expand the card to show the entire frame',
    restoreCard: 'Restore card',
    restoreCardAria: 'Restore compact card height',
    previous: 'Previous frame',
    next: 'Next frame',
    failed: 'This frame preview could not be loaded. Choose another frame or use the download action.',
    rendered: 'Rendered OpenPencil frame',
    thumbnails: 'Frame thumbnails',
    showFrame: 'Show frame',
  },
  zh: {
    frame: '页面',
    carousel: '轮播',
    gallery: 'OpenPencil 页面',
    toolbar: '预览缩放与卡片尺寸控制',
    zoomOut: '缩小预览',
    zoomOutTitle: '缩小 25%（Ctrl/Cmd −）',
    zoomIn: '放大预览',
    zoomInTitle: '放大 25%（Ctrl/Cmd +）',
    previewZoom: '预览缩放',
    reset: '重置',
    resetAria: '将预览缩放重置为 100%',
    resetTitle: '重置为 100%（Ctrl/Cmd 0）',
    fitFrame: '适应画面',
    fitFrameAria: '将整个页面缩放到当前卡片内',
    fitFrameTitle: '不改变卡片大小，完整显示当前页面',
    fitContent: '适应内容',
    fitContentAria: '让卡片高度适应完整页面',
    fitContentTitle: '展开卡片以显示完整页面',
    restoreCard: '还原卡片',
    restoreCardAria: '还原紧凑卡片高度',
    previous: '上一页',
    next: '下一页',
    failed: '当前页面预览加载失败，请选择其他页面或使用下载操作。',
    rendered: 'OpenPencil 页面渲染图',
    thumbnails: '页面缩略图',
    showFrame: '显示页面',
  },
} as const

export function frameGalleryCopy(locale: GalleryLocale) {
  return FRAME_GALLERY_COPY[locale]
}

export function normalizeFrameIndex(index: number, length: number): number {
  if (length <= 0) return 0
  return Math.min(length - 1, Math.max(0, Math.trunc(index)))
}

export function frameLabel(frame: GalleryFrame, index: number, locale: GalleryLocale = 'en'): string {
  return frame.name ?? frame.id ?? `${frameGalleryCopy(locale).frame} ${index + 1}`
}

/** Preview zoom limits are intentionally broad enough for detail inspection. */
export const GALLERY_ZOOM_MIN = 0.25
export const GALLERY_ZOOM_MAX = 4
export const GALLERY_ZOOM_STEP = 0.25

export function clampGalleryZoom(zoom: number): number {
  if (!Number.isFinite(zoom)) return 1
  return Math.min(GALLERY_ZOOM_MAX, Math.max(GALLERY_ZOOM_MIN, zoom))
}

/** Move one predictable 25% stop in either direction. */
export function nextGalleryZoom(zoom: number, direction: -1 | 1): number {
  if (Number.isFinite(zoom) && zoom < GALLERY_ZOOM_MIN) return GALLERY_ZOOM_MIN
  if (Number.isFinite(zoom) && zoom > GALLERY_ZOOM_MAX) return GALLERY_ZOOM_MAX
  const current = clampGalleryZoom(zoom)
  const stops = current / GALLERY_ZOOM_STEP
  const nextStop = direction > 0 ? Math.floor(stops + 1e-8) + 1 : Math.ceil(stops - 1e-8) - 1
  return clampGalleryZoom(nextStop * GALLERY_ZOOM_STEP)
}

export function galleryZoomPercent(zoom: number): string {
  const safeZoom = Number.isFinite(zoom) && zoom > 0 ? zoom : 1
  const percent = safeZoom * 100
  return `${percent < 1 ? Math.max(0.1, Math.round(percent * 10) / 10) : Math.round(percent)}%`
}

/** Contain the entire frame inside the current viewport without resizing the card. */
export function calculateGalleryFitViewZoom(
  viewportWidth: number,
  viewportHeight: number,
  contentWidth: number,
  contentHeight: number,
): number {
  if (
    !Number.isFinite(viewportWidth) || viewportWidth <= 0
    || !Number.isFinite(viewportHeight) || viewportHeight <= 0
    || !Number.isFinite(contentWidth) || contentWidth <= 0
    || !Number.isFinite(contentHeight) || contentHeight <= 0
  ) return 1
  return Math.min(GALLERY_ZOOM_MAX, viewportWidth / contentWidth, viewportHeight / contentHeight)
}

export type GalleryZoomCommand = 'in' | 'out' | 'reset'

/** Resolve a keyboard zoom command without reversing direction at either limit. */
export function galleryZoomCommandTarget(zoom: number, command: GalleryZoomCommand): number | undefined {
  if (command === 'reset') return 1
  if (command === 'in') {
    if (zoom >= GALLERY_ZOOM_MAX - 1e-8) return undefined
    return nextGalleryZoom(zoom, 1)
  }
  if (zoom <= GALLERY_ZOOM_MIN + 1e-8) return undefined
  return nextGalleryZoom(zoom, -1)
}

export function galleryZoomShortcut(key: string, modifier: boolean): GalleryZoomCommand | undefined {
  if (!modifier) return undefined
  if (key === '+' || key === '=') return 'in'
  if (key === '-' || key === '_') return 'out'
  if (key === '0') return 'reset'
  return undefined
}

export const GALLERY_COMPACT_MAX_HEIGHT = 560

/** Shared geometry keeps labels and glyphs on one visual center line. */
export const GALLERY_TOOLBAR_CONTROL_HEIGHT = 28

export const GALLERY_TOOLBAR_CONTROL_LAYOUT: Readonly<React.CSSProperties> = Object.freeze({
  display: 'inline-flex',
  alignItems: 'center',
  justifyContent: 'center',
  boxSizing: 'border-box',
  height: GALLERY_TOOLBAR_CONTROL_HEIGHT,
  lineHeight: 1,
  verticalAlign: 'middle',
})

/** Optical correction for CJK labels and +/- glyphs inside the centered control box. */
export const GALLERY_TOOLBAR_CONTROL_CONTENT_LAYOUT: Readonly<React.CSSProperties> = Object.freeze({
  display: 'inline-block',
  lineHeight: 1,
  transform: 'translateY(-1px)',
  pointerEvents: 'none',
})

export function galleryViewportMaxHeight(fitContent: boolean): number | undefined {
  return fitContent ? undefined : GALLERY_COMPACT_MAX_HEIGHT
}

const styles: Record<string, React.CSSProperties> = {
  gallery: { display: 'flex', flexDirection: 'column', gap: 8 },
  mainShell: { display: 'flex', flexDirection: 'column', gap: 8, minWidth: 0 },
  previewShell: { position: 'relative', minWidth: 0 },
  mainViewport: {
    maxHeight: GALLERY_COMPACT_MAX_HEIGHT,
    overflow: 'auto',
    overscrollBehavior: 'contain',
    borderRadius: 6,
    border: '1px solid rgba(128,128,128,0.25)',
    background: 'rgba(128,128,128,0.06)',
  },
  mainImage: { display: 'block', maxWidth: 'none', height: 'auto', margin: '0 auto' },
  zoomToolbar: {
    display: 'flex', alignItems: 'center', flexWrap: 'wrap', gap: 4,
    marginLeft: 'auto', minWidth: 0,
  },
  zoomButton: {
    ...GALLERY_TOOLBAR_CONTROL_LAYOUT,
    minWidth: GALLERY_TOOLBAR_CONTROL_HEIGHT, padding: '0 8px', borderRadius: 5,
    border: '1px solid var(--ui-border, rgba(128,128,128,0.35))',
    color: 'var(--ui-text, inherit)', background: 'var(--ui-card-bg, rgba(128,128,128,0.08))', cursor: 'pointer',
    fontFamily: 'inherit', fontWeight: 'inherit', fontStyle: 'inherit',
    fontSize: 12, lineHeight: 1, whiteSpace: 'nowrap',
  },
  controlContent: GALLERY_TOOLBAR_CONTROL_CONTENT_LAYOUT,
  zoomPercent: {
    ...GALLERY_TOOLBAR_CONTROL_LAYOUT,
    minWidth: 42, padding: '0 3px', textAlign: 'center',
    fontSize: 11, fontVariantNumeric: 'tabular-nums', lineHeight: 1,
  },
  counter: {
    position: 'absolute', right: 9, top: 9,
    padding: '3px 7px', borderRadius: 99,
    color: '#fff', background: 'rgba(15,15,18,0.72)',
    fontSize: 11, lineHeight: 1.3, pointerEvents: 'none',
    backdropFilter: 'blur(4px)',
  },
  controls: {
    display: 'flex', alignItems: 'center', flexWrap: 'wrap', minWidth: 0, gap: 7,
    fontSize: 12, color: 'var(--ui-text-muted, #888)',
  },
  currentName: {
    flex: '1 1 120px', minWidth: 0, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap',
  },
  arrow: {
    ...GALLERY_TOOLBAR_CONTROL_LAYOUT,
    width: GALLERY_TOOLBAR_CONTROL_HEIGHT, minWidth: GALLERY_TOOLBAR_CONTROL_HEIGHT, padding: 0, borderRadius: 99,
    border: '1px solid var(--ui-border, rgba(128,128,128,0.35))',
    color: 'var(--ui-text, inherit)', background: 'var(--ui-card-bg, rgba(128,128,128,0.08))',
    cursor: 'pointer', fontFamily: 'inherit', fontWeight: 'inherit', fontStyle: 'inherit', fontSize: 20, lineHeight: 1,
  },
  strip: {
    display: 'flex', gap: 8, minWidth: 0, overflowX: 'auto', overflowY: 'hidden',
    padding: '1px 1px 7px', scrollSnapType: 'x proximity', scrollbarWidth: 'thin',
    overscrollBehaviorX: 'contain',
  },
  thumbnail: {
    flex: '0 0 112px', width: 112, height: 84, padding: 3,
    overflow: 'hidden', scrollSnapAlign: 'start',
    borderRadius: 7, border: '1px solid rgba(128,128,128,0.3)',
    background: 'rgba(128,128,128,0.06)', cursor: 'pointer',
  },
  thumbnailSelected: {
    border: '2px solid var(--ui-accent, #0ea5e9)', padding: 2,
    boxShadow: '0 0 0 1px color-mix(in srgb, var(--ui-accent, #0ea5e9) 28%, transparent)',
  },
  thumbnailImage: { display: 'block', width: '100%', height: '100%', objectFit: 'contain', borderRadius: 4 },
  failure: {
    minHeight: 128, display: 'flex', alignItems: 'center', justifyContent: 'center',
    padding: 18, color: 'var(--ui-text-muted, #888)', fontSize: 12, textAlign: 'center',
  },
}

export interface FrameGalleryProps {
  frames: readonly GalleryFrame[]
  selectedIndex: number
  onSelect(index: number): void
  locale: GalleryLocale
}

interface GalleryViewportSize {
  width: number
  height: number
}

/** Large selected preview plus a horizontally-scrollable thumbnail rail. */
export function FrameGallery({ frames, selectedIndex, onSelect, locale }: FrameGalleryProps) {
  const stripRef = useRef<HTMLDivElement>(null)
  const viewportRef = useRef<HTMLDivElement>(null)
  const thumbnailRefs = useRef<Array<HTMLButtonElement | null>>([])
  const [failedUrls, setFailedUrls] = useState<ReadonlySet<string>>(() => new Set())
  const [manualZoom, setManualZoom] = useState(1)
  const [zoomMode, setZoomMode] = useState<'manual' | 'fit-view'>('manual')
  const [fitContent, setFitContent] = useState(false)
  const [viewportSize, setViewportSize] = useState<GalleryViewportSize>({ width: 0, height: 0 })
  const [loadedDimensions, setLoadedDimensions] = useState<Readonly<Record<string, { width: number, height: number }>>>({})
  const currentIndex = normalizeFrameIndex(selectedIndex, frames.length)
  const current = frames[currentIndex]

  useEffect(() => {
    setFailedUrls(new Set())
  }, [frames.map(frame => frame.previewUrl).join('\n')])

  useEffect(() => {
    const viewport = viewportRef.current
    if (viewport === null) return
    const measure = (): void => {
      const next = { width: viewport.clientWidth, height: viewport.clientHeight }
      setViewportSize(previous => previous.width === next.width && previous.height === next.height ? previous : next)
    }
    measure()
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', measure)
      return () => { window.removeEventListener('resize', measure) }
    }
    const observer = new ResizeObserver(measure)
    observer.observe(viewport)
    return () => { observer.disconnect() }
  }, [])

  const select = useCallback((index: number) => {
    const next = normalizeFrameIndex(index, frames.length)
    onSelect(next)
    requestAnimationFrame(() => {
      const strip = stripRef.current
      const item = thumbnailRefs.current[next]
      if (strip === null || item === null || item === undefined) return
      const left = item.offsetLeft - (strip.clientWidth - item.offsetWidth) / 2
      strip.scrollTo({ left: Math.max(0, left), behavior: 'smooth' })
    })
  }, [frames.length, onSelect])

  useEffect(() => {
    viewportRef.current?.scrollTo({ left: 0, top: 0 })
  }, [current?.previewUrl])

  if (current === undefined) return null
  const copy = frameGalleryCopy(locale)
  const failed = failedUrls.has(current.previewUrl)
  const name = frameLabel(current, currentIndex, locale)
  const loaded = loadedDimensions[current.previewUrl]
  const contentWidth = current.width ?? loaded?.width ?? 0
  const contentHeight = current.height ?? loaded?.height ?? 0
  const fitViewZoom = calculateGalleryFitViewZoom(
    viewportSize.width,
    zoomMode === 'fit-view' ? GALLERY_COMPACT_MAX_HEIGHT : viewportSize.height,
    contentWidth,
    contentHeight,
  )
  const zoom = zoomMode === 'fit-view' ? fitViewZoom : manualZoom
  const zoomLabel = galleryZoomPercent(zoom)
  const canZoomOut = zoom > GALLERY_ZOOM_MIN + 1e-8
  const canZoomIn = zoom < GALLERY_ZOOM_MAX - 1e-8

  const setZoom = (nextZoom: number): void => {
    setManualZoom(clampGalleryZoom(nextZoom))
    setZoomMode('manual')
  }

  const resetZoom = (): void => {
    setZoom(1)
    viewportRef.current?.scrollTo({ left: 0, top: 0 })
  }

  const onKeyDown = (event: ReactKeyboardEvent<HTMLDivElement>): void => {
    const command = galleryZoomShortcut(event.key, event.metaKey || event.ctrlKey)
    if (command !== undefined) {
      event.preventDefault()
      if (command === 'reset') {
        resetZoom()
      } else {
        const target = galleryZoomCommandTarget(zoom, command)
        if (target !== undefined) setZoom(target)
      }
      return
    }
    if (event.key === 'ArrowLeft' && currentIndex > 0) {
      event.preventDefault()
      select(currentIndex - 1)
    } else if (event.key === 'ArrowRight' && currentIndex < frames.length - 1) {
      event.preventDefault()
      select(currentIndex + 1)
    }
  }

  return (
    <div
      style={styles.gallery}
      role="region"
      aria-roledescription={copy.carousel}
      aria-label={`${copy.gallery}: ${frames.length}`}
      data-openpencil-frame-gallery="true"
    >
      <div style={styles.mainShell} tabIndex={0} onKeyDown={onKeyDown}>
        <div style={styles.controls}>
          <span style={styles.currentName} title={name}>
            {frames.length > 1 ? `${currentIndex + 1} / ${frames.length} · ` : ''}{name}
          </span>
          <div
            style={styles.zoomToolbar}
            role="toolbar"
            aria-label={copy.toolbar}
            data-openpencil-zoom-toolbar="true"
          >
            <button
              type="button"
              style={{ ...styles.zoomButton, opacity: canZoomOut ? 1 : 0.42 }}
              disabled={!canZoomOut}
              aria-label={copy.zoomOut}
              title={copy.zoomOutTitle}
              onClick={() => { setZoom(nextGalleryZoom(zoom, -1)) }}
            ><span style={styles.controlContent}>−</span></button>
            <output style={styles.zoomPercent} aria-label={`${copy.previewZoom} ${zoomLabel}`} aria-live="polite">
              <span style={styles.controlContent}>{zoomLabel}</span>
            </output>
            <button
              type="button"
              style={{ ...styles.zoomButton, opacity: canZoomIn ? 1 : 0.42 }}
              disabled={!canZoomIn}
              aria-label={copy.zoomIn}
              title={copy.zoomInTitle}
              onClick={() => { setZoom(nextGalleryZoom(zoom, 1)) }}
            ><span style={styles.controlContent}>+</span></button>
            <button
              type="button"
              style={{ ...styles.zoomButton, opacity: zoomMode === 'manual' && manualZoom === 1 ? 0.42 : 1 }}
              disabled={zoomMode === 'manual' && manualZoom === 1}
              aria-label={copy.resetAria}
              title={copy.resetTitle}
              onClick={resetZoom}
            ><span style={styles.controlContent}>{copy.reset}</span></button>
            <button
              type="button"
              style={{
                ...styles.zoomButton,
                background: zoomMode === 'fit-view'
                  ? 'color-mix(in srgb, var(--ui-accent, #0ea5e9) 18%, transparent)'
                  : styles.zoomButton.background,
              }}
              aria-label={copy.fitFrameAria}
              aria-pressed={zoomMode === 'fit-view'}
              title={copy.fitFrameTitle}
              onClick={() => {
                const viewport = viewportRef.current
                if (viewport !== null) {
                  setViewportSize({ width: viewport.clientWidth, height: GALLERY_COMPACT_MAX_HEIGHT })
                }
                setFitContent(false)
                setZoomMode('fit-view')
                viewport?.scrollTo({ left: 0, top: 0 })
              }}
              data-openpencil-fit-view="true"
            ><span style={styles.controlContent}>{copy.fitFrame}</span></button>
            <button
              type="button"
              style={{
                ...styles.zoomButton,
                background: fitContent ? 'color-mix(in srgb, var(--ui-accent, #0ea5e9) 18%, transparent)' : styles.zoomButton.background,
              }}
              aria-label={fitContent ? copy.restoreCardAria : copy.fitContentAria}
              aria-pressed={fitContent}
              title={fitContent
                ? locale === 'zh'
                  ? `${copy.restoreCardAria}（${GALLERY_COMPACT_MAX_HEIGHT}px）`
                  : `${copy.restoreCardAria} (${GALLERY_COMPACT_MAX_HEIGHT}px)`
                : copy.fitContentTitle}
              onClick={() => {
                setZoomMode('manual')
                setFitContent(previous => !previous)
                viewportRef.current?.scrollTo({ left: 0, top: 0 })
              }}
              data-openpencil-card-height-toggle="true"
            ><span style={styles.controlContent}>{fitContent ? copy.restoreCard : copy.fitContent}</span></button>
          </div>
          {frames.length > 1 ? (
            <>
              <button
                type="button"
                style={{ ...styles.arrow, opacity: currentIndex === 0 ? 0.42 : 1 }}
                disabled={currentIndex === 0}
                aria-label={copy.previous}
                title={copy.previous}
                onClick={() => { select(currentIndex - 1) }}
              ><span style={styles.controlContent}>‹</span></button>
              <button
                type="button"
                style={{ ...styles.arrow, opacity: currentIndex === frames.length - 1 ? 0.42 : 1 }}
                disabled={currentIndex === frames.length - 1}
                aria-label={copy.next}
                title={copy.next}
                onClick={() => { select(currentIndex + 1) }}
              ><span style={styles.controlContent}>›</span></button>
            </>
          ) : null}
        </div>
        <div style={styles.previewShell}>
          <div
            ref={viewportRef}
            style={{
              ...styles.mainViewport,
              display: zoomMode === 'fit-view' ? 'flex' : undefined,
              alignItems: zoomMode === 'fit-view' ? 'center' : undefined,
              justifyContent: zoomMode === 'fit-view' ? 'center' : undefined,
              height: zoomMode === 'fit-view' ? GALLERY_COMPACT_MAX_HEIGHT : undefined,
              maxHeight: galleryViewportMaxHeight(fitContent),
              overflow: zoomMode === 'fit-view' ? 'hidden' : styles.mainViewport.overflow,
            }}
            data-openpencil-image-viewport="true"
            data-card-height-mode={fitContent ? 'content' : 'compact'}
            data-preview-zoom-mode={zoomMode}
          >
            {failed ? (
              <div style={styles.failure} role="status">{copy.failed}</div>
            ) : (
              <img
                style={{
                  ...styles.mainImage,
                  width: contentWidth > 0 ? contentWidth * zoom : 'auto',
                }}
                src={current.previewUrl}
                alt={`${copy.rendered}: ${name}`}
                loading="lazy"
                data-openpencil-preview-zoom={zoomLabel}
                onLoad={(event) => {
                  if (current.width !== undefined && current.height !== undefined) return
                  const image = event.currentTarget
                  if (image.naturalWidth <= 0 || image.naturalHeight <= 0) return
                  setLoadedDimensions(previous => ({
                    ...previous,
                    [current.previewUrl]: { width: image.naturalWidth, height: image.naturalHeight },
                  }))
                }}
                onError={() => {
                  setFailedUrls(previous => new Set([...previous, current.previewUrl]))
                }}
              />
            )}
          </div>
          {frames.length > 1 ? <span style={styles.counter}>{currentIndex + 1} / {frames.length}</span> : null}
        </div>
      </div>
      {frames.length > 1 ? (
        <div ref={stripRef} style={styles.strip} aria-label={copy.thumbnails} data-openpencil-frame-strip="true">
            {frames.map((frame, index) => {
              const selected = index === currentIndex
              const label = frameLabel(frame, index, locale)
              return (
                <button
                  key={`${frame.previewUrl}:${index}`}
                  ref={(element) => { thumbnailRefs.current[index] = element }}
                  type="button"
                  style={{ ...styles.thumbnail, ...(selected ? styles.thumbnailSelected : {}) }}
                  aria-label={`${copy.showFrame} ${index + 1}: ${label}`}
                  aria-current={selected ? 'true' : undefined}
                  title={label}
                  onClick={() => { select(index) }}
                >
                  <img style={styles.thumbnailImage} src={frame.previewUrl} alt="" loading="lazy" />
                </button>
              )
            })}
        </div>
      ) : null}
    </div>
  )
}
