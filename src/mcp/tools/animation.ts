import { openDocument, saveDocument, resolveDocPath } from '../document-manager'
import { findNodeInTree, getDocChildren, flattenNodes } from '../utils/node-operations'
import { generateId } from '../utils/id'
import { getAllEffects, getEffectsByCategory, generateClipFromEffect } from '../../animation/effect-registry'
import { getAllPropertyDescriptors } from '../../animation/property-descriptors'
import type { AnimationClip, AnimationClipData, KeyframeV2, CompositionSettings } from '../../types/animation'
import type { PenNode } from '../../types/pen'

// ---------------------------------------------------------------------------
// list_effects
// ---------------------------------------------------------------------------

export interface ListEffectsParams {
  category?: string
}

export function handleListEffects(params: ListEffectsParams) {
  const effects = params.category
    ? getEffectsByCategory(params.category as any)
    : getAllEffects()

  return {
    effects: effects.map((e) => ({
      id: e.id,
      name: e.name,
      category: e.category,
      properties: e.properties,
      defaultDuration: e.defaultDuration,
      parameters: e.parameters.map((p) => ({
        key: p.key,
        type: p.type,
        default: p.default,
        label: p.label,
        options: p.options,
      })),
    })),
  }
}

// ---------------------------------------------------------------------------
// list_animatable_properties
// ---------------------------------------------------------------------------

export interface ListAnimatablePropertiesParams {
  nodeType?: string
}

export function handleListAnimatableProperties(params: ListAnimatablePropertiesParams) {
  let descriptors = getAllPropertyDescriptors()

  if (params.nodeType) {
    descriptors = descriptors.filter(
      (d) => !d.nodeTypes || d.nodeTypes.includes(params.nodeType as any),
    )
  }

  return {
    properties: descriptors.map((d) => ({
      key: d.key,
      type: d.type,
      default: d.default,
      nodeTypes: d.nodeTypes ?? null,
    })),
  }
}

// ---------------------------------------------------------------------------
// add_clip
// ---------------------------------------------------------------------------

export interface AddClipParams {
  filePath?: string
  nodeId: string
  effectId?: string
  startTime: number
  duration: number
  keyframes?: KeyframeV2[]
  params?: Record<string, unknown>
  pageId?: string
}

export async function handleAddClip(
  params: AddClipParams,
): Promise<{ clip: AnimationClipData }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const cloned = structuredClone(doc)

  const children = getDocChildren(cloned, params.pageId)
  const node = findNodeInTree(children, params.nodeId)
  if (!node) throw new Error(`Node not found: ${params.nodeId}`)

  let keyframes: KeyframeV2[]
  let duration = params.duration

  if (params.effectId) {
    const result = generateClipFromEffect(
      params.effectId,
      params.duration,
      params.params,
    )
    if (!result) throw new Error(`Effect not found: ${params.effectId}`)
    keyframes = result.keyframes
    duration = result.duration
  } else if (params.keyframes) {
    keyframes = params.keyframes
  } else {
    throw new Error('Either effectId or keyframes must be provided')
  }

  const clip: AnimationClipData = {
    id: generateId(),
    kind: 'animation',
    startTime: params.startTime,
    duration,
    keyframes,
    effectId: params.effectId,
    params: params.params,
  }

  if (!node.clips) {
    node.clips = []
  }
  node.clips.push(clip)

  await saveDocument(filePath, cloned)
  return { clip }
}

// ---------------------------------------------------------------------------
// update_clip
// ---------------------------------------------------------------------------

export interface UpdateClipParams {
  filePath?: string
  nodeId: string
  clipId: string
  startTime?: number
  duration?: number
  params?: Record<string, unknown>
  pageId?: string
}

export async function handleUpdateClip(
  params: UpdateClipParams,
): Promise<{ clip: AnimationClip }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const cloned = structuredClone(doc)

  const children = getDocChildren(cloned, params.pageId)
  const node = findNodeInTree(children, params.nodeId)
  if (!node) throw new Error(`Node not found: ${params.nodeId}`)
  if (!node.clips) throw new Error(`Node has no clips: ${params.nodeId}`)

  const clip = node.clips.find((c) => c.id === params.clipId)
  if (!clip) throw new Error(`Clip not found: ${params.clipId}`)

  if (params.startTime !== undefined) clip.startTime = params.startTime
  if (params.duration !== undefined) clip.duration = params.duration

  // Regenerate keyframes if effect params changed and clip has an effectId
  if (params.params && clip.kind === 'animation' && clip.effectId) {
    clip.params = { ...clip.params, ...params.params }
    const result = generateClipFromEffect(
      clip.effectId,
      clip.duration,
      clip.params,
    )
    if (result) {
      clip.keyframes = result.keyframes
    }
  }

  await saveDocument(filePath, cloned)
  return { clip }
}

// ---------------------------------------------------------------------------
// remove_clip
// ---------------------------------------------------------------------------

export interface RemoveClipParams {
  filePath?: string
  nodeId: string
  clipId: string
  pageId?: string
}

export async function handleRemoveClip(
  params: RemoveClipParams,
): Promise<{ ok: true }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const cloned = structuredClone(doc)

  const children = getDocChildren(cloned, params.pageId)
  const node = findNodeInTree(children, params.nodeId)
  if (!node) throw new Error(`Node not found: ${params.nodeId}`)
  if (!node.clips) throw new Error(`Node has no clips: ${params.nodeId}`)

  const idx = node.clips.findIndex((c) => c.id === params.clipId)
  if (idx === -1) throw new Error(`Clip not found: ${params.clipId}`)

  node.clips.splice(idx, 1)
  if (node.clips.length === 0) {
    delete node.clips
  }

  await saveDocument(filePath, cloned)
  return { ok: true }
}

// ---------------------------------------------------------------------------
// set_composition
// ---------------------------------------------------------------------------

export interface SetCompositionParams {
  filePath?: string
  duration?: number
  fps?: number
}

export async function handleSetComposition(
  params: SetCompositionParams,
): Promise<{ composition: CompositionSettings }> {
  const filePath = resolveDocPath(params.filePath)
  const doc = await openDocument(filePath)
  const cloned = structuredClone(doc)

  const existing = cloned.composition ?? { duration: 5000, fps: 30 }
  if (params.duration !== undefined) existing.duration = params.duration
  if (params.fps !== undefined) existing.fps = params.fps

  cloned.composition = existing
  await saveDocument(filePath, cloned)
  return { composition: existing }
}

// ---------------------------------------------------------------------------
// Animation context builder (for open_document)
// ---------------------------------------------------------------------------

export function buildAnimationContext(doc: {
  composition?: CompositionSettings
  children: PenNode[]
  pages?: { children: PenNode[] }[]
}): string {
  const allChildren = doc.pages
    ? doc.pages.flatMap((p) => p.children)
    : doc.children
  const allNodes = flattenNodes(allChildren)

  const animatedNodes = allNodes.filter((n) => n.clips && n.clips.length > 0)
  if (animatedNodes.length === 0 && !doc.composition) return ''

  const parts: string[] = ['ANIMATION:']

  if (doc.composition) {
    parts.push(`  Composition: ${doc.composition.duration}ms @ ${doc.composition.fps}fps`)
  }

  if (animatedNodes.length > 0) {
    const totalClips = animatedNodes.reduce(
      (sum, n) => sum + (n.clips?.length ?? 0),
      0,
    )
    parts.push(`  Animated nodes: ${animatedNodes.length}, Total clips: ${totalClips}`)

    // Effect usage summary
    const effectCounts = new Map<string, number>()
    for (const node of animatedNodes) {
      for (const clip of node.clips ?? []) {
        if (clip.kind === 'animation' && clip.effectId) {
          effectCounts.set(clip.effectId, (effectCounts.get(clip.effectId) ?? 0) + 1)
        }
      }
    }
    if (effectCounts.size > 0) {
      const summary = Array.from(effectCounts.entries())
        .map(([id, count]) => `${id}(${count})`)
        .join(', ')
      parts.push(`  Effects used: ${summary}`)
    }
  }

  return parts.join('\n')
}

// ---------------------------------------------------------------------------
// Animation prompt section (for design-prompt)
// ---------------------------------------------------------------------------

export function buildAnimationPromptSection(): string {
  const effects = getAllEffects()
  const properties = getAllPropertyDescriptors()

  const effectsByCategory = new Map<string, typeof effects>()
  for (const e of effects) {
    const list = effectsByCategory.get(e.category) ?? []
    list.push(e)
    effectsByCategory.set(e.category, list)
  }

  const effectLines: string[] = ['AVAILABLE EFFECTS:']
  for (const [category, categoryEffects] of effectsByCategory) {
    effectLines.push(`  ${category}:`)
    for (const e of categoryEffects) {
      const paramStr = e.parameters.length > 0
        ? ` (params: ${e.parameters.map((p) => `${p.key}:${p.type}`).join(', ')})`
        : ''
      effectLines.push(`    - ${e.id}: ${e.name} [${e.defaultDuration}ms]${paramStr}`)
    }
  }

  const propLines: string[] = ['ANIMATABLE PROPERTIES:']
  for (const p of properties) {
    const nodeRestriction = p.nodeTypes ? ` (${p.nodeTypes.join(', ')} only)` : ''
    propLines.push(`  - ${p.key} (${p.type}, default: ${JSON.stringify(p.default)})${nodeRestriction}`)
  }

  return `ANIMATION SYSTEM:

AnimationClip schema:
{
  id: string,
  kind: "animation",
  startTime: number (ms),
  duration: number (ms),
  effectId?: string,
  keyframes: [{ id: string, offset: 0.0-1.0, properties: { key: value }, easing: EasingPreset | [x1,y1,x2,y2] }],
  params?: Record<string, unknown>
}

Easing presets: linear, ease, easeIn, easeOut, easeInOut, snappy, bouncy, gentle, smooth

${effectLines.join('\n')}

${propLines.join('\n')}

COMMON PATTERNS:
- Fade in on enter: use "fade-in" effect (enter category)
- Slide in from left: use "slide-in" effect with direction param
- Scale up entrance: use "scale-in" effect
- Exit animations: use corresponding exit effects (fade-out, slide-out, scale-out)
- Emphasis: use pulse, shake, or bounce effects while element is visible

WORKFLOW:
1. set_composition to define duration and fps
2. add_clip to each node that should animate
3. Use effectId for preset animations, or raw keyframes for custom ones
4. Stagger startTime across nodes for sequential reveals`
}
