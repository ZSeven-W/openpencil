import { create } from 'zustand'
import type {
  PlaybackMode,
  CompositionSettings,
} from '@/types/animation'
import { getDocumentComposition, setDocumentComposition } from '@/stores/composition-accessors'

// --- Editor Mode ---

export type EditorMode = 'design' | 'animate'

// --- Store Interface ---

interface TimelineStoreState {
  // Persisted (saved to .op file)
  duration: number // ms
  fps: 24 | 30 | 60

  // Ephemeral (runtime only)
  currentTime: number
  playbackMode: PlaybackMode
  loopEnabled: boolean
  editorMode: EditorMode

  // Actions — timeline
  setDuration: (ms: number) => void
  setFps: (fps: 24 | 30 | 60) => void

  // Actions — playback
  setCurrentTime: (ms: number) => void
  setPlaybackMode: (mode: PlaybackMode) => void
  toggleLoop: () => void

  // Actions — mode
  setEditorMode: (mode: EditorMode) => void

  // Composition (reads from document store, falls back to local duration/fps)
  getCompositionDuration: () => number
  getCompositionFps: () => number
  setComposition: (settings: Partial<CompositionSettings>) => void
}

export const useTimelineStore = create<TimelineStoreState>((set, get) => ({
  // Persisted defaults
  duration: 5000,
  fps: 30,

  // Ephemeral defaults
  currentTime: 0,
  playbackMode: 'idle',
  loopEnabled: false,
  editorMode: 'design',

  // --- Timeline ---

  setDuration: (ms) => set({ duration: Math.max(100, Math.min(ms, 300000)) }),

  setFps: (fps) => set({ fps }),

  // --- Playback ---

  setCurrentTime: (ms) => set({ currentTime: Math.max(0, ms) }),

  setPlaybackMode: (mode) => set({ playbackMode: mode }),

  toggleLoop: () => set((s) => ({ loopEnabled: !s.loopEnabled })),

  // --- Mode ---

  setEditorMode: (mode) => set({ editorMode: mode }),

  // --- Composition (reads from document store, falls back to local) ---

  getCompositionDuration: () => {
    const composition = getDocumentComposition()
    if (composition?.duration) return composition.duration
    return get().duration
  },

  getCompositionFps: () => {
    const composition = getDocumentComposition()
    if (composition?.fps) return composition.fps
    return get().fps
  },

  setComposition: (settings) => {
    if (settings.duration !== undefined) {
      get().setDuration(settings.duration)
    }
    if (settings.fps !== undefined && (settings.fps === 24 || settings.fps === 30 || settings.fps === 60)) {
      get().setFps(settings.fps)
    }

    setDocumentComposition(settings)
  },
}))
