# pen-core

Pure document tree operations, layout engine, variables, normalization, boolean ops, and merge utilities.

## Structure

- `src/tree-utils.ts` — Tree CRUD: `findNodeInTree`, `findParentInTree`, `removeNodeFromTree`, `updateNodeInTree`, `flattenNodes`, `insertNodeInTree`, `isDescendantOf`, `getNodeBounds`, `findClearX`, `scaleChildrenInPlace`, `rotateChildrenInPlace`, `nodeTreeToSummary`; page helpers: `createEmptyDocument`, `getActivePage`, `getActivePageChildren`, `setActivePageChildren`, `getAllChildren`, `migrateToPages`, `ensureDocumentNodeIds`; clone utilities: `deepCloneNode`, `cloneNodeWithNewIds`, `cloneNodesWithNewIds`; constants: `DEFAULT_FRAME_ID`, `DEFAULT_PAGE_ID`
- `src/normalize.ts` — `normalizePenDocument`: format-only normalization (fill type "color" to "solid", gradient stop position to offset, sizing strings, padding arrays). Preserves `$variable` refs
- `src/boolean-ops.ts` — `canBooleanOp`, `executeBooleanOp` (union/subtract/intersect via Paper.js headless)
- `src/sync-lock.ts` — `isFabricSyncLocked`, `setFabricSyncLock`: prevents circular document-store to canvas sync
- `src/arc-path.ts` — `buildEllipseArcPath`, `isArcEllipse`: SVG arc path generation for partial ellipses
- `src/path-anchors.ts` — `anchorsToPathData`, `pathDataToAnchors`, `getPathBoundsFromAnchors`, `inferPathAnchorPointType`
- `src/font-utils.ts` — `cssFontFamily`: CSS font-family string builder
- `src/node-helpers.ts` — `isBadgeOverlayNode`, `sanitizeName` (PascalCase conversion)
- `src/design-md-parser.ts` — `parseDesignMd`, `generateDesignMd`, `designMdColorsToVariables`, `extractDesignMdFromDocument`
- `src/constants.ts` — Canvas rendering constants (zoom limits, colors, snap thresholds, pen tool sizes, guide styling)
- `src/id.ts` — `generateId` (nanoid wrapper)
- `src/layout/engine.ts` — Auto-layout computation: `resolvePadding`, `getNodeWidth`, `getNodeHeight`, `computeLayoutPositions`, `inferLayout`, `fitContentWidth`, `fitContentHeight`, `isNodeVisible`, `setRootChildrenProvider`, `getRootFillWidthFallback`
- `src/layout/text-measure.ts` — Text measurement: `estimateTextWidth`, `estimateTextWidthPrecise`, `estimateTextHeight`, `resolveTextContent`, `parseSizing`, `defaultLineHeight`, `hasCjkText`, `isCjkCodePoint`, `countWrappedLinesFallback`, `setWrappedLineCounter`
- `src/layout/normalize-tree.ts` — `normalizeTreeLayout`: infers missing layout mode on frames, strips child x/y in layout containers
- `src/layout/unwrap-fake-phone-mockup.ts` — `unwrapFakePhoneMockups`: repairs AI-generated fake phone mockup frames
- `src/layout/strip-redundant-section-fills.ts` — `stripRedundantSectionFills`: removes redundant dark fills from section containers
- `src/normalize/normalize-stroke-fill-schema.ts` — `normalizeStrokeFillSchema`: repairs AI-generated stroke/fill schema violations
- `src/variables/resolve.ts` — `isVariableRef`, `getDefaultTheme`, `resolveVariableRef`, `resolveColorRef`, `resolveNumericRef`, `resolveNodeForCanvas`
- `src/variables/replace-refs.ts` — `replaceVariableRefsInTree`: recursively rename/delete `$variable` refs in node trees
- `src/merge/node-diff.ts` — `diffDocuments`: one-direction diff producing `NodePatch[]` (add/remove/modify/move)
- `src/merge/node-merge.ts` — `mergeDocuments`: pure 3-way merge of PenDocument trees, returns `MergeResult` with conflicts
- `src/merge/merge-helpers.ts` — Shared helpers for diff/merge (node indexing, field comparison)

## Key patterns

- All tree operations are pure functions returning new references (structural sharing)
- `$variable` refs are preserved in the document; resolution happens at render time via `resolveNodeForCanvas()`
- Layout engine resolves `fill_container`/`fit_content` sizing and computes absolute positions

## Testing

```bash
bun --bun vitest run packages/pen-core/src/__tests__/
```
