import type {
  PenFill,
  PenStroke,
  PenEffect,
  StyledTextSegment,
} from './styles'
import type { VariableDefinition } from './variables'

// --- Document Root ---

export interface PenDocument {
  version: string
  name?: string
  themes?: Record<string, string[]>
  variables?: Record<string, VariableDefinition>
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
  | 'ref'

export type SizingBehavior = number | 'fit_content' | 'fill_container' | string

// --- Base ---

export interface PenNodeBase {
  id: string
  type: PenNodeType
  name?: string
  x?: number
  y?: number
  rotation?: number
  opacity?: number | string // number or $variable
  enabled?: boolean | string
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
  | RefNode
