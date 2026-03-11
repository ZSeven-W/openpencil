---
status: complete
priority: p2
issue_id: "005"
tags: [code-review, architecture, circular-dependency]
dependencies: []
---

# Replace require() Pattern in timeline-store.ts

## Problem Statement

`timeline-store.ts` uses `require()` to avoid circular dependency with `document-store.ts`. This bypasses ESM, breaks tree-shaking, and is fragile.

## Findings

- **Architecture Strategist**: `require()` is a code smell for circular dependencies.
- Should extract shared logic into a separate module both stores can import.

## Proposed Solutions

### Option A: Extract composition accessors to shared module (Recommended)
- Create `src/stores/composition-accessors.ts` that reads from document-store
- Both timeline-store and document-store import from it
- Effort: Small | Risk: Low

### Option B: Lazy import via dynamic import()
- Effort: Small | Risk: Low (but async adds complexity)

## Acceptance Criteria

- [ ] No `require()` calls in any store file
- [ ] No circular dependency warnings
- [ ] Composition data accessible from both stores

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
