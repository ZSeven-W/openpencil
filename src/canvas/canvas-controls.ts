import * as fabric from 'fabric'

function rotationCursorSvg(angleDeg: number): string {
  // 270° clockwise arc (radius 4) with small arrowhead — Figma-style minimal.
  // Uses single quotes so the SVG doesn't break the outer CSS url("...").
  const svg = `<svg xmlns='http://www.w3.org/2000/svg' width='24' height='24' viewBox='0 0 24 24'><g transform='rotate(${angleDeg} 12 12)' fill='none' stroke='%23333' stroke-width='1.5' stroke-linecap='round' stroke-linejoin='round'><path d='M16 12 A4 4 0 1 1 12 8'/><polyline points='10 6.5 12 8 10 9.5'/></g></svg>`
  return `url("data:image/svg+xml,${svg}") 12 12, crosshair`
}

const CURSORS = {
  tl: rotationCursorSvg(-45),
  tr: rotationCursorSvg(45),
  br: rotationCursorSvg(135),
  bl: rotationCursorSvg(225),
}

const ROTATION_OFFSET = 14
const ROTATION_SIZE = 14

const ROTATION_POSITIONS = [
  { key: 'rtl', x: -0.5, y: -0.5, ox: -ROTATION_OFFSET, oy: -ROTATION_OFFSET, cursor: CURSORS.tl },
  { key: 'rtr', x: 0.5, y: -0.5, ox: ROTATION_OFFSET, oy: -ROTATION_OFFSET, cursor: CURSORS.tr },
  { key: 'rbr', x: 0.5, y: 0.5, ox: ROTATION_OFFSET, oy: ROTATION_OFFSET, cursor: CURSORS.br },
  { key: 'rbl', x: -0.5, y: 0.5, ox: -ROTATION_OFFSET, oy: ROTATION_OFFSET, cursor: CURSORS.bl },
]

export function applyRotationControls(obj: fabric.FabricObject) {
  for (const pos of ROTATION_POSITIONS) {
    obj.controls[pos.key] = new fabric.Control({
      x: pos.x,
      y: pos.y,
      offsetX: pos.ox,
      offsetY: pos.oy,
      sizeX: ROTATION_SIZE,
      sizeY: ROTATION_SIZE,
      actionName: 'rotate',
      actionHandler: fabric.controlsUtils.rotationWithSnapping,
      cursorStyleHandler: () => pos.cursor,
      render: () => {},
    })
  }
}

/**
 * Fabric.js `_setCursorFromEvent` only checks controls on the `target` found
 * by `findTarget` (the object directly under the mouse). Rotation controls
 * are offset outside the object boundary, so `findTarget` returns null there
 * and the rotation cursor never shows.
 *
 * Fix: patch `_setCursorFromEvent` to also check the active object's controls
 * before falling through to the default behavior.
 */
export function setupRotationCursorHandler(canvas: fabric.Canvas) {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const c = canvas as any
  const original = c._setCursorFromEvent

  c._setCursorFromEvent = function (
    e: MouseEvent,
    target: fabric.FabricObject | undefined,
  ) {
    // Check the active object's rotation controls first
    const activeObject = this.getActiveObject()
    if (activeObject) {
      const pointer = this.getViewportPoint(e)
      const found = activeObject.findControl(pointer)
      if (found && found.control.actionName === 'rotate') {
        this.setCursor(
          found.control.cursorStyleHandler(
            e,
            found.control,
            activeObject,
            found.coord,
          ),
        )
        return
      }
    }
    // No rotation control hit — fall through to default behavior
    original.call(this, e, target)
  }
}
