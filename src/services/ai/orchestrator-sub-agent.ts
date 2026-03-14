/**
 * Sub-agent execution for the orchestrator.
 *
 * Each sub-agent is responsible for generating one spatial section of the
 * design (e.g. "Hero", "Features", "Footer"). This module handles:
 * - Sequential execution
 * - Streaming JSONL parsing and real-time canvas insertion
 * - ID namespace isolation via prefixes
 */

import type { PenNode } from '@/types/pen'
import type { VariableDefinition } from '@/types/variables'
import type {
  AIDesignRequest,
  OrchestratorPlan,
  OrchestrationProgress,
  SubTask,
  SubAgentResult,
} from './ai-types'
import { streamChat } from './ai-service'
import { SUB_AGENT_PROMPT, SUB_AGENT_PROMPT_SIMPLIFIED } from './orchestrator-prompts'
import {
  type PreparedDesignPrompt,
  getSubAgentTimeouts,
} from './orchestrator-prompt-optimizer'
import { resolveModelProfile, needsSimplifiedPrompt } from './model-profiles'
import {
  expandRootFrameHeight,
  extractStreamingNodes,
  extractJsonFromResponse,
  insertStreamingNode,
  buildVariableContext,
  applyPostStreamingTreeHeuristics,
} from './design-generator'
import {
  startNewAnimationBatch,
} from './design-animation'
import { emitProgress } from './orchestrator-progress'
import { addAgentIndicatorRecursive, removeAgentIndicatorsByPrefix } from '@/canvas/agent-indicator'
import { markNodesForAnimation } from './design-animation'

// ---------------------------------------------------------------------------
// Stream timeout configuration (shared with orchestrator.ts)
// ---------------------------------------------------------------------------

export interface StreamTimeoutConfig {
  hardTimeoutMs: number
  noTextTimeoutMs: number
  thinkingResetsTimeout: boolean
  pingResetsTimeout?: boolean
  firstTextTimeoutMs?: number
  thinkingMode?: 'adaptive' | 'disabled' | 'enabled'
  thinkingBudgetTokens?: number
  effort?: 'low' | 'medium' | 'high' | 'max'
}

// ---------------------------------------------------------------------------
// ID namespace isolation
// ---------------------------------------------------------------------------

export function ensureIdPrefix(node: PenNode, prefix: string): void {
  if (!node.id.startsWith(`${prefix}-`)) {
    node.id = `${prefix}-${node.id}`
  }
  // Recursively prefix children (for fallback tree extraction)
  if ('children' in node && Array.isArray(node.children)) {
    for (const child of node.children) {
      ensureIdPrefix(child, prefix)
    }
  }
}

export function ensurePrefixStr(id: string, prefix: string): string {
  if (id.startsWith(`${prefix}-`)) return id
  return `${prefix}-${id}`
}

// ---------------------------------------------------------------------------
// Sub-agent execution (sequential or concurrent)
// ---------------------------------------------------------------------------

export async function executeSubAgents(
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  preparedPrompt: PreparedDesignPrompt,
  progress: OrchestrationProgress,
  concurrency: number = 1,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
  abortSignal?: AbortSignal,
): Promise<SubAgentResult[]> {
  const timeoutOptions = getSubAgentTimeouts(preparedPrompt.originalLength, request.model)

  // Sequential path — each subtask runs one at a time
  if (concurrency <= 1) {
    const results: SubAgentResult[] = []
    for (let i = 0; i < plan.subtasks.length; i++) {
      if (abortSignal?.aborted) break

      const result = await executeSubAgent(
        plan.subtasks[i], plan, request, preparedPrompt, timeoutOptions, progress, i, callbacks,
        undefined, abortSignal,
      )

      if (result.error && result.nodes.length === 0) {
        throw new Error(result.error)
      }

      results.push(result)

      if (result.nodes.length > 0) {
        expandRootFrameHeight()
      }
    }
    return results
  }

  // Concurrent path — screen-grouped parallelism.
  // Subtasks sharing the same screen run sequentially (preserves section order).
  // Different screen groups run in parallel, limited by `concurrency`.
  const total = plan.subtasks.length
  const results: (SubAgentResult | null)[] = new Array(total).fill(null)

  // Group subtasks by screen (same logic as orchestrator.ts)
  const screenGroups: number[][] = []
  const screenMap = new Map<string, number>()
  for (let i = 0; i < total; i++) {
    const screen = plan.subtasks[i].screen ?? plan.subtasks[i].id
    if (screenMap.has(screen)) {
      screenGroups[screenMap.get(screen)!].push(i)
    } else {
      screenMap.set(screen, screenGroups.length)
      screenGroups.push([i])
    }
  }

  // Semaphore to limit total concurrent API calls
  let activeSlots = 0
  const waitQueue: (() => void)[] = []

  async function acquireSlot() {
    if (activeSlots < concurrency) {
      activeSlots++
      return
    }
    await new Promise<void>((resolve) => waitQueue.push(resolve))
    activeSlots++
  }

  function releaseSlot() {
    activeSlots--
    if (waitQueue.length > 0) {
      waitQueue.shift()!()
    }
  }

  // Each screen group runs its subtasks sequentially
  const workers = screenGroups.map(async (indices) => {
    for (const idx of indices) {
      if (abortSignal?.aborted) return

      await acquireSlot()
      try {
        const result = await executeSubAgent(
          plan.subtasks[idx], plan, request, preparedPrompt, timeoutOptions, progress, idx, callbacks,
          undefined, abortSignal,
        )
        results[idx] = result

        if (result.nodes.length > 0) {
          expandRootFrameHeight(plan.subtasks[idx].parentFrameId ?? undefined)
        }
      } catch (err) {
        results[idx] = {
          subtaskId: plan.subtasks[idx].id,
          nodes: [],
          rawResponse: '',
          error: err instanceof Error ? err.message : 'Unknown error',
        }
      } finally {
        releaseSlot()
      }
    }
  })

  await Promise.all(workers)

  // Collect non-null results
  const collected = results.filter((r): r is SubAgentResult => r !== null)

  // If ALL failed with zero nodes, throw
  const totalNodes = collected.reduce((sum, r) => sum + r.nodes.length, 0)
  if (totalNodes === 0 && collected.length > 0) {
    const errors = collected.filter((r) => r.error).map((r) => r.error!)
    const firstError = errors[0] ?? 'The model failed to generate any design output.'
    throw new Error(firstError)
  }

  return collected
}

// ---------------------------------------------------------------------------
// Single sub-agent execution
// ---------------------------------------------------------------------------

async function executeSubAgent(
  subtask: SubTask,
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  preparedPrompt: PreparedDesignPrompt,
  timeoutOptions: StreamTimeoutConfig,
  progress: OrchestrationProgress,
  index: number,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
  promptOverride?: string,
  abortSignal?: AbortSignal,
): Promise<SubAgentResult> {
  const animated = callbacks?.animated ?? false
  const progressEntry = progress.subtasks[index]
  progressEntry.status = 'streaming'
  emitProgress(plan, progress, callbacks)

  // Agent identity for canvas indicators (concurrent mode only)
  const agentColor = progressEntry.agentColor
  const agentName = progressEntry.agentName

  // Context hint is set once at orchestrator level (combining all subtask labels)
  // to avoid race conditions during concurrent execution

  const userPrompt = buildSubAgentUserPrompt(
    subtask,
    plan,
    promptOverride ?? preparedPrompt.subAgentPrompt,
    request.prompt,
    request.context?.variables,
    request.context?.themes,
  )

  // Select prompt variant based on model profile
  const profile = resolveModelProfile(request.model)
  const basePrompt = needsSimplifiedPrompt(profile) ? SUB_AGENT_PROMPT_SIMPLIFIED : SUB_AGENT_PROMPT
  const systemPrompt = preparedPrompt.designPrinciples && !needsSimplifiedPrompt(profile)
    ? `${basePrompt}\n\n${preparedPrompt.designPrinciples}`
    : basePrompt

  // For basic-tier models (MiniMax, GLM, Qwen, etc.) via third-party routers,
  // system prompts may be ignored or lost. Inline the system prompt into the
  // user message to ensure the model sees the instructions.
  const inlineSystem = needsSimplifiedPrompt(profile)
  const effectiveSystem = inlineSystem ? '' : systemPrompt
  const effectiveUserContent = inlineSystem
    ? `${systemPrompt}\n\n---\n\n${userPrompt}`
    : userPrompt

  let rawResponse = ''
  const nodes: PenNode[] = []
  let streamOffset = 0
  let subtaskRootId: string | null = null

  try {
    for await (const chunk of streamChat(
      effectiveSystem,
      [{ role: 'user', content: effectiveUserContent }],
      request.model,
      timeoutOptions,
      request.provider,
      abortSignal,
    )) {
      if (chunk.type === 'text') {
        rawResponse += chunk.content

        // Forward streaming text to panel
        emitProgress(plan, progress, callbacks, rawResponse)

        if (animated) {
          const { results, newOffset } = extractStreamingNodes(
            rawResponse,
            streamOffset,
          )
          if (results.length > 0) {
            streamOffset = newOffset
            startNewAnimationBatch()

            for (const { node, parentId } of results) {
              // Enforce ID prefix
              ensureIdPrefix(node, subtask.idPrefix)

              // Tag node AND all descendants for canvas indicator + preview
              // delay BEFORE insert. insertStreamingNode triggers synchronous
              // canvas sync which checks isPreviewNode — must be set first.
              if (agentColor && agentName) {
                addAgentIndicatorRecursive(node, agentColor, agentName)
              }
              // Pre-populate pendingAnimationNodes for all descendants so
              // canvas-sync animates them (insertStreamingNode only marks the
              // root and skips background frames).
              markNodesForAnimation([node])

              if (parentId !== null) {
                // Prefix the parent reference too
                const prefixedParent = ensurePrefixStr(
                  parentId,
                  subtask.idPrefix,
                )
                insertStreamingNode(node, prefixedParent)
              } else {
                // Sub-agent root → insert under subtask's parent frame
                // In concurrent mode, each subtask has its own root frame;
                // in sequential mode, all share plan.rootFrame.id
                const targetParent = subtask.parentFrameId ?? plan.rootFrame.id
                insertStreamingNode(node, targetParent)
                if (!subtaskRootId) subtaskRootId = node.id
              }
              nodes.push(node)
              progressEntry.nodeCount++
              progress.totalNodes++
            }
            callbacks?.onApplyPartial?.(progress.totalNodes)
            // Expand the subtask's root frame as content grows
            expandRootFrameHeight(subtask.parentFrameId ?? undefined)
            emitProgress(plan, progress, callbacks, rawResponse)
          }
        }
      } else if (chunk.type === 'thinking') {
        // Accumulate and forward thinking content to UI
        progressEntry.thinking = (progressEntry.thinking ?? '') + chunk.content
        emitProgress(plan, progress, callbacks)
      } else if (chunk.type === 'error') {
        progressEntry.status = 'error'
        emitProgress(plan, progress, callbacks)
        return { subtaskId: subtask.id, nodes, rawResponse, error: chunk.content }
      }
    }

    // Fallback: if streaming extraction found nothing, try batch extraction
    if (nodes.length === 0 && rawResponse.trim().length > 0) {
      const fallbackNodes = extractJsonFromResponse(rawResponse)
      if (fallbackNodes && fallbackNodes.length > 0) {
        startNewAnimationBatch()
        for (const node of fallbackNodes) {
          ensureIdPrefix(node, subtask.idPrefix)

          // Tag ALL descendants BEFORE insert — same reason as streaming path
          if (agentColor && agentName) {
            addAgentIndicatorRecursive(node, agentColor, agentName)
          }
          markNodesForAnimation([node])

          const targetParent = subtaskRootId
            ? subtaskRootId
            : (subtask.parentFrameId ?? plan.rootFrame.id)
          insertStreamingNode(node, targetParent)
          if (!subtaskRootId) subtaskRootId = node.id
          nodes.push(node)
          progressEntry.nodeCount++
          progress.totalNodes++
        }
        callbacks?.onApplyPartial?.(progress.totalNodes)
      }
    }

    if (nodes.length === 0) {
      progressEntry.status = 'error'
      emitProgress(plan, progress, callbacks)

      // Build a diagnostic error with a preview of what the model returned
      let errorMsg = 'The model response could not be parsed as design nodes.'
      if (rawResponse.trim().length === 0) {
        errorMsg += ' The model returned an empty response.'
      } else {
        // Show a short snippet so the user can diagnose the issue
        const preview = rawResponse.trim().slice(0, 150)
        const hasJson = rawResponse.includes('{') && rawResponse.includes('"type"')
        if (!hasJson) {
          errorMsg += ' The response did not contain valid JSON. Model output: "' + preview + (rawResponse.length > 150 ? '…' : '') + '"'
        } else {
          errorMsg += ' JSON was found but contained no valid PenNode objects (need "id" and "type" fields).'
        }
      }

      return {
        subtaskId: subtask.id,
        nodes,
        rawResponse,
        error: errorMsg,
      }
    }

    // Apply tree-aware heuristics now that the full subtree is in the store.
    // During streaming, nodes were inserted individually without children, so
    // tree-aware heuristics (button width, frame height, clipContent) couldn't run.
    if (subtaskRootId) {
      applyPostStreamingTreeHeuristics(subtaskRootId)
    }

    progressEntry.status = 'done'
    // Delay indicator removal so the glow effect is visible even when the
    // subtask finishes quickly (e.g. model outputs everything in one chunk).
    setTimeout(() => removeAgentIndicatorsByPrefix(subtask.idPrefix), 1500)
    emitProgress(plan, progress, callbacks)
    return { subtaskId: subtask.id, nodes, rawResponse }
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error'
    progressEntry.status = 'error'
    setTimeout(() => removeAgentIndicatorsByPrefix(subtask.idPrefix), 1500)
    emitProgress(plan, progress, callbacks)
    return { subtaskId: subtask.id, nodes, rawResponse, error: msg }
  }
}

// ---------------------------------------------------------------------------
// Sub-agent prompt builder
// ---------------------------------------------------------------------------

function buildSubAgentUserPrompt(
  subtask: SubTask,
  plan: OrchestratorPlan,
  compactPrompt: string,
  fullPrompt: string,
  variables?: Record<string, VariableDefinition>,
  themes?: Record<string, string[]>,
): string {
  const { region } = subtask

  // Show all sections with their element boundaries so the model knows exact scope
  const sectionList = plan.subtasks
    .map((st) => {
      const marker = st.id === subtask.id ? ' ← YOU' : ''
      const elems = st.elements ? ` [${st.elements}]` : ''
      return `- ${st.label}${elems} (${st.region.width}x${st.region.height})${marker}`
    })
    .join('\n')

  // Build explicit boundary instruction when elements are specified
  const myElements = subtask.elements
    ? `\nYOUR ELEMENTS: ${subtask.elements}\nDo NOT generate elements listed in other sections — they handle their own content.`
    : ''

  let prompt = `Page sections:\n${sectionList}\n\nGenerate ONLY "${subtask.label}" (~${region.height}px of content).${myElements}\n${compactPrompt}

CRITICAL LAYOUT CONSTRAINTS:
- Root frame: id="${subtask.idPrefix}-root", width="fill_container", height="fit_content", layout="vertical". NEVER use fixed pixel height on root — let content determine height.
- Target content amount: ~${region.height}px tall. Generate enough elements to fill this area.
- ALL nodes must be descendants of the root frame. No floating/orphan nodes.
- NEVER set x or y on children inside layout frames.
- Use "fill_container" for children that stretch, "fit_content" for shrink-wrap sizing.
- Use justifyContent="space_between" to distribute items (e.g. navbar: logo | links | CTA). Use padding=[0,80] for horizontal page margins.
- For side-by-side layouts, nest a horizontal frame with child frames using "fill_container" width.
- Phone mockup = ONE frame node, cornerRadius 32. If a placeholder label is needed, allow exactly ONE centered text child inside the phone; otherwise no children. Never place placeholder text below the phone as a sibling. NEVER use ellipse.
- IDs prefix="${subtask.idPrefix}-". No <step> tags. Output \`\`\`json immediately.`

  if (needsNativeDenseCardInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nNATIVE DENSE-CARD MODE (must be solved during generation):
- If you create a horizontal row with 5+ cards (or cards become narrow), compact each card natively BEFORE output.
- Each card: max 2 text blocks only (title + one short metric). Remove long descriptions.
- Rewrite long copy into concise keyword phrases. Never use truncation marks ("..." or "…").
- Prefer removing non-essential decorative elements before shrinking readability.
- Do NOT rely on post-processing to prune card content.`
  }
  if (needsTableStructureInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nTABLE MODE (must be structured natively):
- Build table as explicit grid frames, NOT a single long text line.
- Header must be its own horizontal row with separate cell frames for each column.
- Body rows must align to the same column structure as header.
- Keep level badge/chip inside the level cell; do not merge multiple columns into one text node.
- In table rows, avoid badge/button auto-style patterns unless the node is explicitly a chip.`
  }
  if (needsHeroPhoneTwoColumnInstruction(subtask.label, compactPrompt, fullPrompt)) {
    prompt += `\n\nHERO PHONE LAYOUT MODE (desktop):
- Use a horizontal two-column hero layout: left = headline/subtitle/CTA, right = phone mockup.
- Keep phone as a sibling in the same horizontal row, NOT stacked below the headline.
- Only use stacked layout for mobile/narrow viewport sections.`
  }

  // Inject style guide so sub-agent uses consistent colors/fonts
  if (plan.styleGuide) {
    const sg = plan.styleGuide
    const p = sg.palette
    prompt += `\n\nSTYLE GUIDE (use these consistently):
- Background: ${p.background}  Surface: ${p.surface}
- Text: ${p.text}  Secondary: ${p.secondary}
- Accent: ${p.accent}  Accent2: ${p.accent2}  Border: ${p.border}
- Heading font: ${sg.fonts.heading}  Body font: ${sg.fonts.body}
- Aesthetic: ${sg.aesthetic}`
  }

  const varContext = buildVariableContext(variables, themes)
  if (varContext) {
    prompt += '\n\n' + varContext
  }

  return prompt
}

// ---------------------------------------------------------------------------
// Instruction detection helpers
// ---------------------------------------------------------------------------

function needsNativeDenseCardInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase()
  if (/(dense|密集|多卡片|卡片过多|超过\s*4\s*个|5\+\s*cards?|cards?\s*row|一行.*卡片|横排.*卡片)/.test(text)) {
    return true
  }
  if (/(cefr|a1[\s-]*c2|a1|a2|b1|b2|c1|c2|词库分级|分级词库|学习阶段|等级)/.test(text)) {
    return true
  }
  if (/(feature\s*cards?|cards?\s*section|词库|词汇|card)/.test(text) && /(a1|b1|c1|c2|cefr|等级|阶段)/.test(text)) {
    return true
  }
  return false
}

function needsTableStructureInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase()
  if (/(table|grid|tabular|表格|表头|表体|列|行|字段|等级|级别|词汇量|适用人群|对应考试)/.test(text)) {
    return true
  }
  if (/(cefr|a1[\s-]*c2|a1|a2|b1|b2|c1|c2)/.test(text) && /(level|table|表格|等级)/.test(text)) {
    return true
  }
  return false
}

function needsHeroPhoneTwoColumnInstruction(
  subtaskLabel: string,
  compactPrompt: string,
  fullPrompt: string,
): boolean {
  const text = `${subtaskLabel}\n${compactPrompt}\n${fullPrompt}`.toLowerCase()
  const heroLike = /(hero|首页首屏|首屏|横幅|banner)/.test(text)
  const phoneLike = /(phone|mockup|screenshot|截图|手机|app\s*screen|应用截图)/.test(text)
  return heroLike && phoneLike
}
