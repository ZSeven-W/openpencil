/**
 * Sub-agent execution for the orchestrator.
 *
 * Each sub-agent is responsible for generating one spatial section of the
 * design (e.g. "Hero", "Features", "Footer"). This module handles:
 * - Sequential execution with retry logic
 * - Streaming JSONL parsing and real-time canvas insertion
 * - ID namespace isolation via prefixes
 * - Fallback placeholder generation when a sub-agent fails
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
import { SUB_AGENT_PROMPT } from './orchestrator-prompts'
import {
  type PreparedDesignPrompt,
  getSubAgentTimeouts,
} from './orchestrator-prompt-optimizer'
import {
  expandRootFrameHeight,
  extractStreamingNodes,
  extractJsonFromResponse,
  insertStreamingNode,
  buildVariableContext,
  setGenerationContextHint,
  applyPostStreamingTreeHeuristics,
} from './design-generator'
import {
  startNewAnimationBatch,
} from './design-animation'
import { RETRY_TIMEOUT_CONFIG } from './ai-runtime-config'
import { emitProgress } from './orchestrator-progress'

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
// Sequential sub-agent execution
// ---------------------------------------------------------------------------

export async function executeSubAgentsSequentially(
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  preparedPrompt: PreparedDesignPrompt,
  progress: OrchestrationProgress,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
): Promise<SubAgentResult[]> {
  const results: SubAgentResult[] = []
  const timeoutOptions = getSubAgentTimeouts(preparedPrompt.originalLength)
  const retryTimeoutOptions = getRetrySubAgentTimeouts(timeoutOptions)

  for (let i = 0; i < plan.subtasks.length; i++) {
    let result = await executeSubAgent(
      plan.subtasks[i], plan, request, preparedPrompt, timeoutOptions, progress, i, callbacks,
    )
    if (shouldRetrySubtask(result)) {
      console.warn(
        `[Orchestrator] Retrying subtask "${plan.subtasks[i].label}" with extended timeout after: ${result.error}`,
      )
      result = await executeSubAgent(
        plan.subtasks[i], plan, request, preparedPrompt, retryTimeoutOptions, progress, i, callbacks,
      )
    }

    if (result.nodes.length === 0) {
      const minimalPrompt = buildMinimalFallbackPrompt(preparedPrompt.subAgentPrompt, plan.subtasks[i].label)
      console.warn(
        `[Orchestrator] Retrying subtask "${plan.subtasks[i].label}" in minimal mode because no nodes were produced.`,
      )
      result = await executeSubAgent(
        plan.subtasks[i],
        plan,
        request,
        preparedPrompt,
        retryTimeoutOptions,
        progress,
        i,
        callbacks,
        minimalPrompt,
      )
    }

    if (result.nodes.length === 0) {
      const placeholderNodes = insertLocalSubtaskPlaceholder(
        plan.subtasks[i],
        plan,
        progress.subtasks[i],
        progress,
      )
      if (placeholderNodes.length > 0) {
        callbacks?.onApplyPartial?.(progress.totalNodes)
        emitProgress(plan, progress, callbacks)
        result = {
          ...result,
          nodes: placeholderNodes,
          error: result.error
            ? `${result.error}; local placeholder inserted`
            : 'Local placeholder inserted after repeated empty output',
        }
      }
    }

    results.push(result)

    // Progressively expand root frame after each subtask is done (desktop only).
    // Mobile has a fixed viewport height so no progressive expansion is needed.
    if (result.nodes.length > 0 && plan.rootFrame.width > 480) {
      expandRootFrameHeight()
    }
  }
  return results
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
): Promise<SubAgentResult> {
  const animated = callbacks?.animated ?? false
  const progressEntry = progress.subtasks[index]
  progressEntry.status = 'streaming'
  emitProgress(plan, progress, callbacks)

  // Update context hint with subtask label so heuristics can detect
  // screenshot/mockup context even when the user prompt doesn't mention it
  setGenerationContextHint(`${request.prompt} ${subtask.label}`)

  const userPrompt = buildSubAgentUserPrompt(
    subtask,
    plan,
    promptOverride ?? preparedPrompt.subAgentPrompt,
    request.prompt,
    request.context?.variables,
    request.context?.themes,
  )

  let rawResponse = ''
  const nodes: PenNode[] = []
  let streamOffset = 0
  let subtaskRootId: string | null = null

  try {
    for await (const chunk of streamChat(
      SUB_AGENT_PROMPT,
      [{ role: 'user', content: userPrompt }],
      undefined,
      timeoutOptions,
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
              if (parentId !== null) {
                // Prefix the parent reference too
                const prefixedParent = ensurePrefixStr(
                  parentId,
                  subtask.idPrefix,
                )
                insertStreamingNode(node, prefixedParent)
              } else {
                // Sub-agent root → insert under the orchestrator root frame
                insertStreamingNode(node, plan.rootFrame.id)
                if (!subtaskRootId) subtaskRootId = node.id
              }
              nodes.push(node)
              progressEntry.nodeCount++
              progress.totalNodes++
            }
            callbacks?.onApplyPartial?.(progress.totalNodes)
            // Expand root frame as content grows — children inside sections
            // don't trigger expandRootFrameHeight from insertStreamingNode
            // because they insert under the section root, not DEFAULT_FRAME_ID.
            if (plan.rootFrame.width > 480) {
              expandRootFrameHeight()
            }
            emitProgress(plan, progress, callbacks, rawResponse)
          }
        }
      } else if (chunk.type === 'thinking') {
        // Accumulate and forward thinking content to UI
        progressEntry.thinking = (progressEntry.thinking ?? '') + chunk.content
        emitProgress(plan, progress, callbacks)
      } else if (chunk.type === 'error') {
        if (rawResponse.trim().length > 0) {
          const { recovered } = recoverNodesFromRawResponse(rawResponse, subtask, plan, nodes, progressEntry, progress)
          if (recovered > 0) {
            progressEntry.status = 'done'
            emitProgress(plan, progress, callbacks)
            return {
              subtaskId: subtask.id,
              nodes,
              rawResponse,
              error: `partial recovery after error: ${chunk.content}`,
            }
          }
        }
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
          insertStreamingNode(node, plan.rootFrame.id)
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
      console.warn(
        `[Orchestrator] Subtask "${subtask.label}" produced no parseable nodes. Raw length=${rawResponse.length}`,
      )
      return {
        subtaskId: subtask.id,
        nodes,
        rawResponse,
        error: 'Subtask completed but returned no parseable PenNode output.',
      }
    }

    // Apply tree-aware heuristics now that the full subtree is in the store.
    // During streaming, nodes were inserted individually without children, so
    // tree-aware heuristics (button width, frame height, clipContent) couldn't run.
    if (subtaskRootId) {
      applyPostStreamingTreeHeuristics(subtaskRootId)
    }

    progressEntry.status = 'done'
    emitProgress(plan, progress, callbacks)
    return { subtaskId: subtask.id, nodes, rawResponse }
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error'
    if (rawResponse.trim().length > 0) {
      const { recovered } = recoverNodesFromRawResponse(rawResponse, subtask, plan, nodes, progressEntry, progress)
      if (recovered > 0) {
        progressEntry.status = 'done'
        emitProgress(plan, progress, callbacks)
        return {
          subtaskId: subtask.id,
          nodes,
          rawResponse,
          error: `partial recovery after exception: ${msg}`,
        }
      }
    }
    progressEntry.status = 'error'
    emitProgress(plan, progress, callbacks)
    return { subtaskId: subtask.id, nodes, rawResponse, error: msg }
  }
}

// ---------------------------------------------------------------------------
// Node recovery from partial responses
// ---------------------------------------------------------------------------

function recoverNodesFromRawResponse(
  rawResponse: string,
  subtask: SubTask,
  plan: OrchestratorPlan,
  targetNodes: PenNode[],
  progressEntry: OrchestrationProgress['subtasks'][number],
  progress: OrchestrationProgress,
): { recovered: number; rootId: string | null } {
  const fallbackNodes = extractJsonFromResponse(rawResponse)
  if (!fallbackNodes || fallbackNodes.length === 0) return { recovered: 0, rootId: null }

  let recovered = 0
  let rootId: string | null = null
  startNewAnimationBatch()
  for (const node of fallbackNodes) {
    ensureIdPrefix(node, subtask.idPrefix)
    insertStreamingNode(node, plan.rootFrame.id)
    if (!rootId) rootId = node.id
    targetNodes.push(node)
    progressEntry.nodeCount++
    progress.totalNodes++
    recovered++
  }
  if (rootId) {
    applyPostStreamingTreeHeuristics(rootId)
  }
  return { recovered, rootId }
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

  // Show all sections so the model knows scope — only generate THIS one
  const sectionList = plan.subtasks
    .map((st) => `- ${st.label} (${st.region.width}x${st.region.height})${st.id === subtask.id ? ' ← YOU' : ''}`)
    .join('\n')

  let prompt = `Page sections:\n${sectionList}\n\nGenerate ONLY "${subtask.label}" (~${region.height}px of content).\n${compactPrompt}

CRITICAL LAYOUT CONSTRAINTS:
- Root frame: id="${subtask.idPrefix}-root", width="fill_container", height="fit_content", layout="vertical". NEVER use fixed pixel height on root — let content determine height.
- Target content amount: ~${region.height}px tall. Generate enough elements to fill this area.
- ALL nodes must be descendants of the root frame. No floating/orphan nodes.
- NEVER set x or y on children inside layout frames.
- Use "fill_container" for children that stretch, "fit_content" for shrink-wrap sizing.
- Use justifyContent="space_between" to distribute items (e.g. navbar: logo | links | CTA). Use padding=[0,80] for horizontal page margins.
- For side-by-side layouts, nest a horizontal frame with child frames using "fill_container" width.
- Phone mockup = ONE frame node, cornerRadius 32. If a placeholder label is needed, allow exactly ONE centered text child inside the phone; otherwise no children. Never place placeholder text below the phone as a sibling. NEVER use ellipse.
- Text height must fit content: estimate lines × fontSize × 1.4.
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

// ---------------------------------------------------------------------------
// Retry & fallback helpers
// ---------------------------------------------------------------------------

export function getRetrySubAgentTimeouts(base: StreamTimeoutConfig): StreamTimeoutConfig {
  return {
    ...base,
    hardTimeoutMs: Math.min(
      base.hardTimeoutMs * RETRY_TIMEOUT_CONFIG.multiplier,
      RETRY_TIMEOUT_CONFIG.hardTimeoutMaxMs,
    ),
    noTextTimeoutMs: Math.min(
      base.noTextTimeoutMs * RETRY_TIMEOUT_CONFIG.multiplier,
      RETRY_TIMEOUT_CONFIG.noTextTimeoutMaxMs,
    ),
    firstTextTimeoutMs: base.firstTextTimeoutMs
      ? Math.min(
        base.firstTextTimeoutMs * RETRY_TIMEOUT_CONFIG.multiplier,
        RETRY_TIMEOUT_CONFIG.firstTextTimeoutMaxMs,
      )
      : undefined,
  }
}

function shouldRetrySubtask(result: SubAgentResult): boolean {
  if (result.nodes.length > 0) return false
  if (!result.error) return false
  const error = result.error.toLowerCase()
  return error.includes('timed out') || error.includes('thinking too long')
}

function buildMinimalFallbackPrompt(basePrompt: string, label: string): string {
  return `${basePrompt}

Fallback mode for section "${label}":
- Prioritize completion over detail.
- Output a minimal skeleton only (4-8 nodes).
- Include: section container, heading, short description, and 1-2 placeholder blocks.
- Avoid complex SVG/icon/path details.`
}

function insertLocalSubtaskPlaceholder(
  subtask: SubTask,
  plan: OrchestratorPlan,
  progressEntry: OrchestrationProgress['subtasks'][number],
  progress: OrchestrationProgress,
): PenNode[] {
  const sectionId = `${subtask.idPrefix}-fallback-section`
  const titleId = `${subtask.idPrefix}-fallback-title`
  const descId = `${subtask.idPrefix}-fallback-desc`
  const rowId = `${subtask.idPrefix}-fallback-row`
  const cardAId = `${subtask.idPrefix}-fallback-card-a`
  const cardBId = `${subtask.idPrefix}-fallback-card-b`

  const sectionNode: PenNode = {
    id: sectionId,
    type: 'frame',
    name: `${subtask.label} (Fallback)`,
    width: 'fill_container',
    height: Math.max(120, subtask.region.height),
    layout: 'vertical',
    padding: 24,
    gap: 12,
    fill: [{ type: 'solid', color: '#FFFFFF' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#E2E8F0' }] },
    children: [],
  }

  const titleNode: PenNode = {
    id: titleId,
    type: 'text',
    name: 'Fallback Title',
    content: subtask.label,
    fontSize: 24,
    fontWeight: 700,
    width: 'fill_container',
    height: 32,
    fill: [{ type: 'solid', color: '#0F172A' }],
  }

  const descNode: PenNode = {
    id: descId,
    type: 'text',
    name: 'Fallback Description',
    content: 'Content is loading from a complex generation task. This placeholder keeps layout continuity.',
    fontSize: 14,
    fontWeight: 400,
    width: 'fill_container',
    height: 40,
    fill: [{ type: 'solid', color: '#64748B' }],
  }

  const rowNode: PenNode = {
    id: rowId,
    type: 'frame',
    name: 'Fallback Row',
    width: 'fill_container',
    height: 120,
    layout: 'horizontal',
    gap: 12,
    children: [],
  }

  const cardANode: PenNode = {
    id: cardAId,
    type: 'rectangle',
    name: 'Fallback Card A',
    width: 'fill_container',
    height: 120,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F1F5F9' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#CBD5E1' }] },
  }

  const cardBNode: PenNode = {
    id: cardBId,
    type: 'rectangle',
    name: 'Fallback Card B',
    width: 'fill_container',
    height: 120,
    cornerRadius: 8,
    fill: [{ type: 'solid', color: '#F1F5F9' }],
    stroke: { thickness: 1, fill: [{ type: 'solid', color: '#CBD5E1' }] },
  }

  const insertedNodes = [sectionNode, titleNode, descNode, rowNode, cardANode, cardBNode]

  startNewAnimationBatch()
  insertStreamingNode(sectionNode, plan.rootFrame.id)
  insertStreamingNode(titleNode, sectionId)
  insertStreamingNode(descNode, sectionId)
  insertStreamingNode(rowNode, sectionId)
  insertStreamingNode(cardANode, rowId)
  insertStreamingNode(cardBNode, rowId)

  progressEntry.nodeCount += insertedNodes.length
  progress.totalNodes += insertedNodes.length
  progressEntry.status = 'done'

  console.warn(
    `[Orchestrator] Inserted local placeholder for subtask "${subtask.label}" after repeated empty output.`,
  )

  return insertedNodes
}
