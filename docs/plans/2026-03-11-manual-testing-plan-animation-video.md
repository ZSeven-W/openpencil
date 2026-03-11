# Manual Testing Plan — Animation & Video Features (Phases 1–7)

**Date:** 2026-03-11
**Branch:** `feat/animation-core`
**Commits covered:** `dcbee8d` → `22c7206` → `32dedcc` → `00d0e2f`

---

## How to Use This Plan

Each section has numbered test cases with **steps**, **expected result**, and a **pass/fail** checkbox. Work through them in order — later sections depend on earlier ones. Mark `[x]` when passing, `[!]` for issues found.

**Setup:** `bun install && bun --bun run dev` → open `http://localhost:3000/editor`

---

## 1. Editor Mode Toggle

### 1.1 Keyboard shortcut toggles animate mode
- [ ] Open the editor with a blank canvas
- [ ] Press `Cmd+Shift+A`
- **Expected:** Timeline panel appears below the canvas. Right panel switches to "Animate" tab.

### 1.2 Toggle back to design mode
- [ ] Press `Cmd+Shift+A` again
- **Expected:** Timeline panel disappears. Right panel returns to previous tab.

### 1.3 Tab click switches mode
- [ ] Click the "Animate" tab in the right panel
- **Expected:** `editorMode` switches to `'animate'`, timeline panel appears.

### 1.4 Switching tabs exits animate mode
- [ ] While in animate mode, click the "Design" or "Properties" tab
- **Expected:** Timeline panel disappears, `editorMode` returns to `'design'`.

---

## 2. Animation Presets (Phase 1)

### 2.1 Apply Fade preset
- [ ] Draw a rectangle on the canvas
- [ ] Select it, switch to Animate mode
- [ ] Click "Fade" in the preset grid
- **Expected:** Track appears in the track list. Phase bars (in/while/out) are visible. Keyframe diamonds appear at phase boundaries.

### 2.2 Apply Slide preset (all 4 directions)
- [ ] Select a rectangle. In the Animate tab, set direction to "Left", click "Slide"
- **Expected:** Track created. First keyframe has negative X offset (offscreen left).
- [ ] Repeat for "Right" → positive X offset
- [ ] Repeat for "Top" → negative Y offset
- [ ] Repeat for "Bottom" → positive Y offset

### 2.3 Apply Scale preset
- [ ] Select an element, click "Scale"
- **Expected:** Track created. First keyframe has `scaleX: 0, scaleY: 0`. Last keyframe also scales to 0.

### 2.4 Apply Bounce preset
- [ ] Select an element, click "Bounce"
- **Expected:** Track created. At least one keyframe has scaleX > 1.0 (overshoot effect).

### 2.5 Easing configuration
- [ ] Select "Snappy" from the easing dropdown, then apply any preset
- **Expected:** Keyframes in the track use the "snappy" easing preset.

### 2.6 Reapply preset replaces track
- [ ] Apply "Fade" to an element, then apply "Slide" to the same element
- **Expected:** Only one track exists for that element. The old Fade keyframes are replaced by Slide keyframes.

### 2.7 Apply preset to multiple elements
- [ ] Draw 3 shapes. Select each one individually and apply different presets
- **Expected:** 3 separate tracks appear in the track list, each with correct preset.

### 2.8 Phase info display
- [ ] After applying a preset, check the "Phase info" section in the Animate tab
- **Expected:** Shows in/while/out durations and total keyframe count.

---

## 3. Playback Engine (Phase 1)

### 3.1 Basic play/pause
- [ ] Apply a Slide preset to an element
- [ ] Press Play
- **Expected:** Element animates from offscreen to its position (in phase), holds (while phase), then exits (out phase). Playhead moves in the scrub bar.

### 3.2 Pause mid-animation
- [ ] Press Play, then press Pause during the "in" phase
- **Expected:** Animation freezes. Element stays at its interpolated position. Time display shows current time.

### 3.3 Resume after pause
- [ ] Pause, then press Play again
- **Expected:** Animation resumes from where it paused, not from the beginning.

### 3.4 Stop resets state
- [ ] Play an animation partially, then press Stop
- **Expected:** Element snaps back to its original position. Time resets to 0:00.00. Playhead returns to start.

### 3.5 Loop playback
- [ ] Enable the Loop toggle, press Play
- **Expected:** Animation plays through, then seamlessly restarts from the beginning. Repeats until paused/stopped.

### 3.6 Multi-element playback
- [ ] Apply presets to 5+ elements, press Play
- **Expected:** All elements animate simultaneously. No visible jitter or frame drops. Smooth 30fps.

### 3.7 Playback performance with 10 elements
- [ ] Create 10 shapes, apply "Fade" to each
- [ ] Press Play
- **Expected:** Smooth playback at ≥30fps. No visible stutter. Time display updates ~10x/sec.

### 3.8 Canvas interaction blocked during playback
- [ ] While animation is playing, try to click/drag objects on canvas
- **Expected:** Objects should not be selectable or draggable during playback (locked interaction).

---

## 4. Scrub Bar (Phase 2)

### 4.1 Click to seek
- [ ] Apply a preset, then click at the 50% point of the scrub bar
- **Expected:** Playhead jumps to that position. Canvas shows the interpolated state at that time. Time display updates.

### 4.2 Drag to scrub
- [ ] Click and drag along the scrub bar
- **Expected:** Canvas updates in real-time as you drag. Smooth preview of the animation at each time point.

### 4.3 Scrub during playback
- [ ] Press Play, then click the scrub bar
- **Expected:** Playback pauses. Playhead jumps to click position. Canvas shows state at that time.

### 4.4 Scrub bar bounds
- [ ] Drag past the left edge (before 0)
- **Expected:** Clamps to 0. No negative time.
- [ ] Drag past the right edge (past duration)
- **Expected:** Clamps to duration.

---

## 5. Keyframe Editing (Phase 2)

### 5.1 Keyframe diamonds visible
- [ ] Apply a preset to an element
- **Expected:** Diamond markers appear on the phase bars at correct positions corresponding to keyframe times.

### 5.2 Drag keyframe to new time
- [ ] Select the animated element on canvas, then drag a keyframe diamond along the track
- **Expected:** Keyframe time updates. Phase boundaries recalculate if the keyframe crosses a phase boundary.

### 5.3 Keyframe drag restricted to selected node
- [ ] With element A selected, try to drag a keyframe diamond belonging to element B's track
- **Expected:** Diamond should NOT be draggable (only selected node's keyframes are interactive).

### 5.4 Phase bar coloring
- [ ] Apply a preset and inspect the track visually
- **Expected:** Three distinct colored regions: green/emerald (in), blue (while), red (out).

---

## 6. Animation Persistence (Phase 1)

### 6.1 Save with animation data
- [ ] Apply presets to 2 elements, then save the file (Cmd+S)
- **Expected:** File saves without error.

### 6.2 Reload preserves animation
- [ ] Close and reopen the saved file
- **Expected:** Animation tracks reappear in the track list. Keyframes are at their saved positions. Playing the animation produces the same result.

### 6.3 File without animation opens cleanly
- [ ] Open a `.op` file that was created before animation features existed
- **Expected:** No errors. Timeline store is empty. Animation features work normally when presets are applied.

### 6.4 Orphan track reconciliation
- [ ] Apply a preset to element A, save the file
- [ ] Delete element A from the canvas, save again
- [ ] Reopen the file
- **Expected:** No orphaned animation track for the deleted element. Track list is clean.

### 6.5 Node duplication preserves animation
- [ ] Apply a preset to an element, then duplicate it (Cmd+D)
- **Expected:** Both the original and duplicate have independent animation tracks.
- [ ] Modify one track — the other should be unaffected.

---

## 7. Video Node Engine (Phase 3)

### 7.1 Video import via toolbar
- [ ] Click the shape dropdown in the toolbar, select "Import Video"
- [ ] Choose an MP4 file
- **Expected:** Video appears on canvas as a visual element. Placeholder rect shown immediately, then replaced by first video frame once loaded.

### 7.2 Auto-switch to animate mode on video import
- [ ] Import a video while in design mode
- **Expected:** Editor automatically switches to animate mode. Right panel shows Animate tab. Video clip appears in the video clip track area.

### 7.3 Video node properties
- [ ] Select the video node
- **Expected:** Animate tab shows "Video Clip" section with In/Out/Offset controls and duration readouts.

### 7.4 Video element registration
- [ ] Import a video, check the timeline
- **Expected:** Video clip ID appears in timeline store's `videoClipIds`. Video element is registered in the registry.

### 7.5 Video cleanup on delete
- [ ] Import a video, then delete the video node from canvas
- **Expected:** Video element is unregistered. No memory leak (video `src` cleared). Video clip track entry disappears.

### 7.6 Video frame display at time 0
- [ ] Import a video with a distinctive first frame
- **Expected:** The first frame of the video is visible on canvas (not a blank/black placeholder).

### 7.7 Duration auto-extend
- [ ] With a 3-second composition, import a 10-second video
- **Expected:** Composition duration extends to at least 10 seconds to fit the video.

---

## 8. Video Playback Sync (Phase 3)

### 8.1 Video plays during playback
- [ ] Import a video clip, press Play
- **Expected:** Video frames advance in sync with the composition timeline. Video content is visible and updating on the Fabric canvas.

### 8.2 Video + animation simultaneous playback
- [ ] Import a video. Also add animated shapes (with presets) on top of the video
- [ ] Press Play
- **Expected:** Video plays AND shape animations run simultaneously. No sync drift.

### 8.3 Video hides outside clip range
- [ ] Set video `timelineOffset` to 2000ms (2 seconds)
- [ ] Scrub to 0s
- **Expected:** Video node is hidden on canvas.
- [ ] Scrub to 3s
- **Expected:** Video is visible and showing the correct frame.

### 8.4 Video respects inPoint/outPoint
- [ ] Set `inPoint` to 1000ms, `outPoint` to 3000ms
- [ ] Press Play
- **Expected:** Video starts from the 1-second mark of the source and stops at the 3-second mark.

### 8.5 All videos pause on Stop
- [ ] Import 2 videos, press Play, then Stop
- **Expected:** Both video elements are paused. No audio continues after stop.

---

## 9. Video UI — Timeline Clips (Phase 4)

### 9.1 Video clip bar renders
- [ ] Import a video
- **Expected:** A violet-colored clip bar appears in the video clip track area. Shows film icon + node name + clip duration.

### 9.2 Clip bar positioning
- [ ] Set `timelineOffset` to 1000ms on a 5-second composition
- **Expected:** Clip bar starts at ~20% from the left of the track area.

### 9.3 Drag clip to reposition
- [ ] Click and drag the center of the video clip bar
- **Expected:** `timelineOffset` updates. Clip moves on the timeline. Canvas preview updates to show the video at the new position.

### 9.4 Clip bar selection state
- [ ] Click a video clip bar
- **Expected:** Bar shows selected state (brighter violet). The corresponding video node is selected on canvas.

---

## 10. Video Trim Controls (Phase 4)

### 10.1 Left trim handle (inPoint)
- [ ] Drag the left edge of a video clip bar to the right
- **Expected:** `inPoint` increases. Clip bar shortens from the left. The video starts from a later frame.
- [ ] Verify by pressing Play — video should start from the new in-point.

### 10.2 Right trim handle (outPoint)
- [ ] Drag the right edge of a video clip bar to the left
- **Expected:** `outPoint` decreases. Clip bar shortens from the right. The video ends at an earlier frame.
- [ ] Verify by pressing Play — video should end at the new out-point.

### 10.3 Trim via number inputs
- [ ] In the Animate tab, manually type new In/Out values for a selected video node
- **Expected:** Values update. Clip bar in the timeline reflects the new trim points. Playback respects the new values.

### 10.4 Trim boundaries enforced
- [ ] Try to drag inPoint past outPoint
- **Expected:** Should be prevented or clamped.
- [ ] Try to drag outPoint past source video duration
- **Expected:** Should be clamped to source duration.
- [ ] Try to drag inPoint below 0
- **Expected:** Clamped to 0.

### 10.5 Scrub updates during trim
- [ ] While dragging a trim handle, observe the canvas
- **Expected:** Canvas updates via `seekTo` to show the video frame at the new trim point.

---

## 11. Track List & Visual Display

### 11.1 Track names from node names
- [ ] Name a rectangle "Hero Title", apply a preset
- **Expected:** Track list shows "Hero Title" as the track label.

### 11.2 Empty state
- [ ] Enter animate mode with no presets applied
- **Expected:** Track list shows a helpful empty state message (not blank).

### 11.3 Multiple tracks ordering
- [ ] Apply presets to elements A, B, C in that order
- **Expected:** Tracks appear in the track list in order of creation.

### 11.4 Track removal when node deleted
- [ ] Apply a preset, then delete the node from canvas
- **Expected:** Track disappears from the track list immediately (via `reconcile`).

---

## 12. Canvas Bridge Isolation

### 12.1 No Zustand writes during playback
- [ ] Open browser DevTools, set a conditional breakpoint on `timeline-store` `setCurrentTime`
- [ ] Play an animation
- **Expected:** `setCurrentTime` fires at ~10fps intervals (every ~100ms), NOT every frame. The actual animation runs at native RAF rate.

### 12.2 Fabric events suppressed during playback
- [ ] Play an animation, check that no `object:modified` events fire
- **Expected:** `isPlaybackActive()` returns `true` during playback. Canvas event handlers short-circuit.

### 12.3 State restoration on stop
- [ ] Move an element to position (100, 200). Apply "Slide Left" preset. Play, then Stop.
- **Expected:** Element returns to exactly (100, 200). No drift in position, scale, rotation, or opacity.

---

## 13. Edge Cases & Error Handling

### 13.1 Apply preset with no selection
- [ ] Deselect all elements, click a preset button
- **Expected:** Nothing happens (no error, no crash). Preset buttons should be disabled or silently no-op.

### 13.2 Play with no animations
- [ ] Enter animate mode with no tracks, press Play
- **Expected:** Play button works but nothing visually changes. Time advances. No errors.

### 13.3 Very short composition
- [ ] Set duration to minimum (e.g., 100ms), apply a preset
- **Expected:** Preset compresses into the short duration. Playback works (very fast).

### 13.4 Large canvas with many objects
- [ ] Create 20+ objects, apply presets to 10 of them
- [ ] Press Play
- **Expected:** Playback maintains acceptable frame rate. Non-animated objects remain static.

### 13.5 Video import with unsupported format
- [ ] Try importing a non-video file (e.g., a .txt renamed to .mp4)
- **Expected:** Graceful failure — no crash. Error state or fallback placeholder shown.

### 13.6 Rapid play/stop cycling
- [ ] Click Play/Stop rapidly 10+ times
- **Expected:** No stuck states. No orphaned RAF loops. Final state is correct (stopped, objects at original positions).

### 13.7 Delete animated node during playback
- [ ] Play an animation, then delete one of the animated nodes
- **Expected:** No crash. Deleted node disappears. Other animations continue. Track is removed.

### 13.8 Undo/redo with animation
- [ ] Apply a preset, then Cmd+Z
- **Expected:** Track is removed. Element returns to pre-animation state.
- [ ] Cmd+Shift+Z
- **Expected:** Track is restored with all keyframes.

---

## 14. Multi-Page Interaction

### 14.1 Animation tracks are page-specific
- [ ] On page 1, apply a preset to an element
- [ ] Switch to page 2
- **Expected:** Track list shows tracks relevant to page 2 only (or empty if no animations on page 2).
- [ ] Switch back to page 1
- **Expected:** Original tracks reappear.

### 14.2 Video on different pages
- [ ] Import a video on page 1, switch to page 2
- **Expected:** Video node and clip track are not visible on page 2's timeline.

---

## 15. Integration Smoke Tests

### 15.1 Full workflow: design → animate → save → reload → play
- [ ] Create a design with 3 shapes
- [ ] Switch to animate mode
- [ ] Apply Slide Left to shape 1, Fade to shape 2, Bounce to shape 3
- [ ] Press Play — verify all animate correctly
- [ ] Save the file
- [ ] Close and reopen
- [ ] Switch to animate mode, press Play
- **Expected:** Identical playback after reload.

### 15.2 Full workflow: video + animation composite
- [ ] Import a video clip
- [ ] Add a text overlay, apply Fade preset
- [ ] Add a logo shape, apply Scale preset
- [ ] Trim the video to 3 seconds
- [ ] Press Play
- **Expected:** Video plays trimmed, text fades in, logo scales in — all in sync on the timeline.

### 15.3 Type check passes
- [ ] Run `npx tsc --noEmit`
- **Expected:** No type errors related to animation/video code.

### 15.4 Test suite passes
- [ ] Run `bun --bun run test`
- **Expected:** All tests pass, including `interpolation.test.ts` and `presets.test.ts`.

### 15.5 Build succeeds
- [ ] Run `bun --bun run build`
- **Expected:** Production build completes without errors.

---

## Issue Tracking Template

| # | Test Case | Status | Issue Description | Severity |
|---|-----------|--------|-------------------|----------|
| 1.1 | Mode toggle keyboard | | | |
| 2.1 | Fade preset | | | |
| ... | ... | ... | ... | ... |

**Severity levels:** P0 (blocks release), P1 (major bug), P2 (minor bug), P3 (cosmetic/polish)
