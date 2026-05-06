/**
 * @zseven-w/pen-sdk — OpenPencil SDK
 *
 * High 级 API 用于处理 OpenPencil (.op) 设计文件。
 * Combines 类型、文档操作、代码生成和 Figma 导入。
 *
 * @example
 * ```ts
 * import {
 *   type PenDocument,
 * createEmptyDocument,
 * normalizePenDocument,
 * parseFigFile,
 * } from '@zseven-w/pen-sdk'
 * ```
 */

// ── Types ──────────────────────────────────────────────────────────────
export type {
  // Document 型号
  PenDocument,
  PenNode,
  PenNodeType,
  PenPage,
  PenNodeBase,
  ContainerProps,
  SizingBehavior,
  FrameNode,
  GroupNode,
  RectangleNode,
  EllipseNode,
  LineNode,
  PolygonNode,
  PathNode,
  TextNode,
  ImageNode,
  ImageFitMode,
  IconFontNode,
  RefNode,
  // Styles
  PenFill,
  PenStroke,
  PenEffect,
  SolidFill,
  LinearGradientFill,
  RadialGradientFill,
  ImageFill,
  GradientStop,
  BlendMode,
  BlurEffect,
  ShadowEffect,
  StyledTextSegment,
  // Variables
  VariableDefinition,
  VariableValue,
  ThemedValue,
  // Canvas
  ToolType,
  ViewportState,
  // UIKit
  UIKit,
  KitComponent,
  ComponentCategory,
  // Theme 预设
  ThemePreset,
  ThemePresetFile,
} from '@zseven-w/pen-types';

// ── Core：Document 操作──────────────────────────────────────────
export {
  // ID 一代
  generateId,
  // Document 创建和树操作
  createEmptyDocument,
  DEFAULT_FRAME_ID,
  DEFAULT_PAGE_ID,
  findNodeInTree,
  findParentInTree,
  removeNodeFromTree,
  updateNodeInTree,
  flattenNodes,
  insertNodeInTree,
  isDescendantOf,
  getNodeBounds,
  // Page 操作
  getActivePage,
  getActivePageChildren,
  setActivePageChildren,
  getAllChildren,
  migrateToPages,
  ensureDocumentNodeIds,
  // Variables
  isVariableRef,
  getDefaultTheme,
  resolveVariableRef,
  resolveColorRef,
  resolveNumericRef,
  resolveNodeForCanvas,
  replaceVariableRefsInTree,
  // Normalization
  normalizePenDocument,
  // Layout
  type Padding,
  resolvePadding,
  computeLayoutPositions,
  getNodeWidth,
  getNodeHeight,
  inferLayout,
  // Text 测量
  parseSizing,
  defaultLineHeight,
  estimateTextWidth,
  estimateTextHeight,
  resolveTextContent,
  hasCjkText,
  // Arc 路径
  buildEllipseArcPath,
  isArcEllipse,
  // Boolean 操作
  type BooleanOpType,
  canBooleanOp,
  executeBooleanOp,
} from '@zseven-w/pen-core';

// ── Codegen 类型（来自笔类型） ──────────────────────────────────────
export type {
  Framework,
  PlannedChunk,
  CodePlanFromAI,
  ExecutableChunk,
  CodeExecutionPlan,
  ChunkContract,
  PropDef,
  SlotDef,
  ImportDef,
  ChunkResult,
  ChunkStatus,
  CodeGenProgress,
  ContractValidationResult,
  NodeSnapshot,
  ExecutableChunkPayload,
  ResolvedDepContract,
} from '@zseven-w/pen-types';
export { FRAMEWORKS } from '@zseven-w/pen-types';

// ── Figma: .fig 文件导入 ────────────────────────────────────────────
export {
  parseFigFile,
  figmaToPenDocument,
  figmaAllPagesToPenDocument,
  getFigmaPages,
  figmaNodeChangesToPenNodes,
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
  resolveImageBlobs,
  setIconLookup,
  type FigmaDecodedFile,
  type FigmaImportLayoutMode,
} from '@zseven-w/pen-figma';

// ── Engine：Headless 设计引擎────────────────────────────────────
export {
  DesignEngine,
  TypedEventEmitter,
  HistoryManager,
  DocumentManager,
  SelectionManager,
  PageManager,
  VariableManager,
  ViewportController,
  EngineSpatialIndex,
  createNodeForTool,
  isDrawingTool,
  parseSvgToNodes,
  type DesignEngineOptions,
  type DesignEngineEvents,
  type CodePlatform,
  type CodeResult,
} from '@zseven-w/pen-engine';

// ── React: React 钩子和组件 ──────────────────────────────────
export * from '@zseven-w/pen-react';

// ── Renderer：CanvasKit/Skia 渲染引擎────────────────────────
export {
  // Primary API
  loadCanvasKit,
  PenRenderer,
  // Low 级别
  SkiaNodeRenderer,
  SkiaFontManager,
  SkiaImageLoader,
  SpatialIndex,
  flattenToRenderNodes,
  resolveRefs,
  premeasureTextHeights,
  // Viewport
  viewportMatrix,
  screenToScene,
  sceneToScreen,
  zoomToPoint,
  // Types
  type RenderNode,
  type PenRendererOptions,
  type IconLookupFn,
} from '@zseven-w/pen-renderer';
