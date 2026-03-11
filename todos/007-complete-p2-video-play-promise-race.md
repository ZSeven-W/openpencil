---
status: complete
priority: p2
issue_id: "007"
tags: [code-review, frontend-races, video]
dependencies: []
---

# Video play() Promise Race Condition

## Problem Statement

`video.play()` returns a promise. If `pause()` is called before the promise resolves, browsers throw `AbortError`. The current code doesn't handle this.

## Findings

- **Frontend Races Reviewer**: `syncVideoFramesV2` calls `video.play()` without awaiting or catching the abort error.
- Rapid play/pause toggling will produce uncaught promise rejections.

## Proposed Solutions

### Option A: Catch AbortError on play() promise (Recommended)
- `video.play().catch(e => { if (e.name !== 'AbortError') throw e })`
- Effort: Small | Risk: Low

## Acceptance Criteria

- [ ] No uncaught promise rejections on rapid play/pause
- [ ] Video playback still syncs correctly

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
