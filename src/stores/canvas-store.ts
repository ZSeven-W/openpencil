import { create } from 'zustand'
import type { Canvas } from 'fabric'
import type {
  ToolType,
  ViewportState,
  SelectionState,
  CanvasInteraction,
} from '@/types/canvas'
import type { PenNode } from '@/types/pen'

interface CanvasStoreState {
  activeTool: ToolType
  viewport: ViewportState
  selection: SelectionState
  interaction: CanvasInteraction
  fabricCanvas: Canvas | null
  clipboard: PenNode[]

  setActiveTool: (tool: ToolType) => void
  setZoom: (zoom: number) => void
  setPan: (x: number, y: number) => void
  setSelection: (ids: string[], activeId: string | null) => void
  clearSelection: () => void
  setInteraction: (partial: Partial<CanvasInteraction>) => void
  setFabricCanvas: (canvas: Canvas | null) => void
  setClipboard: (nodes: PenNode[]) => void
}

export const useCanvasStore = create<CanvasStoreState>((set) => ({
  activeTool: 'select',
  viewport: { zoom: 1, panX: 0, panY: 0 },
  selection: { selectedIds: [], activeId: null },
  interaction: {
    isDrawing: false,
    isPanning: false,
    isDragging: false,
    drawStartPoint: null,
  },
  fabricCanvas: null,
  clipboard: [],

  setActiveTool: (tool) => set({ activeTool: tool }),

  setZoom: (zoom) =>
    set((s) => ({ viewport: { ...s.viewport, zoom } })),

  setPan: (panX, panY) =>
    set((s) => ({ viewport: { ...s.viewport, panX, panY } })),

  setSelection: (selectedIds, activeId) =>
    set({ selection: { selectedIds, activeId } }),

  clearSelection: () =>
    set({ selection: { selectedIds: [], activeId: null } }),

  setInteraction: (partial) =>
    set((s) => ({ interaction: { ...s.interaction, ...partial } })),

  setFabricCanvas: (fabricCanvas) => set({ fabricCanvas }),

  setClipboard: (clipboard) => set({ clipboard }),
}))
