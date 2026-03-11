---
status: complete
priority: p3
issue_id: "009"
tags: [code-review, simplicity, yagni]
dependencies: []
---

# Remove YAGNI Asset Swap Queue

## Problem Statement

`queueAssetSwap` and `flushPendingSwaps` in canvas-bridge.ts implement an asset swap queue that has no callers. This is speculative code.

## Findings

- **Code Simplicity Reviewer**: ~40 LOC of unused code. No current use case for asset swapping during animation.

## Proposed Solutions

### Option A: Delete (Recommended)
- Remove `queueAssetSwap`, `flushPendingSwaps`, and the `pendingSwaps` array
- Effort: Small | Risk: None

## Acceptance Criteria

- [ ] Asset swap queue code removed
- [ ] No exports reference removed functions
- [ ] Tests still pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
