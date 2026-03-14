import type * as fabric from 'fabric'

// ---------------------------------------------------------------------------
// Canvas Zoom/Pan Cache
//
// During continuous zoom or pan, instead of re-rendering 200+ Fabric objects
// per frame (~10-15ms each), we capture a snapshot of the canvas pixels and
// apply the viewport delta as a single drawImage transform (~0.1ms).
//
// This is the same technique used by Figma, Pencil, Google Maps, etc.:
//   1. On first viewport change: capture current rendered pixels
//   2. On each subsequent change: draw cached bitmap with relative transform
//   3. When viewport stops changing (debounce): restore full Fabric rendering
//
// The visual trade-off is a slight pixelation during zoom (stretched bitmap)
// that resolves instantly when zoom stops. The performance gain is massive:
// zoom/pan becomes O(1) instead of O(objects).
// ---------------------------------------------------------------------------

let snapshotCanvas: HTMLCanvasElement | null = null
let cachedVpt: number[] | null = null
let isActive = false
let origRenderAll: (() => void) | null = null
let deactivateTimer: number | null = null
let targetCanvas: fabric.Canvas | null = null
let bgColor = '#1e1e1e'
let lastSnapshotTime = 0
const SNAPSHOT_REFRESH_MS = 200

/**
 * Activate the zoom/pan cache. Captures current canvas pixels and overrides
 * Fabric's renderAll to draw the cached bitmap with viewport delta transform.
 *
 * Safe to call on every wheel/pan event — returns immediately if already active.
 */
export function activateZoomCache(canvas: fabric.Canvas) {
  if (isActive) return

  targetCanvas = canvas
  const src = canvas.lowerCanvasEl

  // Synchronous pixel snapshot
  snapshotCanvas = document.createElement('canvas')
  snapshotCanvas.width = src.width
  snapshotCanvas.height = src.height
  const snapCtx = snapshotCanvas.getContext('2d')!
  snapCtx.drawImage(src, 0, 0)

  // Freeze the viewport transform at snapshot time
  cachedVpt = [...canvas.viewportTransform]
  bgColor = (canvas.backgroundColor as string) || '#1e1e1e'

  // Save and override renderAll
  origRenderAll = canvas.renderAll.bind(canvas)
  lastSnapshotTime = performance.now()

  canvas.renderAll = function (this: fabric.Canvas) {
    if (!isActive || !snapshotCanvas || !cachedVpt) {
      return origRenderAll!()
    }

    // Cancel any pending requestRenderAll to match normal Fabric behavior
    if ((this as any).nextRenderHandle) {
      cancelAnimationFrame((this as any).nextRenderHandle)
      ;(this as any).nextRenderHandle = 0
    }

    const ctx = (this as any).contextContainer as CanvasRenderingContext2D
    const pw = this.lowerCanvasEl.width
    const ph = this.lowerCanvasEl.height
    const dpr = pw / this.width // effective device pixel ratio

    // Work in raw pixel space
    ctx.setTransform(1, 0, 0, 1, 0, 0)
    ctx.clearRect(0, 0, pw, ph)

    // Background
    ctx.fillStyle = bgColor
    ctx.fillRect(0, 0, pw, ph)

    // Compute relative transform in pixel space:
    //   cached pixel position:  P_cached = P_scene * oldZoom * dpr + oldPan * dpr
    //   desired pixel position: P_new    = P_scene * newZoom * dpr + newPan * dpr
    // Therefore:
    //   P_new = P_cached * scaleRatio + (newPan - oldPan * scaleRatio) * dpr
    const vpt = this.viewportTransform
    const scaleRatio = vpt[0] / cachedVpt![0]
    const tx = (vpt[4] - cachedVpt![4] * scaleRatio) * dpr
    const ty = (vpt[5] - cachedVpt![5] * scaleRatio) * dpr

    ctx.save()
    ctx.translate(tx, ty)
    ctx.scale(scaleRatio, scaleRatio)
    ctx.drawImage(snapshotCanvas!, 0, 0)
    ctx.restore()

    // Restore DPR transform for subsequent Fabric operations
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0)

    // Clear upper canvas (selection handles are irrelevant during zoom)
    const topCtx = (this as any).contextTop as CanvasRenderingContext2D
    if (topCtx) {
      topCtx.setTransform(1, 0, 0, 1, 0, 0)
      topCtx.clearRect(0, 0, pw, ph)
      topCtx.setTransform(dpr, 0, 0, dpr, 0, 0)
    }
  }

  isActive = true
}

/**
 * Re-capture a fresh snapshot during active zoom/pan so the cached bitmap
 * stays reasonably up-to-date during long drags. Does a single real render,
 * re-snapshots, then continues cached mode. No-op if called too soon after
 * the last snapshot (throttled to SNAPSHOT_REFRESH_MS).
 */
export function refreshZoomCacheIfNeeded() {
  if (!isActive || !targetCanvas || !origRenderAll) return

  const now = performance.now()
  if (now - lastSnapshotTime < SNAPSHOT_REFRESH_MS) return
  lastSnapshotTime = now

  // One real Fabric render at current viewport
  origRenderAll()

  // Re-capture pixels
  const src = targetCanvas.lowerCanvasEl
  snapshotCanvas = document.createElement('canvas')
  snapshotCanvas.width = src.width
  snapshotCanvas.height = src.height
  const snapCtx = snapshotCanvas.getContext('2d')!
  snapCtx.drawImage(src, 0, 0)

  // Update frozen viewport baseline
  cachedVpt = [...targetCanvas.viewportTransform]
  bgColor = (targetCanvas.backgroundColor as string) || '#1e1e1e'
}

/**
 * Deactivate the cache: restore normal Fabric rendering and trigger a
 * full-quality render at the current viewport.
 */
function deactivateZoomCache() {
  if (!isActive || !targetCanvas) return
  isActive = false

  if (origRenderAll) {
    targetCanvas.renderAll = origRenderAll
    origRenderAll = null
  }

  snapshotCanvas = null
  cachedVpt = null

  // Full-quality render at final viewport position
  targetCanvas.requestRenderAll()
  targetCanvas = null
}

/**
 * Schedule cache deactivation after the viewport stops changing.
 * Call on every wheel/pan event to reset the debounce timer.
 */
export function scheduleZoomCacheEnd() {
  if (deactivateTimer !== null) {
    clearTimeout(deactivateTimer)
  }
  deactivateTimer = window.setTimeout(() => {
    deactivateTimer = null
    deactivateZoomCache()
  }, 150)
}

/** Whether zoom/pan cache is currently active. */
export function isZoomCaching(): boolean {
  return isActive
}
