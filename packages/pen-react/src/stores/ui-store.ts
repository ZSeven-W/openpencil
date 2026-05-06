import { create } from 'zustand';

/**
 * Pure UI 声明
 * pen-react 拥有：面板可见性、拖动交互和其他短暂的 UI 问题。 This 是 NOT 引擎状态 —
 * 笔引擎中的工具、选择、视口和文档状态。
 */

export interface UIStoreState {
  layerPanelOpen: boolean;
  rightPanelTab: 'design' | 'code';
  codePanelOpen: boolean;

  // Layer 面板拖动状态
  layerDragId: string | null;
  layerDragOverId: string | null;
  layerDropPosition: 'above' | 'below' | 'inside' | null;

  // Layer 面板折叠节点
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
