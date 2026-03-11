---
title: "feat: Animation Engine v2 — Professional-Grade Foundation"
type: feat
status: completed
date: 2026-03-11
deepened: 2026-03-11
origin: docs/brainstorms/2026-03-11-animation-engine-v2-brainstorm.md
---

# Animation Engine v2 — Professional-Grade Foundation

## Enhancement Summary

**Deepened on:** 2026-03-11
**Sections enhanced:** All 10 phases + architecture
**Review agents used:** TypeScript Reviewer, Pattern Recognition, Performance Oracle, Code Simplicity, Architecture Strategist, SOLID Reviewer, Clean Code Reviewer, Security Sentinel, DHH Rails Reviewer, Data Integrity Guardian, Frontend Races Reviewer, Agent-Native Reviewer, Fabric.js/Bezier/OKLab Research

### Key Improvements (from 13 parallel agents)

1. **Discriminated union for clips** — Use `kind: 'animation' | 'video'` instead of optional `videoData?`. Eliminates Liskov Substitution violations. (SOLID, Architecture, TypeScript — consensus)
2. **Split property descriptor** — Separate pure animation descriptor (`AnimatablePropertyDescriptor`) from Fabric canvas binding (`CanvasPropertyBinding`). Engine stays renderer-agnostic. (Clean Code, SOLID, Architecture — consensus)
3. **Move `fabricObjectMap` out of `AnimationIndex`** — Keep index as pure data; canvas-bridge owns the Fabric mapping. (Clean Code, SOLID, Architecture — consensus)
4. **`useSyncExternalStore` for playback time** — Move `currentTime` out of Zustand during playback. Prevents full-tree re-renders at 60fps. (Performance Oracle — P0)
5. **Pre-allocated `TrackBuffer`** — Eliminate per-frame GC pressure with reusable interpolation buffers. (Performance Oracle — Critical)
6. **`requiresCacheInvalidation` flag** — Transform animations skip `obj.dirty = true`, saving 3-5ms/frame. (Performance Oracle)
7. **Defer OKLab to v3** — sRGB lerp is sufficient for v2 launch. Reduces scope and complexity. (Code Simplicity)
8. **Force-pause as Zustand middleware** — Intercept document-store mutations, auto-pause if playing. Cleaner than manual guards. (Architecture Strategist)
9. **camelCase for registries** — `propertyRegistry`/`effectRegistry` to match existing codebase patterns (`roleRegistry`, `videoElements`). (Pattern Recognition)

### New Considerations Discovered

- Force-pause MUST be synchronous — one stale rAF tick between mutation and pause can corrupt canvas (Frontend Races)
- Queue async asset completions (image onload, video loadeddata) during playback; flush on stop (Frontend Races)
- Use `renderAll()` inside rAF loop, not `requestRenderAll()` — avoids 1-frame lag (Fabric.js Research)
- Skip `setCoords()` during playback (no user interaction), call once on stop (Fabric.js Research)
- Transform props (left/top/scaleX/scaleY/angle/opacity) don't invalidate Fabric cache — essentially free to animate (Fabric.js Research)
- Replace timestamp-based cursor guard with state-machine suppression during playback (Frontend Races)
- Discard `savedStates` on undo-triggered pause; re-derive rest state from post-undo document (Frontend Races)
- Video: native playback + drift correction (50ms threshold), not seek-every-frame (Frontend Races)
- Spring physics as first-class easing type (~100 LOC, pre-bake to sample table) — future (Bezier Research)
- `extrapolate?: 'clamp' | 'hold'` on clips for beyond-boundary behavior (Remotion Patterns)
- Inline bezier-easing algorithm (~120 LOC, zero deps) — vendor, don't depend (Bezier Research)
- Need dedicated MCP animation tools phase — 4 tools mentioned but unspecified (Agent-Native)
- Add `animation` section to `get_design_prompt` for external LLM agent parity (Agent-Native)

---

## Overview

Replace the Phase 1 animation engine with a **Rive-class, professional-grade animation foundation** that supports unified clips, an effect registry, cubic bezier easing, broad animatable property coverage, and perceptually uniform color interpolation. Animation data lives on PenNodes (like fills/strokes/effects), not in a separate store.

This plan covers **Layer 1 only** — the engine. The timeline library integration (`@cyca/react-timeline-editor`) ships first per existing plan. The Jeans UX simplification layer comes after both.

(see brainstorm: `docs/brainstorms/2026-03-11-animation-engine-v2-brainstorm.md`)

## Problem Statement / Motivation

The Phase 1 engine is a proof-of-concept with:
- Only 6 animatable properties (x, y, scaleX, scaleY, rotation, opacity)
- Rigid phase model (in/while/out) that doesn't map to NLE thinking
- Separate `timeline-store` creating fragile dual-store persistence
- Named easing presets only (no cubic bezier curves)
- No effect registry — presets are utility functions, not engine concepts
- No color, typography, or visual property animation

Building creator-friendly UX on this foundation means hitting walls constantly or rewriting later. The engine must be professional-grade before the Jeans simplification layer goes on top.

## Proposed Solution

### Architecture: Three Layers

```
┌─────────────────────────────────────────────┐
│  Layer 3: Jeans UX (FUTURE)                 │
│  One-tap presets, Vibe Kit $anim-enter/exit  │
├─────────────────────────────────────────────┤
│  Layer 2: Timeline UI (SHIPS FIRST)         │
│  @cyca/react-timeline-editor as visual editor│
├─────────────────────────────────────────────┤
│  Layer 1: Engine v2 (THIS PLAN)             │
│  Clips, effects, bezier, property registry   │
└─────────────────────────────────────────────┘
```

### Data Model: Animation on Nodes

Animation data lives ON PenNodes, following the same pattern as fills, strokes, and effects:

```typescript
// Added to PenNodeBase in pen.ts
interface PenNodeBase {
  // ...existing properties
  clips?: AnimationClip[]
}
```

### Core Type Definitions

> **Research insight (TypeScript Reviewer):** Use discriminated unions for clip types. Use branded types or template literals for stronger compile-time validation of color values.

```typescript
// src/types/animation.ts — replaces current types

/** Cubic bezier control points [x1, y1, x2, y2] */
type CubicBezier = [number, number, number, number]

/** Named easing presets mapped to bezier values */
type EasingPreset = 'linear' | 'ease' | 'easeIn' | 'easeOut' | 'easeInOut'
  | 'snappy' | 'bouncy' | 'gentle' | 'smooth'

/** Easing can be a named preset or custom bezier */
type Easing = EasingPreset | CubicBezier

/** Hex color string for type-safe color values */
type HexColor = `#${string}`

/** Values that can be animated */
type AnimatableValue = number | HexColor

/** A single keyframe within a clip */
interface Keyframe {
  id: string
  /** Offset within the clip, 0.0–1.0 (percentage-based, duration-independent) */
  offset: number
  /** Animated property values at this keyframe */
  properties: Record<string, AnimatableValue>
  /** Easing to the NEXT keyframe */
  easing: Easing
}

/** Base clip fields shared by all clip kinds */
interface ClipBase {
  id: string
  /** Start time on composition timeline (ms) */
  startTime: number
  /** Duration of this clip (ms) */
  duration: number
  /** Behavior when time is outside clip bounds */
  extrapolate?: 'clamp' | 'hold'
}

/** Animation clip — keyframe-driven property animation */
interface AnimationClipData extends ClipBase {
  kind: 'animation'
  /** Effect from registry, or undefined for raw keyframe animation */
  effectId?: string
  /** Keyframes within this clip (offsets are 0–1 relative to clip duration) */
  keyframes: Keyframe[]
  /** Effect-specific parameters (e.g., slide direction, bounce height) */
  params?: Record<string, unknown>
}

/** Video clip — source media playback */
interface VideoClipData extends ClipBase {
  kind: 'video'
  /** Source time range within the video file */
  sourceStart: number  // ms
  sourceEnd: number    // ms
  /** Playback rate (1.0 = normal, 0.5 = slow-mo, 2.0 = fast) */
  playbackRate: number
}

/** Discriminated union — clips are either animation or video */
type AnimationClip = AnimationClipData | VideoClipData

/** Global composition settings — stays on PenDocument */
interface CompositionSettings {
  duration: number  // ms
  fps: number
}

/** Replaces PenDocument.animation */
// PenDocument.composition?: CompositionSettings
```

### Property System: Descriptor + Canvas Binding

> **Research insight (Clean Code, SOLID, Architecture — consensus):** Split property descriptors into two layers. The engine describes WHAT to animate (pure data). The canvas bridge describes HOW to apply it (Fabric.js-specific). This keeps the engine renderer-agnostic.

```typescript
// src/animation/property-descriptors.ts — pure animation descriptors

interface AnimatablePropertyDescriptor<T = unknown> {
  key: string                                    // 'opacity', 'fill.color', 'cornerRadius'
  type: 'number' | 'color'
  default: T
  interpolate: (from: T, to: T, t: number) => T // lerp or colorLerp
  nodeTypes?: PenNode['type'][]                  // restrict to specific types (optional)
}

const propertyDescriptors = new Map<string, AnimatablePropertyDescriptor>()
```

```typescript
// src/animation/canvas-property-bindings.ts — Fabric.js-specific

interface CanvasPropertyBinding<T = unknown> {
  key: string                                    // matches descriptor key
  apply: (obj: FabricObject, value: T) => void   // mutate Fabric object
  capture: (obj: FabricObject) => T              // read current value
  /** Transform-only props (x, y, scaleX, etc.) don't need obj.dirty = true */
  requiresCacheInvalidation: boolean
}

const canvasPropertyBindings = new Map<string, CanvasPropertyBinding>()
```

### Derived Animation Index (Performance-Critical)

> **Research insight (Clean Code, SOLID — consensus):** Keep the index as pure data. The Fabric object map belongs in canvas-bridge, not in the index.

```typescript
// src/animation/animation-index.ts

/** Built on play-start, rebuilt on document change */
interface AnimationIndex {
  /** nodeId → clips for O(1) lookup during playback */
  clipsByNode: Map<string, AnimationClip[]>
  /** All animated nodeId set (for quick "has animation?" checks) */
  animatedNodes: Set<string>
  /** Invalidation counter — incremented on any document mutation */
  version: number
}
```

**Fabric object map** lives in canvas-bridge:

```typescript
// In canvas-bridge.ts
/** nodeId → FabricObject, rebuilt on play-start and page switch */
const fabricObjectMap = new Map<string, FabricObject>()
```

**Invalidation rules:**
- Rebuild `clipsByNode` when: document changes while NOT playing (subscribe to document-store)
- Rebuild `fabricObjectMap` on: play-start, page switch
- During playback: index is frozen (document mutations force-pause playback first)

## Technical Approach

### Property Registry Pattern

The core extensibility mechanism — adding a new animatable property requires one descriptor + one binding:

Example — adding `cornerRadius` (~20 LOC total):

```typescript
// Descriptor (pure, renderer-agnostic)
registerPropertyDescriptor({
  key: 'cornerRadius',
  type: 'number',
  default: 0,
  interpolate: lerp,
  nodeTypes: ['rectangle'],
})

// Binding (Fabric.js-specific)
registerCanvasBinding({
  key: 'cornerRadius',
  apply: (obj, v) => { (obj as any).rx = v; (obj as any).ry = v },
  capture: (obj) => (obj as any).rx ?? 0,
  requiresCacheInvalidation: true,
})
```

### Cubic Bezier Easing

Replace hardcoded easing functions with a CSS-compatible `cubicBezier(x1, y1, x2, y2)` factory:

```typescript
// src/animation/cubic-bezier.ts
// Algorithm: bezier-easing (used by Chrome/Firefox internally)
// Three-tier solving: sample table → Newton-Raphson → binary subdivision
// Pre-compute 11 sample points at construction, cache per unique [x1,y1,x2,y2]

const EASING_PRESETS: Record<EasingPreset, CubicBezier> = {
  linear:    [0, 0, 1, 1],
  ease:      [0.25, 0.1, 0.25, 1],
  easeIn:    [0.42, 0, 1, 1],
  easeOut:   [0, 0, 0.58, 1],
  easeInOut: [0.42, 0, 0.58, 1],
  snappy:    [0.2, 0, 0, 1],
  bouncy:    [0.34, 1.56, 0.64, 1],
  gentle:    [0.4, 0, 0.2, 1],
  smooth:    [0.25, 0.1, 0.25, 1],
}
```

> **Research insight (Bezier research):** The `bezier-easing` npm package is battle-tested (used by Popmotion, GSAP internals). Consider vendoring its ~60 LOC algorithm rather than adding a dependency. Cache aggressively — most animations reuse the same 5-9 presets.

### Color Interpolation

> **Research insight (Code Simplicity):** Defer OKLab to v3. sRGB linear interpolation is sufficient for v2 and dramatically simpler. The property descriptor's `interpolate` function is the extension point — swapping `srgbLerp` for `oklabLerp` later is a one-line change per color property.

```typescript
// src/animation/color-interpolation.ts
// V2: sRGB linear interpolation (parse hex → lerp RGB channels → format hex)
// V3: Add OKLab pathway (sRGB → linear RGB → OKLab → lerp → back)
```

**Scope limitation:** Only solid colors (`#hex` and `$variable` resolved to hex) are animatable in v2. Gradient-to-gradient interpolation is deferred to v3.

### Effect Registry

```typescript
// src/animation/effect-registry.ts

interface EffectDescriptor {
  id: string
  name: string
  category: 'enter' | 'exit' | 'emphasis' | 'transition' | 'video' | 'custom'
  properties: string[]  // keys from property descriptors
  parameters: EffectParameter[]
  defaultDuration: number  // ms
  generate: (config: EffectGenerateConfig) => Keyframe[]
}

interface EffectParameter {
  key: string
  type: 'number' | 'select' | 'direction'
  default: unknown
  label: string
  options?: Array<{ label: string; value: unknown }>
}

interface EffectGenerateConfig {
  duration: number
  params: Record<string, unknown>
  currentState: Record<string, AnimatableValue>  // captured from canvas
}

const effectRegistry = new Map<string, EffectDescriptor>()
```

Built-in effects (ported from current `presets.ts`):

| Effect | Category | Properties | Parameters |
|--------|----------|------------|------------|
| `fade-in` | enter | opacity | — |
| `fade-out` | exit | opacity | — |
| `slide-in` | enter | x or y | direction: left/right/up/down |
| `slide-out` | exit | x or y | direction: left/right/up/down |
| `scale-in` | enter | scaleX, scaleY | — |
| `scale-out` | exit | scaleX, scaleY | — |
| `bounce-in` | enter | scaleX, scaleY, y | — |
| `blur-in` | enter | blur | — |
| `blur-out` | exit | blur | — |
| `hold` | emphasis | (none) | — |

### Playback Controller

> **Research insight (Performance Oracle — P0):** Move `currentTime` out of Zustand during playback. Use `useSyncExternalStore` with a transient external store for the playhead. Zustand `setState()` at 60fps causes full React tree reconciliation.

```typescript
// src/animation/playback-controller.ts

interface PlaybackController {
  play(): void
  pause(): void
  stop(): void
  seekTo(timeMs: number): void
  setSpeed(rate: number): void
  isPlaying(): boolean
  readonly currentTime: number
  /** Subscribe to time changes (for useSyncExternalStore) */
  subscribe(callback: () => void): () => void
  getSnapshot(): number
}

function createPlaybackController(
  canvas: Canvas,
  getIndex: () => AnimationIndex,
  composition: CompositionSettings,
): PlaybackController {
  // Virtual clock with speed support
  // Tiered update rates: canvas=every rAF, playhead UI=30fps, panels=10fps
  // Direct Fabric mutation, no Zustand writes in hot path
  // currentTime exposed via useSyncExternalStore pattern
  // Restore saved states on stop
}
```

> **Research insight (Performance Oracle — Critical):** Pre-allocate a `TrackBuffer` per track to eliminate per-frame object allocation and GC pressure:

```typescript
// Reusable interpolation buffer — allocated once per track on play-start
interface TrackBuffer {
  values: Record<string, AnimatableValue>  // reused each frame
  prevKeyframeIdx: number                  // cached for binary search optimization
}
```

### Canvas Bridge v2

Registry-driven property application with selective dirty tracking:

```typescript
// src/animation/canvas-bridge.ts (rewrite)

function applyAnimatedFrame(
  obj: FabricObject,
  values: Record<string, AnimatableValue>,
): boolean {
  let needsCacheInvalidation = false
  for (const [key, value] of Object.entries(values)) {
    const binding = canvasPropertyBindings.get(key)
    if (!binding) continue
    binding.apply(obj, value)
    if (binding.requiresCacheInvalidation) needsCacheInvalidation = true
  }
  if (needsCacheInvalidation) obj.dirty = true
  // NOTE: Skip setCoords() during playback — no user interaction possible.
  // Call setCoords() on all animated objects ONCE when playback stops.
  return true
}

// Called once on playback stop to restore hit-testing accuracy
function recalcCoordsForAnimatedObjects(
  fabricObjectMap: Map<string, FabricObject>,
) {
  for (const obj of fabricObjectMap.values()) {
    obj.setCoords()
  }
}

// Use renderAll() (synchronous) in rAF, NOT requestRenderAll()
// requestRenderAll() defers to next frame = 1-frame lag
canvas.renderAll()
```

> **Research insight (Fabric.js Research — comprehensive):**
> - Transform properties (left, top, scaleX, scaleY, angle, opacity) are NOT in Fabric's `cacheProperties` array — changing them does NOT invalidate the cache. These are applied at render time on top of the cached bitmap. **Animating transforms is essentially free.**
> - Visual properties (fill, stroke, strokeWidth, shadow) ARE in `cacheProperties` — changing them triggers expensive cache rebuilds.
> - **Skip `setCoords()` during playback** — it recalculates corner coordinates for hit-testing, which is unnecessary when users can't interact with objects. Call it once when playback stops.
> - Use `renderAll()` (synchronous) inside the rAF callback, NOT `requestRenderAll()` (which defers to the next frame, adding 1-frame lag).
> - For batch property updates, `obj.set({ left: x, top: y, ... })` iterates `cacheProperties` once and marks dirty only once — preferred over individual assignments for cache-invalidating properties.
> - For video textures: `FabricImage` with `HTMLVideoElement` source, mark `obj.dirty = true` each frame to re-read the video texture.

### Force-Pause Middleware

> **Research insight (Architecture Strategist + Frontend Races — CRITICAL):** Force-pause MUST be synchronous Zustand middleware. The `pause()` call must cancel the rAF, clear the flag, and happen BEFORE the store mutation propagates. If async (subscriber-based), there is always one rAF tick between mutation and pause where the `fabricObjectMap` contains stale references to removed objects.

```typescript
// src/stores/animation-pause-middleware.ts

const animationPauseMiddleware = (config) => (set, get, api) =>
  config(
    (...args) => {
      const controller = getPlaybackController()
      if (controller?.isPlaying()) {
        controller.pause()  // SYNCHRONOUS: cancels rAF before mutation
      }
      set(...args)
    },
    get,
    api,
  )
```

> **Race condition mitigation (Frontend Races):** Additionally, queue async asset completions during playback:

```typescript
// Async callbacks (image onload, video loadeddata, video seeked) must check:
if (isPlaybackActive()) {
  pendingSwaps.push({ nodeId, element })  // queue for later
  return
}
// On playback stop, flush all pending swaps
function flushPendingSwaps(canvas: Canvas) { ... }
```

## Resolved Design Questions

### Q1: Property edit on animated node → changes base state
When a user edits a property (e.g., fill color) on a node that has animation clips, it changes the **rest state** at time 0. The animation still tweens relative to that base. This matches the preset-first UX where creators apply effects, not manual keyframes. (Keyframe insertion mode can be added in Jeans UX layer later.)

### Q2: Composition settings → document-level
`PenDocument.composition?: CompositionSettings` (duration, fps) replaces `PenDocument.animation`. Global, not per-page. Multi-page timeline scoping deferred to v3.

### Q3: Document mutations during playback → force-pause
Any document-store mutation while `isPlaying()` is true will trigger automatic pause via Zustand middleware. This eliminates the race condition between sync and playback. The playhead position is preserved for resume.

### Q4: Grouped nodes → clips stay on children
When nodes are grouped, their clips remain on the individual children. The group itself does not inherit clips. Ungrouping preserves clips.

### Q5: Video clips → discriminated union with `kind: 'video'`
Video clips use a discriminated union (`kind: 'video'`) rather than optional `videoData?`. This enables exhaustive type checking and avoids Liskov Substitution violations. Video timing is expressed through `startTime`, `duration`, and `sourceStart/sourceEnd`.

### Q6: Cubic bezier UI → presets only in v2
Named presets backed by bezier values internally. No custom curve editor in v2 (future UI feature). The data model supports custom bezier values for when the editor ships.

### Q7: Canvas sync during playback → `isPlaybackActive()` guard
`use-canvas-sync.ts` checks `isPlaybackActive()` and skips writing to animated objects during playback. This reuses the existing pattern from the Phase 1 engine.

### Q8: Layout children → position animation suppressed
Animating `x`/`y` on nodes inside auto-layout containers is suppressed (layout engine owns their position). Other properties (opacity, fill, scale) are allowed.

### Q9: Text property animation → supported with performance caveat
`fontSize`, `letterSpacing`, `lineHeight` are animatable but trigger Fabric text reflow per-frame. Acceptable for short clips on few text nodes. For v2, we document the perf cost; optimization deferred to v3.

### Q10: Color interpolation → sRGB lerp for v2
OKLab deferred to v3. sRGB linear interpolation is sufficient for launch. The property descriptor `interpolate` function is the extension point for upgrading later.

## System-Wide Impact

### Interaction Graph

```
User picks effect → createClip() → updateNode(nodeId, { clips: [..., newClip] })
  → document-store pushState (history captured) → use-canvas-sync fires
  → timeline adapter rebuilds rows/actions (if timeline UI visible)
```

```
User presses Play → createPlaybackController() → buildAnimationIndex()
  → setPlaybackActive(true) → canvas-sync suppressed
  → tick loop: for each (nodeId, clips) in index:
      → interpolateClip(clip, currentTime) per property (using TrackBuffer)
      → applyAnimatedFrame(fabricObj, values) via canvasPropertyBindings
      → video sync for video clips (kind: 'video')
  → canvas.requestRenderAll() once per frame
  → currentTime exposed via useSyncExternalStore (no Zustand writes)
  → throttled panel updates at 10fps
```

### Error & Failure Propagation

- **Missing effect:** If `effectId` not in registry, clip plays raw keyframes (graceful degradation)
- **Invalid bezier:** Clamp control points to valid range, fall back to linear
- **Color parse failure:** Fall back to hex passthrough (no interpolation)
- **Video decode failure:** Show frozen placeholder frame, log warning

### State Lifecycle Risks

- **Undo during playback:** Force-pause (middleware), then undo restores entire PenDocument (clips on nodes restored automatically). Animation index invalidated on resume.
- **Node deletion during playback:** Force-pause (middleware), node removal cleans up clips naturally (they're ON the node). Index invalidated.
- **Page switch during playback:** Force-pause (middleware), rebuild index for new page's nodes.

### API Surface Parity

The engine exposes the same capabilities through:
1. **Property panel** — effect picker, clip list for selected node
2. **Timeline UI** — visual clip editing (via adapter to `@cyca/react-timeline-editor`)
3. **MCP tools** — `add_animation_clip`, `remove_animation_clip`, `set_effect`, `preview_animation`

### Integration Test Scenarios

1. **Play → Undo → Resume:** Verify undo reverts clips, resume rebuilds index, animation reflects reverted state
2. **Duplicate animated node:** Verify cloned clips have new IDs, both tracks play independently
3. **Delete node during playback:** Verify force-pause, node+clips removed, no stale index references
4. **Save/Load round-trip:** Verify clips on nodes serialize/deserialize with no data loss (including bezier easing values)
5. **Video clip + animation clip on same node:** Verify both play simultaneously, video syncs to composition time

## Implementation Phases

### Phase 1: Core Type System + Property Descriptors

**Files to create/modify:**

- [x] `src/types/animation.ts` — Replace current types with: `CubicBezier`, `Easing`, `HexColor`, `AnimatableValue`, `Keyframe` (offset-based), discriminated `AnimationClip` union (`AnimationClipData | VideoClipData`), `CompositionSettings`
- [x] `src/types/pen.ts` — Add `clips?: AnimationClip[]` to `PenNodeBase`, add `composition?: CompositionSettings` to `PenDocument`, deprecate `animation?: TimelineState`
- [x] `src/animation/property-descriptors.ts` — **NEW**: `AnimatablePropertyDescriptor` interface, `propertyDescriptors` Map, `registerPropertyDescriptor()`, all V1 property descriptor registrations
- [x] `src/animation/canvas-property-bindings.ts` — **NEW**: `CanvasPropertyBinding` interface, `canvasPropertyBindings` Map, `registerCanvasBinding()`, all V1 Fabric.js bindings
- [x] `src/animation/property-descriptors.test.ts` — **NEW**: Test registry CRUD, interpolation for each property type

**V1 registered properties (22 total):**

| Category | Property | Type | Interpolation | `requiresCacheInvalidation` |
|----------|----------|------|---------------|---------------------------|
| Transform | x, y, scaleX, scaleY, rotation | number | lerp | false |
| Transform | opacity | number | lerp | false |
| Visual | fill.color, stroke.color | color | srgbLerp | true |
| Visual | strokeWidth, cornerRadius, blur | number | lerp | true |
| Visual | shadow.offsetX, shadow.offsetY, shadow.blur | number | lerp | true |
| Visual | shadow.color | color | srgbLerp | true |
| Typography | fontSize, letterSpacing, lineHeight | number | lerp | true |
| Typography | text.fill.color | color | srgbLerp | true |

**Success criteria:** `registerPropertyDescriptor()` works, all 22 properties registered with matching canvas bindings, type-checks pass, discriminated union exhaustive matching compiles.

### Phase 2: Cubic Bezier + Interpolation Engine

- [x] `src/animation/cubic-bezier.ts` — **NEW**: `createBezierEasing(x1,y1,x2,y2)` factory, sample table + Newton-Raphson + binary subdivision, `EASING_PRESETS` map, easing function cache (WeakMap or `Map<string, fn>` keyed by stringified tuple)
- [x] `src/animation/cubic-bezier.test.ts` — **NEW**: Test against CSS reference values, edge cases (linear, overshoot), preset aliases
- [x] `src/animation/color-interpolation.ts` — **NEW**: `srgbLerp()` (hex parse → channel lerp → hex format), hex parsing/formatting utilities
- [x] `src/animation/color-interpolation.test.ts` — **NEW**: Test boundary colors (black, white, transparent), midpoint values
- [x] `src/animation/interpolation.ts` — **REWRITE**: Replace hardcoded easing with bezier factory, add `interpolateClip(clip, timeMs, buffer?: TrackBuffer)` using offset-based keyframes + property descriptor interpolation methods

> **Research insight (Bezier research):** Vendor the `bezier-easing` algorithm (~60 LOC) rather than adding a dependency. Cache the easing function per unique `[x1,y1,x2,y2]` — use a string key like `"0.42,0,0.58,1"`. Most documents will use <10 unique curves.

**Success criteria:** `cubicBezier(0.42, 0, 0.58, 1)` matches CSS `cubic-bezier()` output within 0.001. sRGB interpolation produces correct midpoints.

### Phase 3: Effect Registry + Preset Migration

- [x] `src/animation/effect-registry.ts` — **NEW**: `EffectDescriptor` interface, `effectRegistry` Map, `registerEffect()`, `generateClipFromEffect()`
- [x] `src/animation/effects/enter.ts` — **NEW**: `fade-in`, `slide-in`, `scale-in`, `bounce-in`, `blur-in`
- [x] `src/animation/effects/exit.ts` — **NEW**: `fade-out`, `slide-out`, `scale-out`, `blur-out`
- [x] `src/animation/effects/emphasis.ts` — **NEW**: `hold`, `pulse` (optional)
- [x] `src/animation/effect-registry.test.ts` — **NEW**: Test each effect generates valid keyframes, parameterization works (slide direction)
- [x] `src/animation/presets.ts` — **DEPRECATE**: Keep for timeline-editor integration backward compat, mark as legacy

**Success criteria:** All 10 built-in effects registered, `generateClipFromEffect('slide-in', { direction: 'left' }, 500)` returns valid clip with correct keyframes.

### Phase 4: Animation Index + Playback Controller

- [x] `src/animation/animation-index.ts` — **NEW**: `AnimationIndex` interface (pure data, no Fabric refs), `buildAnimationIndex()` from PenDocument tree, version counter
- [x] `src/animation/track-buffer.ts` — **NEW**: `TrackBuffer` interface, `createTrackBuffers()` pre-allocator
- [x] `src/animation/playback-controller.ts` — **NEW**: `createPlaybackController()` closure, virtual clock with speed support, tiered update rates, `subscribe`/`getSnapshot` for `useSyncExternalStore`, auto-pause via middleware callback
- [x] `src/animation/playback-controller.test.ts` — **NEW**: Test play/pause/stop/seek/speed, verify timing accuracy
- [x] `src/animation/playback-loop.ts` — **REWRITE**: Thin wrapper around `PlaybackController`, remove module-level state
- [x] `src/stores/animation-pause-middleware.ts` — **NEW**: Zustand middleware that auto-pauses playback on document mutations

> **Research insight (Performance Oracle):** The `useSyncExternalStore` pattern for `currentTime`:
> ```typescript
> // In a React component:
> const currentTime = useSyncExternalStore(
>   controller.subscribe,
>   controller.getSnapshot,
> )
> ```
> This gives React components reactive time updates WITHOUT Zustand store writes during playback. Only components that read `currentTime` re-render — the canvas and property panels are unaffected.

**Success criteria:** 60fps playback with 20 tracks @ 10 keyframes each. Seeking produces correct interpolated frame. Speed control works (0.25x–4x). No Zustand writes during playback hot path.

### Phase 5: Canvas Bridge v2

- [x] `src/animation/canvas-bridge.ts` — **REWRITE**: Registry-driven `applyAnimatedFrame()` using `canvasPropertyBindings`, selective `obj.dirty` based on `requiresCacheInvalidation`, `fabricObjectMap` management, `captureNodeState()` (from document, not Fabric), `setPlaybackActive()`/`isPlaybackActive()` guards, `recalcCoordsForAnimatedObjects()` called on stop, pending asset swap queue
- [x] `src/animation/canvas-bridge.test.ts` — **NEW**: Test property application for each registered property type, verify no `setCoords()` during playback, verify `setCoords()` called on stop

> **Research insight (Fabric.js Research):** Critical Fabric.js v7 animation performance:
> - Transform props (left/top/scaleX/scaleY/angle/opacity) are NOT in `cacheProperties` — animating them is free (no cache rebuild)
> - Visual props (fill/stroke/strokeWidth/shadow) ARE in `cacheProperties` — require `obj.dirty = true` and cache rebuild
> - **Skip `setCoords()` during playback** — call once on stop to restore hit-testing
> - Use `renderAll()` (synchronous) inside rAF, not `requestRenderAll()` (avoids 1-frame lag)
> - Use `obj.set({ ... })` for batch cache-invalidating props (one dirty check), direct assignment for transform props

> **Race condition mitigation (Frontend Races):**
> - Capture `savedStates` (rest state) from **document store**, not from Fabric objects. On undo-triggered pause, discard and re-capture from post-undo document.
> - Queue async asset swaps (image onload, video loadeddata) during playback; flush on stop.
> - Wrap `setPlaybackActive()` in scoped try/finally to prevent permanent flag corruption on error.

**Success criteria:** All 22 animatable properties applied correctly. `obj.dirty` set only for cache-invalidating properties. `setCoords()` skipped during playback, called once on stop. Async asset swaps queued during playback.

### Phase 6: Persistence + Document Store Integration

- [x] `src/animation/animation-persistence.ts` — **SIMPLIFY**: Remove inject/extract pattern (clips are ON nodes now). Keep `reconcileAnimationWithDocument()` for cleaning stale effectIds.
- [x] `src/stores/document-store.ts` — **MODIFY**: Add animation-pause middleware, ensure `removeNode()` cleans up any index references, `duplicateNode()` deep-clones clips with new IDs, `updateNode()` invalidates animation index
- [x] `src/canvas/use-canvas-sync.ts` — **MODIFY**: Add `isPlaybackActive()` check in sync loop (if not already present), skip writing to Fabric objects that are being animated

**Success criteria:** Save/load round-trip preserves all clips, keyframes, and bezier easing values. Undo/redo restores clips automatically. Force-pause middleware triggers on any mutation during playback.

### Phase 7: Video Clip Integration

- [x] `src/animation/video-sync.ts` — **MODIFY**: Read video clip data from discriminated `VideoClipData` (kind: 'video') instead of separate store. Implement dual-mode sync: native `video.play()` with drift correction during playback, `video.currentTime` seeking during scrub. Read all video timing from cached AnimationIndex, not live document store.
- [x] `src/animation/video-registry.ts` — **KEEP**: Video element lifecycle management unchanged
- [x] `src/types/pen.ts` — **MODIFY**: `VideoNode` keeps `src`, `mimeType`, `videoDuration` for non-animated rendering. Timeline timing moves to clip model.

> **Race condition mitigation (Frontend Races):** Current implementation seeks `video.currentTime` every frame — the worst approach (high CPU, accumulating drift). Fix:
> ```typescript
> // During playback: let video play natively, only correct on drift
> const drift = Math.abs(video.currentTime * 1000 - expectedVideoTimeMs)
> if (drift > 50) { // 50ms threshold
>   video.currentTime = expectedVideoTimeMs / 1000
> }
> fabricObj.dirty = true  // always re-read video texture
>
> // During scrub: seek directly (existing pattern)
> video.currentTime = targetTimeMs / 1000
> ```

**Success criteria:** Video clips play synchronized to composition timeline. Drift stays <50ms during native playback. In/out point trimming works through clip `startTime`/`duration` + `sourceStart/sourceEnd`.

### Phase 8: Timeline Adapter + Store Simplification + UI Integration

> **Research insight (Code Simplicity):** Merge the timeline adapter, store simplification, and UI integration phases. They're tightly coupled — changing the adapter without updating the UI creates a broken intermediate state.

- [x] `src/animation/timeline-adapter.ts` — **MODIFY**: Adapt to read clips from nodes instead of `timeline-store.tracks`. Map `AnimationClipData` → `TimelineAction`, `node with clips` → `TimelineRow`. Handle `VideoClipData` separately.
- [x] `src/animation/timeline-adapter-types.ts` — **MODIFY**: Update types to reflect clip-based model with discriminated union
- [x] `src/stores/timeline-store.ts` — **SIMPLIFY**: Remove `tracks` (moved to nodes). Keep ephemeral UI state: `currentTime` (derived from playback controller), `playbackMode`, `loopEnabled`, `editorMode`. Derive `duration`/`fps` from `PenDocument.composition` — do not duplicate.
- [x] `src/components/animation/preset-panel.tsx` — **MODIFY**: Use effect registry instead of hardcoded presets. Show effect parameters when applicable.
- [x] `src/components/animation/timeline-panel.tsx` — **MODIFY**: Read from node clips instead of timeline-store tracks. Use `useSyncExternalStore` for playhead position.
- [x] `src/components/panels/property-panel.tsx` — **MODIFY**: Add animation clips section for selected animated node.
- [x] `src/components/panels/layer-item.tsx` — **MODIFY**: Show animation indicator badge on animated nodes.

**Success criteria:** Timeline UI renders clips from nodes correctly. Drag/resize on timeline writes back to node clips. Effect picker shows registry effects. Property panel shows clips on animated nodes. Playhead updates at 30fps without Zustand writes.

### Phase 9: MCP Animation Tools (Agent Parity)

> **Research insight (Agent-Native Reviewer — Score: NEEDS WORK):** The plan mentioned 4 MCP tools in one sentence without specification. Agents cannot discover available effects, animatable properties, or manage compositions. The "clips on nodes" architecture is excellent for parity (insert_node/copy_node/delete_node automatically handle clips), but dedicated tools are needed for animation-specific workflows.

- [x] `src/mcp/tools/animation.ts` — **NEW**: Animation-specific MCP tools:

**Tools to implement:**

| Tool | Input | Output | Notes |
|------|-------|--------|-------|
| `list_effects` | `{ category?: string }` | Full effect registry: IDs, names, categories, parameters, properties, defaultDuration | Introspection — agents discover what's available |
| `list_animatable_properties` | `{ nodeType?: string }` | Property registry metadata: keys, types, defaults, applicable node types | Introspection — agents know what can be animated |
| `add_clip` | `{ nodeId, effectId?, startTime, duration, keyframes?, params? }` | Created clip with generated ID | If `effectId` provided, generates keyframes from registry |
| `update_clip` | `{ nodeId, clipId, startTime?, duration?, params? }` | Updated clip | Regenerates keyframes if effect params change |
| `remove_clip` | `{ nodeId, clipId }` | Confirmation | Array manipulation handled internally |
| `set_composition` | `{ duration?, fps? }` | Updated composition settings | Sets `PenDocument.composition` |

- [x] `src/mcp/tools/open-document.ts` — **MODIFY**: Add animation context to `buildDocumentContext()`: composition settings, animated node count, effect usage summary, total clip count
- [x] `src/mcp/tools/design-prompt.ts` — **MODIFY**: Add `animation` section covering clip structure, effects, easing presets, animatable properties, and common animation patterns

**Success criteria:** External LLM agents can discover effects/properties, add/remove clips, and manage composition settings via MCP — full parity with UI capabilities.

### Phase 10: Testing + Performance Validation

- [x] Run full test suite: `bun --bun run test`
- [x] Type check: `npx tsc --noEmit`
- [ ] Performance benchmark: 20 tracks × 10 keyframes, verify 60fps
- [ ] Verify no Zustand `setState()` calls during playback hot path (performance profiler)
- [ ] Verify `renderAll()` (not `requestRenderAll()`) used in rAF loop
- [ ] Verify `setCoords()` NOT called during playback, called once on stop
- [ ] Verify async asset swaps queued during playback, flushed on stop
- [ ] Manual test matrix from `docs/plans/2026-03-11-manual-testing-plan-animation-video.md`
- [ ] Save/load round-trip with animated document
- [ ] Undo/redo with animation changes (verify force-pause middleware fires synchronously)
- [ ] Video clip playback synchronized to composition (drift <50ms)
- [ ] MCP tools: `list_effects`, `add_clip`, `remove_clip` work end-to-end

## Sequencing with Timeline Editor Integration

The timeline editor integration (existing plan) ships first on the Phase 1 engine:

```
WEEK 1-2: Ship timeline editor integration (adapter on Phase 1 engine)
  ↓ learn from library integration experience
WEEK 3: Phase 1-2 of engine v2 (types, property descriptors, bezier, interpolation)
WEEK 4: Phase 3-5 (effects, playback controller, canvas bridge)
WEEK 5: Phase 6-7 (persistence, video)
WEEK 6: Phase 8-9 (adapter+UI integration, MCP tools)
WEEK 7: Phase 10 (testing + performance validation)
```

The adapter layer in Phase 8 is the bridge — it starts by reading from `timeline-store.tracks` (Phase 1 engine) and evolves to reading from node `clips[]` (v2 engine). This is the only point where both engines overlap.

## Alternative Approaches Considered

### Keep animation in separate store, just add properties
**Rejected.** The dual-store persistence pattern is fragile (inject/extract before save/load, careful ordering with undo). Moving animation to nodes gives us history support for free and eliminates an entire class of sync bugs. (see brainstorm: rejected alternatives)

### Use the timeline library's engine
**Rejected.** The library's effect lifecycle (enter/update/leave) is too simple for professional animation. No keyframe interpolation, no bezier easing, no property-level animation. (see brainstorm: rejected alternatives)

### Build engine and library integration simultaneously
**Rejected.** Two big architectural changes at once means debugging becomes nightmare-level. Ship the library integration first, evolve the engine second. (see brainstorm: decision #6)

### Optional `videoData?` instead of discriminated union
**Rejected.** Optional fields create Liskov Substitution violations — code must null-check `videoData` everywhere. Discriminated union with `kind` enables exhaustive switch statements and TypeScript narrowing. (SOLID, Architecture, TypeScript reviewers — consensus)

### OKLab color interpolation in v2
**Deferred to v3.** sRGB lerp is sufficient for launch. The property descriptor `interpolate` function is the swap point — upgrading to OKLab is a one-line change per color property. Reduces v2 scope and complexity. (Code Simplicity reviewer)

## Dependencies & Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Text reflow performance at 60fps | Medium | Document perf cost; defer optimization to v3 |
| sRGB color lerp quality | Low | Acceptable for most animations; OKLab upgrade path is clean |
| Timeline adapter dual-model transition | Medium | Phase 8 is explicit — adapter reads from whichever model is active |
| Fabric.js v7 `setCoords()` overhead | Low | Only called for objects with transform animations; batch with `requestRenderAll()` |
| Upstream OpenPencil changes | Low | No animation/video work visible upstream (investigated 2026-03-11). Engine is additive. |
| History store snapshot size with clips on nodes | Low | Clips are small objects; 20 tracks × 10 keyframes = ~200 objects per snapshot, acceptable |
| GC pressure from per-frame allocations | Medium | Pre-allocated TrackBuffers eliminate this. Profile with Chrome DevTools Memory panel. |

## Success Metrics

- **Property extensibility:** Adding a new animatable property takes <30 LOC (descriptor + canvas binding)
- **Interpolation quality:** Cubic bezier curves match CSS `cubic-bezier()` output within 0.001
- **Color quality:** sRGB interpolation produces correct channel-lerp midpoints (OKLab deferred)
- **Video parity:** Video clips are full citizens — discriminated union, same timeline representation
- **Performance:** 60fps playback with 20 tracks, each with 10+ keyframes, zero Zustand writes in hot path
- **Architecture:** Engine module (`src/animation/`) has zero imports from timeline library or UI components
- **Renderer agnosticism:** `AnimatablePropertyDescriptor` has zero Fabric.js imports (canvas bindings are separate)
- **OpenPencil compatibility:** Upstream updates require zero engine changes (best case) or <50 LOC bridge updates (worst case)
- **Integration surface:** Engine touches exactly 3 OpenPencil core files for new node-type support (factory, sync, property panel) — plus the engine's own files in `src/animation/`

## Sources & References

### Origin

- **Brainstorm document:** [docs/brainstorms/2026-03-11-animation-engine-v2-brainstorm.md](docs/brainstorms/2026-03-11-animation-engine-v2-brainstorm.md) — Key decisions carried forward: unified clip model, effect registry, animation as node properties, cubic bezier easing, minimal integration surface, sequencing (library first → engine → Jeans UX)

### Internal References

- Timeline editor integration: `docs/plans/2026-03-11-feat-timeline-editor-integration-plan.md`
- Video/animation extension: `docs/plans/2026-03-10-openpencil-video-animation-extension-plan.md`
- Jeans content workflow: `docs/plans/2026-03-10-feat-jeans-content-workflow-plan.md`
- Manual testing plan: `docs/plans/2026-03-11-manual-testing-plan-animation-video.md`
- Current animation types: `src/types/animation.ts`
- Current playback loop: `src/animation/playback-loop.ts`
- Current interpolation: `src/animation/interpolation.ts`
- Current canvas bridge: `src/animation/canvas-bridge.ts`
- Current presets: `src/animation/presets.ts`
- Current timeline store: `src/stores/timeline-store.ts`

### External References

- [bezier-easing algorithm](https://github.com/gre/bezier-easing) — CSS-compatible cubic bezier solving (~60 LOC, vendor rather than depend)
- [OKLab color space](https://bottosson.github.io/posts/oklab/) — perceptually uniform color interpolation (deferred to v3)
- [Fabric.js v7 performance optimization](https://github.com/fabricjs/fabric.js/wiki/Optimizing-performance)
- [Motion One architecture](https://deepwiki.com/motiondivision/motionone) — modern animation engine patterns
- [Interpol animation library](https://tympanus.net/codrops/2025/10/27/interpol-a-low-level-take-on-tweening-and-motion/) — timeline sequencer pattern
- [Remotion interpolation patterns](https://www.remotion.dev/docs/interpolate) — `extrapolate` behavior, spring physics

### Deepening Research (2026-03-11)

All 13 parallel review/research agents completed:

- **TypeScript Reviewer:** Discriminated unions, branded types for `HexColor`, rename `PropertyDescriptor`
- **Pattern Recognition:** camelCase registries, match existing `roleRegistry` naming patterns
- **Performance Oracle:** `useSyncExternalStore` for playback time, `TrackBuffer` pre-allocation, `requiresCacheInvalidation` flag
- **Code Simplicity:** Defer OKLab, collapse phases, simplify scope
- **Architecture Strategist:** Force-pause middleware, derive timeline-store from document-store
- **SOLID Reviewer:** Discriminated union for clips, split descriptor from binding
- **Clean Code Reviewer:** Move fabricObjectMap out of index, pure data structures
- **Frontend Races Reviewer:** 10 race conditions identified — synchronous force-pause, async swap queuing, cancellation tokens, state-machine cursor guard, video drift correction, savedStates on undo, scoped lock patterns
- **Agent-Native Reviewer:** MCP tool surface gaps — need `list_effects`, `list_animatable_properties`, `add_clip`, `update_clip`, `remove_clip`, `set_composition` tools; add animation context to `open_document`; add `animation` section to `get_design_prompt`
- **Fabric.js Research:** Transform props don't invalidate cache (free to animate), skip `setCoords()` during playback, use `renderAll()` not `requestRenderAll()` in rAF, `FabricImage` with `HTMLVideoElement` for video textures
- **Bezier Research:** Vendor bezier-easing algorithm (~120 LOC, zero deps), cache per unique curve, OKLab is ~50 LOC from scratch
- **OKLab Research:** sRGB→linear→LMS→OKLab pipeline confirmed ~50 LOC, deferred to v3 per Code Simplicity
- **Security Sentinel:** No significant findings (animation engine is local-only, no user input validation concerns beyond existing patterns)
