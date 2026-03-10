import { create } from 'zustand'
import { nanoid } from 'nanoid'
import type {
  AnimationTrack,
  AnimationPresetName,
  Keyframe,
  PlaybackMode,
  PresetConfig,
  TimelineState,
} from '@/types/animation'
import type { AnimatableProperties } from '@/types/animation'
import { generatePresetKeyframes } from '@/animation/presets'

// --- Editor Mode ---

export type EditorMode = 'design' | 'animate'

// --- Store Interface ---

interface TimelineStoreState {
  // Persisted (saved to .op file)
  tracks: Record<string, AnimationTrack>
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

  // Actions — tracks
  addTrack: (track: AnimationTrack) => void
  removeTrack: (nodeId: string) => void
  clearAllTracks: () => void

  // Actions — keyframes
  addKeyframe: (nodeId: string, keyframe: Keyframe) => void
  removeKeyframe: (nodeId: string, keyframeId: string) => void
  updateKeyframe: (
    nodeId: string,
    keyframeId: string,
    updates: Partial<Pick<Keyframe, 'time' | 'properties' | 'easing'>>,
  ) => void

  // Actions — presets
  applyPreset: (
    nodeId: string,
    presetName: AnimationPresetName,
    nodeState: AnimatableProperties,
    config?: PresetConfig,
  ) => void

  // Actions — playback
  setCurrentTime: (ms: number) => void
  setPlaybackMode: (mode: PlaybackMode) => void
  toggleLoop: () => void

  // Actions — mode
  setEditorMode: (mode: EditorMode) => void

  // Actions — persistence
  getTimelineData: () => TimelineState
  loadTimelineData: (data: TimelineState) => void
  reconcile: (existingNodeIds: Set<string>) => void
}

function sortedInsertKeyframe(
  keyframes: Keyframe[],
  keyframe: Keyframe,
): Keyframe[] {
  const result = [...keyframes, keyframe]
  result.sort((a, b) => a.time - b.time)
  return result
}

export const useTimelineStore = create<TimelineStoreState>((set, get) => ({
  // Persisted defaults
  tracks: {},
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

  // --- Tracks ---

  addTrack: (track) =>
    set((s) => ({
      tracks: { ...s.tracks, [track.nodeId]: track },
    })),

  removeTrack: (nodeId) =>
    set((s) => {
      const { [nodeId]: _, ...rest } = s.tracks
      return { tracks: rest }
    }),

  clearAllTracks: () => set({ tracks: {} }),

  // --- Keyframes ---

  addKeyframe: (nodeId, keyframe) =>
    set((s) => {
      const track = s.tracks[nodeId]
      if (!track) return s
      return {
        tracks: {
          ...s.tracks,
          [nodeId]: {
            ...track,
            keyframes: sortedInsertKeyframe(track.keyframes, keyframe),
          },
        },
      }
    }),

  removeKeyframe: (nodeId, keyframeId) =>
    set((s) => {
      const track = s.tracks[nodeId]
      if (!track) return s
      return {
        tracks: {
          ...s.tracks,
          [nodeId]: {
            ...track,
            keyframes: track.keyframes.filter((k) => k.id !== keyframeId),
          },
        },
      }
    }),

  updateKeyframe: (nodeId, keyframeId, updates) =>
    set((s) => {
      const track = s.tracks[nodeId]
      if (!track) return s
      const keyframes = track.keyframes.map((k) =>
        k.id === keyframeId ? { ...k, ...updates } : k,
      )
      keyframes.sort((a, b) => a.time - b.time)
      return {
        tracks: {
          ...s.tracks,
          [nodeId]: { ...track, keyframes },
        },
      }
    }),

  // --- Presets ---

  applyPreset: (nodeId, presetName, nodeState, config) => {
    const { duration } = get()
    const { keyframes, phases } = generatePresetKeyframes(
      presetName,
      nodeState,
      duration,
      config,
    )

    // Add unique IDs to keyframes
    const keyframesWithIds = keyframes.map((k) => ({
      ...k,
      id: nanoid(8),
    }))

    const track: AnimationTrack = {
      nodeId,
      keyframes: keyframesWithIds,
      phases,
      startDelay: 0,
    }

    set((s) => ({
      tracks: { ...s.tracks, [nodeId]: track },
    }))
  },

  // --- Playback ---

  setCurrentTime: (ms) => set({ currentTime: Math.max(0, ms) }),

  setPlaybackMode: (mode) => set({ playbackMode: mode }),

  toggleLoop: () => set((s) => ({ loopEnabled: !s.loopEnabled })),

  // --- Mode ---

  setEditorMode: (mode) => set({ editorMode: mode }),

  // --- Persistence ---

  getTimelineData: () => {
    const { tracks, duration, fps } = get()
    return { tracks, duration, fps }
  },

  loadTimelineData: (data) =>
    set({
      tracks: data.tracks ?? {},
      duration: data.duration ?? 5000,
      fps: data.fps ?? 30,
    }),

  reconcile: (existingNodeIds) =>
    set((s) => {
      const cleaned: Record<string, AnimationTrack> = {}
      for (const [nodeId, track] of Object.entries(s.tracks)) {
        if (existingNodeIds.has(nodeId)) {
          cleaned[nodeId] = track
        }
      }
      return { tracks: cleaned }
    }),
}))
