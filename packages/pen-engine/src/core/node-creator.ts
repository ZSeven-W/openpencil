import type { PenNode, ToolType } from '@zseven-w/pen-types';
import {
  generateId,
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  DEFAULT_FRAME_FILL,
  DEFAULT_TEXT_FILL,
} from '@zseven-w/pen-core';

/**
 * Create a default PenNode for the given tool type.
 * Extracted from apps/web/src/canvas/canvas-node-creator.ts.
 * No DOM/React dependencies.
 */
export function createNodeForTool(
  tool: ToolType,
  x: number,
  y: number,
  width: number,
  height: number,
): PenNode | null {
  const id = generateId();

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
      } as PenNode;
    case 'frame':
      return {
        id,
        type: 'frame',
        name: 'Frame',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        fill: [{ type: 'solid', color: DEFAULT_FRAME_FILL }],
        children: [],
      } as PenNode;
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
      } as PenNode;
    case 'polygon':
      return {
        id,
        type: 'polygon',
        name: 'Polygon',
        x,
        y,
        width: Math.abs(width),
        height: Math.abs(height),
        polygonCount: 3,
        fill: [{ type: 'solid', color: DEFAULT_FILL }],
        stroke: {
          thickness: DEFAULT_STROKE_WIDTH,
          fill: [{ type: 'solid', color: DEFAULT_STROKE }],
        },
      } as PenNode;
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
      } as PenNode;
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
        fill: [{ type: 'solid', color: DEFAULT_TEXT_FILL }],
      } as PenNode;
    default:
      return null;
  }
}

/** Whether a tool draws shapes (everything except select/hand). */
export function isDrawingTool(tool: ToolType): boolean {
  return tool !== 'select' && tool !== 'hand';
}
