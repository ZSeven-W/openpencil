---
title: "feat: Integrate react-timeline-editor for unified animation/video timeline"
type: feat
status: active
date: 2026-03-11
deepened: 2026-03-11
reviewed: 2026-03-11
---

# Integrate react-timeline-editor for Unified Animation/Video Timeline

## Enhancement Summary

**Deepened on:** 2026-03-11
**Research agents used:** TypeScript reviewer, Performance oracle, Architecture strategist, Frontend races reviewer, Code simplicity reviewer, Pattern recognition specialist, React 19 compatibility researcher, Frontend design specialist, Agent-native architecture reviewer

### Critical Discovery

**`@xzdarcy/react-timeline-editor` v1.0.0 is NOT compatible with React 19.** The library depends on `react-virtualized`, which uses the removed `ReactDOM.findDOMNode` API. This is a hard runtime crash, not a warning.

**Decision: Fork the original, cherry-pick React 19 fixes from `@cyca`.** The original repo (`xzdarcy/react-timeline-editor`) is actively maintained (~4,700 downloads/week, v1.0.0 released Jan 2026). The `@cyca` fork has only 47 downloads/week and is alpha. Strategy: fork `xzdarcy/react-timeline-editor` v1.0.0 to the OpenPencil org, apply the React 19 compatibility patches (`react-virtualized` → `@tanstack/virtual`, `interactjs` → `@use-gesture/vanilla`) cherry-picked from `@cyca`'s work. This gives us the stable foundation + React 19 compat + full control.

### Key Improvements from Research

1. **Library strategy**: Fork `xzdarcy/react-timeline-editor` v1.0.0, cherry-pick React 19 fixes from `@cyca` fork, publish as `@openpencil/react-timeline-editor`
2. **Metadata map pattern**: Keep domain data (`phase`, `keyframes`, `nodeId`) in a separate `Map<string, ActionMetadata>` keyed by `action.id`, not smuggled through the library's `data` property
3. **Mutable ref layer during drag**: Let the library own data during gestures, only commit to Zustand on gesture end — eliminates round-trip jitter and per-pixel cloning
4. **Undo ordering helper**: Extract `withTimelineUndoBatch()` to guarantee correct `injectAnimationData` → `startBatch` → mutate → `endBatch` sequence
5. **Timestamp-based cursor guard**: Replace boolean sync lock with `performance.now()` timestamp comparison — more robust for async RAF ↔ library event timing
6. **Performance**: Write to stores only on gesture end (not per-pixel), memoize `toTimelineRows()` with Zustand selectors, throttle `setTime()` to 30fps during playback
7. **Fabric Map cache (mandatory)**: Build `Map<nodeId, FabricObject>` in `canvas-bridge.ts`, shared with `video-sync.ts` — eliminates O(n) linear scan per track per frame
8. **MCP parity gap**: Animation MCP tools referenced in the plan don't exist yet — must be created alongside or immediately after this integration
9. **Visual design**: Industrial/utilitarian NLE aesthetic with oklch phase colors, monospace timecodes, CSS variable overrides scoped to library classes

### Technical Review Refinements (2026-03-11)

Applied from unanimous/consensus feedback across 4 reviewers (TypeScript, Performance, Simplicity, Architecture):

1. **Removed `unstable_batchedUpdates`** — no-op in React 19 with concurrent rendering and Zustand's built-in batching
2. **Fabric Map cache promoted to mandatory Phase 2 task** — both `canvas-bridge.ts` and `video-sync.ts` do O(n) scans per frame
3. **`endBatch()` receives `currentDoc`** for no-op detection
4. **`'scrubbing'` added to `PlaybackMode`** type
5. **`reconcile()` extended** to clean up `videoClipIds` for deleted nodes
6. **Fork strategy**: Fork original v1.0.0, cherry-pick `@cyca`'s React 19 patches, install from own fork
7. **`TimelineStores` interface** for adapter testability (dependency injection over direct store access)
8. **Unmount cleanup** for `isDragging`/`frozenRows` refs

---

## Overview

Replace OpenPencil's custom timeline UI (~700 LOC across 6 components) with our own fork of `xzdarcy/react-timeline-editor` v1.0.0 (with React 19 compatibility patches cherry-picked from `@cyca`'s fork) to get professional drag/resize/snap/zoom/virtualization for free, while keeping all animation engine internals untouched. The library handles timeline UI; our `playback-loop.ts`, `canvas-bridge.ts`, `interpolation.ts`, `video-registry.ts`, and `video-sync.ts` continue to drive actual animation.

**Key insight:** The library's engine is fully separable. We bypass it entirely, using only the UI layer. Our playback loop calls `timelineState.setTime()` to move the cursor. The library's `onActionMoveEnd`/`onActionResizeEnd` callbacks write back to our Zustand stores.

## Problem Statement / Motivation

The current custom timeline UI has limitations that will become painful as Jeans grows into a light video editor:

1. **No zoom** — compositions >10s are unusable without horizontal zoom
2. **No snap** — keyframe/clip positioning requires manual precision
3. **No virtualization** — performance degrades with many tracks
4. **Manual drag implementation** — ~150 LOC of raw `mousemove`/`mouseup` handlers per component, fragile and hard to extend
5. **Separate video and animation tracks** — video clips and animation keyframes render in disconnected UI sections

The library provides all of these out of the box, letting us focus on the content workflow rather than timeline infrastructure.

## Proposed Solution

### Architecture: Adapter Pattern

```text
Zustand Stores (source of truth)
  ├── timeline-store (tracks, keyframes, phases)
  └── document-store (VideoNode.inPoint/outPoint/timelineOffset)
          │
          ▼
  timeline-adapter.ts (pure transform functions)
  timeline-adapter-types.ts (metadata map, TimelineStores interface)
          │
    ┌─────┴──────┐
    ▼            ▼
TimelineRow[]   onActionMoveEnd → applyActionMove()
    │                              │
    ▼                              ▼
@openpencil/react-timeline-editor   timeline-store / document-store
    │
    ▼
getActionRender → custom phase bars, keyframe diamonds, video clips
```

**Principle:** The library never owns the data at rest. It receives a projection and reports mutations back. During an active drag gesture, the library owns its mutable data — we only commit to stores on gesture end.

### Data Model Mapping

#### Decision: Phase-as-Action with Keyframe Markers

Each animation track becomes one `TimelineRow`. Each **phase** (in/while/out) becomes one `TimelineAction`. Keyframe positions are rendered as diamond markers via `getActionRender` inside their parent phase action. This preserves the existing visual model and gives actions meaningful start/end times for drag/resize.

#### Metadata Map (not smuggled through library types)

Domain-specific metadata is kept in a separate `Map` keyed by `action.id`, never attached to `TimelineAction` objects that flow through the library. This avoids type safety issues from extending third-party types and prevents data loss if the library clones or serializes actions internally.

```typescript
// src/animation/timeline-adapter-types.ts

// Time unit convention: use _s and _ms suffixes for clarity
// Library interactions use seconds, store interactions use milliseconds
function msToSec(ms: number): number {
  return Math.round(ms) / 1000
}

function secToMs(s: number): number {
  return Math.round(s * 1000)
}

type ActionMetadata =
  | { type: 'animation-phase'; phase: 'in' | 'while' | 'out'; keyframes: Keyframe[]; nodeId: string }
  | { type: 'video-clip'; nodeId: string }

type ActionMetadataMap = Map<string, ActionMetadata>

// Adapter accepts stores via interface for testability
interface TimelineStores {
  getTimelineState: () => TimelineStoreState
  getDocumentState: () => DocumentStoreState
  updateKeyframe: (trackId: string, keyframeId: string, partial: Partial<Keyframe>) => void
  updateNode: (id: string, partial: Partial<PenNode>) => void
}
```

The discriminated union on `type` lets TypeScript narrow metadata fields — `phase` and `keyframes` are only accessible when `type === 'animation-phase'`, eliminating null checks. The `effectId` on `TimelineAction` already discriminates animation vs video, so no separate `STORE_ROUTES` map is needed.

#### Video Clip Mapping

Video clips become `TimelineRow` entries with a single `TimelineAction` per clip:

```typescript
{
  id: videoNodeId,
  actions: [{
    id: `${videoNodeId}-video`,
    start: msToSec(node.timelineOffset),
    end: msToSec(node.timelineOffset + clipDuration),
    effectId: 'video-clip',
    flexible: true,
    movable: true,
  }]
}
// Metadata map entry: { type: 'video-clip', nodeId: videoNodeId }
```

Where `clipDuration = node.outPoint - node.inPoint`.

#### Time Unit Convention

All library interactions use **seconds**. All store interactions use **milliseconds**. The adapter handles conversion via `msToSec()` / `secToMs()`. Variables use `_s` / `_ms` suffixes for clarity (e.g., `newStart_s`, `offset_ms`). No branded types — naming convention is sufficient for a single adapter module.

## Technical Approach

### Implementation Phases

#### Phase 1: Adapter Layer + Minimal Render (~2 days)

Install the library, build the pure-function adapter, render existing tracks read-only.

**Tasks:**
- [x] ~~Fork `xzdarcy/react-timeline-editor` v1.0.0 to OpenPencil GitHub org~~ (deferred — using `@cyca/react-timeline-editor` directly for now)
- [x] ~~Cherry-pick React 19 compatibility patches from `@cyca/react-timeline-editor`~~ (using @cyca directly)
- [x] Install from fork: `bun add @cyca/react-timeline-editor`
- [x] Audit peer deps — check for radix-ui version conflicts with existing shadcn/ui
- [x] Verify the fork compiles and renders without errors against React 19
- [x] Create `src/animation/timeline-adapter-types.ts`:
  - `msToSec()` / `secToMs()` conversion functions (use `_s`/`_ms` naming convention, not branded types)
  - `ActionMetadata` discriminated union
  - `ActionMetadataMap` type
  - `TimelineStores` interface for dependency injection (adapter testability)
- [x] Create `src/animation/timeline-adapter.ts`:
  - `toTimelineRows(tracks, videoNodes, duration): { rows: TimelineRow[], metadata: ActionMetadataMap }`
  - `applyActionMove(actionId, newStart_s, newEnd_s, metadata, stores: TimelineStores)` — routes via `effectId`
  - `applyActionResize(actionId, newStart_s, newEnd_s, dir, metadata, stores: TimelineStores)` — handles trim vs phase resize
  - Input type for videoNodes: `ReadonlyArray<Pick<VideoNode, 'id' | 'inPoint' | 'outPoint' | 'timelineOffset' | 'videoDuration' | 'name'>>`
- [x] Create `src/animation/timeline-adapter.test.ts`:
  - Round-trip fidelity tests (store → rows → mutate → apply back)
  - Time unit conversion with values that cause IEEE 754 drift (e.g., 7ms = 0.007s)
  - Phase boundary calculation
  - Video clip projection from document-store
  - Cross-phase overlap prevention
  - Adapter with mock `TimelineStores` (no real store dependency in tests)
- [x] Create `src/components/animation/timeline-editor.tsx` — wrapper component:
  - Reads from `useTimelineStore` with Zustand selector on `tracks` and `videoClipIds` only (not `currentTime`)
  - Calls `toTimelineRows()` wrapped in `useMemo` keyed on structural data
  - Renders `<Timeline>` with `autoReRender: false`
  - Passes `ref` for external cursor control
- [x] Create `src/components/animation/timeline-editor.css` — library CSS overrides using shadcn/ui design tokens

**Success criteria:** Existing animation tracks and video clips appear in the library's timeline UI with correct positions and durations. No interaction wired yet. No React 19 runtime errors.

#### Phase 2: Cursor Sync + Playback Integration + Perf Infrastructure (~1.5 days)

Wire bidirectional cursor: our engine drives the library cursor during playback, library cursor drags drive our engine for scrubbing. Also build mandatory performance infrastructure.

**Tasks:**
- [x] **Mandatory: Build Fabric object Map cache** in `canvas-bridge.ts`:
  - `const fabricObjectMap = new Map<string, FabricObject>()`
  - Populate on `play()` from `canvas.getObjects()`, clear on `stop()`
  - Export for shared use by `video-sync.ts` (eliminates duplicate O(n) scans)
  - Replace `findFabricObject()` linear scan with Map lookup
- [x] **Add `'scrubbing'` to `PlaybackMode` type** in `timeline-store.ts`:
  - Current: `'idle' | 'playing'` → New: `'idle' | 'playing' | 'scrubbing'`
- [x] **Extend `reconcile()` in `timeline-store.ts`** to clean up `videoClipIds` for deleted nodes
- [x] Add `timelineRef` to `playback-loop.ts` — call `timelineRef.current?.setTime(currentTime / 1000)` throttled to 30fps (separate from the 100ms store throttle):
  ```typescript
  const CURSOR_UPDATE_INTERVAL_MS = 33 // ~30fps
  let lastCursorUpdate_ms = 0
  // In tick(): if (timestamp - lastCursorUpdate_ms > CURSOR_UPDATE_INTERVAL_MS) { ... }
  ```
- [x] Wire `onCursorDragStart` → pause playback, set `playbackMode: 'scrubbing'`
- [x] Wire `onCursorDrag` → `seekTo(time * 1000)` for live canvas preview
- [x] Wire `onCursorDragEnd` → set `playbackMode: 'idle'`
- [x] Add timestamp-based cursor guard (in `canvas-bridge.ts` alongside `playbackActive`):
  ```typescript
  let lastExternalCursorSetAt = 0
  // In playback tick: lastExternalCursorSetAt = performance.now()
  // In onCursorDrag: if (performance.now() - lastExternalCursorSetAt < 20) return
  ```
- [x] Wire `onClickTimeArea` → `seekTo(time * 1000)` for click-to-seek

**Research insight:** A boolean sync lock (like `canvas-sync-lock.ts`) is insufficient here because it bridges two async systems (RAF and library event dispatch). The timestamp approach gives a ~1 frame grace period that handles timing gaps without needing set/reset state management.

**Success criteria:** Playback drives cursor smoothly at 30fps. Dragging cursor scrubs canvas preview. No feedback loops.

#### Phase 3: Action Mutation (Drag + Resize) (~2 days)

Wire drag/resize callbacks to update stores, with undo/redo support.

**Tasks:**
- [x] Implement mutable ref layer for drag operations:
  ```typescript
  const isDragging = useRef(false)
  const frozenRows = useRef<TimelineRow[] | null>(null)
  // onActionMoveStart: freeze rows, set isDragging
  // During drag: library owns its data, NO store writes
  // onActionMoveEnd: commit to stores, unfreeze
  // In render: isDragging.current ? frozenRows.current : memoizedRows
  // useEffect cleanup: reset isDragging + frozenRows on unmount (prevents stale ref leak)
  ```
- [x] Wire `onActionMoveEnd` → `applyActionMove()`:
  - Animation phases: update keyframe times proportionally within the phase
  - Video clips: update `timelineOffset` via `document-store.updateNode()`
  - React 19 + Zustand batch automatically — no `unstable_batchedUpdates` needed
- [x] Wire `onActionResizeEnd` → `applyActionResize()`:
  - Animation phases: move first/last keyframe of the phase
  - Video clips (left resize): update `inPoint`
  - Video clips (right resize): update `outPoint`
  - Enforce cross-phase overlap prevention (in phase end cannot exceed while phase start)
- [x] Wire `onActionMoving` / `onActionResizing` — return `false` to reject when:
  - `start >= end` (fast drag guard)
  - Duration < 50ms minimum
  - Video `inPoint < 0` or `outPoint > videoDuration`
  - Cross-phase temporal overlap
- [x] Create `src/animation/timeline-undo.ts` — undo bridge helper:
  ```typescript
  function withTimelineUndoBatch(fn: () => void): void {
    injectAnimationData()              // 1. serialize current timeline state
    const doc = useDocumentStore.getState().document
    startBatch(doc)                     // 2. capture pre-mutation snapshot
    fn()                                // 3. execute mutations
    injectAnimationData()              // 4. serialize post-mutation state
    const currentDoc = useDocumentStore.getState().document
    endBatch(currentDoc)               // 5. push to history (pass currentDoc for no-op detection)
  }
  ```
  - Use in all `onActionMoveEnd` / `onActionResizeEnd` callbacks

**Research insight:** Do NOT write to stores during drag (per-pixel). Only commit on gesture end. This eliminates the need for `structuredClone` debouncing and prevents `use-canvas-sync` from firing on every pixel of video clip drag.

**Research insight:** The undo sequence ordering is critical — `injectAnimationData()` must run BEFORE `startBatch(doc)` so the batch base state includes current timeline data. Extract into a named helper to prevent ordering mistakes.

**Success criteria:** Keyframes and video clips can be dragged/resized. Undo/redo works. No data corruption from fast drags. No visual jitter during drag.

#### Phase 4: Custom Rendering + Visual Polish (~2 days)

Replace default action rendering with custom visuals matching our design.

**Tasks:**
- [x] Implement `getActionRender` for `effectId: 'animation-phase'` (in a `.tsx` file as named subcomponents):
  - `PhaseActionRenderer` component:
    - Phase colors (oklch, muted for dark theme):
      - In: `oklch(0.55 0.15 155 / 0.20)` bg, `oklch(0.65 0.18 155 / 0.50)` border
      - While: `oklch(0.55 0.12 250 / 0.15)` bg, `oklch(0.60 0.16 250 / 0.35)` border
      - Out: `oklch(0.55 0.18 25 / 0.20)` bg, `oklch(0.60 0.20 25 / 0.50)` border
    - Keyframe diamonds: 8x8px rotated 45deg, `box-shadow: 0 0 0 1px var(--card)` halo
    - Diamond hover: `scale(1.3)` 80ms ease
    - Diamond click → select keyframe (for property editing in preset panel)
- [x] Implement `getActionRender` for `effectId: 'video-clip'`:
  - `VideoClipRenderer` component:
    - Background: `oklch(0.45 0.12 300 / 0.25)` (muted violet)
    - Layout: `[Film 10px] [name truncated] ... [duration timecode]`
    - Timecode: monospace `ui-monospace` 8px, `font-variant-numeric: tabular-nums`
- [x] Implement track header sidebar (synchronized with timeline vertical scroll):
  - `width: 120px`, `border-right: 1px solid var(--border)`
  - Row: type icon (reuse `TYPE_ICONS` from `layer-item.tsx`) + node name at 11px
  - Click → `canvas-store.setSelection()` for bidirectional selection
  - Scroll sync via library's `onScroll` callback → `ref.scrollTop`
- [x] Style timeline via `timeline-editor.css`:
  - All colors reference `var(--card)`, `var(--border)`, `var(--muted-foreground)`, `var(--ring)`
  - Ruler labels: monospace 9px with `tabular-nums`
  - Playhead: `oklch(0.985 0 0 / 0.9)` (near-white, high contrast)
  - Snap guides: `oklch(0.623 0.214 259 / 0.5)` (matches `--primary`)
  - All transitions ≤120ms (timeline interactions are high-frequency)
- [ ] Configure zoom: mouse wheel on timeline area, min/max scale bounds (deferred — library handles basic zoom)
- [ ] Configure snap: `gridSnap` for FPS-based grid (deferred — basic gridSnap enabled)

**Research insight:** Use named React subcomponents in `.tsx` files for action renderers (following existing `PhaseBar`/`KeyframeDiamond` pattern from `track-list.tsx`), not inline render functions. Do NOT propagate hardcoded Tailwind colors (`bg-emerald-500`, etc.) — use oklch values or CSS custom properties.

**Success criteria:** Timeline looks integrated with OpenPencil's design system. Phase colors, keyframe diamonds, and video clips render correctly. Track click selects canvas object.

#### Phase 5: Remove Old Components + Cleanup (~1 day)

Delete replaced UI components, update imports.

**Tasks:**
- [x] Remove `src/components/animation/scrub-bar.tsx`
- [x] Remove `src/components/animation/track-list.tsx`
- [x] Remove `src/components/animation/video-clip-track.tsx`
- [x] Update `timeline-panel.tsx` to compose: PlaybackControls + TimelineEditor
- [ ] Update `preset-panel.tsx` — video trim controls can be removed (handled by action resize now)
- [x] Verify `playback-controls.tsx` still works (kept as separate component above timeline)
- [x] Run full test suite, update/remove tests for deleted components
- [x] Verify `editor-layout.tsx` animation mode toggle still works

**Success criteria:** No dead code. All existing animation features work through the new timeline. Tests pass.

#### Phase 6: MCP Animation Tools (follow-up, ~1 day)

The animation MCP tools referenced in this plan do not yet exist in `src/mcp/`. Create them to maintain agent-UI parity.

**Tasks:**
- [ ] Create `src/mcp/tools/animation.ts` with:
  - Read: `get_animation_state` (tracks, duration, fps, currentTime)
  - Track CRUD: `add_track`, `remove_track`
  - Keyframe CRUD: `add_keyframe`, `update_keyframe`, `remove_keyframe`
  - Preset: `apply_preset` (domain shortcut — encodes multi-keyframe patterns)
  - Config: `set_playhead`, `set_duration`, `set_fps`
- [ ] Register tools in `src/mcp/server.ts`
- [ ] Verify `update_node` handles `inPoint`/`outPoint`/`timelineOffset` for video clip timing (if not, add `update_video_clip_timing` tool)

**Success criteria:** An agent can "animate nodeX with a fade-in preset" and "trim video clip to first 5 seconds starting at 2s" entirely via MCP tools.

## System-Wide Impact

### Interaction Graph

```
User drags action in timeline
  → Library mutates its internal data (mutable ref layer — no store writes)
  → onActionMoveEnd fires
    → withTimelineUndoBatch(() => {
        applyActionMove() called
          → timeline-store.updateKeyframe() (animation)
             OR document-store.updateNode() (video)
             (React 19 + Zustand batch automatically)
      })
    → use-canvas-sync detects change (single render cycle)
      → Fabric.js objects updated
        → canvas.requestRenderAll()
```

```
Playback loop tick (60fps)
  → playback-loop.ts calculates currentTime
    → interpolation.ts computes properties per track
      → canvas-bridge.ts mutates Fabric objects
        → video-sync.ts seeks video elements
          → (every 33ms) timelineRef.setTime(currentTime/1000) updates library cursor
            → library re-renders cursor position (NO onChange fired)
  → (every 100ms) timeline-store.setCurrentTime() updates UI displays
```

### Error & Failure Propagation

- Library `onChange` with `start > end` → rejected by `onActionMoving` returning `false` → library reverts visual position
- `seekTo()` during playback → playback pauses first (existing behavior)
- Video element load failure → placeholder rect shown (existing behavior), clip action still renders in timeline
- Undo after timeline edit → `PenDocument.animation` restored → `extractAnimationData()` re-populates timeline-store → wrapper re-derives rows from updated store
- Undo during playback → `withTimelineUndoBatch` calls `stop()` first, then restores. `reconcile()` removes tracks for deleted nodes.

### State Lifecycle Risks

**Risk: Stale library state after undo.** When undo restores a previous `PenDocument` snapshot, `extractAnimationData()` must be called to refresh `timeline-store`, which then re-projects to the library.

**Mitigation:** The timeline-editor wrapper subscribes to `timeline-store` tracks via Zustand selector — any change (including undo-triggered) causes `useMemo` recomputation and re-render with fresh rows.

**Risk: Video clip data split across two stores.** Video properties live in document-store, animation tracks in timeline-store. An undo could restore document-store but not timeline-store (or vice versa).

**Mitigation:** `withTimelineUndoBatch()` calls `injectAnimationData()` before `endBatch()`, ensuring both stores are captured in the same history entry.

**Risk: Round-trip jitter during drag.** If store writes happen per-pixel, the store → `toTimelineRows()` → library re-render cycle could produce 0.001s discrepancies from time unit conversion, causing visual jitter.

**Mitigation:** Mutable ref layer freezes projection during drag. Library owns data during gesture, stores own at rest. No round-trip conversion during active drag.

**Risk: Property edits during playback lost on stop.** `savedStates` is captured once at `play()` time. If user modifies properties during playback, `stop()` restores pre-play state.

**Mitigation:** Disable property panel editing during playback (or capture state continuously). Flag for future resolution.

### Sync Lock Inventory

Document all sync locks for future contributors:

| Lock | Location | Prevents |
|------|----------|----------|
| `_locked` | `canvas-sync-lock.ts` | Fabric → store writes suppressing store → Fabric sync |
| `playbackActive` | `canvas-bridge.ts` | Fabric event handlers firing during animation playback |
| `lastExternalCursorSetAt` | `canvas-bridge.ts` (new) | Playback cursor updates triggering library scrub callbacks |

### API Surface Parity

**Gap identified:** The MCP animation tools referenced in this plan (`add_keyframe`, `update_keyframe`, `remove_keyframe`, `set_playhead`) do not exist in the MCP server yet. Phase 6 addresses this. The adapter pattern ensures that once tools write to `timeline-store`, the library automatically reflects the changes on next render.

## Acceptance Criteria

### Functional Requirements

- [x] Animation tracks with phase-colored regions render in the library timeline
- [x] Keyframe diamonds are visible at correct positions within phase actions
- [x] Video clips render as draggable/resizable actions with correct timing
- [x] Playback drives the timeline cursor smoothly (no jitter, no feedback loops)
- [x] Scrubbing the timeline cursor updates canvas preview in real-time
- [x] Dragging animation phases repositions keyframes proportionally
- [x] Resizing video clips updates inPoint/outPoint correctly
- [x] Undo/redo captures all timeline mutations (via `withTimelineUndoBatch`)
- [x] Clicking a track row selects the canvas object
- [ ] Zoom (mouse wheel) works on the timeline
- [ ] Snap to grid during drag
- [x] Minimum action duration of 50ms enforced
- [x] Cross-phase overlap prevention (in end ≤ while start, while end ≤ out start)

### Non-Functional Requirements

- [x] No frame drops during playback with 20+ tracks
- [x] Drag operations maintain <16ms frame budget (no per-pixel store writes)
- [x] Library CSS overrides use only shadcn/ui design tokens (no hardcoded Tailwind colors)
- [x] Total new code <700 LOC (adapter ~200, types ~50, wrapper ~200, renderers ~100, CSS ~100, tests ~200, undo helper ~30)
- [x] All transitions ≤120ms for timeline interaction feedback

### Quality Gates

- [x] `timeline-adapter.test.ts` covers: round-trip fidelity, time unit conversion, IEEE 754 drift edge cases, phase boundary calculation, cross-phase overlap rejection
- [x] Existing animation tests pass without modification
- [x] `bun --bun run build` succeeds
- [x] `npx tsc --noEmit` passes
- [x] No `findDOMNode` errors (React 19 compatibility verified)

### Agent Parity (Phase 6)

- [ ] MCP tools exist: `add_track`, `remove_track`, `add_keyframe`, `update_keyframe`, `remove_keyframe`, `apply_preset`, `set_playhead`, `get_animation_state`
- [ ] Video clip timing writable via MCP (`update_node` or dedicated tool)

## Alternative Approaches Considered

### Keep custom UI, add zoom/snap manually
**Rejected.** Would require ~500+ LOC for zoom alone (scale calculation, scroll sync, ruler rendering). Snap adds another ~200 LOC. Virtualization another ~300 LOC. The library provides all of this for free with better interaction quality.

### Use the library's built-in engine
**Rejected.** Our playback loop already handles video sync, canvas-bridge integration, and phase-aware interpolation. The library's engine would duplicate this logic and create two sources of truth for playback state. We bypass the engine and use the library purely for UI.

### Replace entire animation system with the library
**Rejected.** The library's effect system (`enter`/`update`/`leave`) is less capable than our interpolation engine (binary search, 5 easing functions, property-level keyframes). Our engine is well-tested and handles the Fabric.js integration layer that the library knows nothing about.

### Map keyframes as individual zero-width actions
**Rejected.** The library's drag/resize behavior is designed for actions with duration. Zero-width actions cannot be resized and are hard to click. Phase-as-action with keyframe markers inside gives better UX.

### Use `@xzdarcy/react-timeline-editor` directly (no fork)
**Rejected.** Crashes on React 19 due to `react-virtualized` using removed `ReactDOM.findDOMNode`. Requires patching.

### Use `@cyca/react-timeline-editor` directly
**Rejected.** Only 47 downloads/week, alpha status, less-proven base. The original has ~4,700 downloads/week and active maintenance. Better to fork the stable original and cherry-pick `@cyca`'s React 19 fixes.

### Smuggle domain data through `TimelineAction.data`
**Rejected.** `TimelineAction` does not declare a `data` property. Extending it relies on structural subtyping that breaks in callbacks where the library types the action as `TimelineAction`, requiring unsafe casts. A separate `ActionMetadataMap` keyed by `action.id` is type-safe and immune to library serialization/cloning.

### Boolean sync lock for cursor feedback
**Rejected.** The cursor feedback loop bridges two async systems (RAF tick and library event dispatch). A boolean requires explicit set/reset timing that is error-prone. A `performance.now()` timestamp with ~20ms grace period is self-managing and handles all edge cases.

### Write to stores during drag (per-pixel)
**Rejected.** Creates three problems: (1) `structuredClone` GC pressure at 120Hz, (2) `toTimelineRows()` re-projection causes round-trip jitter from ms→s→ms conversion, (3) `use-canvas-sync` fires on every pixel for video clip drags. Writing only on gesture end (mutable ref layer) eliminates all three.

## Dependencies & Prerequisites

- **OpenPencil fork of `xzdarcy/react-timeline-editor` v1.0.0** — forked to OpenPencil org, with React 19 compat patches cherry-picked from `@cyca` (react-virtualized → @tanstack/virtual, interactjs → @use-gesture/vanilla). Installed via `github:openpencil/react-timeline-editor#<sha>`. Full control, can pull upstream updates.
- **Existing `feat/animation-core` branch** — video support commits (32dedcc, 00d0e2f) must be merged first

## Risk Analysis & Mitigation

| Risk | Probability | Impact | Mitigation |
|------|------------|--------|------------|
| ~~React 19 incompatibility~~ | ~~Low~~ | ~~High~~ | **RESOLVED**: Fork original v1.0.0, cherry-pick `@cyca`'s React 19 patches. |
| Fork maintenance burden | Low | Low | Original is actively maintained — can pull upstream. Cherry-picked patches are isolated (virtualization + gesture lib swaps). |
| Performance with many tracks | Low | Medium | `@tanstack/virtual` handles virtualization. No per-pixel store writes. `toTimelineRows()` memoized. |
| Cursor feedback loop | Medium | Medium | Timestamp-based guard (~20ms grace). Verified: `setTime()` likely does NOT fire `onCursorDrag` (programmatic vs user-initiated), but guard is cheap insurance. |
| Undo ordering bugs | Medium | High | Extracted into `withTimelineUndoBatch()` helper with guaranteed sequence. |
| Phase resize causes cross-phase overlap | Medium | Medium | `onActionMoving` rejects overlapping positions. `recomputePhases()` derives boundaries from keyframes. |
| Fast drag start>end | Medium | Low | Reject in `onActionMoving` callback. |
| Library abandonment | Low | Medium | MIT licensed, fork-friendly. Adapter layer isolates from internals. |

### Race Conditions (from frontend races review)

| Race | Severity | Mitigation |
|------|----------|------------|
| Cursor feedback loop (RAF ↔ library events) | Medium | Timestamp guard, not boolean |
| Round-trip jitter during drag | High | Mutable ref layer — no store writes during gesture |
| Dual-store split-render (animation + video) | Low | React 19 + Zustand auto-batch (no `unstable_batchedUpdates` needed) |
| Undo during playback (stale savedStates) | Medium | `withTimelineUndoBatch` calls `stop()` + `reconcile()` |
| Video seek flicker during trim | Medium | Gate seeks behind `requestVideoFrameCallback` (Electron 35+) |
| Stale setTimeout from canvas-sync animations | Low | Pre-existing; not amplified if store writes are gesture-end only |

## Performance Budget

### Playback Frame Budget (30fps = 33ms)

| Operation | Time (20 tracks) | Time (100 tracks) |
|-----------|------------------|-------------------|
| Interpolation O(n log m) | 0.05ms | 0.25ms |
| Fabric object lookup (shared Map cache, mandatory) | 0.01ms | 0.05ms |
| Apply properties (direct mutation) | 0.02ms | 0.1ms |
| Video sync (5 elements) | 0.05ms | 0.05ms |
| `requestRenderAll()` | 1-5ms | 1-5ms |
| `setTime()` (at 30fps, skips half) | 0ms | 0ms |
| **Total** | **~1-5ms** | **~1.5-5.5ms** |

### Optimization Tasks

1. **[Mandatory, Phase 2] Build shared Fabric object Map** in `canvas-bridge.ts`: `Map<nodeId, FabricObject>` populated on `play()`, cleared on `stop()`. Shared with `video-sync.ts`. Eliminates O(n) linear scan per track per frame in both modules.
2. **Cache video node metadata** (timelineOffset, inPoint, outPoint) at playback start in `video-sync.ts` to avoid `getNodeById()` tree traversals per frame.
3. **Memoize `toTimelineRows()`** with Zustand selectors on `tracks` and `videoClipIds` only — never `currentTime`.
4. **Throttle `setTime()` to 30fps** during playback (separate from 100ms store throttle).
5. **No per-pixel store writes** during drag — mutable ref layer.

## Visual Design Spec

### Phase Colors (oklch, dark theme)

| Phase | Background | Border | Diamond (selected) |
|-------|-----------|--------|-------------------|
| In | `oklch(0.55 0.15 155 / 0.20)` | `oklch(0.65 0.18 155 / 0.50)` | `oklch(0.65 0.18 155)` |
| While | `oklch(0.55 0.12 250 / 0.15)` | `oklch(0.60 0.16 250 / 0.35)` | `oklch(0.60 0.16 250)` |
| Out | `oklch(0.55 0.18 25 / 0.20)` | `oklch(0.60 0.20 25 / 0.50)` | `oklch(0.60 0.20 25)` |

### Video Clip

- Background: `oklch(0.45 0.12 300 / 0.25)` (muted violet)
- Border: `oklch(0.55 0.15 300 / 0.40)`
- Icon/text: lighter violet variants

### Typography

- Ruler/timecodes: `ui-monospace`, 9px, `font-variant-numeric: tabular-nums`
- Track names: system sans, 11px, `var(--muted-foreground)`
- All transitions ≤120ms

### Key CSS Overrides

Target library BEM classes in `timeline-editor.css`. Reference `var(--card)`, `var(--border)`, `var(--muted-foreground)`, `var(--ring)`. Use `oklch()` with alpha for subtle tints. Never use hardcoded Tailwind color classes.

## File Structure

```
src/animation/
  timeline-adapter-types.ts    — Time conversion, ActionMetadata union, metadata map, TimelineStores interface
  timeline-adapter.ts          — Pure transform functions (toTimelineRows, applyAction*)
  timeline-adapter.test.ts     — Round-trip, conversion, edge case tests
  timeline-undo.ts             — withTimelineUndoBatch() helper

src/components/animation/
  timeline-editor.tsx           — Wrapper component with mutable ref layer
  timeline-editor.css           — Library CSS overrides
  phase-action-renderer.tsx     — Phase bar + keyframe diamond subcomponents
  video-clip-renderer.tsx       — Video clip action subcomponent
  track-headers.tsx             — Synchronized left-column track labels

src/mcp/tools/
  animation.ts                  — MCP animation tools (Phase 6)
```

## Sources & References

### Internal References

- `src/stores/timeline-store.ts` — animation track source of truth (285 LOC)
- `src/animation/playback-loop.ts` — RAF playback engine (159 LOC)
- `src/animation/canvas-bridge.ts` — Fabric.js property application (87 LOC)
- `src/animation/interpolation.ts` — keyframe interpolation with binary search (121 LOC)
- `src/animation/video-registry.ts` — HTMLVideoElement management (53 LOC)
- `src/animation/video-sync.ts` — video frame sync during playback (82 LOC)
- `src/animation/animation-persistence.ts` — document serialization (59 LOC)
- `src/canvas/canvas-sync-lock.ts` — sync lock pattern reference (17 LOC)
- `src/components/animation/` — current UI components to replace (~700 LOC total)
- `docs/plans/2026-03-10-openpencil-video-animation-extension-plan.md` — architecture decisions

### External References

- [react-timeline-editor GitHub](https://github.com/xzdarcy/react-timeline-editor) — 708 stars, MIT, actively maintained (v1.0.0 Jan 2026). Base for our fork.
- [@cyca/react-timeline-editor npm](https://www.npmjs.com/package/@cyca/react-timeline-editor) — React 19 fork (47 downloads/week, alpha). Source for cherry-picked React 19 patches.
- [react-virtualized findDOMNode crash — Issue #1858](https://github.com/bvaughn/react-virtualized/issues/1858)
- Library TypeScript types: `TimelineRow`, `TimelineAction`, `TimelineEffect`, `TimelineState`

### Related Work

- Video support commits: `32dedcc` (engine), `00d0e2f` (UI) on `feat/animation-core`
- Jeans content workflow: `worktrees/feat-jeans-content-workflow` (phases 1-3 complete)
