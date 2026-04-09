// Styles
export type {
  BlendMode,
  SolidFill,
  GradientStop,
  LinearGradientFill,
  RadialGradientFill,
  ImageFill,
  PenFill,
  PenStroke,
  BlurEffect,
  ShadowEffect,
  PenEffect,
  StyledTextSegment,
} from './styles.js';

// Variables
export type { VariableDefinition, VariableValue, ThemedValue } from './variables.js';

// Canvas
export type { ToolType, ViewportState, SelectionState, CanvasInteraction } from './canvas.js';

// Document model
export type {
  PenPage,
  PenDocument,
  PenNodeType,
  SizingBehavior,
  PenNodeBase,
  ContainerProps,
  FrameNode,
  GroupNode,
  RectangleNode,
  EllipseNode,
  LineNode,
  PolygonNode,
  PenPathHandle,
  PenPathAnchor,
  PenPathPointType,
  PathNode,
  TextNode,
  ImageFitMode,
  ImageNode,
  IconFontNode,
  RefNode,
  PenNode,
} from './pen.js';

// UIKit
export type { ComponentCategory, KitComponent, UIKit } from './uikit.js';

// Theme presets
export type { ThemePreset, ThemePresetFile } from './theme-preset.js';

// Design.md
export type { DesignMdSpec, DesignMdColor, DesignMdTypography } from './design-md.js';

// Layout
export type { Padding } from './layout.js';

// Engine
export type {
  DesignEngineOptions,
  DesignEngineEvents,
  CodePlatform,
  CodeResult,
  TextEditState,
  AgentIndicatorEntry,
  AgentFrameEntry,
  InsertionIndicator,
  ContainerHighlight,
  IconLookupFn,
} from './engine.js';

// Codegen
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
} from './codegen.js';
export { FRAMEWORKS } from './codegen.js';
