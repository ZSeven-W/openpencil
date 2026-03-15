import type {
  PenFill,
  PenStroke,
  PenEffect,
  StyledTextSegment,
} from './styles'
import type { VariableDefinition } from './variables'

// --- Page ---

export interface PenPage {
  id: string
  name: string
  children: PenNode[]
}

// --- Document Root ---

export interface PenDocument {
  version: string
  name?: string
  themes?: Record<string, string[]>
  variables?: Record<string, VariableDefinition>
  pages?: PenPage[]
  children: PenNode[]
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
  | 'ref'

export type SizingBehavior = number | 'fit_content' | 'fill_container' | string

// --- Base ---

export interface PenNodeBase {
  id: string
  type: PenNodeType
  name?: string
  role?: string // semantic role for AI generation ("button", "card", "heading", etc.)
  x?: number
  y?: number
  rotation?: number
  opacity?: number | string // number or $variable
  enabled?: boolean | string
  visible?: boolean // default true
  locked?: boolean // default false
  flipX?: boolean
  flipY?: boolean
  theme?: Record<string, string>
}

// --- Container (shared layout props) ---

export interface ContainerProps {
  width?: SizingBehavior
  height?: SizingBehavior
  layout?: 'none' | 'vertical' | 'horizontal'
  gap?: number | string
  padding?:
    | number
    | [number, number]
    | [number, number, number, number]
    | string
  justifyContent?:
    | 'start'
    | 'center'
    | 'end'
    | 'space_between'
    | 'space_around'
  alignItems?: 'start' | 'center' | 'end'
  clipContent?: boolean
  children?: PenNode[]
  cornerRadius?: number | [number, number, number, number]
  fill?: PenFill[]
  stroke?: PenStroke
  effects?: PenEffect[]
}

// --- Concrete Nodes ---

export interface FrameNode extends PenNodeBase, ContainerProps {
  type: 'frame'
  reusable?: boolean
  slot?: string[]
}

export interface GroupNode extends PenNodeBase, ContainerProps {
  type: 'group'
}

export interface RectangleNode extends PenNodeBase, ContainerProps {
  type: 'rectangle'
}

export interface EllipseNode extends PenNodeBase {
  type: 'ellipse'
  width?: SizingBehavior
  height?: SizingBehavior
  innerRadius?: number
  startAngle?: number
  sweepAngle?: number
  fill?: PenFill[]
  stroke?: PenStroke
  effects?: PenEffect[]
}

export interface LineNode extends PenNodeBase {
  type: 'line'
  x2?: number
  y2?: number
  stroke?: PenStroke
  effects?: PenEffect[]
}

export interface PolygonNode extends PenNodeBase {
  type: 'polygon'
  polygonCount: number
  width?: SizingBehavior
  height?: SizingBehavior
  fill?: PenFill[]
  stroke?: PenStroke
  effects?: PenEffect[]
}

export interface PathNode extends PenNodeBase {
  type: 'path'
  iconId?: string // Iconify icon ID, e.g. "lucide:home"
  d: string
  width?: SizingBehavior
  height?: SizingBehavior
  fill?: PenFill[]
  stroke?: PenStroke
  effects?: PenEffect[]
}

export interface TextNode extends PenNodeBase {
  type: 'text'
  width?: SizingBehavior
  height?: SizingBehavior
  content: string | StyledTextSegment[]
  fontFamily?: string
  fontSize?: number
  fontWeight?: number | string
  fontStyle?: 'normal' | 'italic'
  letterSpacing?: number
  lineHeight?: number
  textAlign?: 'left' | 'center' | 'right' | 'justify'
  textAlignVertical?: 'top' | 'middle' | 'bottom'
  textGrowth?: 'auto' | 'fixed-width' | 'fixed-width-height'
  underline?: boolean
  strikethrough?: boolean
  fill?: PenFill[]
  effects?: PenEffect[]
}

export type ImageFitMode = 'fill' | 'fit' | 'crop' | 'tile'

export interface ImageNode extends PenNodeBase {
  type: 'image'
  src: string
  objectFit?: ImageFitMode
  width?: SizingBehavior
  height?: SizingBehavior
  cornerRadius?: number | [number, number, number, number]
  effects?: PenEffect[]
  exposure?: number    // -100 to 100
  contrast?: number    // -100 to 100
  saturation?: number  // -100 to 100
  temperature?: number // -100 to 100
  tint?: number        // -100 to 100
  highlights?: number  // -100 to 100
  shadows?: number     // -100 to 100
}

export interface IconFontNode extends PenNodeBase {
  type: 'icon_font'
  iconFontName: string
  iconFontFamily?: string
  width?: SizingBehavior
  height?: SizingBehavior
  fill?: PenFill[]
  stroke?: PenStroke
}

export interface RefNode extends PenNodeBase {
  type: 'ref'
  ref: string
  descendants?: Record<string, Partial<PenNode>>
  children?: PenNode[]
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
  | RefNode
