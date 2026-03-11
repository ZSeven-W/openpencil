---
title: "refactor: Full v2 animation engine migration — delete v1, wire timeline to v2"
type: refactor
status: active
date: 2026-03-11
deepened: 2026-03-11
origin: docs/brainstorms/2026-03-11-v2-engine-migration-brainstorm.md
---

## Enhancement Summary

**Deepened on:** 2026-03-11
**Sections enhanced:** 3 (Phase 1, Phase 5, Dependencies & Risks)
**Research agents used:** useSyncExternalStore patterns, HTML5 video+rAF sync, Vitest animation testing, v2 controller API analysis

### Key Improvements
1. **Phase 1 grounded with concrete hook architecture** — two separate hooks (`usePlaybackTime` + `usePlaybackPlaying`), module-level subscribe functions, throttled notify at ~30fps, SSR `getServerSnapshot` for TanStack Start
2. **Video sync upgraded from single 50ms threshold to tiered correction** — 30ms soft (playbackRate nudge ±3%), 100ms hard seek, `requestVideoFrameCallback` for accurate drift measurement, iOS Safari precision handling
3. **Phase 5 test strategy fully specified** — manual `flushRAF()` for unit tests, explicit `vi.useFakeTimers({ toFake })` for integration, `renderHook` + `act()` for hook tests, `vitest bench` for hot-path regression

# refactor: Full v2 Animation Engine Migration

## Overview

Cold-cut migration from the v1 animation playback engine to v2. Delete v1 entirely — no feature flags, no dual-engine coexistence. Wire the timeline UI (Play/Pause/Seek) directly to the v2 `PlaybackController`. Canvas is the preview surface, with MP4/WebM export as the future goal. (see brainstorm: docs/brainstorms/2026-03-11-v2-engine-migration-brainstorm.md)

## Problem Statement

The v2 engine is fully built (playback controller, interpolation, canvas bridge, video sync, effect registry, 175 tests) but sits orphaned. The timeline UI still drives v1's `playback-loop.ts`. Two engines coexist, creating race conditions, confusion, and dead code. v1 is ~600 LOC of deprecated functions nobody calls except the timeline UI.

## Proposed Solution

Delete v1 code. Create a `usePlaybackController` hook that lazily instantiates the v2 `PlaybackController` as a global singleton. Rewire `playback-controls.tsx` and `timeline-editor.tsx` to use it. Strip v1 state from `timeline-store.ts`. Remove v1 tests.

## Technical Approach

### Implementation Phases

#### Phase 1: Create v2 Playback Hook + Wire Controller Lifecycle

Create the bridge between React and the v2 `PlaybackController`.

**New file: `src/animation/use-playback-controller.ts`**

The v2 `PlaybackController` already implements the `useSyncExternalStore` contract: `subscribe(cb): () => void` adds to a `Set<() => void>`, `getSnapshot()` returns `number` (currentTime), and `notify()` fires all listeners on every state change including rAF ticks.

**Architecture: Two hooks, not one.** Split time (changes every frame) from isPlaying (changes rarely) into separate `useSyncExternalStore` calls. Both subscribe to the same `notify()`, but `Object.is(true, true)` means `usePlaybackPlaying` consumers skip re-renders during playback.

```typescript
// Module-level subscribe functions — referentially stable, no closure per render
let controllerRef: PlaybackController | null = null

export function setPlaybackControllerRef(c: PlaybackController | null): void {
  controllerRef = c
}

export function getPlaybackController(): PlaybackController | null {
  return controllerRef
}

// Hook 1: time (re-renders at notify rate, ~30fps throttled)
function subscribeTime(cb: () => void): () => void {
  if (!controllerRef) return () => {}
  return controllerRef.subscribe(cb)
}
function getTimeSnapshot(): number {
  return controllerRef?.currentTime ?? 0
}
export function usePlaybackTime(): number {
  return useSyncExternalStore(subscribeTime, getTimeSnapshot, () => 0)
  //                                                          ^^^^^ SSR snapshot required — TanStack Start does SSR
}

// Hook 2: isPlaying (re-renders only on play/pause/stop)
function subscribePlaying(cb: () => void): () => void {
  if (!controllerRef) return () => {}
  return controllerRef.subscribe(cb)
}
function getPlayingSnapshot(): boolean {
  return controllerRef?.isPlaying() ?? false
}
export function usePlaybackPlaying(): boolean {
  return useSyncExternalStore(subscribePlaying, getPlayingSnapshot, () => false)
}
```

**Throttle `notify()` to ~30fps for time updates** in the controller's `tick` function. Canvas rendering stays at full 60fps (imperative, outside React). Play/pause/stop call `notify()` immediately (unthrottled) so button state updates instantly:

```typescript
// Inside tick():
const NOTIFY_INTERVAL = 33 // ~30fps for React, full 60fps for canvas
let lastNotifyTime = 0

onFrame(time)  // canvas update at full rAF rate
const now = performance.now()
if (now - lastNotifyTime > NOTIFY_INTERVAL) {
  notify()     // React re-render check, throttled
  lastNotifyTime = now
}
```

- [x] Create `use-playback-controller.ts` — lazy singleton factory
- [x] `getOrCreateController()` reads `canvasStore.fabricCanvas`, builds `AnimationIndex` from document store
- [x] `onFrame` callback: `interpolateClip()` → `applyAnimatedFrame()` → `syncVideoFramesV2()` → `canvas.renderAll()`
- [x] `onStop` callback: `restoreNodeStates()`, `recalcCoordsForAnimatedObjects()`, `pauseAllVideosV2()`
- [x] Call `setPlaybackControllerRef(controller)` so pause-middleware works
- [x] Export `usePlaybackTime()` — `useSyncExternalStore` with number snapshot + SSR fallback `() => 0`
- [x] Export `usePlaybackPlaying()` — `useSyncExternalStore` with boolean snapshot + SSR fallback `() => false`
- [ ] Throttle `notify()` in tick to ~30fps; keep play/pause/stop unthrottled
- [x] Register with engine-coordinator as `'v2'` (via createPlaybackController)
- [x] Dispose controller in `useEffect` cleanup (cancels rAF, clears listeners, sets `disposed = true`)
- [x] Handle null `controllerRef` in subscribe — return no-op unsubscribe
- [ ] Write tests for hook lifecycle (see Phase 5 test strategy)

**Success criteria:** Controller can be created, plays/pauses, applies frames to canvas. `usePlaybackTime` re-renders at ~30fps during playback. `usePlaybackPlaying` re-renders only on state transitions.

#### Phase 2: Rewire Timeline UI to v2 Controller

Replace all v1 playback-loop imports in UI components.

**`src/components/animation/playback-controls.tsx`**
- [ ] Replace `import { play, pause, stop, isPlaying } from playback-loop` with `usePlaybackController()` hook
- [ ] `handlePlay` calls `controller.play()`
- [ ] `handlePause` calls `controller.pause()`
- [ ] `handleStop` calls `controller.stop()`
- [ ] Time display uses `usePlaybackTime()` instead of reading from timeline-store

**`src/components/animation/timeline-editor.tsx`**
- [ ] Remove `toTimelineRows(tracks, videoNodes, ...)` call (v1)
- [ ] Use `buildTimelineRowsFromNodes(pageChildren)` as sole row data source
- [ ] Remove `tracks` and `videoClipIds` subscriptions from timeline-store
- [ ] Remove `setTimelineRef(...)` from playback-loop (v1 cursor sync)
- [ ] Remove `consumeCursorGuard()` — replace cursor sync with v2 controller's `getSnapshot().timeMs`
- [ ] Remove `EFFECT_ANIMATION_PHASE` usage and phase rendering paths
- [ ] Remove `PhaseActionRenderer` usage — DELETE `src/components/animation/phase-action-renderer.tsx`
- [ ] Scrub/seek: `onCursorDrag` calls `controller.seekTo(timeMs)` + `seekVideoClipsV2()`
- [ ] Remove v1 row merging logic (lines ~156-181)

**`src/canvas/use-canvas-events.ts`**
- [ ] Replace `import { pause as pausePlayback, isPlaying } from playback-loop`
- [ ] Use `isPlaybackActive()` from canvas-bridge (already uses coordinator) for guard
- [ ] Pause via `playbackControllerRef.pause()` or import from use-playback-controller

**`src/components/animation/preset-panel.tsx`**
- [ ] Remove `captureCurrentState` import (v1) — keep `captureNodeState` (v2)
- [ ] Remove `applyPreset` action and v1 preset buttons/easing/direction UI
- [ ] Remove `tracks` subscription from timeline-store
- [ ] Keep v2 effects section (effect registry, generateClipFromEffect)
- [ ] Keep clips summary and video clip controls

**Success criteria:** Timeline Play/Pause/Seek drives v2 engine. Animation clips animate on canvas. Video clips play and seek.

#### Phase 3: Strip v1 from Timeline Store

Remove all v1 track/keyframe/preset state from `timeline-store.ts`.

**`src/stores/timeline-store.ts`**
- [ ] Remove state: `tracks`, `videoClipIds`
- [ ] Remove actions: `addTrack`, `removeTrack`, `clearAllTracks`, `addKeyframe`, `removeKeyframe`, `updateKeyframe`, `applyPreset`, `addVideoClip`, `removeVideoClip`, `getTimelineData`, `loadTimelineData`, `reconcile`
- [ ] Remove helpers: `recomputePhases`, `sortedInsertKeyframe`
- [ ] Remove v1 type imports: `AnimationTrack`, `AnimationPhases`, `AnimationPresetName`, `Keyframe`, `PlaybackMode`, `PresetConfig`, `TimelineState`, `AnimatableProperties`
- [ ] Keep shared: `duration`, `fps`, `currentTime`, `setCurrentTime`, `playbackMode`, `setPlaybackMode`, `loopEnabled`, `toggleLoop`, `editorMode`, `setEditorMode`
- [ ] Keep v2: `getCompositionDuration`, `getCompositionFps`, `setComposition`

**`src/stores/document-store.ts`**
- [ ] Remove `useTimelineStore.getState().tracks[id]` reference in `duplicateNode` (v2 clips are on nodes, cloned automatically)

**Success criteria:** Timeline store is lean — only shared playback state + v2 composition accessors.

#### Phase 4: Delete v1 Files and Functions

Remove all v1 code that's no longer referenced.

**Files to DELETE entirely:**
- [ ] `src/animation/playback-loop.ts`
- [ ] `src/animation/presets.ts`
- [ ] `src/animation/presets.test.ts`
- [ ] `src/components/animation/phase-action-renderer.tsx`

**Functions to remove from mixed files:**

`src/animation/canvas-bridge.ts`:
- [ ] Delete `applyAnimatedProperties` (v1 deprecated)
- [ ] Delete `captureCurrentState` (v1 deprecated)
- [ ] Delete `setPlaybackActive` (v1 only — v2 uses coordinator)
- [ ] Delete `consumeCursorGuard` (v1 deprecated)
- [ ] Simplify `isPlaybackActive()` to just return `isAnyEnginePlaying()`
- [ ] Remove `playbackActive` local flag and `cursorUpdateCount`/`getCursorUpdateCount`

`src/animation/video-sync.ts`:
- [ ] Delete `syncVideoFrames` (v1)
- [ ] Delete `pauseAllVideos` (v1)

`src/animation/interpolation.ts`:
- [ ] Delete `getEasingFunction` (v1 deprecated)
- [ ] Delete `lerp` (v1 deprecated)
- [ ] Delete `interpolateProperties` (v1 deprecated)
- [ ] Delete `getInterpolatedProperties` (v1 deprecated)
- [ ] Delete all private easing functions (`easeOut`, `easeIn`, `easeInOut`, `bounce`, `easingFunctions` map)

`src/animation/timeline-adapter.ts`:
- [ ] Delete `toTimelineRows` (v1)
- [ ] Delete phase-related move/resize helpers (`applyPhaseMove`, `applyPhaseResize`)
- [ ] Remove phase code paths from `applyActionMove`/`applyActionResize`/`validateActionMove`/`validateActionResize`

`src/animation/timeline-adapter-types.ts`:
- [ ] Delete `AnimationPhaseMetadata`, `EFFECT_ANIMATION_PHASE`, `VideoNodeProjection`, `TimelineStores`
- [ ] Remove `AnimationPhaseMetadata` from `ActionMetadata` union

`src/types/animation.ts`:
- [ ] Delete all `@deprecated` types: `AnimatableProperties`, `EasingPreset`, `KeyframePhase`, `Keyframe`, `AnimationPhase`, `AnimationPhases`, `AnimationTrack`, `TimelineState`, `PlaybackMode`, `PlaybackState`, `AnimationPresetName`, `SlideDirection`, `PresetConfig`, `CANVAS_WIDTH`, `CANVAS_HEIGHT`

`src/animation/engine-coordinator.ts`:
- [ ] Simplify `EngineId` to just `'v2'` (or remove the type)

**Success criteria:** Zero references to deleted code. `npx tsc --noEmit` passes.

#### Phase 5: Update Tests

**Remove v1 test cases:**
- [ ] `src/animation/interpolation.test.ts` — remove v1 test cases (getEasingFunction, interpolateProperties, getInterpolatedProperties)
- [ ] `src/animation/video-sync.test.ts` — remove v1 syncVideoFrames/pauseAllVideos tests
- [ ] `src/animation/canvas-bridge.test.ts` — remove v1 applyAnimatedProperties/captureCurrentState tests
- [ ] `src/animation/timeline-adapter.test.ts` — remove toTimelineRows and phase-related tests

**Add integration test: play → frames applied → stop → state restored**

Use the existing manual `flushRAF()` pattern from `playback-controller.test.ts` (lines 45-50) — it gives exact millisecond control and is the gold standard for deterministic animation testing.

```typescript
describe('playback integration: play → frames → stop → restore', () => {
  let nowValue = 0
  let rafCallbacks: ((time: number) => void)[] = []

  beforeEach(() => {
    nowValue = 0
    rafCallbacks = []
    vi.spyOn(performance, 'now').mockImplementation(() => nowValue)
    globalThis.requestAnimationFrame = vi.fn((cb) => {
      rafCallbacks.push(cb as (t: number) => void)
      return ++rafId
    })
    globalThis.cancelAnimationFrame = vi.fn(() => { rafCallbacks = [] })
  })

  function flushRAF(advanceMs: number): void {
    nowValue += advanceMs
    const cbs = [...rafCallbacks]
    rafCallbacks = []
    for (const cb of cbs) cb(nowValue)
  }

  it('applies interpolated values and restores on stop', () => {
    const ctrl = createPlaybackController({ composition: { duration: 5000, fps: 30 }, ... })
    ctrl.play()
    flushRAF(1000)  // 20% through
    expect(onFrame).toHaveBeenCalledWith(1000)
    ctrl.stop()
    expect(onStop).toHaveBeenCalledOnce()
    expect(ctrl.currentTime).toBe(0)
  })
})
```

**Add `usePlaybackTime` hook test** using `renderHook` + `act()`:

```typescript
import { renderHook, act } from '@testing-library/react'

it('returns current time and re-renders on tick', () => {
  const ctrl = createPlaybackController({ ... })
  setPlaybackControllerRef(ctrl)
  const { result } = renderHook(() => usePlaybackTime())
  expect(result.current).toBe(0)
  act(() => { ctrl.play() })
  act(() => { flushRAF(500) })
  expect(result.current).toBe(500)
  act(() => { ctrl.stop() })
  expect(result.current).toBe(0)
  ctrl.dispose()
})
```

**Key testing patterns:**
- Wrap every mutation that triggers `notify()` in `act()` — ensures React processes the sync external store update before assertions
- Use typed Fabric.js mock objects (`as unknown as Canvas`) — do NOT instantiate real `fabric.Canvas` (requires real DOM canvas element)
- For `vi.useFakeTimers` integration tests, always be explicit: `vi.useFakeTimers({ toFake: ['requestAnimationFrame', 'cancelAnimationFrame', 'performance'] })`
- `vi.advanceTimersToNextFrame()` advances by exactly 16ms if using fake timers (alternative to manual `flushRAF`)

**Optional: hot-path performance benchmark** via `vitest bench`:

```typescript
// playback-hot-path.bench.ts
bench('applyAnimatedFrame (5 properties)', () => {
  applyAnimatedFrame(mockObj as any, frame)
})
```

- [ ] Add integration test: play → frames applied → stop → state restored (manual `flushRAF` pattern)
- [ ] Add `usePlaybackTime` hook test with `renderHook` + `act()`
- [ ] Add `usePlaybackPlaying` hook test — verify no re-render during sustained playback
- [ ] `bun --bun run test` passes
- [ ] `npx tsc --noEmit` passes

**Success criteria:** All tests pass, zero type errors, no v1 references anywhere.

## Acceptance Criteria

- [ ] Timeline Play button starts v2 playback — animated nodes move on canvas
- [ ] Timeline Pause/Stop buttons work via v2 controller
- [ ] Timeline scrub/seek updates canvas and video positions via v2
- [ ] Video clips play inline on canvas during playback
- [ ] Animation effects (fade, slide, scale, etc.) apply via v2 interpolation
- [ ] `useSyncExternalStore` drives time display (no Zustand writes at 60fps)
- [ ] Force-pause middleware stops playback on document mutations
- [ ] No v1 code remains — `playback-loop.ts`, `presets.ts`, `phase-action-renderer.tsx` deleted
- [ ] All `@deprecated` types removed from `types/animation.ts`
- [ ] `timeline-store.ts` has no track/keyframe/preset state
- [ ] All tests pass, zero type errors
- [ ] Export path is unblocked (canvas renders each frame, future work can capture frames)

## Dependencies & Risks

- **Risk: Missing v1 consumer** — The repo research mapped every import, but runtime-only references (dynamic requires, string-based lookups) could be missed. Mitigated by type checking.
- **Risk: Timeline editor complexity** — `timeline-editor.tsx` is the most complex file to refactor (v1 row merging, cursor sync, action renderers). Do it carefully.
- **Dependency: Fabric canvas must be available** — Controller reads canvas from `canvasStore`. If canvas isn't mounted yet, controller creation must be deferred.

### Video Sync Timing (Research-Grounded)

The original plan flagged "50ms drift correction may need tuning." Research (Remotion source, W3C Media & Entertainment IG, Shaka Player) grounds this with concrete thresholds and a tiered correction strategy.

**Replace single 50ms threshold with tiered correction in `video-sync.ts`:**

| Drift | Action | Rationale |
|-------|--------|-----------|
| < 30ms | Do nothing | Below perceptual threshold (~20ms lip-sync) |
| 30–100ms | `playbackRate` nudge ±3% | Converges in ~1s (0.5ms/frame at 60fps). Invisible to users. No audio pop. |
| > 100ms | Hard seek (`video.currentTime = x`) | Necessary for large drifts. Causes brief decode stutter. |
| Scrub/paused | Hard seek with 10ms threshold | Users expect frame accuracy when scrubbing |

**Additional improvements to apply during implementation:**

- **`requestVideoFrameCallback`** for drift measurement — `metadata.mediaTime` is ground truth for displayed frame (more reliable than `video.currentTime` which is backed by audio clock). Universal browser support as of 2025. Track `presentedFrames` to detect dropped frames. Use as data source only — keep driving Fabric dirty-marking from rAF tick.

- **iOS Safari precision** — round seek targets to 1 decimal: `Number(t.toFixed(1))`. Chrome rounds `currentTime` to 6 decimal places. Firefox with `privacy.reduceTimerPrecision` rounds to 2ms.

- **Audio-aware thresholds** — raise hard-seek threshold to 100ms for clips with audio (avoids pops/clicks). For muted clips, can be more aggressive (50ms hard seek). Set `video.preservesPitch = true` during rate nudges.

- **Don't read back `video.currentTime` after setting it** — browsers don't guarantee the value you set is the value you get back. Track last-set value internally.

```typescript
const DRIFT_SOFT_MS = 30
const DRIFT_HARD_MS = 100
const DRIFT_SCRUB_MS = 10
const RATE_NUDGE = 0.03

function syncSingleVideoClip(video: HTMLVideoElement, expectedSec: number, isPlaying: boolean): void {
  const driftMs = Math.abs(video.currentTime - expectedSec) * 1000
  if (!isPlaying) {
    if (driftMs > DRIFT_SCRUB_MS) video.currentTime = expectedSec
    return
  }
  if (driftMs > DRIFT_HARD_MS) {
    video.currentTime = expectedSec
    video.playbackRate = 1.0
  } else if (driftMs > DRIFT_SOFT_MS) {
    const direction = video.currentTime > expectedSec ? -1 : 1
    video.playbackRate = 1.0 + direction * RATE_NUDGE
  } else if (video.playbackRate !== 1.0) {
    video.playbackRate = 1.0
  }
}
```

**References:** Remotion `use-media-playback.ts` (450ms preview / 10ms scrub thresholds), Shaka Player `trickPlay(1.02)` for live latency, W3C TPAC frame-accurate sync (20ms target), `requestVideoFrameCallback` MDN docs.

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-11-v2-engine-migration-brainstorm.md](docs/brainstorms/2026-03-11-v2-engine-migration-brainstorm.md) — Key decisions: cold cut (no backward compat), global singleton controller, canvas is preview.

### Internal References

- `src/animation/playback-controller.ts` — v2 controller (already built)
- `src/animation/interpolation.ts` — v2 `interpolateClip` (already built)
- `src/animation/canvas-bridge.ts` — v2 `applyAnimatedFrame` (already built)
- `src/animation/video-sync.ts` — v2 `syncVideoFramesV2` (already built)
- `src/animation/animation-index.ts` — `buildAnimationIndex` (already built)
- `src/animation/track-buffer.ts` — pre-allocated buffers (already built)
- `src/stores/animation-pause-middleware.ts` — force-pause (already built)
- `src/stores/composition-accessors.ts` — composition data bridge (already built)

### Key Learnings Applied

- `useSyncExternalStore` for playback time avoids 60fps Zustand re-renders (from v2 plan)
- `renderAll()` not `requestRenderAll()` in rAF loop to avoid 1-frame lag (from v2 plan)
- Skip `setCoords()` during playback, call once on stop (from v2 plan)
- Transform props don't invalidate Fabric cache — free to animate (from v2 plan)
- Force-pause must be synchronous — even one stale rAF tick corrupts canvas (from v2 plan)

### Research Insights (from deepening)

- **Two hooks, not one** — split `usePlaybackTime()` (number, ~30fps re-renders) from `usePlaybackPlaying()` (boolean, skips re-renders via `Object.is(true, true)`). Module-level subscribe functions for referential stability. (React docs, Zustand internals, Epic React)
- **SSR `getServerSnapshot` required** — TanStack Start does SSR; omitting third arg is a hard React error. Return `0` / `false`. (React docs)
- **Throttle notify to ~30fps** — canvas stays at 60fps (imperative), React re-renders throttled. Play/pause/stop notify immediately. (v1 already discovered this with `UI_UPDATE_INTERVAL = 100`)
- **Tiered video drift correction** — 30ms soft (rate nudge ±3%), 100ms hard seek. Single 50ms threshold is suboptimal. (Remotion source, Shaka Player, W3C Media IG)
- **`requestVideoFrameCallback`** — `metadata.mediaTime` is ground truth for displayed frame, more reliable than `video.currentTime`. Universal browser support. (MDN, web.dev)
- **iOS Safari seek precision** — round to 1 decimal: `Number(t.toFixed(1))`. (Remotion `seek.ts`)
- **Manual `flushRAF()` for deterministic tests** — existing pattern in `playback-controller.test.ts` is gold standard. `vi.useFakeTimers` needs explicit `toFake` for rAF + performance. `renderHook` + `act()` for hook tests. (Vitest docs, testing-library)
