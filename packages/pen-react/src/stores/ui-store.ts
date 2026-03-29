import { create } from 'zustand';

/**
 * Pure UI state that pen-react owns: panel visibility, drag interactions,
 * and other ephemeral UI concerns. This is NOT engine state — tool, selection,
 * viewport, and document state live in pen-engine.
 */

export interface UIStoreState {
  layerPanelOpen: boolean;
  rightPanelTab: 'design' | 'code';
  codePanelOpen: boolean;

  // Layer panel drag state
  layerDragId: string | null;
  layerDragOverId: string | null;
  layerDropPosition: 'above' | 'below' | 'inside' | null;

  // Layer panel collapsed nodes
  collapsedLayerIds: Set<string>;

  // Actions
  toggleLayerPanel: () => void;
  setRightPanelTab: (tab: 'design' | 'code') => void;
  setCodePanelOpen: (open: boolean) => void;
  setLayerDrag: (
    dragId: string | null,
    overId: string | null,
    position: 'above' | 'below' | 'inside' | null,
  ) => void;
  toggleLayerCollapse: (id: string) => void;
}

export const useUIStore = create<UIStoreState>((set, _get) => ({
  layerPanelOpen: true,
  rightPanelTab: 'design',
  codePanelOpen: false,

  layerDragId: null,
  layerDragOverId: null,
  layerDropPosition: null,

  collapsedLayerIds: new Set<string>(),

  toggleLayerPanel: () => set((s) => ({ layerPanelOpen: !s.layerPanelOpen })),
  setRightPanelTab: (tab) => set({ rightPanelTab: tab }),
  setCodePanelOpen: (open) => set({ codePanelOpen: open }),

  setLayerDrag: (dragId, overId, position) =>
    set({ layerDragId: dragId, layerDragOverId: overId, layerDropPosition: position }),

  toggleLayerCollapse: (id) =>
    set((s) => {
      const next = new Set(s.collapsedLayerIds);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return { collapsedLayerIds: next };
    }),
}));
