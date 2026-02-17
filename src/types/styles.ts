export type BlendMode =
  | 'normal'
  | 'darken'
  | 'multiply'
  | 'screen'
  | 'overlay'
  | 'lighten'
  | 'difference'
  | 'hue'
  | 'saturation'
  | 'color'
  | 'luminosity'

// --- Fill Types ---

export interface SolidFill {
  type: 'solid'
  color: string // #RRGGBB or #RRGGBBAA
  opacity?: number
  blendMode?: BlendMode
}

export interface GradientStop {
  offset: number // 0 to 1
  color: string
}

export interface LinearGradientFill {
  type: 'linear_gradient'
  angle?: number
  stops: GradientStop[]
  opacity?: number
  blendMode?: BlendMode
}

export interface RadialGradientFill {
  type: 'radial_gradient'
  cx?: number
  cy?: number
  radius?: number
  stops: GradientStop[]
  opacity?: number
  blendMode?: BlendMode
}

export interface ImageFill {
  type: 'image'
  url: string
  mode?: 'stretch' | 'fill' | 'fit'
  opacity?: number
}

export type PenFill =
  | SolidFill
  | LinearGradientFill
  | RadialGradientFill
  | ImageFill

// --- Stroke ---

export interface PenStroke {
  thickness: number | [number, number, number, number]
  align?: 'inside' | 'center' | 'outside'
  join?: 'miter' | 'bevel' | 'round'
  cap?: 'none' | 'round' | 'square'
  dashPattern?: number[]
  fill?: PenFill[]
}

// --- Effects ---

export interface BlurEffect {
  type: 'blur' | 'background_blur'
  radius: number
}

export interface ShadowEffect {
  type: 'shadow'
  inner?: boolean
  offsetX: number
  offsetY: number
  blur: number
  spread: number
  color: string
}

export type PenEffect = BlurEffect | ShadowEffect

// --- Text ---

export interface StyledTextSegment {
  text: string
  fontFamily?: string
  fontSize?: number
  fontWeight?: number
  fontStyle?: 'normal' | 'italic'
  fill?: string
  underline?: boolean
  strikethrough?: boolean
  href?: string
}
