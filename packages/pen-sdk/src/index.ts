/**
 * @zseven-w/pen-sdk — OpenPencil SDK
 *
 * High-level API for working with OpenPencil (.op) design files.
 * Combines types, document operations, code generation, and Figma import.
 *
 * @example
 * ```ts
 * import {
 *   type PenDocument,
 *   createEmptyDocument,
 *   normalizePenDocument,
 *   generateReactFromDocument,
 *   parseFigFile,
 * } from '@zseven-w/pen-sdk'
 * ```
 */

// ── Types ──────────────────────────────────────────────────────────────
export type {
  // Document model
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
  // Theme presets
  ThemePreset,
  ThemePresetFile,
} from '@zseven-w/pen-types'

// ── Core: Document operations ──────────────────────────────────────────
export {
  // ID generation
  generateId,
  // Document creation & tree operations
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
  // Page operations
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
  // Text measurement
  parseSizing,
  defaultLineHeight,
  estimateTextWidth,
  estimateTextHeight,
  resolveTextContent,
  hasCjkText,
  // Arc path
  buildEllipseArcPath,
  isArcEllipse,
  // Boolean operations
  type BooleanOpType,
  canBooleanOp,
  executeBooleanOp,
} from '@zseven-w/pen-core'

// ── Codegen: Multi-platform code generation ────────────────────────────
export {
  // CSS Variables
  variableNameToCSS,
  generateCSSVariables,
  // React + Tailwind
  generateReactCode,
  generateReactFromDocument,
  // HTML + CSS
  generateHTMLCode,
  generateHTMLFromDocument,
  // Vue 3
  generateVueCode,
  generateVueFromDocument,
  // Svelte
  generateSvelteCode,
  generateSvelteFromDocument,
  // Flutter
  generateFlutterCode,
  generateFlutterFromDocument,
  // SwiftUI
  generateSwiftUICode,
  generateSwiftUIFromDocument,
  // Android Compose
  generateComposeCode,
  generateComposeFromDocument,
  // React Native
  generateReactNativeCode,
  generateReactNativeFromDocument,
} from '@zseven-w/pen-codegen'

// ── Figma: .fig file import ────────────────────────────────────────────
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
} from '@zseven-w/pen-figma'

// ── Renderer: CanvasKit/Skia rendering engine ────────────────────────
export {
  // Primary API
  loadCanvasKit,
  PenRenderer,
  // Low-level
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
} from '@zseven-w/pen-renderer'
