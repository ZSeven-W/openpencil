import * as fabric from 'fabric'

function rotationCursorSvg(angleDeg: number): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24"><g transform="rotate(${angleDeg} 12 12)"><path d="M12 5a7 7 0 0 1 7 7" fill="none" stroke="%23111" stroke-width="1.5" stroke-linecap="round"/><polyline points="16 4 19 7.5 15 8" fill="none" stroke="%23111" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/></g></svg>`
  return `url("data:image/svg+xml,${svg}") 12 12, crosshair`
}

const CURSORS = {
  tl: rotationCursorSvg(-45),
  tr: rotationCursorSvg(45),
  br: rotationCursorSvg(135),
  bl: rotationCursorSvg(225),
}

const ROTATION_POSITIONS = [
  { key: 'rtl', x: -0.5, y: -0.5, ox: -10, oy: -10, cursor: CURSORS.tl },
  { key: 'rtr', x: 0.5, y: -0.5, ox: 10, oy: -10, cursor: CURSORS.tr },
  { key: 'rbr', x: 0.5, y: 0.5, ox: 10, oy: 10, cursor: CURSORS.br },
  { key: 'rbl', x: -0.5, y: 0.5, ox: -10, oy: 10, cursor: CURSORS.bl },
]

export function applyRotationControls(obj: fabric.FabricObject) {
  for (const pos of ROTATION_POSITIONS) {
    obj.controls[pos.key] = new fabric.Control({
      x: pos.x,
      y: pos.y,
      offsetX: pos.ox,
      offsetY: pos.oy,
      sizeX: 20,
      sizeY: 20,
      actionName: 'rotate',
      actionHandler: fabric.controlsUtils.rotationWithSnapping,
      cursorStyleHandler: () => pos.cursor,
      render: () => {},
    })
  }
}
