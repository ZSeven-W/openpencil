import { create } from 'zustand';
import type { ToolType, ViewportState, SelectionState, CanvasInteraction } from '@/types/canvas';
import type { PenNode } from '@/types/pen';
import type { DesignEngine } from '@zseven-w/pen-engine';
import { DEFAULT_PAGE_ID } from '@/stores/document-tree-utils';
import { appStorage } from '@/utils/app-storage';

const PREFS_KEY = 'openpencil-canvas-preferences';

export type RightPanelTab = 'design' | 'code';

interface CanvasPreferences {
  layerPanelOpen: boolean;
  variablesPanelOpen: boolean;
  designMdPanelOpen: boolean;
  codePanelOpen: boolean;
  rightPanelTab?: RightPanelTab;
}

interface CanvasStoreState {
  activeTool: ToolType;
  viewport: ViewportState;
  selection: SelectionState;
  interaction: CanvasInteraction;
  clipboard: PenNode[];
  layerPanelOpen: boolean;
  variablesPanelOpen: boolean;
  designMdPanelOpen: boolean;
  codePanelOpen: boolean;
  rightPanelTab: RightPanelTab;
  figmaImportDialogOpen: boolean;
  pendingFigmaFile: File | null;
  exportDialogOpen: boolean;
  activePageId: string | null;

  setActiveTool: (tool: ToolType) => void;
  setZoom: (zoom: number) => void;
  setPan: (x: number, y: number) => void;
  setSelection: (ids: string[], activeId: string | null) => void;
  clearSelection: () => void;
  setHoveredId: (id: string | null) => void;
  enterFrame: (frameId: string) => void;
  exitFrame: () => void;
  exitAllFrames: () => void;
  setInteraction: (partial: Partial<CanvasInteraction>) => void;
  setClipboard: (nodes: PenNode[]) => void;
  toggleLayerPanel: () => void;
  toggleVariablesPanel: () => void;
  toggleDesignMdPanel: () => void;
  toggleCodePanel: () => void;
  setCodePanelOpen: (open: boolean) => void;
  setRightPanelTab: (tab: RightPanelTab) => void;
  setFigmaImportDialogOpen: (open: boolean) => void;
  setPendingFigmaFile: (file: File | null) => void;
  setExportDialogOpen: (open: boolean) => void;
  toggleExportDialog: () => void;
  setActivePageId: (pageId: string | null) => void;
  imageSearchStatuses: Map<string, 'pending' | 'found' | 'failed'>;
  setImageSearchStatus: (nodeId: string, status: 'pending' | 'found' | 'failed') => void;
  clearImageSearchStatuses: () => void;
  hydrate: () => void;
}

function persistPrefs(prefs: CanvasPreferences) {
  try {
    appStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
  } catch {
    /* ignore */
  }
}

export const useCanvasStore = create<CanvasStoreState>((set, get) => ({
  activeTool: 'select',
  viewport: { zoom: 1, panX: 0, panY: 0 },
  selection: {
    selectedIds: [],
    activeId: null,
    hoveredId: null,
    enteredFrameId: null,
    enteredFrameStack: [],
  },
  interaction: {
    isDrawing: false,
    isPanning: false,
    isDragging: false,
    drawStartPoint: null,
  },
  clipboard: [],
  layerPanelOpen: true,
  variablesPanelOpen: false,
  designMdPanelOpen: false,
  codePanelOpen: false,
  rightPanelTab: 'design',
  figmaImportDialogOpen: false,
  pendingFigmaFile: null,
  exportDialogOpen: false,
  activePageId: DEFAULT_PAGE_ID,
  imageSearchStatuses: new Map(),
  setImageSearchStatus: (nodeId, status) =>
    set((s) => {
      const next = new Map(s.imageSearchStatuses);
      next.set(nodeId, status);
      return { imageSearchStatuses: next };
    }),
  clearImageSearchStatuses: () => set({ imageSearchStatuses: new Map() }),

  setActiveTool: (tool) => set({ activeTool: tool }),

  setZoom: (zoom) => set((s) => ({ viewport: { ...s.viewport, zoom } })),

  setPan: (panX, panY) => set((s) => ({ viewport: { ...s.viewport, panX, panY } })),

  setSelection: (selectedIds, activeId) =>
    set((s) => ({ selection: { ...s.selection, selectedIds, activeId } })),

  clearSelection: () =>
    set((s) => ({ selection: { ...s.selection, selectedIds: [], activeId: null } })),

  setHoveredId: (hoveredId) => set((s) => ({ selection: { ...s.selection, hoveredId } })),

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
      const stack = s.selection.enteredFrameStack.slice(0, -1);
      return {
        selection: {
          ...s.selection,
          enteredFrameId: stack[stack.length - 1] ?? null,
          enteredFrameStack: stack,
          hoveredId: null,
          selectedIds: [],
          activeId: null,
        },
      };
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

  setInteraction: (partial) => set((s) => ({ interaction: { ...s.interaction, ...partial } })),

  setClipboard: (clipboard) => set({ clipboard }),

  toggleLayerPanel: () => {
    const next = !get().layerPanelOpen;
    set({ layerPanelOpen: next });
    const { variablesPanelOpen, designMdPanelOpen, codePanelOpen } = get();
    persistPrefs({ layerPanelOpen: next, variablesPanelOpen, designMdPanelOpen, codePanelOpen });
  },
  toggleVariablesPanel: () => {
    const next = !get().variablesPanelOpen;
    set({ variablesPanelOpen: next });
    const { layerPanelOpen, designMdPanelOpen, codePanelOpen } = get();
    persistPrefs({ layerPanelOpen, variablesPanelOpen: next, designMdPanelOpen, codePanelOpen });
  },
  toggleDesignMdPanel: () => {
    const next = !get().designMdPanelOpen;
    set({ designMdPanelOpen: next });
    const { layerPanelOpen, variablesPanelOpen, codePanelOpen } = get();
    persistPrefs({ layerPanelOpen, variablesPanelOpen, designMdPanelOpen: next, codePanelOpen });
  },
  toggleCodePanel: () => {
    const next = !get().codePanelOpen;
    set({ codePanelOpen: next });
    const { layerPanelOpen, variablesPanelOpen, designMdPanelOpen } = get();
    persistPrefs({ layerPanelOpen, variablesPanelOpen, designMdPanelOpen, codePanelOpen: next });
  },
  setCodePanelOpen: (open) => {
    set({ codePanelOpen: open });
    const { layerPanelOpen, variablesPanelOpen, designMdPanelOpen } = get();
    persistPrefs({ layerPanelOpen, variablesPanelOpen, designMdPanelOpen, codePanelOpen: open });
  },
  setRightPanelTab: (tab) => {
    set({ rightPanelTab: tab });
    const { layerPanelOpen, variablesPanelOpen, designMdPanelOpen, codePanelOpen } = get();
    persistPrefs({
      layerPanelOpen,
      variablesPanelOpen,
      designMdPanelOpen,
      codePanelOpen,
      rightPanelTab: tab,
    });
  },
  setFigmaImportDialogOpen: (open) =>
    set({ figmaImportDialogOpen: open, ...(!open && { pendingFigmaFile: null }) }),
  setPendingFigmaFile: (file) => set({ pendingFigmaFile: file }),
  setExportDialogOpen: (open) => set({ exportDialogOpen: open }),
  toggleExportDialog: () => set((s) => ({ exportDialogOpen: !s.exportDialogOpen })),
  setActivePageId: (activePageId) => set({ activePageId }),

  hydrate: () => {
    try {
      const raw = appStorage.getItem(PREFS_KEY);
      if (!raw) return;
      const data = JSON.parse(raw) as Partial<CanvasPreferences>;
      if (typeof data.layerPanelOpen === 'boolean') set({ layerPanelOpen: data.layerPanelOpen });
      if (typeof data.variablesPanelOpen === 'boolean')
        set({ variablesPanelOpen: data.variablesPanelOpen });
      if (typeof data.designMdPanelOpen === 'boolean')
        set({ designMdPanelOpen: data.designMdPanelOpen });
      if (typeof data.codePanelOpen === 'boolean') set({ codePanelOpen: data.codePanelOpen });
      if (data.rightPanelTab === 'design' || data.rightPanelTab === 'code')
        set({ rightPanelTab: data.rightPanelTab });
    } catch {
      /* ignore */
    }
  },
}));

/**
 * Bridge: mirror pen-engine state into canvas-store so that legacy readers
 * (components not yet migrated to pen-react hooks) continue to work.
 *
 * Call this once after DesignEngine is created.
 * Remove this bridge once ALL readers have migrated to pen-react hooks.
 */
export function initCanvasStoreBridge(engine: DesignEngine): () => void {
  const unsubs: Array<() => void> = [];

  unsubs.push(
    engine.on('tool:change', (tool) => {
      useCanvasStore.setState({ activeTool: tool });
    }),
  );

  unsubs.push(
    engine.on('selection:change', (ids) => {
      useCanvasStore.setState((s) => ({
        selection: { ...s.selection, selectedIds: ids },
      }));
    }),
  );

  unsubs.push(
    engine.on('node:hover', (id) => {
      useCanvasStore.setState((s) => ({
        selection: { ...s.selection, hoveredId: id },
      }));
    }),
  );

  return () => unsubs.forEach((u) => u());
}
