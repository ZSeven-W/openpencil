---
status: complete
priority: p1
issue_id: "002"
tags: [code-review, performance, animation, 60fps]
dependencies: []
---

# Hot Path Allocations in 60fps Animation Loop

## Problem Statement

The animation frame loop (`applyAnimatedFrame`, `interpolateClip`) allocates objects on every frame at 60fps, causing GC pressure and potential frame drops.

## Findings

- **Performance Oracle**: `Object.entries()` in `applyAnimatedFrame` creates a new array every frame.
- `interpolateClip` creates `new Set()` + spread operator for property deduplication every frame.
- `parseHex` uses regex matching on every color interpolation (multiple times per frame per property).
- These allocations compound: 10 animated nodes x 3 properties x 60fps = 1800 allocations/sec minimum.

## Proposed Solutions

### Option A: Pre-allocate and reuse (Recommended)
- Cache `Object.entries()` result in TrackBuffer
- Replace `new Set()` with pre-built property list from AnimationIndex
- Cache parsed hex values (already partially done in color-interpolation.ts)
- Pros: Zero-alloc hot path, measurable FPS improvement
- Cons: Slightly more complex buffer management
- Effort: Small
- Risk: Low

### Option B: Use typed arrays for property values
- Pros: Maximum performance
- Cons: Over-engineering for current scale
- Effort: Large
- Risk: Medium

## Acceptance Criteria

- [ ] `applyAnimatedFrame` creates zero new objects per frame
- [ ] `interpolateClip` reuses property lists from index
- [ ] Hex color parsing is cached per unique color value
- [ ] No regression in interpolation correctness

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | Object.entries + new Set in hot path |
