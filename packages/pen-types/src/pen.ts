import type { PenFill, PenStroke, PenEffect, StyledTextSegment } from './styles.js';
import type { VariableDefinition } from './variables.js';
import type { DesignMdSpec } from './design-md.js';

// --- Page ---

export interface PenPage {
  id: string;
  name: string;
  children: PenNode[];
}

// --- Document Root ---

export interface PenDocument {
  version: string;
  name?: string;
  themes?: Record<string, string[]>;
  variables?: Record<string, VariableDefinition>;
  /**
   * Design system specification attached to this document. Lives on the
   * document (not in a side store) so it serializes with `.pen`/`.op`
   * files and travels between sessions/users without leaking across
   * documents (see `openpencil-docs/…/plans/2026-04-20-design-md-per-document.md`).
   */
  designMd?: DesignMdSpec;
  pages?: PenPage[];
  children: PenNode[];
}

// --- Node Types ---

export type PenNodeType =
  | 'frame'
  | 'group'
  | 'rectangle'
  | 'ellipse'
  | 'line'
  | 'polygon'
  | 'path'
  | 'text'
  | 'image'
  | 'icon_font'
  | 'text_input'
  | 'text_area'
  | 'number_input'
  | 'select'
  | 'radio_group'
  | 'switch'
  | 'checkbox'
  | 'slider'
  | 'progress'
  | 'tabs'
  | 'ref';

export type SizingBehavior = number | 'fit_content' | 'fill_container' | string;

// --- Base ---

export interface PenNodeBase {
  id: string;
  type: PenNodeType;
  name?: string;
  role?: string; // semantic role for AI generation ("button", "card", "heading", etc.)
  explain?: string; // explanatory semantic layer for the AI consumer view
  x?: number;
  y?: number;
  rotation?: number;
  opacity?: number | string; // number or $variable
  enabled?: boolean | string;
  visible?: boolean; // default true
  locked?: boolean; // default false
  flipX?: boolean;
  flipY?: boolean;
  theme?: Record<string, string>;
}

// --- Container (shared layout props) ---

export interface ContainerProps {
  width?: SizingBehavior;
  height?: SizingBehavior;
  layout?: 'none' | 'vertical' | 'horizontal';
  gap?: number | string;
  padding?: number | [number, number] | [number, number, number, number] | string;
  justifyContent?: 'start' | 'center' | 'end' | 'space_between' | 'space_around';
  alignItems?: 'start' | 'center' | 'end';
  clipContent?: boolean;
  children?: PenNode[];
  cornerRadius?: number | [number, number, number, number];
  fill?: PenFill[];
  stroke?: PenStroke;
  effects?: PenEffect[];
}

// --- Concrete Nodes ---

export interface FrameNode extends PenNodeBase, ContainerProps {
  type: 'frame';
  reusable?: boolean;
  slot?: string[];
}

export interface GroupNode extends PenNodeBase, ContainerProps {
  type: 'group';
}

export interface RectangleNode extends PenNodeBase, ContainerProps {
  type: 'rectangle';
}

export interface EllipseNode extends PenNodeBase {
  type: 'ellipse';
  width?: SizingBehavior;
  height?: SizingBehavior;
  cornerRadius?: number;
  innerRadius?: number;
  startAngle?: number;
  sweepAngle?: number;
  fill?: PenFill[];
  stroke?: PenStroke;
  effects?: PenEffect[];
}

export interface LineNode extends PenNodeBase {
  type: 'line';
  x2?: number;
  y2?: number;
  stroke?: PenStroke;
  effects?: PenEffect[];
}

export interface PolygonNode extends PenNodeBase {
  type: 'polygon';
  polygonCount: number;
  width?: SizingBehavior;
  height?: SizingBehavior;
  cornerRadius?: number;
  fill?: PenFill[];
  stroke?: PenStroke;
  effects?: PenEffect[];
}

export interface PenPathHandle {
  x: number;
  y: number;
}

export type PenPathPointType = 'corner' | 'mirrored' | 'independent';

export interface PenPathAnchor {
  x: number;
  y: number;
  handleIn: PenPathHandle | null;
  handleOut: PenPathHandle | null;
  pointType?: PenPathPointType;
}

export interface PathNode extends PenNodeBase {
  type: 'path';
  iconId?: string; // Iconify icon ID, e.g. "lucide:home"
  d: string;
  anchors?: PenPathAnchor[];
  closed?: boolean;
  width?: SizingBehavior;
  height?: SizingBehavior;
  fill?: PenFill[];
  stroke?: PenStroke;
  effects?: PenEffect[];
}

export interface TextNode extends PenNodeBase {
  type: 'text';
  width?: SizingBehavior;
  height?: SizingBehavior;
  content: string | StyledTextSegment[];
  fontFamily?: string;
  fontSize?: number;
  fontWeight?: number | string;
  fontStyle?: 'normal' | 'italic';
  letterSpacing?: number;
  lineHeight?: number;
  textAlign?: 'left' | 'center' | 'right' | 'justify';
  textAlignVertical?: 'top' | 'middle' | 'bottom';
  textGrowth?: 'auto' | 'fixed-width' | 'fixed-width-height';
  underline?: boolean;
  strikethrough?: boolean;
  fill?: PenFill[];
  effects?: PenEffect[];
}

export type ImageFitMode = 'fill' | 'fit' | 'crop' | 'tile';

export interface ImageNode extends PenNodeBase {
  type: 'image';
  src: string;
  objectFit?: ImageFitMode;
  width?: SizingBehavior;
  height?: SizingBehavior;
  cornerRadius?: number | [number, number, number, number];
  effects?: PenEffect[];
  exposure?: number; // -100 to 100
  contrast?: number; // -100 to 100
  saturation?: number; // -100 to 100
  temperature?: number; // -100 to 100
  tint?: number; // -100 to 100
  highlights?: number; // -100 to 100
  shadows?: number; // -100 to 100
  imagePrompt?: string; // Descriptive prompt for AI image generation (long)
  imageSearchQuery?: string; // Short keywords for image search (e.g. "burger fries")
}

export interface IconFontNode extends PenNodeBase {
  type: 'icon_font';
  iconFontName: string;
  iconFontFamily?: string;
  width?: SizingBehavior;
  height?: SizingBehavior;
  fill?: PenFill[];
  stroke?: PenStroke;
}

// --- Interactive widget nodes (first-class, runtime-interactive in jian) ---

/** Option entry shared by select, radio_group and tabs. */
export interface SelectOption {
  value: string;
  label: string;
}

/**
 * Authored per-state style overrides for the auto-derived interaction states.
 * Loosely typed on the TS side (the jian runtime owns the strict schema);
 * `Record` keeps the web/MCP product line permissive without duplicating the
 * runtime's state graph here.
 */
export type WidgetStates = Record<string, Record<string, unknown>>;

/** Shared optional fields carried by every interactive widget node. */
export interface WidgetNodeProps {
  width?: SizingBehavior;
  height?: SizingBehavior;
  fill?: PenFill[];
  stroke?: PenStroke;
  effects?: PenEffect[];
  cornerRadius?: number | [number, number, number, number];
  states?: WidgetStates;
  bindings?: Record<string, string>;
  events?: Record<string, unknown>;
}

export interface TextInputNode extends PenNodeBase, WidgetNodeProps {
  type: 'text_input';
  placeholder?: string;
  value?: string;
}

export interface TextAreaNode extends PenNodeBase, WidgetNodeProps {
  type: 'text_area';
  placeholder?: string;
  value?: string;
  maxVisibleLines?: number;
}

export interface NumberInputNode extends PenNodeBase, WidgetNodeProps {
  type: 'number_input';
  placeholder?: string;
  value?: number | string;
  min?: number;
  max?: number;
  step?: number;
}

export interface SelectNode extends PenNodeBase, WidgetNodeProps {
  type: 'select';
  placeholder?: string;
  value?: string;
  options?: SelectOption[];
}

export interface RadioGroupNode extends PenNodeBase, WidgetNodeProps {
  type: 'radio_group';
  value?: string;
  options?: SelectOption[];
}

export interface SwitchNode extends PenNodeBase, WidgetNodeProps {
  type: 'switch';
  checked?: boolean | string;
}

export interface CheckboxNode extends PenNodeBase, WidgetNodeProps {
  type: 'checkbox';
  checked?: boolean | string;
  label?: string;
}

export interface SliderNode extends PenNodeBase, WidgetNodeProps {
  type: 'slider';
  min?: number;
  max?: number;
  step?: number;
  value?: number | string;
}

export interface ProgressNode extends PenNodeBase, WidgetNodeProps {
  type: 'progress';
  value?: number | string;
  max?: number;
  indeterminate?: boolean;
}

export interface TabsNode extends PenNodeBase, WidgetNodeProps {
  type: 'tabs';
  tabs?: SelectOption[];
  value?: string;
  children?: PenNode[];
}

export interface RefNode extends PenNodeBase {
  type: 'ref';
  ref: string;
  descendants?: Record<string, Partial<PenNode>>;
  children?: PenNode[];
}

// --- Union ---

export type PenNode =
  | FrameNode
  | GroupNode
  | RectangleNode
  | EllipseNode
  | LineNode
  | PolygonNode
  | PathNode
  | TextNode
  | ImageNode
  | IconFontNode
  | TextInputNode
  | TextAreaNode
  | NumberInputNode
  | SelectNode
  | RadioGroupNode
  | SwitchNode
  | CheckboxNode
  | SliderNode
  | ProgressNode
  | TabsNode
  | RefNode;
