---
status: complete
priority: p2
issue_id: "006"
tags: [code-review, simplicity, architecture]
dependencies: []
---

# Merge Property Descriptors and Canvas Bindings

## Problem Statement

Two separate registries (`property-descriptors.ts` and `canvas-property-bindings.ts`) maintain parallel lists of the same 22 properties. They could be a single registry with optional canvas binding.

## Findings

- **Code Simplicity Reviewer**: ~200 LOC reduction by merging into one registry.
- Both files register the same property keys — easy to get out of sync.

## Proposed Solutions

### Option A: Merge into single PropertyDescriptor with optional canvasBinding field
- Effort: Medium | Risk: Low

### Option B: Keep separate but add compile-time check for parity
- Effort: Small | Risk: Low

## Acceptance Criteria

- [ ] Single source of truth for property definitions
- [ ] Canvas bindings co-located or validated against descriptors
- [ ] No functionality regression

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
