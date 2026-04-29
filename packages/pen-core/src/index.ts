// ID generation
export { generateId } from './id.js';

// Tree utilities
export {
  DEFAULT_FRAME_ID,
  DEFAULT_PAGE_ID,
  createEmptyDocument,
  getActivePage,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
  migrateToPages,
  ensureDocumentNodeIds,
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  flattenNodes,
  insertNodeInTree,
  isDescendantOf,
  getNodeBounds,
  findClearX,
  scaleChildrenInPlace,
  rotateChildrenInPlace,
  deepCloneNode,
  cloneNodeWithNewIds,
  cloneNodesWithNewIds,
  nodeTreeToSummary,
} from './tree-utils.js';

// Variables
export {
  isVariableRef,
  getDefaultTheme,
  resolveVariableRef,
  resolveColorRef,
  resolveNumericRef,
  resolveNodeForCanvas,
} from './variables/resolve.js';
export { replaceVariableRefsInTree } from './variables/replace-refs.js';

// Normalization
export { normalizePenDocument } from './normalize.js';

// Layout
export {
  type Padding,
  resolvePadding,
  isNodeVisible,
  setRootChildrenProvider,
  getRootFillWidthFallback,
  inferLayout,
  fitContentWidth,
  fitContentHeight,
  getNodeWidth,
  getNodeHeight,
  computeLayoutPositions,
} from './layout/engine.js';
export { normalizeTreeLayout } from './layout/normalize-tree.js';
export { unwrapFakePhoneMockups } from './layout/unwrap-fake-phone-mockup.js';
export { stripRedundantSectionFills } from './layout/strip-redundant-section-fills.js';
export { normalizeStrokeFillSchema } from './normalize/normalize-stroke-fill-schema.js';

// Text measurement
export {
  parseSizing,
  defaultLineHeight,
  isCjkCodePoint,
  hasCjkText,
  estimateGlyphWidth,
  estimateLineWidth,
  widthSafetyFactor,
  estimateTextWidth,
  estimateTextWidthPrecise,
  resolveTextContent,
  countExplicitTextLines,
  getTextOpticalCenterYOffset,
  countWrappedLinesFallback,
  type WrappedLineCounter,
  setWrappedLineCounter,
  estimateTextHeight,
} from './layout/text-measure.js';

// Constants
export {
  MIN_ZOOM,
  MAX_ZOOM,
  ZOOM_STEP,
  SNAP_THRESHOLD,
  DEFAULT_FILL,
  DEFAULT_STROKE,
  DEFAULT_STROKE_WIDTH,
  CANVAS_BACKGROUND_LIGHT,
  CANVAS_BACKGROUND_DARK,
  SELECTION_BLUE,
  COMPONENT_COLOR,
  INSTANCE_COLOR,
  HOVER_BLUE,
  HOVER_LINE_WIDTH,
  HOVER_DASH,
  INDICATOR_BLUE,
  INDICATOR_LINE_WIDTH,
  INDICATOR_DASH,
  INDICATOR_ENDPOINT_RADIUS,
  FRAME_LABEL_FONT_SIZE,
  FRAME_LABEL_OFFSET_Y,
  FRAME_LABEL_COLOR,
  PEN_ANCHOR_FILL,
  PEN_ANCHOR_RADIUS,
  PEN_ANCHOR_FIRST_RADIUS,
  PEN_HANDLE_DOT_RADIUS,
  PEN_HANDLE_LINE_STROKE,
  PEN_RUBBER_BAND_STROKE,
  PEN_RUBBER_BAND_DASH,
  PEN_CLOSE_HIT_THRESHOLD,
  DIMENSION_LABEL_OFFSET_Y,
  DEFAULT_FRAME_FILL,
  DEFAULT_TEXT_FILL,
  GUIDE_COLOR,
  GUIDE_LINE_WIDTH,
  GUIDE_DASH,
} from './constants.js';

// Sync lock
export { isFabricSyncLocked, setFabricSyncLock } from './sync-lock.js';

// Arc path
export { buildEllipseArcPath, isArcEllipse } from './arc-path.js';
export {
  anchorsToPathData,
  getPathBoundsFromAnchors,
  inferPathAnchorPointType,
  pathDataToAnchors,
  type PathBounds,
  type PathAnchorParseResult,
} from './path-anchors.js';

// Boolean operations
export { type BooleanOpType, canBooleanOp, executeBooleanOp } from './boolean-ops.js';

// Font utilities
export { cssFontFamily } from './font-utils.js';

// Node helpers
export { isOverlayNode, isBadgeOverlayNode, sanitizeName } from './node-helpers.js';

// Design-MD parser
export {
  parseDesignMd,
  generateDesignMd,
  designMdColorsToVariables,
  extractDesignMdFromDocument,
} from './design-md-parser.js';

// --- Merge module ---
export type { NodePatch } from './merge/node-diff.js';
export { diffDocuments } from './merge/node-diff.js';
export type {
  MergeInput,
  MergeResult,
  NodeConflict,
  NodeConflictReason,
  DocFieldConflict,
  DocFieldName,
} from './merge/node-merge.js';
export { mergeDocuments } from './merge/node-merge.js';
