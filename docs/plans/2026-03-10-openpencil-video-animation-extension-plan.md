# OpenPencil Video & Animation Extension Plan

## Enhancement Summary

**Deepened on:** 2026-03-10
**Research agents used:** 12 (architecture strategist, performance oracle, TypeScript reviewer, agent-native reviewer, code simplicity reviewer, frontend races reviewer, security sentinel, UI/UX designer, best practices researcher, framework docs researcher, video-in-electron researcher, Remotion researcher)

### Key Improvements from Research
1. **Scope dramatically reduced** — plan reframed from "general-purpose motion editor" to "social media animation tool" with preset-first UX
2. **Store architecture simplified** — 3 proposed stores → 2 stores (timeline-store + media-store) based on architecture review
3. **AI integration moved to Phase 1** — inverted the phase ordering: AI generates animations first, manual UI is secondary
4. **Critical race conditions identified** — 8 specific race conditions with solutions (playback vs React render, video seeking, undo during playback)
5. **Performance budgets established** — 33ms/frame for 30fps, raw RGBA piping to FFmpeg (50-70% faster than PNG), GIF capped at 720p
6. **Type system redesigned** — Map→Record for serialization, EasingFunction as discriminated union, separated persisted vs ephemeral state
7. **MCP tools expanded** — from 9 proposed tools to 16 required (added context, feedback, and batch tools)

### New Considerations Discovered
- **Fabric.js `renderAll()` costs 8-15ms** for complex scenes — leaves only 15-25ms headroom at 30fps
- **Animation engine must bypass Zustand entirely during playback** — direct Fabric.js object mutation, zero React re-renders
- **Video clip support should be deferred to Phase 3** — v1 is animated graphics, not video-in-video
- **`design_animation` as plan-then-apply pattern** — matches existing MCP layered workflow (skeleton → content → refine)
- **Formal CanvasBridge interface needed** — highest-risk integration point between spatial and temporal engines

---

## Overview

Extend the open-source OpenPencil design tool (https://github.com/ZSeven-W/openpencil) with video editing and keyframe animation capabilities to create a unified social media content creation tool that handles both static graphics and motion content.

### Problem Statement

OpenPencil provides strong static vector design capabilities (Fabric.js v7, React 19, Zustand stores, AI-native design generation) but has zero temporal/motion features. Social media content increasingly requires video clips, animated graphics, and motion sequences. Currently, creators must switch between separate tools (design tool → animation tool → video editor) to produce social media content.

### Goals

1. Add a simple video editor that allows clips to be added and trimmed
2. Introduce a keyframe-based animation engine for canvas objects
3. Support animated sequences and transitions
4. Enable export of motion content as video (MP4, WebM) and GIF
5. Maintain compatibility with OpenPencil's existing static design workflow

### Non-Goals (Initial Version)

- Full NLE (non-linear editor) with multi-track audio mixing
- 3D animation or particle systems
- Real-time collaboration on animation timelines
- Mobile/tablet editing

---

## Current Architecture Analysis

### OpenPencil's Existing Stack

| Layer | Technology | Role |
|-------|-----------|------|
| UI Framework | React 19 + TanStack Start | Component rendering, routing |
| Canvas Engine | Fabric.js v7 | 2D vector/raster rendering |
| State Management | Zustand v5 (3 stores) | Document store, canvas store, history store |
| Desktop Shell | Electron 35 | Native app, file system access, Node.js |
| Styling | Tailwind CSS v4 + shadcn/ui | UI components |
| AI | Anthropic SDK, MCP server | Design generation, agent tools |
| Build | Bun + Vite 7 | Fast builds, HMR |

### Key Architectural Constraints

1. **Fabric.js is frame-based, not time-based** — renders a single static state per draw call
2. **Zustand document store is spatial** — tree of nodes with position/style properties, no temporal dimension
3. **MCP server exposes 30+ tools** — all static operations, no animation/video tools
4. **Export pipeline outputs static formats** — PNG, JPG, WebP, SVG, code
5. **Electron provides Node.js access** — enables native FFmpeg, file I/O, hardware acceleration

### Extension Points

- Zustand store system is modular — new stores can be added alongside existing ones
- React component tree is composable — timeline UI can be added as a new panel
- Fabric.js supports programmatic property updates — animation loop can drive object properties
- Electron's Node.js layer enables native binary execution (FFmpeg)
- MCP server is extensible — new tools can be registered for animation/video operations

### Research Insights: Fabric.js v7 Render Performance

**Benchmarks (from Performance Oracle review):**
- Simple scene (10 rects, no filters): ~2-3ms per `renderAll()`
- Complex scene (20+ objects, text, images, gradients): ~8-15ms per `renderAll()`
- With filters (blur, shadow): ~15-30ms per `renderAll()`

**At 30fps the budget is 33ms per frame.** After interpolation (~0.5ms) and property writes (~0.05ms), the remaining headroom is 15-25ms — sufficient for preview, but video compositing eats into this margin.

**Critical constraint:** `toDatalessJSON()` must NEVER be called in the animation loop (5-50ms per call). Disable all Fabric.js serialization during playback.

---

## Technical Approach

### Architecture: Dual-Engine with CanvasBridge (Revised)

```
┌─────────────────────────────────────────────────┐
│                OpenPencil Editor                 │
│                                                  │
│  ┌──────────────┐    ┌────────────────────────┐ │
│  │  Fabric.js   │    │  Animation Engine       │ │
│  │  Canvas      │◄──▶│  (Keyframe Interpolator │ │
│  │  (spatial)   │    │   + Playback Loop)      │ │
│  └──────┬───────┘    └──────────┬─────────────┘ │
│         │                       │                │
│         └───────┬───────────────┘                │
│                 │                                │
│         ┌───────▼───────┐                        │
│         │  CanvasBridge  │  ← Critical interface │
│         │  (sync layer)  │                       │
│         └───────┬───────┘                        │
│                 │                                │
│         ┌───────▼──────────────────────────┐    │
│         │     Timeline Panel (React)        │    │
│         │  [▶] 0:00 ━━━━●━━━━━━━━━ 0:30    │    │
│         │  Elements: ◆──◆──◆  ◆────◆       │    │
│         └──────────────────────────────────┘    │
│                      │                           │
│         ┌────────────▼─────────────────────┐    │
│         │    Export: FFmpeg (raw RGBA pipe)  │    │
│         │    → MP4, GIF, WebM               │    │
│         └──────────────────────────────────┘    │
└─────────────────────────────────────────────────┘
```

### Research Insights: CanvasBridge Interface

> **Architecture Strategist:** "The plan underspecifies the critical boundary between the two engines. You need a formal synchronization contract. During playback: one-directional data flow (animation → Fabric.js). During editing: bidirectional (canvas drag → keyframe insert at playhead)."

```typescript
interface CanvasBridge {
  applyAnimatedProperties(nodeId: string, properties: Record<string, number | string>): void;
  captureCurrentState(nodeId: string): AnimatableProperties;
  lockObjectInteraction(nodeId: string): void;
  unlockObjectInteraction(nodeId: string): void;
  isPlaybackActive(): boolean;
}
```

**Playback isolation (mandatory):** All Fabric.js event handlers must check `isPlaybackActive()` and suppress during playback:
```typescript
canvas.on('object:modified', (e) => {
  if (canvasBridge.isPlaybackActive()) return; // suppress during playback
  // normal handling
});
```

### Component 1: Animation Engine

**Purpose:** Drive Fabric.js object properties over time using keyframes and interpolation.

**Keyframe Store (new Zustand store: `timeline-store`):**

> **TypeScript Reviewer:** "Map<string, AnimationTrack> will silently produce `{}` in JSON.stringify. Use Record. Also, EasingFunction must be a serializable discriminated union, not a function type."

> **Architecture Strategist:** "Merge to two stores (timeline + media), not three. Playback state has no independent identity."

```typescript
// === PERSISTED STATE (saved to .op file) ===

interface TimelineState {
  tracks: Record<string, AnimationTrack>; // nodeId → track (NOT Map — serialization)
  duration: number;                        // total composition duration in ms
  fps: 24 | 30 | 60;
}

interface AnimationTrack {
  nodeId: string;
  keyframes: Keyframe[];  // MUST be sorted by time — enforce on insert
  visibility: { inPoint: number; outPoint: number };
}

interface Keyframe {
  id: string;             // nanoid for stable references
  time: number;           // ms from start
  properties: Partial<AnimatableProperties>;
  easing: EasingFunction;
}

// Easing as serializable discriminated union (NOT a function)
type EasingPreset = "linear" | "ease-in" | "ease-out" | "ease-in-out" | "bounce";

interface CubicBezierEasing {
  type: "cubic-bezier";
  controlPoints: readonly [number, number, number, number];
}

type EasingFunction = EasingPreset | CubicBezierEasing;

interface AnimatableProperties {
  x: number;           // maps to fabric.Object.left
  y: number;           // maps to fabric.Object.top
  scaleX: number;
  scaleY: number;
  rotation: number;    // maps to fabric.Object.angle
  opacity: number;
  fill: string;
  strokeColor: string; // maps to fabric.Object.stroke
  cornerRadius: number; // maps to fabric.Rect.rx / ry
}

// === EPHEMERAL STATE (runtime only, not serialized) ===

interface PlaybackState {
  currentTime: number;
  isPlaying: boolean;
  selectedKeyframeIds: Set<string>;
  dragState: TimelineDragState | null;
}

type TimelineDragState =
  | { kind: "scrubbing"; startTime: number }
  | { kind: "moving-keyframe"; keyframeId: string; originalTime: number }
  | { kind: "selecting-range"; startTime: number; endTime: number };

// === EXPORT CONFIG ===

interface AnimationExportConfig {
  format: "mp4" | "webm" | "gif";
  width: number;
  height: number;
  fps: number;
  quality: number; // 0-1
  timeRange: { start: number; end: number } | "full";
}
```

### Research Insights: Fabric.js Integration Layer

> **TypeScript Reviewer:** "AnimatableProperties should NOT import Fabric.js types. Your animation system must be decoupled from the rendering layer. Create a bridge function."

```typescript
// This lives in the rendering/bridge layer — the ONLY place Fabric.js is imported
function applyFrameToFabricObject(
  obj: fabric.Object,
  properties: Partial<AnimatableProperties>
): void {
  if (properties.x !== undefined) obj.left = properties.x;     // direct assignment, no set()
  if (properties.y !== undefined) obj.top = properties.y;
  if (properties.rotation !== undefined) obj.angle = properties.rotation;
  if (properties.strokeColor !== undefined) obj.stroke = properties.strokeColor;
  // Direct property assignment avoids Fabric event emission
}
```

**Interpolation Engine:**

> **Code Simplicity Reviewer:** "Custom interpolation IS over-engineering. You need 5 easing functions for social media. That's ~20 lines of math, not an 'engine'."

- 5 easing functions for v1: linear, ease-in, ease-out, ease-in-out, bounce
- Numeric lerp for position/scale/opacity/rotation
- Color interpolation in OKLCH for fill/stroke (defer to v2 if needed)
- Sort keyframes on insert (store invariant), binary search for current frame pair
- NO spring physics in v1 — add only if user demand emerges

**Playback Loop:**

> **Performance Oracle:** "The animation playback engine MUST be completely decoupled from React's render cycle. Zero React re-renders during playback. The engine writes directly to Fabric.js object properties."

```
Animation Frame Loop (requestAnimationFrame)
  └── Keyframe Interpolation (pure math, reads from plain data)
       └── Direct Fabric.js Object Mutation (obj.left = x)
            └── canvas.requestRenderAll() (once per frame, debounced)

React UI updates (throttled to ~10fps via ref subscription):
  - Playhead position display
  - Current time indicator
```

**Performance Targets:**
- `renderAll()` under 12ms for 30fps, under 8ms for 60fps
- Zero Zustand store updates during playback — use refs for timeline position
- `objectCaching: true` on static objects (render from cached bitmaps)
- Skip `setCoords()` during playback (saves transform matrix recalculation)

### Component 2: Video Clip Manager (Phase 3 — Deferred)

> **Code Simplicity Reviewer:** "Video-in-video is a complex feature that is not required for animated graphics. Defer entirely; v1 is animated graphics."

**Deferred to Phase 3.** MVP focuses on animating existing canvas objects. Video clip support is a separate feature that adds significant complexity (codec handling, memory management, frame sync).

When implemented:

```typescript
interface VideoClip {
  id: string;
  nodeId: string;           // connects to AnimationTrack and Fabric canvas object
  source: string;           // file path (filesystem-backed, NOT blob URL)
  inPoint: number;
  outPoint: number;
  timelineStart: number;
  dimensions: Readonly<{ width: number; height: number }>;
  volume: number;           // 0-1
}
```

### Research Insights: Video Memory Management

> **Architecture Strategist:** "Since this runs in Electron, store video files on disk. Use file:// URLs. Implement a video pool that loads/unloads based on playhead proximity."

> **Performance Oracle:** "Each decoded 1080p RGBA frame: ~8MB. With 3 simultaneous clips: ~72-192MB for decoded buffers. Limit to 2-3 simultaneous 1080p videos."

- Use a custom Fabric.js object that overrides `_render()` with `ctx.drawImage(videoElement)` — NO new `fabric.Image` per frame
- Use `requestVideoFrameCallback()` (Chromium 83+, Electron 35) for frame-accurate sync
- Preload only clips within 10-second window of playhead
- Clean up Object URLs on clip removal: `URL.revokeObjectURL()`

### Component 3: Timeline UI (Simplified)

> **Code Simplicity Reviewer:** "Multi-track timelines are for complex compositions. Social media creators need a single scrubber + element list, not After Effects."

> **UI/UX Researcher (Rive patterns):** "Rive uses Design mode and Animate mode as distinct states toggled via `tab`. Objects appear on timeline only after they've been keyed. Color-coded key indicators: grey (collapsed/multiple), blue (individual), blue-filled (at playhead)."

**Simplified React Component Structure:**
```
TimelinePanel/
├── PlaybackControls    (play, pause, rewind, loop)
├── TimeRuler           (time markers, zoom)
├── ElementList         (animated elements with keyframe bars — NOT full tracks)
│   └── KeyframeBar     (diamond indicators, visibility in/out range)
├── AnimationPresets     ("Fade In", "Slide Left", "Scale Up", "Bounce In")
└── EasingPresetPicker  (5 named presets in a dropdown — NO bezier editor)
```

**Cut from v1 (per simplicity review):**
- ~~Property curve graph view~~ — power-user tool, 0% of target audience
- ~~Easing curve bezier editor~~ — named presets only
- ~~AudioTrack waveform~~ — defer with video clips
- ~~Multi-track timeline~~ — simple element list instead

**Timeline UI Performance (per Performance Oracle):**
- Use CSS `transform: translateX()` for keyframe positioning during drag
- Virtualize element rows if count exceeds 50
- Canvas-based rendering for timeline ruler (not DOM)
- Throttle canvas preview during fast scrubbing to every 2nd frame

### Component 4: Export Pipeline

**Rendering Process (revised):**

> **Performance Oracle:** "Use raw RGBA frame piping to FFmpeg via stdin, NOT per-frame PNG encoding. This eliminates 20-80ms PNG encoding per frame — 50-70% total speedup."

```typescript
async function exportFrames(duration: number, fps: number) {
  const frameCount = Math.ceil(duration * fps / 1000);
  for (let i = 0; i < frameCount; i++) {
    const time = (i / fps) * 1000;
    interpolateAndApply(time);       // pure math → direct Fabric mutation
    canvas.renderAll();              // synchronous render
    const imageData = ctx.getImageData(0, 0, width, height);
    await pipeToFFmpeg(imageData.data); // raw RGBA buffer
  }
}
```

**Performance Estimates (30-second video @ 30fps = 900 frames):**
| Step | Per Frame | Total (900 frames) |
|------|-----------|-------------------|
| Interpolation + apply | ~0.6ms | ~0.5s |
| `renderAll()` | ~8-15ms | ~7-14s |
| `getImageData()` | ~2-5ms | ~2-5s |
| FFmpeg pipe | ~1-3ms | ~1-3s |
| **Total render** | ~12-24ms | **~11-22s** |
| FFmpeg H.264 encoding | — | ~5-15s |
| **Grand total** | — | **~20-40s** |

**Target: Export at 2x realtime or better** (30-second video in under 60 seconds).

**GIF Export (revised):**
> **Performance Oracle:** "Nobody exports 1080p GIFs. Cap at 720p, default 10fps. Use FFmpeg for GIF generation — significantly faster than JS-based encoding."

- Cap GIF at 720p max, recommend 480p
- Default 10fps (standard for web GIFs)
- Limit duration to 15 seconds with warning
- Prefer FFmpeg GIF pipeline: `fps=10,scale=480:-1:flags=lanczos,palettegen`

**Output Formats (revised):**
- MP4 (H.264) — universal compatibility
- GIF — social media previews (FFmpeg-based, capped at 720p)
- ~~WebM~~ — defer to v2 (MP4 covers all platforms)
- ~~PNG sequence~~ — cut (no target audience)

---

## Implementation Phases (Revised: 3 Phases, AI-First)

> **Code Simplicity Reviewer:** "The plan has it backwards. It builds complex manual UI first, then adds AI on top. For social media creators, AI should handle the hard parts (timing, easing, staging). Manual UI only needs to support light editing."

### Phase 1: Animation Core + AI Presets (3-4 weeks)

**Deliverables:**
- `timeline-store.ts` — Single Zustand store for all temporal state (Record-based, serializable)
- `interpolation.ts` — 5 easing functions + numeric lerp (~50 lines total)
- `canvas-bridge.ts` — Formal interface mediating animation ↔ Fabric.js
- `playback-loop.ts` — requestAnimationFrame loop, direct Fabric.js mutation, zero React renders
- Animation preset panel: "Fade In", "Slide Left", "Slide Right", "Scale Up", "Bounce In", "Slide Down"
- Play/pause button + scrubber (no multi-track timeline yet)
- MCP context tools: `get_animation_state`, `get_timeline` (agent must read before writing)
- MCP primitives: `add_keyframe`, `remove_keyframe`, `update_keyframe`
- MCP presets: `plan_animation` (returns plan), `apply_animation_plan` (executes)
- Animation data in `.op` file (separate section, backward-compatible)
- 3 canvas size presets: Square (1080x1080), Vertical (1080x1920), Horizontal (1200x675)

**Acceptance Criteria:**
- Select element → click "Slide Left" → press Play → see smooth entrance animation
- AI agent: "animate the title sliding in from the left" → correct keyframes generated
- Animation data persists across save/reload
- Playback at ≥ 30fps with 10 animated objects

### Phase 2: Export + Timeline Polish (2-3 weeks)

**Deliverables:**
- MP4 export via FFmpeg (raw RGBA piping, Electron child_process)
- GIF export via FFmpeg (720p max, 10fps default)
- Export progress UI with cancel
- Element-level timeline (simple bar visualization with keyframe diamonds)
- Keyframe editing: add/move/delete keyframes visually
- 5 easing presets in dropdown
- Undo/redo integration (command pattern with merge support for drag operations)
- MCP feedback tools: `preview_animation`, `get_animation_frame`, `validate_animation`
- MCP batch: `batch_animate`
- MCP export: `export_video`, `export_gif`

**Acceptance Criteria:**
- Can export 15-second animation as MP4 in under 30 seconds
- GIF export at 480p produces < 10MB file
- Manual keyframe editing with undo/redo
- AI agent can verify its animations via `preview_animation`

### Phase 3: Video Clip Support (3-4 weeks)

**Deliverables:**
- Video import (MP4, WebM, MOV)
- Custom Fabric.js video object (renders via `ctx.drawImage()` in `_render()` override)
- `media-store.ts` — Zustand store for video clip metadata and loading state
- Trim controls (in/out point drag handles)
- Video clip positioning on timeline
- Video-over-graphics compositing
- Video pool: load/unload based on playhead proximity
- Basic audio volume control
- MCP tools: `import_video_clip`, `trim_clip`

**Acceptance Criteria:**
- Can import video clip and see it play composited with animated canvas objects
- Trim handles adjust clip boundaries
- Memory stays under 800MB with 2 video clips + 20 animated objects
- Export includes video frames at correct timing

---

## Key Technical Decisions (Revised)

### Decision 1: Animation Library Choice → Custom (~50 lines)

> **Code Simplicity Reviewer:** "Use 20 lines of easing math, not an 'engine'."
> **Architecture Strategist:** "Custom is correct for decoupling from Fabric.js property system."

**Decision: Custom.** The interpolation math is trivial for 5 easing presets. No external dependency. The Fabric.js bridge layer is the only place rendering knowledge lives. Animation engine is pure math, fully unit-testable without a canvas.

### Decision 2: Video Decoding Strategy → HTMLVideoElement + requestVideoFrameCallback

**Decision:** HTMLVideoElement for preview, with `requestVideoFrameCallback()` for frame-accurate sync (available in Electron 35). WebCodecs as optional optimization for export-time frame extraction.

### Decision 3: File Format Extension → Separate Section

**Decision: Separate animation section in .op file.** Add a `reconcileAnimationData()` function that runs on file load, after node deletion/duplication, and before save to prevent orphaned animation references.

```typescript
function reconcileAnimationData(nodes: NodeTree, animations: TimelineState): TimelineState {
  // Remove tracks for nodes that no longer exist
  // Flag orphaned keyframes
  // Validate all animation targets reference valid node IDs
  return cleanedAnimations;
}
```

### Decision 4: Store Architecture → 2 Stores (Not 3)

> **Architecture Strategist:** "Playback state has no independent identity. It's consumed by both animation and video systems."

**Decision: Two new stores.**
- `timeline-store` — clock + all animation data (keyframes, tracks, easing, playback state)
- `media-store` — video clip metadata, audio, source references, loading state (Phase 3 only)

### Decision 5: History/Undo System → Unified Command Pattern

> **Architecture Strategist:** "A single undo stack where every command is tagged with its domain. The `merge` method is critical for animation — drag operations should coalesce into single undoable actions."

```typescript
interface HistoryCommand {
  domain: 'spatial' | 'temporal' | 'media';
  description: string;
  execute(): void;
  undo(): void;
  merge?(previous: HistoryCommand): HistoryCommand | null;
}
```

---

## Race Condition Analysis

> **Frontend Races Reviewer:** 8 specific race conditions identified.

| Race Condition | Risk | Solution |
|---|---|---|
| Animation loop vs React render cycle | High | Engine bypasses Zustand entirely; throttled ref subscription for UI updates |
| Fabric.js events during playback | High | `isPlaybackActive()` check in ALL event handlers; suppress `object:modified` |
| Video seeking is async | Medium | Use `requestVideoFrameCallback()` instead of polling `currentTime` |
| Export frame capture timing | High | Manual stepping loop, NOT `requestAnimationFrame`; synchronous `renderAll()` + `getImageData()` |
| User interaction during playback | Medium | Pause animation on canvas click; lock object interaction via CanvasBridge |
| Undo during playback | Medium | Pause playback before executing any undo; state machine: `idle ↔ playing ↔ exporting` |
| MCP tool calls during playback | Low | Queue animation-modifying MCP calls; execute on next pause |
| Video clip loading race | Medium | Track loading state per clip; show placeholder frame until `canplay` event |

**State machine for playback control:**
```
States: idle | playing | scrubbing | exporting
Transitions:
  idle → playing (play button / MCP play)
  playing → idle (pause / click canvas / undo / MCP mutation)
  idle → scrubbing (mousedown on timeline)
  scrubbing → idle (mouseup)
  idle → exporting (export start)
  exporting → idle (export complete/cancel)
```

---

## Security Considerations

> **Security Sentinel review highlights:**

| Threat | Risk | Mitigation |
|---|---|---|
| FFmpeg command injection via file paths | High | Whitelist arguments; use array-based `spawn()`, never string interpolation; validate file paths against allowlist |
| Malicious .op files with animation data | Medium | Validate all animation property values are numbers/strings within expected ranges; no `eval()` |
| Resource exhaustion (infinite loops, huge exports) | Medium | Hard limits: max duration 5min, max FPS 60, max resolution 4K; timeout on export |
| Video file codec vulnerabilities | Low | Rely on Chromium's sandboxed media decoder; don't parse video files in Node.js |
| Electron IPC escalation | Medium | Define typed IPC channels; never pass arbitrary commands from renderer to main |
| Memory leaks from video elements | Medium | Explicit video pool lifecycle; `video.src = ''` + DOM removal on cleanup |

---

## MCP Tool Design (Revised)

> **Agent-Native Reviewer:** "The proposed tools score 6/16 required capabilities. Critical gaps: no read tools (agent is blind), no feedback loop (can't evaluate results), no batch operation."

### Complete MCP Tool Set

**Context Tools (agent must read before writing):**

| Tool | Purpose | Notes |
|---|---|---|
| `get_animation_state` | Returns keyframes, easing, duration for a node | Core — agent is blind without this |
| `get_timeline` | Returns full timeline structure | Needed for sequencing decisions |
| `get_animation_frame` | Returns visual state at a timestamp | Multimodal agent can verify visually |

**Primitive Tools (CRUD):**

| Tool | Purpose | Notes |
|---|---|---|
| `add_keyframe` | Add keyframe to a track | Core primitive — keep |
| `update_keyframe` | Modify existing keyframe | MISSING from original — add (avoids delete+add) |
| `remove_keyframe` | Delete keyframe | Core primitive — keep |
| Animation properties on `update_node` | Set duration, easing, delay | Merge into existing tool, don't create separate setters |

**Batch/Workflow Tools:**

| Tool | Purpose | Notes |
|---|---|---|
| `batch_animate` | Apply animation to multiple nodes | Mirrors existing `batch_design` pattern |
| `plan_animation` | Generate animation plan (no side effects) | Returns plan for agent review |
| `apply_animation_plan` | Execute a reviewed plan | Agent confirms before applying |
| `validate_animation` | Check for common issues | Timing conflicts, missing entrances |

**Feedback Tools:**

| Tool | Purpose | Notes |
|---|---|---|
| `preview_animation` | Render low-fidelity preview | Agent self-correction loop |
| `set_playhead` | Navigate to specific time | Inspect state at any point |

**I/O Tools:**

| Tool | Purpose | Notes |
|---|---|---|
| `import_video_clip` | Import video file | Phase 3 |
| `trim_clip` | Set in/out points | Phase 3 |
| `export_video` | Export MP4 | Phase 2 |
| `export_gif` | Export GIF | Phase 2 |

**MCP Namespacing:**
```
Existing:  design:*   (insert_node, update_node, batch_design, etc.)
New:       animation:* (add_keyframe, get_timeline, plan_animation, etc.)
New:       video:*    (import_video_clip, trim_clip — Phase 3)
New:       playback:* (play, pause, seek, set_playhead)
New:       export:*   (export_video, export_gif)
```

### Motion Design Guidelines (System Prompt Injection)

> **Agent-Native Reviewer:** "The agent cannot learn motion design from tools alone. This is a system prompt responsibility."

```
## Motion Design Guidelines (inject into MCP system prompt)

### Easing
- Entrances: use ease-out (fast start, gentle stop — element "arrives")
- Exits: use ease-in (gentle start, fast end — element "departs")
- Emphasis: use ease-in-out (smooth both ends)
- Never use linear for UI motion (feels mechanical)

### Timing
- Micro-interactions: 100-200ms
- Entrances/exits: 200-500ms
- Page transitions: 300-700ms
- Complex sequences: budget 1.5-3s total

### Staging
- Animate one focal element first, then supporting elements
- Use 50-100ms stagger between sequential elements
- Hierarchy: larger/more important elements animate first

### Spatial
- Elements enter from the direction of their source
- "Slide from left" means translateX: -(elementWidth + margin) → 0
- Maintain consistent direction within a sequence
```

---

## Memory Budget

> **Performance Oracle analysis:**

| Component | Memory (estimated) |
|---|---|
| Electron base + React app | ~150-200MB |
| Fabric.js canvas (1080p, 20 objects) | ~30-50MB |
| Object caching (20 cached bitmaps) | ~40-80MB |
| Keyframe data (1000 keyframes) | ~1-2MB |
| Timeline UI React tree | ~10-20MB |
| **Phase 1-2 total** | **~230-350MB** |
| + Video clip 1 (1080p, decoded buffers) | +50-80MB |
| + Video clip 2 | +50-80MB |
| + Undo history (10 command snapshots) | +50-100MB |
| **Phase 3 total** | **~430-700MB** |

**Target:** Under 800MB during editing, under 1.2GB during export.

---

## Risk Assessment (Revised)

| Risk | Impact | Likelihood | Mitigation | Source |
|------|--------|------------|------------|--------|
| Fabric.js `renderAll()` exceeds 33ms budget | High | Medium | Object caching, skip `setCoords()`, profile early | Performance Oracle |
| Animation/React state divergence during playback | High | High | CanvasBridge with `isPlaybackActive()` flag | Architecture Strategist, Races Reviewer |
| History system doesn't support animation undo | High | Medium | Unified command pattern with merge(); assess if current system is snapshot-based | Architecture Strategist |
| Animation data orphaned when nodes deleted | High | Medium | `reconcileAnimationData()` on load, delete, duplicate, save | Architecture Strategist |
| FFmpeg command injection | High | Low | Array-based `spawn()`, no string interpolation, path validation | Security Sentinel |
| OpenPencil upstream API changes | High | High | Fork and maintain, or contribute upstream | Original assessment |
| Over-engineering timeline UI for social media users | Medium | High | Preset-first UX, simple element list (not NLE tracks) | Code Simplicity Reviewer |

---

## Dependencies & Prerequisites

### Required New Dependencies

| Package | Purpose | License | Phase |
|---------|---------|---------|-------|
| `ffmpeg-static` | Video/GIF encoding via Electron | MIT | 2 |
| None (custom ~50 lines) | Keyframe interpolation | N/A | 1 |

### Development Environment

- Node.js 20+ (via Electron)
- Bun (existing OpenPencil build tool)
- FFmpeg binary (for export testing, Phase 2)
- Sample video assets (Phase 3)

---

## Success Metrics (Revised)

1. **Animation authoring:** User applies preset animation to 5 elements in < 1 minute
2. **AI animation:** Agent generates entrance animations for a full page from natural language in < 10 seconds
3. **Export quality:** Exported MP4 matches canvas preview at 1080p 30fps
4. **Performance:** Animation preview at ≥ 30fps with 10 animated objects
5. **Export speed:** 15-second animation exports as MP4 in < 30 seconds
6. **GIF size:** GIF exports at 480p are < 10MB
7. **Memory:** Editor stays under 800MB during authoring (Phase 1-2)
