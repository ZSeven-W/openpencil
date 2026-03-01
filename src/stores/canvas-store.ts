import { create } from 'zustand'
import type { Canvas } from 'fabric'
import type {
  ToolType,
  ViewportState,
  SelectionState,
  CanvasInteraction,
} from '@/types/canvas'
import type { PenNode } from '@/types/pen'
import { DEFAULT_PAGE_ID } from '@/stores/document-tree-utils'

interface CanvasStoreState {
  activeTool: ToolType
  viewport: ViewportState
  selection: SelectionState
  interaction: CanvasInteraction
  fabricCanvas: Canvas | null
  clipboard: PenNode[]
  layerPanelOpen: boolean
  variablesPanelOpen: boolean
  figmaImportDialogOpen: boolean
  activePageId: string | null

  setActiveTool: (tool: ToolType) => void
  setZoom: (zoom: number) => void
  setPan: (x: number, y: number) => void
  setSelection: (ids: string[], activeId: string | null) => void
  clearSelection: () => void
  setHoveredId: (id: string | null) => void
  enterFrame: (frameId: string) => void
  exitFrame: () => void
  exitAllFrames: () => void
  setInteraction: (partial: Partial<CanvasInteraction>) => void
  setFabricCanvas: (canvas: Canvas | null) => void
  setClipboard: (nodes: PenNode[]) => void
  toggleLayerPanel: () => void
  toggleVariablesPanel: () => void
  setFigmaImportDialogOpen: (open: boolean) => void
  setActivePageId: (pageId: string | null) => void
}

export const useCanvasStore = create<CanvasStoreState>((set) => ({
  activeTool: 'select',
  viewport: { zoom: 1, panX: 0, panY: 0 },
  selection: { selectedIds: [], activeId: null, hoveredId: null, enteredFrameId: null, enteredFrameStack: [] },
  interaction: {
    isDrawing: false,
    isPanning: false,
    isDragging: false,
    drawStartPoint: null,
  },
  fabricCanvas: null,
  clipboard: [],
  layerPanelOpen: true,
  variablesPanelOpen: false,
  figmaImportDialogOpen: false,
  activePageId: DEFAULT_PAGE_ID,

  setActiveTool: (tool) => set({ activeTool: tool }),

  setZoom: (zoom) =>
    set((s) => ({ viewport: { ...s.viewport, zoom } })),

  setPan: (panX, panY) =>
    set((s) => ({ viewport: { ...s.viewport, panX, panY } })),

  setSelection: (selectedIds, activeId) =>
    set((s) => ({ selection: { ...s.selection, selectedIds, activeId } })),

  clearSelection: () =>
    set((s) => ({ selection: { ...s.selection, selectedIds: [], activeId: null } })),

  setHoveredId: (hoveredId) =>
    set((s) => ({ selection: { ...s.selection, hoveredId } })),

  enterFrame: (frameId) =>
    set((s) => ({
      selection: {
        ...s.selection,
        enteredFrameId: frameId,
        enteredFrameStack: [...s.selection.enteredFrameStack, frameId],
        hoveredId: null,
        selectedIds: [],
        activeId: null,
      },
    })),

  exitFrame: () =>
    set((s) => {
      const stack = s.selection.enteredFrameStack.slice(0, -1)
      return {
        selection: {
          ...s.selection,
          enteredFrameId: stack[stack.length - 1] ?? null,
          enteredFrameStack: stack,
          hoveredId: null,
          selectedIds: [],
          activeId: null,
        },
      }
    }),

  exitAllFrames: () =>
    set((s) => ({
      selection: {
        ...s.selection,
        enteredFrameId: null,
        enteredFrameStack: [],
        hoveredId: null,
        selectedIds: [],
        activeId: null,
      },
    })),

  setInteraction: (partial) =>
    set((s) => ({ interaction: { ...s.interaction, ...partial } })),

  setFabricCanvas: (fabricCanvas) => set({ fabricCanvas }),

  setClipboard: (clipboard) => set({ clipboard }),

  toggleLayerPanel: () => set((s) => ({ layerPanelOpen: !s.layerPanelOpen })),
  toggleVariablesPanel: () => set((s) => ({ variablesPanelOpen: !s.variablesPanelOpen })),
  setFigmaImportDialogOpen: (open) => set({ figmaImportDialogOpen: open }),
  setActivePageId: (activePageId) => set({ activePageId }),
}))
