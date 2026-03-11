---
status: complete
priority: p1
issue_id: "001"
tags: [code-review, frontend-races, animation, playback]
dependencies: []
---

# v1/v2 Playback Coexistence Race Condition

## Problem Statement

Two animation engines (v1 playback-loop.ts and v2 playback-controller.ts) share a single `isPlaybackActive` boolean and the same Fabric canvas. During the transition period, if both are invoked (e.g., v1 timeline play button triggers v1 loop while v2 API is also available), they will fight over canvas state, causing visual glitches and potential data corruption.

## Findings

- **Frontend Races Reviewer**: v1/v2 coexistence is the biggest risk. Two engines sharing one boolean means one can silently override the other.
- `cursorSetByEngine` in canvas-bridge.ts is consumed destructively — if v1 sets it, v2 won't see it.
- v1 `seekTo` doesn't reset `startTimestamp`, causing drift after seek.
- `fabricObjectMap` can become stale if canvas objects are recreated during playback.

## Proposed Solutions

### Option A: Gate v1 behind feature flag (Recommended)
- Add `useV2Engine` flag, disable v1 playback entirely when v2 is active
- Pros: Clean separation, no race possible
- Cons: Requires ensuring v2 covers all v1 use cases
- Effort: Small
- Risk: Low

### Option B: Mutex lock on playback
- Only one engine can be playing at a time, second call force-pauses first
- Pros: Both engines available
- Cons: More complex, still has edge cases
- Effort: Medium
- Risk: Medium

## Acceptance Criteria

- [ ] Only one playback engine can be active at a time
- [ ] No race conditions between v1 and v2 canvas state
- [ ] `isPlaybackActive()` correctly reflects which engine is running
- [ ] `fabricObjectMap` is validated before frame application

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | v1/v2 coexistence identified as top risk |
