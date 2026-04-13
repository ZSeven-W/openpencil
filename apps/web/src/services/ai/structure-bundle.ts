import type { PenNode } from '@/types/pen'
import { encode as encodeZip } from 'uzip'
import {
  extractCodegenAssets,
  hashBytesToSha256Hex,
  type CodegenAssetFile,
} from './codegen-assets'

export type AIStructureBundleScopeMode = 'selection' | 'page'

export interface AIStructureBundleScope {
  mode: AIStructureBundleScopeMode
  activePageId: string | null
  selectedIds: string[]
  exportedRootIds: string[]
  exportedRootCount: number
  exportedNodeCount: number
}

export interface AIStructureBundleViewFile {
  kind: 'ai-structure-view'
  version: 1
  view: 'raw' | 'sanitized'
  consumer: boolean
  nodeCount: number
  summary?: string
  highlights?: string[]
  nodes: PenNode[]
}

export interface AIStructureBundleRef {
  pointer: string
  nodeId: string
  field: 'src' | 'fill.url'
  value: string
}

export interface AIStructureBundleAssetIndex {
  id: string
  relativePath: string
  zipPath: string
  mimeType: string
  size: number
  sha256: string
  sourceNodeId: string
  sourceNodeName?: string
  sourceKind: 'image-node' | 'image-fill'
  rawRefs: AIStructureBundleRef[]
  sanitizedRefs: AIStructureBundleRef[]
}

export interface AIStructureBundleManifest {
  kind: 'ai-structure-bundle'
  version: 1
  consumerView: 'sanitized'
  generatedAt: string
  scope: AIStructureBundleScope
  views: {
    raw: {
      path: 'views/raw.json'
      nodeCount: number
      assetReferencePrefix: 'asset://'
    }
    sanitized: {
      path: 'views/sanitized.json'
      nodeCount: number
      assetBasePath: './assets/'
    }
  }
  assets: AIStructureBundleAssetIndex[]
}

export interface AIStructureBundle {
  fileName: string
  manifest: AIStructureBundleManifest
  rawView: AIStructureBundleViewFile
  sanitizedView: AIStructureBundleViewFile
  assets: CodegenAssetFile[]
  zipEntries: Record<string, Uint8Array>
}

export interface BuildAIStructureBundleOptions {
  nodes: PenNode[]
  activePageId: string | null
  selectedIds: string[]
}

const RAW_ASSET_PREFIX = 'asset://'
const RAW_VIEW_PATH = 'views/raw.json' as const
const SANITIZED_VIEW_PATH = 'views/sanitized.json' as const
const STRUCTURE_BUNDLE_FILE_NAME = 'ai-structure-bundle.zip'

interface MutableAssetIndexRecord {
  id: string
  relativePath: string
  zipPath: string
  mimeType: string
  size: number
  sha256: string
  sourceNodeId: string
  sourceNodeName?: string
  sourceKind: 'image-node' | 'image-fill'
  rawRefs: AIStructureBundleRef[]
  sanitizedRefs: AIStructureBundleRef[]
}

function serializeJson(value: unknown): Uint8Array {
  return new TextEncoder().encode(JSON.stringify(value, null, 2))
}

function countNodes(nodes: PenNode[]): number {
  let total = 0

  const visit = (node: PenNode) => {
    total += 1

    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) visit(child)
    }
  }

  for (const node of nodes) visit(node)
  return total
}

function buildSanitizedViewSummary(nodes: PenNode[]): { summary: string; highlights: string[] } {
  const signals = {
    hasVariables: false,
    hasThemeOverrides: false,
    hasReusableOrRef: false,
    hasImageTransform: false,
    hasGradients: false,
    hasLayout: false,
    hasClip: false,
    hasTextSemantics: false,
  }

  const visit = (node: PenNode) => {
    if (typeof node.explain === 'string') {
      if (node.explain.includes('设计变量')) signals.hasVariables = true
      if (node.explain.includes('主题覆写上下文')) signals.hasThemeOverrides = true
      if (node.explain.includes('可复用组件定义节点') || node.explain.includes('组件实例节点')) {
        signals.hasReusableOrRef = true
      }
      if (node.explain.includes('auto-layout')) signals.hasLayout = true
      if (node.explain.includes('裁剪超出自身边界')) signals.hasClip = true
      if (node.explain.includes('文本节点') || node.explain.includes('行高倍率')) {
        signals.hasTextSemantics = true
      }
    }

    const fillNode = node as PenNode & { fill?: Array<{ type?: string; transform?: unknown }> }
    if (Array.isArray(fillNode.fill)) {
      for (const fill of fillNode.fill) {
        if (fill.type === 'image' && fill.transform) signals.hasImageTransform = true
        if (fill.type === 'linear_gradient' || fill.type === 'radial_gradient') signals.hasGradients = true
      }
    }

    if ('children' in node && Array.isArray(node.children)) {
      for (const child of node.children) visit(child)
    }
  }

  for (const node of nodes) visit(node)

  const highlights: string[] = []
  if (signals.hasVariables) highlights.push('包含设计变量引用')
  if (signals.hasThemeOverrides) highlights.push('包含主题覆写上下文')
  if (signals.hasReusableOrRef) highlights.push('包含组件定义与实例引用关系')
  if (signals.hasImageTransform) highlights.push('包含图片裁剪/映射语义')
  if (signals.hasGradients) highlights.push('包含渐变填充语义')
  if (signals.hasLayout) highlights.push('包含 auto-layout 容器语义')
  if (signals.hasClip) highlights.push('包含裁剪容器语义')
  if (signals.hasTextSemantics) highlights.push('包含文本排版语义')
  if (highlights.length === 0) highlights.push('以基础几何与样式结构为主')

  return {
    summary: `这是给 AI 直接消费的 sanitized 结构视图, 共 ${countNodes(nodes)} 个节点。主要特征: ${highlights.join('、')}。读取具体节点前, 建议先把这些高层语义作为默认约束。`,
    highlights,
  }
}

function buildScope(options: BuildAIStructureBundleOptions): AIStructureBundleScope {
  const exportedRootIds = options.nodes.map((node) => node.id)

  return {
    mode: options.selectedIds.length > 0 ? 'selection' : 'page',
    activePageId: options.activePageId,
    selectedIds: [...options.selectedIds],
    exportedRootIds,
    exportedRootCount: exportedRootIds.length,
    exportedNodeCount: countNodes(options.nodes),
  }
}

function createAssetIndexSeed(asset: CodegenAssetFile): MutableAssetIndexRecord {
  return {
    id: asset.id,
    relativePath: asset.relativePath,
    zipPath: asset.zipPath,
    mimeType: asset.mimeType,
    size: asset.bytes.byteLength,
    sha256: '',
    sourceNodeId: asset.sourceNodeId,
    sourceNodeName: asset.sourceNodeName,
    sourceKind: asset.sourceKind,
    rawRefs: [],
    sanitizedRefs: [],
  }
}

function buildRawAssetUri(assetId: string): string {
  return `${RAW_ASSET_PREFIX}${assetId}`
}

function collectAssetRefs(options: {
  rawNodes: PenNode[]
  sanitizedNodes: PenNode[]
  assets: CodegenAssetFile[]
}): MutableAssetIndexRecord[] {
  const assetByPath = new Map(options.assets.map((asset) => [asset.relativePath, asset]))
  const records = new Map(options.assets.map((asset) => [asset.id, createAssetIndexSeed(asset)]))

  const pushRef = (
    asset: CodegenAssetFile,
    bucket: 'rawRefs' | 'sanitizedRefs',
    ref: AIStructureBundleRef,
  ) => {
    const record = records.get(asset.id)
    if (!record) return

    record[bucket].push(ref)
  }

  const visit = (rawNode: PenNode, sanitizedNode: PenNode, nodePointer: string) => {
    const nodeId = rawNode.id
    const rawNodeWithImage = rawNode as PenNode & { src?: string; fill?: Array<{ type?: string; url?: string }> }
    const sanitizedNodeWithImage = sanitizedNode as PenNode & { src?: string; fill?: Array<{ type?: string; url?: string }> }

    // ---------------------------------------------------------------------
    // 图像节点: raw 视图用 `asset://asset-id` 保留“这里原本是图片源字段”的语义,
    // sanitized 视图继续沿用当前 codegen 的 `./assets/...` 相对路径。
    // ---------------------------------------------------------------------
    if (typeof sanitizedNodeWithImage.src === 'string') {
      const asset = assetByPath.get(sanitizedNodeWithImage.src)
      if (asset) {
        const pointer = `${nodePointer}/src`
        rawNodeWithImage.src = buildRawAssetUri(asset.id)
        pushRef(asset, 'rawRefs', { pointer, nodeId, field: 'src', value: rawNodeWithImage.src })
        pushRef(asset, 'sanitizedRefs', { pointer, nodeId, field: 'src', value: sanitizedNodeWithImage.src })
      }
    }

    // ---------------------------------------------------------------------
    // 图片填充: 保留原始 fill 结构, 只把超大的 data URL 用稳定 asset:// 引用替换掉,
    // 这样 raw / sanitized 两边都能顺着同一 asset id 去追查。
    // ---------------------------------------------------------------------
    if (Array.isArray(rawNodeWithImage.fill) && Array.isArray(sanitizedNodeWithImage.fill)) {
      const fillCount = Math.min(rawNodeWithImage.fill.length, sanitizedNodeWithImage.fill.length)

      for (let index = 0; index < fillCount; index += 1) {
        const rawFill = rawNodeWithImage.fill[index]
        const sanitizedFill = sanitizedNodeWithImage.fill[index]
        if (!rawFill || !sanitizedFill || sanitizedFill.type !== 'image') continue
        if (typeof sanitizedFill.url !== 'string') continue

        const asset = assetByPath.get(sanitizedFill.url)
        if (!asset) continue

        const pointer = `${nodePointer}/fill/${index}/url`
        rawNodeWithImage.fill[index] = {
          ...rawFill,
          url: buildRawAssetUri(asset.id),
        }

        pushRef(asset, 'rawRefs', {
          pointer,
          nodeId,
          field: 'fill.url',
          value: rawNodeWithImage.fill[index]?.url ?? buildRawAssetUri(asset.id),
        })
        pushRef(asset, 'sanitizedRefs', {
          pointer,
          nodeId,
          field: 'fill.url',
          value: sanitizedFill.url,
        })
      }
    }

    if ('children' in rawNode && Array.isArray(rawNode.children) && 'children' in sanitizedNode && Array.isArray(sanitizedNode.children)) {
      const childCount = Math.min(rawNode.children.length, sanitizedNode.children.length)
      for (let index = 0; index < childCount; index += 1) {
        visit(rawNode.children[index], sanitizedNode.children[index], `${nodePointer}/children/${index}`)
      }
    }
  }

  for (let index = 0; index < options.rawNodes.length; index += 1) {
    visit(options.rawNodes[index], options.sanitizedNodes[index], `#/nodes/${index}`)
  }

  return Array.from(records.values())
}

function buildViews(options: {
  rawNodes: PenNode[]
  sanitizedNodes: PenNode[]
}): { rawView: AIStructureBundleViewFile; sanitizedView: AIStructureBundleViewFile } {
  const sanitizedSummary = buildSanitizedViewSummary(options.sanitizedNodes)

  return {
    rawView: {
      kind: 'ai-structure-view',
      version: 1,
      view: 'raw',
      consumer: false,
      nodeCount: countNodes(options.rawNodes),
      nodes: options.rawNodes,
    },
    sanitizedView: {
      kind: 'ai-structure-view',
      version: 1,
      view: 'sanitized',
      consumer: true,
      nodeCount: countNodes(options.sanitizedNodes),
      summary: sanitizedSummary.summary,
      highlights: sanitizedSummary.highlights,
      nodes: options.sanitizedNodes,
    },
  }
}

function buildManifest(options: {
  scope: AIStructureBundleScope
  rawView: AIStructureBundleViewFile
  sanitizedView: AIStructureBundleViewFile
  assets: AIStructureBundleAssetIndex[]
}): AIStructureBundleManifest {
  return {
    kind: 'ai-structure-bundle',
    version: 1,
    consumerView: 'sanitized',
    generatedAt: new Date().toISOString(),
    scope: options.scope,
    views: {
      raw: {
        path: RAW_VIEW_PATH,
        nodeCount: options.rawView.nodeCount,
        assetReferencePrefix: RAW_ASSET_PREFIX,
      },
      sanitized: {
        path: SANITIZED_VIEW_PATH,
        nodeCount: options.sanitizedView.nodeCount,
        assetBasePath: './assets/',
      },
    },
    assets: options.assets,
  }
}

function buildZipEntries(options: {
  manifest: AIStructureBundleManifest
  rawView: AIStructureBundleViewFile
  sanitizedView: AIStructureBundleViewFile
  assets: CodegenAssetFile[]
}): Record<string, Uint8Array> {
  return {
    'manifest.json': serializeJson(options.manifest),
    [RAW_VIEW_PATH]: serializeJson(options.rawView),
    [SANITIZED_VIEW_PATH]: serializeJson(options.sanitizedView),
    ...Object.fromEntries(options.assets.map((asset) => [asset.zipPath, asset.bytes])),
  }
}

export async function buildAIStructureBundle(
  options: BuildAIStructureBundleOptions,
): Promise<AIStructureBundle> {
  const scope = buildScope(options)
  const rawNodes = structuredClone(options.nodes) as PenNode[]
  const { nodes: sanitizedNodes, assets } = extractCodegenAssets(options.nodes)

  const assetIndexRecords = collectAssetRefs({
    rawNodes,
    sanitizedNodes,
    assets,
  })

  // -----------------------------------------------------------------------
  // `sha256` 需要真正对资产字节计算, 所以放在 collectAssetRefs 之后统一补齐,
  // 保证 manifest 既有引用关系, 也有可追溯的内容哈希。
  // -----------------------------------------------------------------------
  const assetIndex = await Promise.all(
    assetIndexRecords.map(async (record) => {
      const asset = assets.find((item) => item.id === record.id)
      if (!asset) return record
      return {
        ...record,
        sha256: await hashBytesToSha256Hex(asset.bytes),
      }
    }),
  )

  const { rawView, sanitizedView } = buildViews({
    rawNodes,
    sanitizedNodes,
  })
  const manifest = buildManifest({
    scope,
    rawView,
    sanitizedView,
    assets: assetIndex,
  })

  return {
    fileName: STRUCTURE_BUNDLE_FILE_NAME,
    manifest,
    rawView,
    sanitizedView,
    assets,
    zipEntries: buildZipEntries({
      manifest,
      rawView,
      sanitizedView,
      assets,
    }),
  }
}

export function encodeAIStructureBundleZip(entries: Record<string, Uint8Array>): ArrayBuffer {
  return encodeZip(entries)
}
