---
title: Clip Model Redesign — Visibility Windows with In/Out Properties
type: refactor
status: completed
date: 2026-03-13
origin: docs/brainstorms/2026-03-13-clip-model-redesign-brainstorm.md
---

# Clip Model Redesign — Visibility Windows with In/Out Properties

## Enhancement Summary

**Deepened on:** 2026-03-13
**Reviewed on:** 2026-03-13
**Sections enhanced:** All
**Research agents used:** best-practices-researcher, framework-docs-researcher, architecture-strategist, performance-oracle, pattern-recognition-specialist, code-simplicity-reviewer, kieran-typescript-reviewer, repo-research-analyst

### Key Improvements from Research
1. **Keep discriminated union** — all reviewers agree: preserve `kind` field, extend with effect properties rather than flattening to unified type
2. **Keep stored keyframes** — generate at authoring time, store on clip, interpolate at playback (matches Lottie/industry pattern)
3. **Effects on AnimationClipData only** — video clips don't use In/Out effects per brainstorm; putting effects on ClipBase is a type-level lie
4. **Two effect config types** — `TimedEffectConfig` (required duration for in/out) and `LoopEffectConfig` (no duration for emphasis); single `EffectConfig` with optional duration hides distinct invariants
5. **Flat sourceStart/sourceEnd** — `SourceRange` nesting rejected; adds access-path churn across 6+ files for zero type safety gain
6. **Emphasis deferred** — only 2 effects exist, loop interpolation under-specified, no UI for loop control
7. **Video spine deferred** — zero consumers exist, separate future plan
8. **Identified 5 missing source files and 6 missing test files** that would break

### Critical Regressions Caught
- Plan originally dropped `extrapolate` field (actively used in interpolation engine)
- Plan originally dropped `playbackRate` field (used in video sync with tests)
- Plan originally dropped `kind` discriminant (breaks exhaustive checking codebase-wide)

### Review Findings Applied (P1 + P2)
- **P1**: Merged Phase 8 source files into Phase 1 — video-sync.ts, toolbar.tsx must ship atomically with type change
- **P2**: Moved `inEffect`/`outEffect` from `ClipBase` to `AnimationClipData` only
- **P2**: Split `EffectConfig` into `TimedEffectConfig` and `LoopEffectConfig`
- **P2**: Kept flat `sourceStart`/`sourceEnd` — removed `SourceRange` nesting
- **P2**: Deferred `emphasisEffect` entirely from this plan
- **P2**: Added preset-panel video controls to Phase 6 scope
- **P2**: Scoped Phase 7 In/Out dividers as display-only (`pointer-events: none`)
- **P2**: Fixed pre-existing bug in `buildTimelineRowsFromNodes` metadata (Phase 3)

---

## Overview

Extend the clip data model so a **clip = visibility window** — the duration for which a layer is visible on the timeline. In/Out animations become properties on `AnimationClipData`, not separate clips. Keep the discriminated union (`AnimationClipData | VideoClipData`) for type safety. Move video temporal fields from `VideoNode` to `VideoClipData`.

(see brainstorm: `docs/brainstorms/2026-03-13-clip-model-redesign-brainstorm.md`)

## Problem Statement / Motivation

The current model creates **separate clips for each effect** (fade-in, hold, fade-out). Each is an independent `AnimationClipData` with its own `startTime`, `duration`, and `keyframes`. This creates problems:

1. **Clips can overlap, gap, or be moved independently** — doesn't match the mental model of "this thing is on screen from t0 to t1"
2. **No concept of visibility** — clips are just keyframe containers, not "the layer is visible here"
3. **Video timing is split** — `VideoClipData` has `sourceStart`/`sourceEnd` but `VideoNode` also has `inPoint`/`outPoint`/`timelineOffset`, creating duplication and ambiguity
4. **No In/Out as first-class concepts** — effects are referenced by `effectId` but there's no structural distinction between an entrance and exit effect on a clip

## Proposed Solution

Extend the existing discriminated union:
- `startTime` + `duration` = the visibility window (on `ClipBase`)
- `inEffect` / `outEffect` = optional timed animation configs on `AnimationClipData` only (pinned to left/right edges)
- Move video temporal fields (`inPoint`/`outPoint`/`timelineOffset`) from `VideoNode` to `VideoClipData`
- Keep `kind` discriminant for exhaustive type checking
- Emphasis effects deferred to future plan

### Research Insights

**Industry validation:** The clip-as-visibility-window pattern is universal across Remotion (`<Sequence>`), MoviePy (`Clip`), FFmpeg filter graphs, and all major NLEs. Every system models clips as `startTime + duration = when content is visible`, with effects as properties. (Source: best-practices-researcher)

**Keyframe strategy:** Lottie stores keyframes with bezier control points in JSON and interpolates at render time — it does NOT pre-compute intermediate frames. Our current hybrid (generate from effects at authoring time, store on clip, interpolate at playback) matches this pattern and is optimal. (Source: best-practices-researcher)

**Type design:** Discriminated unions are used consistently throughout this codebase (`PenNode` on `type`, `PenFill` on `type`, `PenEffect` on `type`, `AnimationClip` on `kind`). Flattening to optional fields would be a pattern break that loses compile-time narrowing. (Source: pattern-recognition-specialist, kieran-typescript-reviewer)

## Technical Approach

### Architecture

```
Current:
  node.clips: (AnimationClipData | VideoClipData)[] — one clip per effect
  VideoNode: { timelineOffset, inPoint, outPoint } — timing duplicated on node

New:
  node.clips: (AnimationClipData | VideoClipData)[] — each clip is a visibility window
  AnimationClipData: extends ClipBase + { kind, keyframes, inEffect?, outEffect? }
  VideoClipData: extends ClipBase + { kind, sourceStart, sourceEnd, playbackRate }
  VideoNode: only intrinsic metadata (src, videoDuration, mimeType)
```

### Data Model (revised from brainstorm + all review rounds)

```typescript
// Timed effect config — for in/out transitions that have a required duration
interface TimedEffectConfig {
  effectId: string
  duration: number          // ms — required, how long the transition lasts
  params?: Record<string, unknown>
}

// Loop effect config — for emphasis effects that derive duration from hold
// DEFERRED: not included in this plan, shown here for future reference
// interface LoopEffectConfig {
//   effectId: string
//   params?: Record<string, unknown>
// }

// Base clip — shared between animation and video
interface ClipBase {
  id: string
  startTime: number         // ms — when this visibility window begins on the timeline
  duration: number          // ms — how long the layer is visible
  extrapolate?: 'clamp' | 'hold'  // PRESERVED — actively used in interpolation
}

// Animation clip — keeps stored keyframes + kind discriminant
// In/Out effects live HERE, not on ClipBase (video clips don't use them)
interface AnimationClipData extends ClipBase {
  kind: 'animation'
  /** @deprecated Use inEffect/outEffect instead. Kept for migration. */
  effectId?: string
  keyframes: KeyframeV2[]   // KEPT — generated at authoring time, stored on clip
  /** @deprecated Use effect config params instead. */
  params?: Record<string, unknown>
  inEffect?: TimedEffectConfig    // entrance animation pinned to left edge
  outEffect?: TimedEffectConfig   // exit animation pinned to right edge
  // emphasisEffect DEFERRED to future plan
}

// Video clip — absorbs timing from VideoNode + kind discriminant
interface VideoClipData extends ClipBase {
  kind: 'video'
  sourceStart: number       // ms — in-point into source media (kept flat)
  sourceEnd: number         // ms — out-point into source media (kept flat)
  playbackRate: number      // PRESERVED — used in video-sync
}

type Clip = AnimationClipData | VideoClipData

// Type guards for .filter() usage
function isVideoClip(clip: Clip): clip is VideoClipData {
  return clip.kind === 'video'
}

function isAnimationClip(clip: Clip): clip is AnimationClipData {
  return clip.kind === 'animation'
}
```

**Derived values (pure helpers in `clip-utils.ts`):**
```typescript
function getHoldDuration(clip: AnimationClipData): number {
  return clip.duration - (clip.inEffect?.duration ?? 0) - (clip.outEffect?.duration ?? 0)
}

function getInEnd(clip: AnimationClipData): number {
  return clip.startTime + (clip.inEffect?.duration ?? 0)
}

function getOutStart(clip: AnimationClipData): number {
  return clip.startTime + clip.duration - (clip.outEffect?.duration ?? 0)
}
```

**Validation rules:**
- `(inEffect?.duration ?? 0) + (outEffect?.duration ?? 0) <= clip.duration`
- `inEffect?.duration >= 0`, `outEffect?.duration >= 0`
- For video: `sourceEnd - sourceStart` defines available media
- For video transitions (future): adjacent clips must have sufficient media handles

### Research Insights: Type Design

**Why effects on AnimationClipData only (not ClipBase):**
- Video clips don't have In/Out animations per brainstorm decision
- Putting effects on `ClipBase` creates a type-level lie — `VideoClipData` would inherit `inEffect`/`outEffect` that can never be used
- The derived helpers (`getHoldDuration`, `getInEnd`, `getOutStart`) only make sense for animation clips
- (Source: kieran-typescript-reviewer)

**Why two effect config types (not one with optional duration):**
- `TimedEffectConfig` has required `duration` — in/out transitions MUST specify how long they last
- `LoopEffectConfig` (deferred) has NO duration — emphasis derives from hold duration
- A single `EffectConfig` with optional `duration` hides this distinction: callers must null-check a field that's always present for in/out, never present for emphasis
- (Source: kieran-typescript-reviewer)

**Why flat sourceStart/sourceEnd (not SourceRange):**
- Nesting adds access-path churn (`clip.sourceRange.start` vs `clip.sourceStart`) across 6+ files
- Zero type safety gain — both are just `number`
- Matches existing pattern: `ClipBase` already has flat `startTime`/`duration`
- (Source: code-simplicity-reviewer, kieran-typescript-reviewer)

**Why keep `kind` discriminant (critical):**
- All reviewers agree: removing `kind` breaks exhaustive checking
- `video-sync.ts` filters by `c.kind === 'video'` in 3 places — structural narrowing is essential
- Future clip types (audio, transition) extend naturally with discriminated unions
- Impossible states (clip with both `keyframes` AND `sourceStart`) remain unrepresentable

**Why keep stored keyframes (critical):**
- Generating keyframes 60x/sec per clip = 1,200-2,400 object allocations/second = GC pressure and jank (Source: performance-oracle)
- `kfId()` in effects uses `Math.random().toString(36)` — non-trivial string allocation per keyframe
- Current hybrid (generate at authoring, store, interpolate at playback) is identical to Lottie's approach

### Implementation Phases

#### Phase 1: Type System, Data Model, and Dependent Source Files

Extend `ClipBase` with effect properties on `AnimationClipData`. Restructure `VideoClipData`. Keep discriminated union. **Update all files that read VideoNode timing fields atomically** — removing `timelineOffset`/`inPoint`/`outPoint` from `VideoNode` immediately breaks video-sync.ts and toolbar.tsx.

**Files to modify:**

- [x] **`src/types/animation.ts`** — Add `TimedEffectConfig` interface. Extend `AnimationClipData` with `inEffect?`, `outEffect?` as `TimedEffectConfig`. Keep flat `sourceStart`/`sourceEnd` on `VideoClipData`. Keep `kind` discriminant. Keep `keyframes` on `AnimationClipData`. Keep `extrapolate` on `ClipBase`. Keep `playbackRate` on `VideoClipData`. Add `isVideoClip`/`isAnimationClip` type guards. Rename `AnimationClip` type alias to `Clip`.

- [x] **`src/types/pen.ts`** — Remove video-specific timing fields from `VideoNode` (`timelineOffset`, `inPoint`, `outPoint`) — these are now on `VideoClipData` fields and `ClipBase.startTime`. Keep `videoDuration`, `src`, `mimeType` on `VideoNode` as intrinsic metadata.

- [x] **`src/animation/video-sync.ts`** — **ATOMIC with Phase 1.** Filters by `c.kind === 'video'` in 3 places. Currently reads `videoNode.timelineOffset`, `videoNode.inPoint`, `videoNode.outPoint`. Must update to read from `VideoClipData.sourceStart`/`sourceEnd` and `ClipBase.startTime`.

- [x] **`src/components/editor/toolbar.tsx`** — **ATOMIC with Phase 1.** Currently sets `inPoint`, `outPoint`, `timelineOffset` when creating new video nodes (lines 214-216). Must create a `VideoClipData` with `sourceStart`/`sourceEnd` instead of setting timing fields on the node.

- [x] **`src/animation/animation-persistence.ts`** — Uses `clip.kind !== 'animation'` for filtering. Update for new effect config fields on `AnimationClipData`.

- [x] **`src/components/panels/property-panel.tsx`** — Uses `clip.kind === 'animation'` for display text. Update labels for new clip structure.

- [x] **`src/mcp/tools/animation.ts`** — Imports `AnimationClip`, `AnimationClipData`; checks `clip.kind`; writes `clip.keyframes`. Update imports (`AnimationClip` → `Clip`) and ensure new clip creation uses `TimedEffectConfig` for effects.

#### Phase 2: Clip Utilities

- [x] **Add `src/animation/clip-utils.ts`** — 3 pure derivation functions:
  - `getHoldDuration(clip: AnimationClipData): number`
  - `getInEnd(clip: AnimationClipData): number`
  - `getOutStart(clip: AnimationClipData): number`

### Research Insights: Utility Design

Only 3 functions needed now. `validateClip()` — enforce at the single creation point (preset-panel). `isVideoClip()` — the `kind` discriminant handles this. `clampInOutDurations()` — defensive code for a constraint the setter should enforce. (Source: code-simplicity-reviewer)

#### Phase 3: Timeline Adapter Rewrite

Each `Clip` becomes a **single** `TimelineAction`. The renderer visualizes In/Hold/Out segments within the bar. **Fix pre-existing bug**: `buildTimelineRowsFromNodes` currently stores ALL clips as `type: 'animation-clip'` in metadata regardless of actual `clip.kind`, so video clips never get video-specific move/resize handling.

- [x] **`src/animation/timeline-adapter-types.ts`** — Simplify metadata to unified `ClipMetadata`:

```typescript
interface ClipMetadata {
  type: 'clip'
  nodeId: string
  clipId: string
  clipKind: 'animation' | 'video'
}
```

- [x] **`src/animation/timeline-adapter.ts`** — Rewrite `buildTimelineRowsFromNodes`:
  - Each node with `clips` → one `TimelineRow`
  - Each `Clip` → one `TimelineAction` with `ClipMetadata`
  - **Fix bug**: Set `clipKind` from actual `clip.kind` (not hardcoded `'animation-clip'`)
  - `applyActionMove` / `applyActionResize` update `ClipBase.startTime`/`duration` directly
  - Video clip resize: adjusts `sourceStart`/`sourceEnd` when trimming left/right
  - Remove separate `applyVideoClipResize`, `validateVideoClipBounds` (unified handling)

#### Phase 4: Animation Index Update

- [x] **`src/animation/animation-index.ts`** — Update type import from `AnimationClip` to `Clip`. Structure stays the same.

#### Phase 5: Playback Controller Updates

Update interpolation to handle In/Hold/Out segments within a clip. **Keep `interpolateClip` as a pure function** — do NOT couple it to the effect registry.

- [x] **`src/animation/interpolation.ts`** — Add segment-aware interpolation:
  1. Check if `timeMs` is within the clip's visibility window → return `null` if outside
  2. For animation clips with `inEffect`/`outEffect`: determine segment using `getInEnd`/`getOutStart` from clip-utils
  3. For in/out segments: the clip's stored `keyframes` still provide the animation data. The `inEffect`/`outEffect` metadata guides which portion of keyframes to use.
  4. For hold: use rest-state keyframe values
  5. Interpolate using existing binary search + bezier logic

### Research Insights: Playback Performance

**Generate keyframes at authoring time, not per frame (critical):**
- Front-load generation: when user applies an effect in preset-panel, generate keyframes immediately and store on clip
- At playback: only interpolate stored keyframes (current hot path unchanged)
- If switching to on-the-fly generation later: cache in `Map<clipId, KeyframeV2[]>` built at play-start, not per frame
- Wire up `TrackBuffer` in `onFrame` for per-frame result object reuse (already exists at `src/animation/track-buffer.ts`, just not wired up)
- Skip interpolation for clips outside their time window BEFORE calling `interpolateClip` (set `obj.visible = false` and `continue`)

(Source: performance-oracle)

- [x] **`src/animation/use-playback-controller.ts`** — Update `onFrame` callback:
  - Use `isVideoClip(clip)` type guard instead of `clip.kind === 'video'`
  - Pass `TrackBuffer` to `interpolateClip` for object reuse
  - Guard: `buildAnimationIndex` must never be called during RAF loop (only at play-start)

#### Phase 6: Preset Panel — New Clip Creation Flow

- [x] **`src/components/animation/preset-panel.tsx`** — Rewrite effect application:
  - **Current**: Each effect creates a new `AnimationClipData` appended to `node.clips[]`
  - **New**: If no clip exists, create a visibility-window clip first. Then set `inEffect`/`outEffect` on it.
  - Enter effects → `clip.inEffect = { effectId, duration }` + generate keyframes for the in segment
  - Exit effects → `clip.outEffect = { effectId, duration }` + generate keyframes for the out segment
  - Toggling off: clear the effect config and remove corresponding keyframes
  - Enforce `inEffect.duration + outEffect.duration <= clip.duration` at this point
  - **Video controls** (lines 79-133): Update to read from `VideoClipData.sourceStart`/`sourceEnd` instead of `VideoNode.inPoint`/`outPoint`/`timelineOffset`

#### Phase 7: Timeline Renderers — Unified Clip Visualization

- [x] **`src/components/animation/animation-clip-renderer.tsx`** — Rewrite to show In/Hold/Out regions:
  - Three internal flex regions: In (accent left), Hold (main body), Out (accent right)
  - In/Out widths proportional to `inEffect.duration / clip.duration`
  - Effect name labels in In/Out segments
  - In/Out divider edges are **display-only** (`pointer-events: none`) — no sub-action dragging in this plan
  - No In/Out = solid bar (pop on/off)

- [x] **`src/components/animation/video-clip-renderer.tsx`** — Update for new `VideoClipData`:
  - Read flat `sourceStart`/`sourceEnd` (no change from current field names)
  - Film strip background for full clip
  - No In/Out segments (video clips don't have animation effects per brainstorm)

- [x] **`src/components/animation/timeline-editor.tsx`** — Update `getActionRender`:
  - Read `ClipMetadata` from unified metadata map
  - Route to animation clip renderer or video clip renderer based on `meta.clipKind`

### Research Insights: Timeline Library

**Sub-action dragging (In/Out segment edge dragging within a clip):**
The `@cyca/react-timeline-editor` library does NOT support sub-action dragging natively. Two options:
- **Option A**: Model In/Hold/Out as separate `TimelineAction` items within the same row, with `onActionMoving` constraints to enforce adjacency.
- **Option B**: Custom `onPointerDown`/`onPointerMove` handlers on internal divider elements with `e.stopPropagation()` to prevent library drag. More control, keeps single-action-per-clip model.
- **Recommendation**: Option B is the right approach but **deferred from this plan**. In/Out dividers render as display-only visual indicators. Segment edge dragging is a follow-up.

(Source: framework-docs-researcher)

#### Phase 8: Test Updates

All test files need type/assertion updates for the new clip shape:

- [x] `src/animation/timeline-adapter.test.ts`
- [x] `src/animation/animation-persistence.test.ts`
- [x] `src/animation/video-sync.test.ts`
- [x] `src/animation/animation-index.test.ts`
- [x] `src/animation/interpolation.test.ts`
- [x] `src/mcp/tools/animation.test.ts`

## System-Wide Impact

### Interaction Graph

- `preset-panel.tsx` → creates/modifies `Clip` on node → triggers `document-store.updateNode` → triggers `use-canvas-sync` re-render → triggers timeline recompute via `clipVersion` hash
- `timeline-editor.tsx` drag/resize → calls `timeline-adapter.applyActionMove/Resize` → updates `document-store` → same flow
- `use-playback-controller.ts` → reads `AnimationIndex` → calls `interpolateClip` → applies to Fabric objects
- `animation-persistence.ts` → filters clips on load (uses `clip.kind`)
- `video-sync.ts` → reads `VideoClipData.sourceStart`/`sourceEnd` for frame seeking (was reading VideoNode fields)
- `toolbar.tsx` → creates video nodes with clips (was setting timing on VideoNode directly)
- `mcp/tools/animation.ts` → creates clips programmatically (must use new type shape)

### Error Propagation

- Invalid clip durations (in+out > clip) → enforce in preset-panel at creation time
- Missing effect IDs → `interpolateClip` returns `null` for that segment, layer stays at rest state
- Video source range exceeded → clamped at creation time in preset-panel

### State Lifecycle Risks

- **Migration**: The v2 clip format was introduced in the last few commits on this dev branch. No external users have `.pen` files with the old format. **Simplified strategy**: In `normalize-pen-file.ts`, drop clips that have the old `kind: 'animation'` shape without `inEffect`/`outEffect` fields. Log a console warning. Do not attempt complex merging logic for clips that are days old.
- **Video nodes**: Migrate `timelineOffset`/`inPoint`/`outPoint` from `VideoNode` to `VideoClipData.sourceStart`/`sourceEnd` + `ClipBase.startTime`. Keep `videoDuration` on `VideoNode` as intrinsic metadata.

### Research Insights: Migration

The simplicity reviewer found that the v2 engine was merged in the last few commits (`67dcb66`, `80c0c25`, `90b64cf`). Complex clip-merging migration logic is over-engineering for clips created days ago on a dev branch. Drop-and-warn is sufficient. (Source: code-simplicity-reviewer)

### API Surface Parity

- `src/animation/timeline-undo.ts` — Imports `injectAnimationData()` from persistence (currently a no-op). If no-op stubs are removed, this import breaks. Update import.
- `src/animation/animation-persistence.ts` — Filter logic uses `clip.kind` checks that remain valid with discriminated union preserved.
- `src/mcp/tools/animation.ts` — Must create clips with new shape: `AnimationClipData` with `inEffect`/`outEffect` configs.

### Integration Test Scenarios

1. **Create clip with in+out effects → playback shows fade-in, hold, fade-out correctly**
2. **Resize clip left edge → in effect stays pinned to left, hold shrinks, out stays pinned to right**
3. **Remove in effect from clip → layer pops on instantly, hold starts immediately**
4. **Video clip with temporal crop → sourceStart/sourceEnd defines which portion plays**
5. **Drop-and-warn migration → old clips are removed on load, console warning logged**
6. **New video node creation → toolbar creates VideoClipData with sourceStart/sourceEnd, not node-level fields**

## Acceptance Criteria

### Functional Requirements

- [x] `AnimationClipData` extended with `inEffect`, `outEffect` as `TimedEffectConfig` properties
- [x] Discriminated union preserved: `AnimationClipData | VideoClipData` with `kind` field
- [x] In/Out are optional effect configs on animation clips, not separate clips
- [x] Timeline shows one bar per clip with In/Hold/Out visual regions (display-only dividers)
- [x] Resizing clip left edge adjusts appearance time; right edge adjusts disappearance time
- [x] Video timing moved from `VideoNode` to `VideoClipData.sourceStart`/`sourceEnd` + `ClipBase.startTime`
- [x] Playback correctly interpolates In, Hold, and Out phases
- [x] All 7 source files updated for new types (including 3 files atomic with Phase 1)
- [x] All 6 test files updated and passing
- [x] Pre-existing metadata bug in `buildTimelineRowsFromNodes` fixed

### Non-Functional Requirements

- [x] Keyframes remain stored on clips (generated at authoring time, not per-frame)
- [x] `extrapolate` and `playbackRate` fields preserved
- [x] No new GC pressure in playback hot path
- [x] Type check passes (`npx tsc --noEmit`)

### Quality Gates

- [x] All existing animation tests pass (updated for new types)
- [x] Type check passes
- [ ] Manual test: create clip with fade-in + fade-out, play, verify smooth animation
- [ ] Manual test: create video node, verify timeline shows clip with sourceStart/sourceEnd

## Dependencies & Risks

**Dependencies:**
- Effect registry must be loaded before playback (already the case)
- `normalize-pen-file.ts` migration runs before any clip access on file load

**Risks:**
- **VideoNode field removal scope**: `toolbar.tsx`, `video-sync.ts`, `preset-panel.tsx`, and `timeline-adapter.ts` all read/write VideoNode timing fields. Must update all atomically in Phase 1.
- **Timeline library compatibility**: Rendering In/Out/Hold segments within a single action requires custom pointer handling for segment edge dragging. **Deferred** — dividers are display-only in this plan.
- **MCP tool breakage**: `mcp/tools/animation.ts` creates clips programmatically. External LLMs using MCP may send old clip shapes — need graceful handling.

### Research Insights: Risk Mitigation

**Performance:** Keep `interpolateClip` as a pure function — do NOT add effect-registry dependency. The caller resolves keyframes. Wire up `TrackBuffer` for object reuse. Skip interpolation for out-of-window clips early. (Source: architecture-strategist, performance-oracle)

**Atomicity:** `interpolation.ts` and `use-playback-controller.ts` form an atomic unit of change — they cannot be modified independently. The onFrame callback calls interpolateClip directly. (Source: architecture-strategist)

## Alternative Approaches Considered

**Option A (original plan): Flatten to unified `Clip` type, drop `kind` discriminant.** Rejected by all reviewers. Loses compile-time narrowing, allows impossible states (clip with both keyframes AND sourceStart), breaks exhaustive checking pattern used throughout codebase.

**Option B: Keep separate clips but add a "container" parent clip.** Rejected — adds complexity without solving the core problem.

**Option C: Remove stored keyframes, generate on-the-fly per frame.** Rejected by performance-oracle — 1,200-2,400 allocations/second from `effect.generate()` calls, `kfId()` uses `Math.random().toString(36)` per keyframe. If pursued later, must front-load to play-start and cache.

**Option D: Single `EffectConfig` with optional `duration` for all effect types.** Rejected by TypeScript reviewer — hides distinct invariants between timed (in/out) and loop (emphasis) effects.

**Option E: Nested `SourceRange` for video clips.** Rejected — adds access-path churn across 6+ files for zero type safety gain.

**Chosen: Extend discriminated union with `TimedEffectConfig` on `AnimationClipData`, flat video source fields.** Matches Lottie pattern (store keyframes, interpolate at runtime). Preserves all existing type safety. Minimal hot-path changes.

(see brainstorm: "What Changes From Current Architecture" table)

## Deferred to Future Plans

These items from the brainstorm are explicitly deferred:

| Item | Reason | When to Revisit |
|------|--------|----------------|
| Emphasis effects (`emphasisEffect`) | Only 2 effects exist, loop interpolation under-specified, no UI | When emphasis effects are a user-facing feature |
| Video spine / magnetic behavior | Zero consumers, needs drag handler rewrite | When multiple video clips on timeline is a real use case |
| Frame-borrowing transitions | No transition renderer/effects/UI exist | After video spine ships |
| `videoSpineOrder` on timeline-store | Dead state with no readers | With video spine plan |
| Visibility toggling (hide nodes outside clip range) | New runtime behavior layered on type refactor | Separate small PR after type refactor lands |
| In/Out segment edge dragging | Requires custom pointer handling (library limitation) | Follow-up after this plan ships; Option B (`stopPropagation`) is the right approach |
| Custom keyframe editing | Users can't author custom curves if only effect presets generate keyframes | Open question from brainstorm — defer until power-user demand |
| TrackBuffer wiring | Performance optimization with no measured problem | When playback perf is actually degraded |

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-13-clip-model-redesign-brainstorm.md](docs/brainstorms/2026-03-13-clip-model-redesign-brainstorm.md) — Key decisions: clip = visibility window, In/Out as properties, video spine with magnetic behavior (deferred), frame-borrowing transitions (deferred)

### Internal References

- Current types: `src/types/animation.ts:42-68` — `ClipBase`, `AnimationClipData`, `VideoClipData`
- Current adapter: `src/animation/timeline-adapter.ts:227-257` — `buildTimelineRowsFromNodes`
- Current playback: `src/animation/use-playback-controller.ts:81-99` — `onFrame` callback
- Current preset panel: `src/components/animation/preset-panel.tsx:37-66` — effect toggle logic
- Effect registry: `src/animation/effect-registry.ts:51-73` — `generateClipFromEffect`
- Animation index: `src/animation/animation-index.ts:28-33` — `buildAnimationIndex`
- Video node type: `src/types/pen.ts:184-192` — `VideoNode` with timing fields
- Track buffer: `src/animation/track-buffer.ts` — per-node interpolation cache (wire up deferred)
- Video sync: `src/animation/video-sync.ts` — video clip filtering and frame seeking
- Toolbar: `src/components/editor/toolbar.tsx:214-216` — video node creation with timing fields
- Property panel: `src/components/panels/property-panel.tsx:375` — clip kind display
- MCP animation tool: `src/mcp/tools/animation.ts` — programmatic clip creation
- Pre-existing bug: `src/animation/timeline-adapter.ts:239` — metadata always stores `type: 'animation-clip'`

### External References

- Apple FCPXML timing model: `offset` (timeline position), `start` (source in-point), `duration`
- Remotion Sequence: clip = visibility window with `from` + `durationInFrames`
- Lottie spec: keyframes stored with bezier handles, interpolated at render time
- @cyca/react-timeline-editor: `getActionRender` returns ReactNode inside gesture-managed wrapper; resize handles are siblings; sub-action dragging requires custom pointer handling with `stopPropagation`

### Related Work

- Previous plan: `docs/plans/2026-03-11-feat-animation-engine-v2-plan.md` — v2 engine foundation
- Previous plan: `docs/plans/2026-03-11-refactor-v2-engine-migration-plan.md` — v1→v2 migration
