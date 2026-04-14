import type { PenNode } from '@/types/pen'
import type { ImageFill, ImageOriginalSize, ImageTransform, PenEffect, PenFill } from '@/types/styles'

const DATA_URL_PREFIX = 'data:'
const IMAGE_TRANSFORM_EPSILON = 0.000001

function decodeBase64(base64: string): Uint8Array {
  if (typeof Buffer !== 'undefined') {
    return Uint8Array.from(Buffer.from(base64, 'base64'))
  }

  const binary = atob(base64)
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i)
  }
  return bytes
}

function parseDataUrl(url: string): { mimeType: string; bytes: Uint8Array } | null {
  if (!url.startsWith(DATA_URL_PREFIX)) return null

  const match = url.match(/^data:([^;,]+);base64,([\s\S]+)$/)
  if (!match) return null

  const [, mimeType, base64] = match
  return {
    mimeType,
    bytes: decodeBase64(base64),
  }
}

function readBigEndianUint32(bytes: Uint8Array, offset: number): number {
  return (
    (bytes[offset] << 24)
    | (bytes[offset + 1] << 16)
    | (bytes[offset + 2] << 8)
    | bytes[offset + 3]
  ) >>> 0
}

function readBigEndianUint16(bytes: Uint8Array, offset: number): number {
  return (bytes[offset] << 8) | bytes[offset + 1]
}

function readLittleEndianUint16(bytes: Uint8Array, offset: number): number {
  return bytes[offset] | (bytes[offset + 1] << 8)
}

function normalizeImageDimension(value: number): number | null {
  if (!Number.isFinite(value) || value <= 0) return null
  const rounded = Math.round(value)
  if (Math.abs(value - rounded) < 0.001) return rounded
  return Number(value.toFixed(3))
}

function normalizeOriginalSize(width: number, height: number): ImageOriginalSize | undefined {
  const normalizedWidth = normalizeImageDimension(width)
  const normalizedHeight = normalizeImageDimension(height)
  if (!normalizedWidth || !normalizedHeight) return undefined
  return {
    width: normalizedWidth,
    height: normalizedHeight,
  }
}

function isValidOriginalSize(size: ImageOriginalSize | undefined): size is ImageOriginalSize {
  return Boolean(
    size
    && Number.isFinite(size.width)
    && size.width > 0
    && Number.isFinite(size.height)
    && size.height > 0,
  )
}

function tryReadPngSize(bytes: Uint8Array): ImageOriginalSize | undefined {
  if (bytes.length < 24) return undefined
  const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]
  if (!signature.every((value, index) => bytes[index] === value)) return undefined
  return normalizeOriginalSize(
    readBigEndianUint32(bytes, 16),
    readBigEndianUint32(bytes, 20),
  )
}

function tryReadGifSize(bytes: Uint8Array): ImageOriginalSize | undefined {
  if (bytes.length < 10) return undefined
  const header = String.fromCharCode(...bytes.slice(0, 6))
  if (header !== 'GIF87a' && header !== 'GIF89a') return undefined
  return normalizeOriginalSize(
    readLittleEndianUint16(bytes, 6),
    readLittleEndianUint16(bytes, 8),
  )
}

function tryReadJpegSize(bytes: Uint8Array): ImageOriginalSize | undefined {
  if (bytes.length < 4 || bytes[0] !== 0xff || bytes[1] !== 0xd8) return undefined

  let offset = 2
  while (offset + 8 < bytes.length) {
    if (bytes[offset] !== 0xff) {
      offset += 1
      continue
    }

    while (offset < bytes.length && bytes[offset] === 0xff) offset += 1
    if (offset >= bytes.length) return undefined

    const marker = bytes[offset]
    offset += 1

    if (marker === 0xd8 || marker === 0xd9 || (marker >= 0xd0 && marker <= 0xd7)) {
      continue
    }

    if (offset + 1 >= bytes.length) return undefined
    const segmentLength = readBigEndianUint16(bytes, offset)
    if (segmentLength < 2 || offset + segmentLength > bytes.length) return undefined

    const isStartOfFrame = [
      0xc0, 0xc1, 0xc2, 0xc3,
      0xc5, 0xc6, 0xc7,
      0xc9, 0xca, 0xcb,
      0xcd, 0xce, 0xcf,
    ].includes(marker)

    if (isStartOfFrame) {
      if (segmentLength < 7) return undefined
      return normalizeOriginalSize(
        readBigEndianUint16(bytes, offset + 5),
        readBigEndianUint16(bytes, offset + 3),
      )
    }

    offset += segmentLength
  }

  return undefined
}

function inferImageSizeFromDataUrl(url: string): ImageOriginalSize | undefined {
  const parsed = parseDataUrl(url)
  if (!parsed) return undefined

  return (
    tryReadPngSize(parsed.bytes)
    ?? tryReadGifSize(parsed.bytes)
    ?? tryReadJpegSize(parsed.bytes)
  )
}

function inferOriginalSizeFromTransform(
  node: PenNode,
  transform: ImageTransform | undefined,
): ImageOriginalSize | undefined {
  if (!transform) return undefined
  const measurableNode = node as PenNode & { width?: number | string; height?: number | string }
  if (typeof measurableNode.width !== 'number' || typeof measurableNode.height !== 'number') return undefined

  if (
    Math.abs(transform.m01) > IMAGE_TRANSFORM_EPSILON
    || Math.abs(transform.m10) > IMAGE_TRANSFORM_EPSILON
    || Math.abs(transform.m00) < IMAGE_TRANSFORM_EPSILON
    || Math.abs(transform.m11) < IMAGE_TRANSFORM_EPSILON
  ) {
    return undefined
  }

  return normalizeOriginalSize(
    Math.abs(measurableNode.width / transform.m00),
    Math.abs(measurableNode.height / transform.m11),
  )
}

function inferImageFillOriginalSize(node: PenNode, fill: ImageFill): ImageOriginalSize | undefined {
  if (isValidOriginalSize(fill.originalSize)) return fill.originalSize

  return (
    inferImageSizeFromDataUrl(fill.url)
    ?? inferOriginalSizeFromTransform(node, fill.transform)
  )
}

function buildImageFillExplain(fill: ImageFill): string | undefined {
  if (typeof fill.explain === 'string' && fill.explain.trim().length > 0) {
    return fill.explain
  }
  if (!fill.transform) return undefined

  const mode = fill.mode ?? 'fill'
  return `这不是整图 ${mode}, 而是先裁剪原图后再映射到目标框`
}

export function enrichImageFillForAIConsumerView(node: PenNode, fill: ImageFill): ImageFill {
  const originalSize = inferImageFillOriginalSize(node, fill)
  const explain = buildImageFillExplain(fill)

  return {
    ...fill,
    ...(originalSize ? { originalSize } : {}),
    ...(explain ? { explain } : {}),
  }
}

function buildLinearGradientExplain(fill: Extract<PenFill, { type: 'linear_gradient' }>): string | undefined {
  if (typeof fill.explain === 'string' && fill.explain.trim().length > 0) return fill.explain
  const stopCount = Array.isArray(fill.stops) ? fill.stops.length : 0
  const angle = typeof fill.angle === 'number' ? Math.round(fill.angle * 100) / 100 : 0
  return `这是线性渐变填充, 角度 ${angle}deg, 共 ${stopCount} 个色标, 表示颜色会沿该方向平滑过渡`
}

function buildRadialGradientExplain(fill: Extract<PenFill, { type: 'radial_gradient' }>): string | undefined {
  if (typeof fill.explain === 'string' && fill.explain.trim().length > 0) return fill.explain
  const stopCount = Array.isArray(fill.stops) ? fill.stops.length : 0
  const cx = typeof fill.cx === 'number' ? Math.round(fill.cx * 100) : 50
  const cy = typeof fill.cy === 'number' ? Math.round(fill.cy * 100) : 50
  const radius = typeof fill.radius === 'number' ? Math.round(fill.radius * 100) : 50
  return `这是径向渐变填充, 中心约在 ${cx}% ${cy}%, 半径约 ${radius}%, 共 ${stopCount} 个色标`
}

function enrichFillForAIConsumerView(node: PenNode, fill: PenFill): PenFill {
  if (fill.type === 'image') return enrichImageFillForAIConsumerView(node, fill)
  if (fill.type === 'linear_gradient') {
    const explain = buildLinearGradientExplain(fill)
    return explain ? { ...fill, explain } : fill
  }
  if (fill.type === 'radial_gradient') {
    const explain = buildRadialGradientExplain(fill)
    return explain ? { ...fill, explain } : fill
  }
  return fill
}

function buildImageNodeExplain(node: PenNode): string | undefined {
  if (node.type !== 'image') return undefined
  const fit = node.objectFit ?? 'fill'
  switch (fit) {
    case 'fit':
      return '这是图像节点, objectFit=fit 表示完整显示整张图, 允许出现留白'
    case 'crop':
      return '这是图像节点, objectFit=crop 表示按 cover 铺满容器, 可能裁掉边缘'
    case 'tile':
      return '这是图像节点, objectFit=tile 表示用原图重复平铺容器'
    default:
      return '这是图像节点, objectFit=fill 表示拉伸图片以填满容器'
  }
}

function buildTextNodeExplain(node: PenNode): string | undefined {
  if (node.type !== 'text') return undefined

  const parts: string[] = []

  if (node.textGrowth === 'auto') {
    parts.push('这是文本节点, textGrowth=auto 表示文本更偏向单行自然展开, 一般不会主动按固定宽度换行')
  } else if (node.textGrowth === 'fixed-width') {
    parts.push('这是文本节点, textGrowth=fixed-width 表示文本会按当前宽度换行, 高度随内容自动增长')
  } else if (node.textGrowth === 'fixed-width-height') {
    parts.push('这是文本节点, textGrowth=fixed-width-height 表示文本在固定宽高框内排版, 内容过多时可能被裁切')
  }

  if (typeof node.lineHeight === 'number') {
    parts.push(`行高倍率为 ${Math.round(node.lineHeight * 1000) / 1000}`)
  }

  if (node.textAlign && node.textAlign !== 'left') {
    parts.push(`水平对齐方式为 ${describeTextAlign(node.textAlign)}`)
  }

  if (node.textAlignVertical && node.textAlignVertical !== 'top') {
    parts.push(`垂直对齐方式为 ${describeTextAlignVertical(node.textAlignVertical)}`)
  }

  if (parts.length === 0) return undefined
  return parts.join('。')
}

function buildLayoutExplain(node: PenNode): string | undefined {
  const containerNode = node as PenNode & {
    layout?: 'none' | 'vertical' | 'horizontal'
    gap?: number | string
    padding?: number | [number, number] | [number, number, number, number] | string
    justifyContent?: string
    alignItems?: string
  }
  if (containerNode.layout !== 'vertical' && containerNode.layout !== 'horizontal') return undefined

  const layoutLabel = containerNode.layout === 'vertical' ? '纵向' : '横向'
  const parts = [
    `这是一个${layoutLabel} auto-layout 容器`,
  ]

  if (containerNode.gap !== undefined) parts.push(`子元素间距为 ${String(containerNode.gap)}`)
  if (containerNode.padding !== undefined) parts.push(`容器内边距为 ${formatPadding(containerNode.padding)}`)
  if (containerNode.justifyContent) parts.push(`主轴对齐方式为 ${describeFlexAlign(containerNode.justifyContent)}`)
  if (containerNode.alignItems) parts.push(`交叉轴对齐方式为 ${describeFlexAlign(containerNode.alignItems)}`)

  return parts.join(', ')
}

function buildClipExplain(node: PenNode): string | undefined {
  const containerNode = node as PenNode & { clipContent?: boolean }
  if (containerNode.clipContent !== true) return undefined
  return '该容器会裁剪超出自身边界的子元素'
}

function parseSizingHint(value: string): { kind: string; hint?: string } {
  const match = value.match(/^([a-z_]+)\(([^)]+)\)$/)
  if (!match) return { kind: value }
  return {
    kind: match[1],
    hint: match[2]?.trim(),
  }
}

function describeSizingValue(
  axis: 'width' | 'height',
  value: number | string | undefined,
): string | undefined {
  if (typeof value === 'number') {
    return `${axis === 'width' ? '宽度' : '高度'}固定为 ${value}px`
  }

  if (typeof value !== 'string' || value.length === 0) return undefined
  const { kind, hint } = parseSizingHint(value)

  if (kind === 'fill_container') {
    const base = `${axis === 'width' ? '宽度' : '高度'}会跟随父容器可用空间拉伸`
    return hint ? `${base}, 提示值约 ${hint}px` : base
  }

  if (kind === 'fit_content') {
    const base = `${axis === 'width' ? '宽度' : '高度'}会由内容自动撑开`
    return hint ? `${base}, 提示值约 ${hint}px` : base
  }

  return `${axis === 'width' ? '宽度' : '高度'}使用表达式 ${value}`
}

function buildSizingExplain(node: PenNode): string | undefined {
  const measurableNode = node as PenNode & { width?: number | string; height?: number | string }
  const parts = [
    describeSizingValue('width', measurableNode.width),
    describeSizingValue('height', measurableNode.height),
  ].filter((part): part is string => Boolean(part))

  if (parts.length === 0) return undefined
  return parts.join(', ')
}

function isVariableRef(value: unknown): value is string {
  return typeof value === 'string' && value.startsWith('$') && value.length > 1
}

function collectVariableRefHints(node: PenNode): string[] {
  const hints = new Set<string>()

  if (isVariableRef(node.opacity)) hints.add(`opacity 使用设计变量 ${node.opacity}`)

  const fillNode = node as PenNode & { fill?: PenFill[] }
  if (Array.isArray(fillNode.fill)) {
    for (const fill of fillNode.fill) {
      if (fill.type === 'solid' && isVariableRef(fill.color)) {
        hints.add(`填充颜色使用设计变量 ${fill.color}`)
      }
      if ((fill.type === 'linear_gradient' || fill.type === 'radial_gradient') && Array.isArray(fill.stops)) {
        for (const stop of fill.stops) {
          if (isVariableRef(stop.color)) {
            hints.add(`渐变色标使用设计变量 ${stop.color}`)
          }
        }
      }
    }
  }

  const strokeNode = node as PenNode & {
    stroke?: {
      thickness?: number | string | [number, number, number, number]
      fill?: PenFill[]
    }
  }
  if (strokeNode.stroke) {
    if (isVariableRef(strokeNode.stroke.thickness)) {
      hints.add(`描边粗细使用设计变量 ${strokeNode.stroke.thickness}`)
    }
    if (Array.isArray(strokeNode.stroke.fill)) {
      for (const fill of strokeNode.stroke.fill) {
        if (fill.type === 'solid' && isVariableRef(fill.color)) {
          hints.add(`描边颜色使用设计变量 ${fill.color}`)
        }
      }
    }
  }

  const effectNode = node as PenNode & { effects?: PenEffect[] }
  if (Array.isArray(effectNode.effects)) {
    for (const effect of effectNode.effects) {
      if (effect.type === 'shadow') {
        if (isVariableRef(effect.color)) hints.add(`阴影颜色使用设计变量 ${effect.color}`)
        if (isVariableRef(effect.blur)) hints.add(`阴影模糊半径使用设计变量 ${effect.blur}`)
        if (isVariableRef(effect.offsetX)) hints.add(`阴影水平偏移使用设计变量 ${effect.offsetX}`)
        if (isVariableRef(effect.offsetY)) hints.add(`阴影垂直偏移使用设计变量 ${effect.offsetY}`)
        if (isVariableRef(effect.spread)) hints.add(`阴影扩散使用设计变量 ${effect.spread}`)
      }
      if ((effect.type === 'blur' || effect.type === 'background_blur') && isVariableRef(effect.radius)) {
        hints.add(`${effect.type === 'blur' ? '模糊半径' : '背景模糊半径'}使用设计变量 ${effect.radius}`)
      }
    }
  }

  return Array.from(hints)
}

function buildVariableExplain(node: PenNode): string | undefined {
  const hints = collectVariableRefHints(node)
  if (hints.length === 0) return undefined
  return `${hints.join('。')}。这些值来自设计系统变量, 不是普通硬编码常量`
}

function buildThemeExplain(node: PenNode): string | undefined {
  if (!node.theme || Object.keys(node.theme).length === 0) return undefined
  const pairs = Object.entries(node.theme).map(([axis, value]) => `${axis}=${value}`)
  return `该节点带有主题覆写上下文: ${pairs.join(', ')}`
}

function buildEffectsExplain(node: PenNode): string | undefined {
  const effectNode = node as PenNode & { effects?: PenEffect[] }
  if (!Array.isArray(effectNode.effects) || effectNode.effects.length === 0) return undefined

  const parts: string[] = []
  for (const effect of effectNode.effects) {
    if (effect.type === 'shadow') {
      const shadowKind = effect.inner ? '内阴影' : '投影'
      const usesVariable =
        isVariableRef(effect.offsetX)
        || isVariableRef(effect.offsetY)
        || isVariableRef(effect.blur)
        || isVariableRef(effect.spread)
        || isVariableRef(effect.color)

      if (usesVariable) {
        parts.push(`带有${shadowKind}效果`)
      } else {
        parts.push(
          `带有${shadowKind}, 偏移 ${effect.offsetX}px ${effect.offsetY}px, 模糊 ${effect.blur}px, 扩散 ${effect.spread}px`,
        )
      }
      continue
    }

    if (effect.type === 'blur') {
      parts.push(
        isVariableRef(effect.radius)
          ? '带有前景模糊效果'
          : `带有前景模糊, 半径 ${effect.radius}px`,
      )
      continue
    }

    if (effect.type === 'background_blur') {
      parts.push(
        isVariableRef(effect.radius)
          ? '带有背景模糊效果'
          : `带有背景模糊, 半径 ${effect.radius}px`,
      )
    }
  }

  if (parts.length === 0) return undefined
  return parts.join('。')
}

function buildReusableExplain(node: PenNode): string | undefined {
  if (node.type !== 'frame') return undefined
  const frameNode = node as PenNode & { reusable?: boolean; slot?: string[] }

  const parts: string[] = []
  if (frameNode.reusable === true) {
    parts.push('这是一个可复用组件定义节点, 其他实例可以引用它')
  }
  if (Array.isArray(frameNode.slot) && frameNode.slot.length > 0) {
    parts.push(`它声明了可插槽区域: ${frameNode.slot.join(', ')}`)
  }

  if (parts.length === 0) return undefined
  return parts.join('。')
}

function buildRefExplain(node: PenNode): string | undefined {
  if (node.type !== 'ref') return undefined
  const refNode = node as PenNode & {
    ref: string
    descendants?: Record<string, Partial<PenNode>>
  }

  const overrideCount = refNode.descendants ? Object.keys(refNode.descendants).length : 0
  const parts = [`这是一个组件实例节点, 引用源节点 ${refNode.ref}`]
  if (overrideCount > 0) {
    parts.push(`当前实例对 ${overrideCount} 个后代节点带有覆写`)
  }
  return parts.join('。')
}

function appendExplain(baseExplain: string | undefined, extraExplain: string | undefined): string | undefined {
  const segments = [baseExplain?.trim(), extraExplain?.trim()].filter(
    (segment): segment is string => Boolean(segment && segment.length > 0),
  )
  if (segments.length === 0) return undefined
  return Array.from(new Set(segments)).join('。')
}

function formatPadding(
  padding: number | [number, number] | [number, number, number, number] | string,
): string {
  if (!Array.isArray(padding)) return String(padding)
  if (padding.length === 2) return `${padding[0]} ${padding[1]}`
  if (padding.length === 4) return `${padding[0]} ${padding[1]} ${padding[2]} ${padding[3]}`
  return String(padding)
}

function describeFlexAlign(value: string): string {
  switch (value) {
    case 'start':
      return '起点对齐'
    case 'center':
      return '居中对齐'
    case 'end':
      return '终点对齐'
    case 'space_between':
      return '两端分布'
    case 'space_around':
      return '环绕分布'
    default:
      return value
  }
}

function describeTextAlign(value: string): string {
  switch (value) {
    case 'center':
      return '居中'
    case 'right':
      return '右对齐'
    case 'justify':
      return '两端对齐'
    default:
      return value
  }
}

function describeTextAlignVertical(value: string): string {
  switch (value) {
    case 'middle':
      return '垂直居中'
    case 'bottom':
      return '底部对齐'
    default:
      return value
  }
}

export function enrichNodeLocallyForAIConsumerView(node: PenNode): PenNode {
  const nextNode = { ...node } as PenNode

  if ('fill' in nextNode && Array.isArray(nextNode.fill) && nextNode.fill.length > 0) {
    nextNode.fill = nextNode.fill.map((fill: PenFill) => enrichFillForAIConsumerView(nextNode, fill))
  }

  const baseExplain = typeof nextNode.explain === 'string' ? nextNode.explain : undefined
  const withImageExplain = appendExplain(baseExplain, buildImageNodeExplain(nextNode))
  const withTextExplain = appendExplain(withImageExplain, buildTextNodeExplain(nextNode))
  const withLayoutExplain = appendExplain(withTextExplain, buildLayoutExplain(nextNode))
  const withSizingExplain = appendExplain(withLayoutExplain, buildSizingExplain(nextNode))
  const withClipExplain = appendExplain(withSizingExplain, buildClipExplain(nextNode))
  const withEffectsExplain = appendExplain(withClipExplain, buildEffectsExplain(nextNode))
  const withReusableExplain = appendExplain(withEffectsExplain, buildReusableExplain(nextNode))
  const withRefExplain = appendExplain(withReusableExplain, buildRefExplain(nextNode))
  const withVariableExplain = appendExplain(withRefExplain, buildVariableExplain(nextNode))
  const finalExplain = appendExplain(withVariableExplain, buildThemeExplain(nextNode))
  if (finalExplain) nextNode.explain = finalExplain

  return nextNode
}

/**
 * Add lightweight semantic enrichment to the canonical AI consumer view.
 *
 * Design principles:
 * - do not change node topology
 * - do not introduce runtime noise
 * - only add explanation fields for details the model cannot infer directly
 *   but that still affect reconstruction quality
 *
 * Current coverage includes image / gradient / sizing / layout / clip / effects /
 * reusable/ref. Future text-layout or variable semantics should extend this layer.
 */
export function enrichNodeForAIConsumerView(node: PenNode): PenNode {
  const nextNode = enrichNodeLocallyForAIConsumerView(node)

  if ('children' in nextNode && Array.isArray(nextNode.children) && nextNode.children.length > 0) {
    nextNode.children = nextNode.children.map(enrichNodeForAIConsumerView)
  }

  return nextNode
}
