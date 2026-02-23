/**
 * Orchestrator for parallel design generation.
 *
 * Flow:
 * 1. Fast "architect" API call decomposes the prompt into spatial sub-tasks
 * 2. Root frame is created on canvas
 * 3. Multiple sub-agents execute in parallel, each streaming JSONL
 * 4. Nodes are inserted to canvas in real-time with animation
 * 5. Post-generation screenshot validation (optional, requires API key)
 *
 * Falls back to single-call generation on any orchestrator failure.
 */

import type { PenNode, FrameNode } from '@/types/pen'
import type {
  AIDesignRequest,
  OrchestratorPlan,
  OrchestrationProgress,
  SubAgentResult,
} from './ai-types'
import { streamChat } from './ai-service'
import { ORCHESTRATOR_PROMPT } from './orchestrator-prompts'
import {
  buildFallbackPlanFromPrompt,
  getOrchestratorTimeouts,
  prepareDesignPrompt,
} from './orchestrator-prompt-optimizer'
import {
  adjustRootFrameHeightToContent,
  insertStreamingNode,
  resetGenerationRemapping,
  setGenerationContextHint,
  setGenerationCanvasWidth,
} from './design-generator'
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { zoomToFitContent } from '@/canvas/use-fabric-canvas'
import { resetAnimationState } from './design-animation'
import { runPostGenerationValidation } from './design-validation'
import { executeSubAgentsSequentially } from './orchestrator-sub-agent'
import { emitProgress, buildFinalStepTags } from './orchestrator-progress'

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
  setGenerationContextHint(request.prompt)
  const animated = callbacks?.animated ?? false
  const preparedPrompt = prepareDesignPrompt(request.prompt)

  const renderPlanningStatus = (message: string) => {
    callbacks?.onTextUpdate?.(
      `<step title="Planning layout" status="streaming">${message}</step>`,
    )
  }

  if (preparedPrompt.wasCompressed) {
    console.log(
      '[Orchestrator] Compressed long prompt:',
      `${preparedPrompt.originalLength} -> ${preparedPrompt.subAgentPrompt.length} chars`,
    )
  }

  try {
    // -- Phase 1: Planning (streaming) --
    renderPlanningStatus('Analyzing design structure...')

    let plan: OrchestratorPlan
    try {
      plan = await callOrchestrator(
        preparedPrompt.orchestratorPrompt,
        preparedPrompt.originalLength,
        (thinking) => {
          renderPlanningStatus(thinking)
        },
      )
    } catch (err) {
      const message = err instanceof Error ? err.message : 'Unknown orchestrator error'
      console.warn('[Orchestrator] Planner failed, using fallback plan:', message)
      renderPlanningStatus('Planner timeout, switching to fallback layout plan...')
      plan = buildFallbackPlanFromPrompt(preparedPrompt.orchestratorPrompt)
    }

    // Assign ID prefixes
    for (const st of plan.subtasks) {
      st.idPrefix = st.id
      st.parentFrameId = plan.rootFrame.id
    }

    // Set canvas width hint for accurate text height estimation
    setGenerationCanvasWidth(plan.rootFrame.width)

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

    // Mobile screens have fixed dimensions (375x812), use plan height directly.
    // Desktop: pre-allocate total planned height so the root frame's background
    // covers the entire page from the start. Without this, children streaming into
    // later sections appear outside the root's visible background area.
    const isMobile = plan.rootFrame.width <= 480
    const totalPlannedHeight = plan.subtasks.reduce((sum, st) => sum + st.region.height, 0)
    const initialHeight = isMobile
      ? (plan.rootFrame.height || 812)
      : Math.max(320, totalPlannedHeight)
    const rootNode: FrameNode = {
      id: plan.rootFrame.id,
      type: 'frame',
      name: plan.rootFrame.name,
      x: 0,
      y: 0,
      width: plan.rootFrame.width,
      height: initialHeight,
      layout: plan.rootFrame.layout ?? 'vertical',
      gap: plan.rootFrame.gap ?? 0,
      fill: (plan.rootFrame.fill as FrameNode['fill']) ?? [
        { type: 'solid', color: plan.styleGuide?.palette?.background ?? '#FFFFFF' },
      ],
      children: [],
    }

    insertStreamingNode(rootNode, null)
    if (typeof window !== 'undefined') {
      // Wait for store -> canvas sync, then fit/center around generated root frame.
      requestAnimationFrame(() => {
        requestAnimationFrame(() => zoomToFitContent())
      })
    }

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
        preparedPrompt,
        progress,
        callbacks,
      )
      if (animated) {
        adjustRootFrameHeightToContent()
      }
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

    const generatedNodeCount = allNodes.length - 1
    if (generatedNodeCount === 0) {
      throw new Error('Orchestration produced no nodes beyond root frame')
    }

    if (!animated) {
      adjustRootFrameHeightToContent()
    }
    const adjustedRoot = useDocumentStore.getState().getNodeById(DEFAULT_FRAME_ID)
    if (adjustedRoot && adjustedRoot.type === 'frame') {
      rootNode.height = adjustedRoot.height
    }

    // -- Phase 5: Visual validation (optional, non-blocking) --
    const validationEntry: OrchestrationProgress['subtasks'][number] = {
      id: '_validation',
      label: 'Validating design',
      status: 'pending',
      nodeCount: 0,
    }
    progress.subtasks.push(validationEntry)
    // Also add to plan.subtasks so buildFinalStepTags includes it
    plan.subtasks.push({
      id: '_validation',
      label: 'Validating design',
      region: { width: 0, height: 0 },
      idPrefix: '_validation',
      parentFrameId: null,
    })
    emitProgress(plan, progress, callbacks)

    try {
      const validationResult = await runPostGenerationValidation({
        onStatusUpdate: (status, message) => {
          validationEntry.status = status === 'streaming' ? 'streaming' : status === 'done' ? 'done' : status === 'error' ? 'error' : 'pending'
          validationEntry.thinking = message
          emitProgress(plan, progress, callbacks)
        },
      })
      if (validationResult.applied > 0) {
        validationEntry.nodeCount = validationResult.applied
      }
      validationEntry.status = 'done'
    } catch (err) {
      console.warn('[Orchestrator] Validation failed (non-blocking):', err instanceof Error ? err.message : err)
      validationEntry.status = 'done'
      validationEntry.thinking = 'Skipped'
    }
    emitProgress(plan, progress, callbacks)

    // Build final rawResponse that includes step tags so the chat message
    // shows the complete pipeline progress after streaming ends
    const finalStepTags = buildFinalStepTags(plan, progress)

    return { nodes: allNodes, rawResponse: finalStepTags }
  } finally {
    setGenerationContextHint('')
    setGenerationCanvasWidth(1200) // Reset to default
  }
}

// ---------------------------------------------------------------------------
// Orchestrator call — fast decomposition
// ---------------------------------------------------------------------------

async function callOrchestrator(
  prompt: string,
  timeoutHintLength: number,
  onThinking?: (thinking: string) => void,
): Promise<OrchestratorPlan> {
  console.log('[Orchestrator] Calling streamChat...')

  let rawResponse = ''
  let thinkingContent = ''

  for await (const chunk of streamChat(
    ORCHESTRATOR_PROMPT,
    [{ role: 'user', content: prompt }],
    undefined,
    getOrchestratorTimeouts(timeoutHintLength),
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
    if (!rf.id || !rf.width || (rf.height == null)) return null

    for (const st of obj.subtasks as Record<string, unknown>[]) {
      if (!st.id || !st.region) return null
    }

    const plan = obj as unknown as OrchestratorPlan

    // Extract optional styleGuide if present and well-formed
    if (obj.styleGuide && typeof obj.styleGuide === 'object') {
      const sg = obj.styleGuide as Record<string, unknown>
      if (sg.palette && typeof sg.palette === 'object' && sg.fonts && typeof sg.fonts === 'object') {
        plan.styleGuide = sg as unknown as import('./ai-types').StyleGuide
      }
    }

    return plan
  } catch {
    return null
  }
}
