import { create } from 'zustand';

interface CloudVersionPanelState {
  open: boolean;
  setOpen: (open: boolean) => void;
  toggle: () => void;
}

export const useCloudVersionPanelStore = create<CloudVersionPanelState>((set, get) => ({
  open: false,
  setOpen: (open) => set({ open }),
  toggle: () => set({ open: !get().open }),
}));
