export type ToolType =
  | 'select'
  | 'frame'
  | 'rectangle'
  | 'ellipse'
  | 'line'
  | 'polygon'
  | 'path'
  | 'text'
  | 'hand'

export interface ViewportState {
  zoom: number
  panX: number
  panY: number
}

export interface SelectionState {
  selectedIds: string[]
  activeId: string | null
}

export interface CanvasInteraction {
  isDrawing: boolean
  isPanning: boolean
  isDragging: boolean
  drawStartPoint: { x: number; y: number } | null
}
