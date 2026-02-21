/**
 * Orchestrator for parallel design generation.
 *
 * Flow:
 * 1. Fast "architect" API call decomposes the prompt into spatial sub-tasks
 * 2. Root frame is created on canvas
 * 3. Multiple sub-agents execute in parallel, each streaming JSONL
 * 4. Nodes are inserted to canvas in real-time with animation
 *
 * Falls back to single-call generation on any orchestrator failure.
 */

import type { PenNode, FrameNode } from '@/types/pen'
import type { VariableDefinition } from '@/types/variables'
import type {
  AIDesignRequest,
  OrchestratorPlan,
  OrchestrationProgress,
  SubTask,
  SubAgentResult,
} from './ai-types'
import { streamChat } from './ai-service'
import { ORCHESTRATOR_PROMPT, ORCHESTRATOR_TIMEOUTS, SUB_AGENT_PROMPT } from './orchestrator-prompts'
import {
  extractStreamingNodes,
  extractJsonFromResponse,
  insertStreamingNode,
  buildVariableContext,
  resetGenerationRemapping,
} from './design-generator'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import {
  resetAnimationState,
  startNewAnimationBatch,
} from './design-animation'

const SUB_AGENT_TIMEOUTS = {
  hardTimeoutMs: 60_000,
  noTextTimeoutMs: 30_000,
  thinkingResetsTimeout: true,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

export async function executeOrchestration(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  const animated = callbacks?.animated ?? false

  // -- Phase 1: Planning (streaming) --
  callbacks?.onTextUpdate?.(
    '<step title="Planning layout" status="streaming">Analyzing design structure...</step>',
  )

  const plan = await callOrchestrator(request.prompt, (thinking) => {
    const truncated = thinking.length > 200
      ? thinking.slice(-200) + '...'
      : thinking
    callbacks?.onTextUpdate?.(
      `<step title="Planning layout" status="streaming">${truncated}</step>`,
    )
  })

  // Assign ID prefixes
  for (const st of plan.subtasks) {
    st.idPrefix = st.id
    st.parentFrameId = plan.rootFrame.id
  }

  // Show planning done + all subtask steps as pending
  emitProgress(plan, {
    phase: 'generating',
    subtasks: plan.subtasks.map((st) => ({
      id: st.id, label: st.label, status: 'pending' as const, nodeCount: 0,
    })),
    totalNodes: 0,
  }, callbacks)

  // -- Phase 2: Setup canvas --
  resetGenerationRemapping()

  if (animated) {
    resetAnimationState()
    useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
  }

  // Insert root frame
  const rootNode: FrameNode = {
    id: plan.rootFrame.id,
    type: 'frame',
    name: plan.rootFrame.name,
    x: 0,
    y: 0,
    width: plan.rootFrame.width,
    height: plan.rootFrame.height,
    layout: plan.rootFrame.layout ?? 'vertical',
    gap: plan.rootFrame.gap ?? 0,
    fill: (plan.rootFrame.fill as FrameNode['fill']) ?? [
      { type: 'solid', color: '#FFFFFF' },
    ],
    children: [],
  }

  insertStreamingNode(rootNode, null)

  // -- Phase 3: Parallel sub-agent execution --
  const progress: OrchestrationProgress = {
    phase: 'generating',
    subtasks: plan.subtasks.map((st) => ({
      id: st.id,
      label: st.label,
      status: 'pending' as const,
      nodeCount: 0,
    })),
    totalNodes: 0,
  }

  let results: SubAgentResult[]
  try {
    results = await executeSubAgentsSequentially(
      plan,
      request,
      progress,
      callbacks,
    )
  } finally {
    if (animated) {
      useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
    }
  }

  // -- Phase 4: Collect results --
  // Mark all completed subtasks as done in final progress
  for (const entry of progress.subtasks) {
    if (entry.status !== 'error') {
      entry.status = 'done'
    }
  }
  progress.phase = 'done'
  emitProgress(plan, progress, callbacks)

  const allNodes: PenNode[] = [rootNode]
  for (const r of results) {
    allNodes.push(...r.nodes)
  }

  // Build final rawResponse that includes step tags so the chat message
  // shows the complete pipeline progress after streaming ends
  const finalStepTags = buildFinalStepTags(plan, progress)

  return { nodes: allNodes, rawResponse: finalStepTags }
}

// ---------------------------------------------------------------------------
// Orchestrator call — fast decomposition
// ---------------------------------------------------------------------------

async function callOrchestrator(
  prompt: string,
  onThinking?: (thinking: string) => void,
): Promise<OrchestratorPlan> {
  console.log('[Orchestrator] Calling streamChat...')

  let rawResponse = ''
  let thinkingContent = ''

  for await (const chunk of streamChat(
    ORCHESTRATOR_PROMPT,
    [{ role: 'user', content: prompt }],
    undefined,
    {
      ...ORCHESTRATOR_TIMEOUTS,
      thinkingResetsTimeout: true,
    },
  )) {
    if (chunk.type === 'text') {
      rawResponse += chunk.content
    } else if (chunk.type === 'thinking') {
      thinkingContent += chunk.content
      onThinking?.(thinkingContent)
    } else if (chunk.type === 'error') {
      throw new Error(`Orchestrator failed: ${chunk.content}`)
    }
  }

  console.log('[Orchestrator] Raw response:', rawResponse.slice(0, 500))

  const plan = parseOrchestratorResponse(rawResponse)
  if (!plan) {
    console.error('[Orchestrator] Failed to parse plan from:', rawResponse.slice(0, 500))
    throw new Error('Failed to parse orchestrator plan')
  }

  console.log('[Orchestrator] Plan:', plan.subtasks.length, 'subtasks')
  return plan
}

function parseOrchestratorResponse(raw: string): OrchestratorPlan | null {
  const trimmed = raw.trim()

  // Try direct parse
  const plan = tryParsePlan(trimmed)
  if (plan) return plan

  // Try extracting from code fences
  const fenceMatch = trimmed.match(/```(?:json)?\s*\n?([\s\S]*?)\n?```/)
  if (fenceMatch) {
    const fenced = tryParsePlan(fenceMatch[1].trim())
    if (fenced) return fenced
  }

  // Try extracting first { ... } block
  const firstBrace = trimmed.indexOf('{')
  const lastBrace = trimmed.lastIndexOf('}')
  if (firstBrace >= 0 && lastBrace > firstBrace) {
    const braced = tryParsePlan(trimmed.slice(firstBrace, lastBrace + 1))
    if (braced) return braced
  }

  return null
}

function tryParsePlan(text: string): OrchestratorPlan | null {
  try {
    const obj = JSON.parse(text) as Record<string, unknown>
    if (!obj.rootFrame || typeof obj.rootFrame !== 'object') return null
    if (!Array.isArray(obj.subtasks) || obj.subtasks.length === 0) return null

    const rf = obj.rootFrame as Record<string, unknown>
    if (!rf.id || !rf.width || !rf.height) return null

    for (const st of obj.subtasks as Record<string, unknown>[]) {
      if (!st.id || !st.region) return null
    }

    return obj as unknown as OrchestratorPlan
  } catch {
    return null
  }
}

// ---------------------------------------------------------------------------
// Sequential sub-agent execution
// ---------------------------------------------------------------------------

async function executeSubAgentsSequentially(
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  progress: OrchestrationProgress,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
): Promise<SubAgentResult[]> {
  const results: SubAgentResult[] = []
  for (let i = 0; i < plan.subtasks.length; i++) {
    const result = await executeSubAgent(
      plan.subtasks[i], plan, request, progress, i, callbacks,
    )
    results.push(result)
  }
  return results
}

async function executeSubAgent(
  subtask: SubTask,
  plan: OrchestratorPlan,
  request: AIDesignRequest,
  progress: OrchestrationProgress,
  index: number,
  callbacks?: {
    onApplyPartial?: (count: number) => void
    onTextUpdate?: (text: string) => void
    animated?: boolean
  },
): Promise<SubAgentResult> {
  const animated = callbacks?.animated ?? false
  const progressEntry = progress.subtasks[index]
  progressEntry.status = 'streaming'
  emitProgress(plan, progress, callbacks)

  const userPrompt = buildSubAgentUserPrompt(
    subtask,
    plan,
    request.prompt,
    request.context?.variables,
    request.context?.themes,
  )

  let rawResponse = ''
  const nodes: PenNode[] = []
  let streamOffset = 0

  try {
    for await (const chunk of streamChat(
      SUB_AGENT_PROMPT,
      [{ role: 'user', content: userPrompt }],
      undefined,
      SUB_AGENT_TIMEOUTS,
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
              }
              nodes.push(node)
              progressEntry.nodeCount++
              progress.totalNodes++
            }
            callbacks?.onApplyPartial?.(progress.totalNodes)
            emitProgress(plan, progress, callbacks, rawResponse)
          }
        }
      } else if (chunk.type === 'thinking') {
        // Forward thinking progress so UI doesn't look stuck
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
          insertStreamingNode(node, plan.rootFrame.id)
          nodes.push(node)
          progressEntry.nodeCount++
          progress.totalNodes++
        }
        callbacks?.onApplyPartial?.(progress.totalNodes)
      }
    }

    progressEntry.status = 'done'
    emitProgress(plan, progress, callbacks)
    return { subtaskId: subtask.id, nodes, rawResponse }
  } catch (err) {
    const msg = err instanceof Error ? err.message : 'Unknown error'
    progressEntry.status = 'error'
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
  originalPrompt: string,
  variables?: Record<string, VariableDefinition>,
  themes?: Record<string, string[]>,
): string {
  const { region } = subtask

  // Show all sections so the model knows scope — only generate THIS one
  const sectionList = plan.subtasks
    .map((st) => `- ${st.label} (${st.region.width}x${st.region.height})${st.id === subtask.id ? ' ← YOU' : ''}`)
    .join('\n')

  let prompt = `Page sections:\n${sectionList}\n\nGenerate ONLY "${subtask.label}" (${region.width}x${region.height}px).\n${originalPrompt}\nRoot: id="${subtask.idPrefix}-root", width="fill_container", height=${region.height}. IDs prefix="${subtask.idPrefix}-". No <step> tags. Output \`\`\`json immediately.`

  const varContext = buildVariableContext(variables, themes)
  if (varContext) {
    prompt += '\n\n' + varContext
  }

  return prompt
}

// ---------------------------------------------------------------------------
// ID namespace isolation
// ---------------------------------------------------------------------------

function ensureIdPrefix(node: PenNode, prefix: string): void {
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

function ensurePrefixStr(id: string, prefix: string): string {
  if (id.startsWith(`${prefix}-`)) return id
  return `${prefix}-${id}`
}

// ---------------------------------------------------------------------------
// Progress emission — updates UI via <step> tags
// ---------------------------------------------------------------------------

function emitProgress(
  plan: OrchestratorPlan,
  progress: OrchestrationProgress,
  callbacks?: {
    onTextUpdate?: (text: string) => void
  },
  streamingText?: string,
): void {
  if (!callbacks?.onTextUpdate) return

  // Always show "Planning layout" as done first
  const planningStep = '<step title="Planning layout" status="done">Analyzing design structure...</step>'

  const subtaskSteps = plan.subtasks
    .map((st, i) => {
      const entry = progress.subtasks[i]
      const status = entry.status === 'streaming' ? 'streaming'
        : entry.status === 'done' ? 'done'
        : entry.status === 'error' ? 'error'
        : 'pending'
      const nodeInfo = entry.nodeCount > 0 ? ` (${entry.nodeCount} elements)` : ''
      return `<step title="${st.label}${nodeInfo}" status="${status}"></step>`
    })
    .join('\n')

  let output = `${planningStep}\n${subtaskSteps}`
  if (streamingText) {
    output += '\n\n' + streamingText
  }
  callbacks.onTextUpdate(output)
}

/** Build step tags for the final rawResponse (shown in message after streaming ends) */
function buildFinalStepTags(
  plan: OrchestratorPlan,
  progress: OrchestrationProgress,
): string {
  const planningStep = '<step title="Planning layout" status="done">Analyzing design structure...</step>'
  const subtaskSteps = plan.subtasks
    .map((st, i) => {
      const entry = progress.subtasks[i]
      const status = entry.status
      const nodeInfo = entry.nodeCount > 0 ? ` (${entry.nodeCount} elements)` : ''
      return `<step title="${st.label}${nodeInfo}" status="${status}"></step>`
    })
    .join('\n')
  return `${planningStep}\n${subtaskSteps}`
}
