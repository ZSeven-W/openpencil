# OpenPencil Animation Extension — Functional Requirements

## Purpose

Extend OpenPencil from a static design tool into a motion content creation tool for social media. Creators should be able to animate their designs and export short video/GIF content — without leaving the editor.

---

## Animatable Scope

### What can be animated?

**Any layer** on the canvas can be animated, regardless of type (shape, text, image, icon, path, group, frame, component instance).

**Animatable properties — Core (v1):**

| Property | Description | Applies to |
|----------|-------------|------------|
| Position (x, y) | Move across canvas | All layers |
| Scale (x, y) | Grow/shrink | All layers |
| Rotation | Spin/tilt | All layers |
| Opacity | Fade in/out | All layers |

**Animatable properties — Extended (v2 candidates):**

| Property | Description | Applies to |
|----------|-------------|------------|
| Fill color | Color transitions | Shapes, text, icons |
| Stroke color | Border color transitions | Shapes, text |
| Corner radius | Sharp ↔ rounded | Rectangles |
| Font size | Text scaling | Text layers |
| Shadow (offset, blur) | Shadow animation | Any layer with effects |
| Blur radius | Focus/defocus | Any layer with effects |

### Groups and nested layers

- Animating a **group** transforms the entire group (all children move/scale/rotate/fade together)
- Individual children inside a group **can also be animated independently** — their animations compose with the group's animation
- **Component instances** can be animated as a whole; animating internal overrides is deferred to v2

---

## Functional Requirements

### 1. Animation Presets

Presets are complete animation sequences with three phases: **In** (entrance), **While** (looping emphasis during hold), and **Out** (exit). Each preset defines all three.

**4 presets for v1:**

| Preset | In | While | Out |
|--------|-----|-------|-----|
| **Fade** | Fade in (opacity 0→1) | Gentle pulse (subtle scale oscillation) | Fade out (opacity 1→0) |
| **Slide** | Slide in from edge | Hold position | Slide out to opposite edge |
| **Scale** | Scale up from 0 | Subtle breathe (scale oscillation) | Scale down to 0 |
| **Bounce** | Bounce in (overshoot settle) | Gentle bob (y-axis oscillation) | Bounce out (compress then exit) |

**Preset behavior:**
- Select a layer → choose a preset → all three phases (in/while/out) are applied as a set
- Each phase generates editable keyframes (user can fine-tune after applying)
- The **while** phase loops for the duration between in and out
- Presets adapt to the element's current position and size
- The in/while/out durations are configurable after applying
- Applying a new preset replaces the existing animation on that layer

### 2. Preview Animations

**As a** creator,
**I want to** preview my animation directly on the canvas timeline,
**so that** I can see how it looks before exporting.

**Requirements:**
- Play/pause toggle
- Scrub bar to jump to any point in the composition
- Current time indicator showing position in timeline
- Loop toggle for continuous playback
- Clicking the canvas during playback pauses the animation
- Visual indication on timeline of which layers have animations
- Preview renders at minimum 30fps with up to 10 animated layers
- In/while/out phases are visually distinct on the timeline (e.g., different shading or markers)

### 3. Edit Animation Duration

**As a** creator,
**I want to** control how long each animation phase lasts and when it starts,
**so that** I can sequence multiple layer animations into a cohesive motion piece.

**Requirements:**
- Set total composition duration (default: 5 seconds, max: 5 minutes)
- Per-layer timing controls:
  - **In duration** — how long the entrance takes
  - **While duration** — how long the hold/loop phase lasts
  - **Out duration** — how long the exit takes
  - **Start delay** — when this layer's animation begins relative to composition start
- Visual timeline showing each animated layer as a bar with in/while/out segments
- Drag edges of segments to adjust durations
- Drag entire bar to adjust start delay
- Easing per phase — offered as named presets: smooth, snappy, bouncy, gentle, linear
- Choose frame rate: 24, 30, or 60 fps

### 4. AI-Generated Animations

**As a** creator,
**I want to** describe what I want in natural language and have the AI animate my design,
**so that** I can create animations without manual configuration.

**Requirements:**
- AI can read the current animation state before making changes
- AI can generate animation plans and apply them
- AI follows motion design best practices (appropriate easing, timing, staging)
- Supported prompts include:
  - "Animate the title sliding in from the left"
  - "Make all elements fade in one after another"
  - "Add a bounce entrance to the logo"
  - "Create an exit animation for everything"
  - "Stagger all elements entering with 200ms delays"
- AI animations produce the same in/while/out keyframe structure as presets
- AI can preview and self-correct its results
- User can fine-tune AI-generated animations manually

### 5. Export as Video

**As a** creator,
**I want to** export my animated design as a video file,
**so that** I can share it on social media platforms.

**Requirements:**
- Export formats:
  - **MP4** (H.264) — primary, universal compatibility
  - **GIF** — for platforms/contexts that need it
- Export settings:
  - Resolution: match canvas size (with option to downscale)
  - Frame rate: 24, 30, or 60 fps
  - Quality slider (affects file size)
- GIF-specific constraints:
  - Maximum resolution: 720p
  - Default frame rate: 10fps
  - Duration warning if over 15 seconds
- Progress indicator during export with cancel option
- Export target: 15-second animation exports in under 30 seconds

### 6. Import and Trim Video Clips *(Phase 3)*

**As a** creator,
**I want to** bring in video clips and overlay them with my animated designs,
**so that** I can create richer content combining footage with graphics.

**Requirements:**
- Import MP4, WebM, and MOV files
- Position and resize video clips on canvas like any other layer
- Trim clips by adjusting in/out points with drag handles
- Video clips play in sync with canvas animations during preview
- Basic volume control per clip
- Limit of 2-3 simultaneous video clips

### 7. Canvas Format

The animation canvas is fixed to **9:16 vertical format (1080×1920)** — the dominant format for short-form social video (Instagram Stories/Reels, TikTok, YouTube Shorts).

No other aspect ratios or custom dimensions for v1.

---

## Cross-Cutting Requirements

### Save & Persistence

- Animation data saves with the design file (`.op` format)
- Opening a file with animations restores full animation state
- Files without animations remain backward-compatible
- Deleting a layer removes its animation data
- Duplicating a layer duplicates its animations

### Undo/Redo

- All animation operations are undoable
- Drag operations (adjusting durations, start delays) coalesce into a single undo step
- Undo triggered during playback pauses first, then undoes

### Performance

- Preview: minimum 30fps with 10 animated layers
- Editor remains responsive while animation panel is open
- Export runs in background with progress indicator
- No runaway memory growth from long editing sessions

### Mode Switching

- Clear distinction between **Design mode** (static editing) and **Animate mode** (temporal editing)
- Switching modes preserves all state
- Layers remain visible and selectable in Animate mode for context

---

## Phasing

| Phase | Focus | What the User Gets |
|-------|-------|-------------------|
| **1** | Animation core + AI | 4 animation presets (in/while/out), AI-generated animations, play/pause/scrub, save/load |
| **2** | Export + timeline editing | MP4 and GIF export, visual timeline with duration editing, easing presets, undo/redo |
| **3** | Video clips | Import/trim video clips, video+animation compositing, volume control |

---

## Out of Scope (v1)

- Multi-track audio mixing or audio editing
- 3D animation or particle effects
- Real-time collaboration on animations
- Mobile/tablet editing
- WebM export (MP4 covers all platforms)
- Custom bezier easing curves (named presets only)
- Property graph/curve editor
- Frame-by-frame sprite animation
- Animating properties inside component instances (group-level only)
- Extended animatable properties (fill color, corner radius, shadow, etc. — v2)
