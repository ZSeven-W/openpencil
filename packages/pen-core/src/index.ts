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
export {
  applySemanticPalette,
  getSemanticPalette,
  getSemanticPaletteDescription,
  getSemanticPaletteHex,
  hasSemanticPalette,
  SEMANTIC_PALETTE_NAMES,
  SEMANTIC_PALETTE_THEME_AXIS,
  SEMANTIC_PALETTE_THEME_DARK,
  SEMANTIC_PALETTE_THEME_LIGHT,
  type SemanticPalette,
  type SemanticPaletteMode,
} from './variables/semantic-palette.js';

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

// --- Element builders (shared by pen-mcp handlers + apps/web client shims) ---
// Pure tree-build functions matching pen-mcp's add_*_v0 tools. Browser-safe:
// no node:fs, no document-manager imports — just PenNode shape generation.
// Callers layer their own insert pipeline on top (pen-mcp adds parent_id
// validation + rollback; apps/web client shim calls document-store.addNode).
export {
  assignIdsRecursively,
  buildScrollWrapper,
  buildCardRow,
  buildMetricRow,
  buildBottomNav,
  buildSectionHeader,
  buildTopNavBar,
  buildHeading,
  buildBodyText,
  buildTextButton,
  buildSearchBar,
  buildListRow,
  buildDivider,
  buildBadge,
  buildAvatar,
  buildIconButton,
  buildIconLabel,
  buildStatGrid,
  buildSwitch,
  buildCheckbox,
  buildRadio,
  buildTabs,
  buildSegmentedControl,
  buildEmptyState,
  buildAlert,
  buildToast,
  buildProgressBar,
  buildFab,
  buildBreadcrumb,
  buildStepper,
  buildFormField,
  buildTextarea,
  buildSkeleton,
  buildSelect,
  buildChartLine,
  buildChartPie,
  buildImagePlaceholder,
  buildVideoPlaceholder,
  buildComment,
  buildModalShell,
  buildStatusBadge,
  buildSpinner,
  buildTooltip,
  buildMetricComparison,
  buildNotificationRow,
  buildNavChipRow,
  buildActivityRing,
  buildRatingStars,
  buildCarouselDots,
  buildLink,
  buildKbd,
  buildPrice,
  buildQuoteBlock,
  buildCodeBlock,
  buildColorSwatch,
  buildChartBars,
  buildTimeline,
  buildCalendarGrid,
  buildPagination,
  buildFaqItem,
  buildChipInput,
  buildEmptyChart,
  buildActionMenu,
  buildDatePicker,
  buildModalShellV1,
  buildUploadDropzone,
  buildOtpInput,
  cjkFontFamily,
  detectCjkScript,
  type ElementTree,
  type CjkScript,
  type CardRowItem,
  type CardRowParams,
  type MetricRowItem,
  type MetricRowParams,
  type BottomNavItem,
  type BottomNavParams,
  type SectionHeaderAction,
  type SectionHeaderParams,
  type TopNavBarParams,
  type HeadingLevel,
  type HeadingParams,
  type BodyTextParams,
  type TextButtonParams,
  type SearchBarParams,
  type ListRowParams,
  type DividerOrientation,
  type DividerParams,
  type BadgeParams,
  type AvatarParams,
  type IconButtonParams,
  type IconLabelParams,
  type StatGridItem,
  type StatGridParams,
  type SwitchParams,
  type CheckboxParams,
  type RadioParams,
  type TabsItem,
  type TabsParams,
  type SegmentedControlItem,
  type SegmentedControlParams,
  type EmptyStateParams,
  type AlertParams,
  type ToastParams,
  type ProgressBarParams,
  type FabParams,
  type BreadcrumbItem,
  type BreadcrumbParams,
  type StepperParams,
  type FormFieldParams,
  type TextareaParams,
  type SkeletonParams,
  type SelectParams,
  type ChartLineParams,
  type ChartPieParams,
  type ImagePlaceholderParams,
  type VideoPlaceholderParams,
  type CommentParams,
  type ModalShellParams,
  type StatusBadgeParams,
  type StatusBadgeTone,
  type SpinnerParams,
  type TooltipParams,
  type MetricComparisonParams,
  type MetricTrend,
  type NotificationRowParams,
  type NavChipRowItem,
  type NavChipRowParams,
  type ActivityRingParams,
  type RatingStarsParams,
  type CarouselDotsParams,
  type LinkParams,
  type KbdParams,
  type PriceParams,
  type QuoteBlockParams,
  type CodeBlockParams,
  type ColorSwatchParams,
  type ChartBarsParams,
  type TimelineItem,
  type TimelineParams,
  type CalendarGridParams,
  type PaginationParams,
  type FaqItemParams,
  type ChipInputParams,
  type EmptyChartParams,
  type ActionMenuItem,
  type ActionMenuParams,
  type DatePickerParams,
  type ModalShellV1Params,
  type ModalShellV1Theme,
  type UploadDropzoneParams,
  type OtpInputParams,
} from './element-builders/index.js';
