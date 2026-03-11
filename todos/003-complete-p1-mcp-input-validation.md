---
status: complete
priority: p1
issue_id: "003"
tags: [code-review, security, mcp, validation]
dependencies: []
---

# MCP Animation Tool Input Validation Gaps

## Problem Statement

MCP animation tools accept untrusted input from external LLMs but lack numeric bounds checking, keyframe offset validation, and property key sanitization.

## Findings

- **Security Sentinel**: No bounds on `startTime`, `duration` (negative values, Infinity accepted).
- Keyframe `offset` not validated to be 0.0-1.0 range.
- Arbitrary property keys in keyframes accepted — could inject non-animatable properties.
- Effect registry `registerEffect` allows overwrites of built-in effects.
- `as any` casts in MCP tool handlers bypass TypeScript safety.

## Proposed Solutions

### Option A: Add Zod schemas for all MCP inputs (Recommended)
- Validate numeric ranges, keyframe offsets, property keys against descriptor registry
- Pros: Comprehensive, type-safe, self-documenting
- Cons: Adds Zod dependency (or use manual validation)
- Effort: Small
- Risk: Low

### Option B: Manual validation in each handler
- Pros: No new dependencies
- Cons: Repetitive, easy to miss edge cases
- Effort: Small
- Risk: Low

## Acceptance Criteria

- [ ] `startTime` and `duration` validated as non-negative finite numbers
- [ ] Keyframe `offset` validated to [0, 1] range
- [ ] Property keys in keyframes validated against property descriptor registry
- [ ] Effect registry prevents overwriting built-in effects
- [ ] Remove `as any` casts where possible

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | Open MCP inputs from external LLMs |
