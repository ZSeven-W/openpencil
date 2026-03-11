---
status: complete
priority: p2
issue_id: "008"
tags: [code-review, typescript, mcp]
dependencies: []
---

# Remove as any Casts in MCP Animation Tools

## Problem Statement

Several `as any` casts in MCP tool handlers bypass TypeScript safety, masking potential type mismatches.

## Findings

- **TypeScript Reviewer**: `getEffectsByCategory(params.category as any)` should use a validated category type.
- `d.nodeTypes.includes(params.nodeType as any)` should validate nodeType.

## Proposed Solutions

### Option A: Add type guards and validation
- Validate category/nodeType against known values before using
- Effort: Small | Risk: Low

## Acceptance Criteria

- [ ] No `as any` casts in animation MCP tools
- [ ] Invalid categories/nodeTypes return helpful errors

## Work Log

| Date | Action | Learnings |
|------|--------|-----------|
| 2026-03-11 | Created from code review | |
