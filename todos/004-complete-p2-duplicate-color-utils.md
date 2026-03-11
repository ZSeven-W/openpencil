---
status: complete
priority: p2
issue_id: "004"
tags: [code-review, quality, duplication]
dependencies: []
---

# Duplicate Color Interpolation Utilities

## Problem Statement

`parseHex`, `formatHex`, and `srgbLerp` are duplicated in both `property-descriptors.ts` and `color-interpolation.ts`.

## Findings

- **TypeScript Reviewer + Code Simplicity**: Same functions appear in two files.
- `color-interpolation.ts` is the canonical location.
- `property-descriptors.ts` should import from `color-interpolation.ts`.

## Proposed Solutions

### Option A: Import from color-interpolation.ts (Recommended)
- Remove duplicates from property-descriptors.ts, import from color-interpolation.ts
- Effort: Small | Risk: Low

## Acceptance Criteria

- [ ] Single source of truth for parseHex, formatHex, srgbLerp
- [ ] All imports point to color-interpolation.ts
- [ ] Tests still pass

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
