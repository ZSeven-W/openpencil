import * as fabric from 'fabric'
import { generateId } from '@/stores/document-store'
import type { PenNode } from '@/types/pen'
import type { ToolType } from '@/types/canvas'
import {
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
} from './canvas-constants'

export function createNodeForTool(
  tool: ToolType,
  x: number,
  y: number,
  width: number,
  height: number,
): PenNode | null {
  const id = generateId()

  switch (tool) {
    case 'rectangle':
      return {
        id,
        type: 'rectangle',
        name: 'Rectangle',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: DEFAULT_FILL }],
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'frame':
      return {
        id,
        type: 'frame',
        name: 'Frame',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: '#ffffff' }],
        children: [],
      }
    case 'ellipse':
      return {
        id,
        type: 'ellipse',
        name: 'Ellipse',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: DEFAULT_FILL }],
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'line':
      return {
        id,
        type: 'line',
        name: 'Line',
        x,
        y,
        x2: x + width,
        y2: y + height,
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      }
    case 'text':
      return {
        id,
        type: 'text',
        name: 'Text',
        x,
        y,
        content: 'Type here',
        fontSize: 16,
        fontFamily: 'Inter, sans-serif',
        fill: [{ type: 'solid', color: '#000000' }],
      }
    default:
      return null
  }
}

export function isDrawingTool(tool: ToolType): boolean {
  return tool !== 'select' && tool !== 'hand'
}

/**
 * Convert a pointer event to scene coordinates using Fabric's own method
 * which correctly handles DPR / retina scaling and viewport transform.
 */
export function toScene(
  canvas: fabric.Canvas,
  e: PointerEvent,
): { x: number; y: number } {
  canvas.calcOffset()
  const point = canvas.getScenePoint(e)
  return { x: point.x, y: point.y }
}
