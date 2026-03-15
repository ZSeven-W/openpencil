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
  getOrchestratorTimeouts,
  prepareDesignPrompt,
} from './orchestrator-prompt-optimizer'
import { resolveModelProfile, needsSimplifiedPrompt } from './model-profiles'
import {
  adjustRootFrameHeightToContent,
  insertStreamingNode,
  resetGenerationRemapping,
  setGenerationContextHint,
  setGenerationCanvasWidth,
  getGenerationRemappedIds,
  getGenerationRootFrameId,
} from './design-generator'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import { zoomToFitContent } from '@/canvas/skia-engine-ref'
import { resetAnimationState } from './design-animation'
import { VALIDATION_ENABLED } from './ai-runtime-config'
import { runPostGenerationValidation } from './design-validation'
import { executeSubAgents } from './orchestrator-sub-agent'
import { emitProgress, buildFinalStepTags } from './orchestrator-progress'
import { assignAgentIdentities } from './agent-identity'
import { addAgentFrame, clearAgentIndicators } from '@/canvas/agent-indicator'

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
  abortSignal?: AbortSignal,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  setGenerationContextHint(request.prompt)
  const animated = callbacks?.animated ?? false
  const preparedPrompt = prepareDesignPrompt(request.prompt)

  const renderPlanningStatus = (message: string) => {
    callbacks?.onTextUpdate?.(
      `<step title="Planning layout" status="streaming">${message}</step>`,
    )
  }

  try {
    // -- Phase 1: Planning (streaming) --
    renderPlanningStatus('Analyzing design structure...')

    const plan = await callOrchestrator(
      preparedPrompt.orchestratorPrompt,
      preparedPrompt.originalLength,
      request.model,
      request.provider,
      (thinking) => {
        renderPlanningStatus(thinking)
      },
      abortSignal,
    )

    // Assign ID prefixes
    for (const st of plan.subtasks) {
      st.idPrefix = st.id
      st.parentFrameId = plan.rootFrame.id
    }

    // Set canvas width hint for accurate text height estimation
    setGenerationCanvasWidth(plan.rootFrame.width)

    // Set context hint once with all subtask labels to avoid race conditions
    // during concurrent sub-agent execution
    setGenerationContextHint(
      request.prompt + ' ' + plan.subtasks.map((st) => st.label).join(' '),
    )

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
    const concurrency = request.concurrency ?? 1

    // Group subtasks by screen for concurrent mode.
    // Only use concurrent path when there are MULTIPLE distinct screens.
    // Single-page designs always use the sequential path (proven, simpler).
    const screenGroups: { screen: string; indices: number[] }[] = []
    if (concurrency > 1) {
      const hasAnyScreen = plan.subtasks.some((st) => st.screen)
      if (hasAnyScreen) {
        const screenMap = new Map<string, number>()
        const firstScreen = plan.subtasks.find((st) => st.screen)?.screen ?? 'page'
        for (let i = 0; i < plan.subtasks.length; i++) {
          const screen = plan.subtasks[i].screen ?? firstScreen
          if (screenMap.has(screen)) {
            screenGroups[screenMap.get(screen)!].indices.push(i)
          } else {
            screenMap.set(screen, screenGroups.length)
            screenGroups.push({ screen, indices: [i] })
          }
        }
      }
    }

    // Effective concurrency: only parallel when there are multiple screen groups
    const effectiveConcurrency = screenGroups.length > 1 ? concurrency : 1

    // Assign agent identities — one per screen group (concurrent) or per subtask (sequential)
    const subtaskIdentity = new Map<number, { color: string; name: string }>()
    if (effectiveConcurrency > 1) {
      const agentIdentities = assignAgentIdentities(screenGroups.length)
      for (let g = 0; g < screenGroups.length; g++) {
        if (agentIdentities[g]) {
          for (const idx of screenGroups[g].indices) {
            subtaskIdentity.set(idx, agentIdentities[g])
          }
        }
      }
    } else {
      // Sequential mode: single agent handles all subtasks
      const [identity] = assignAgentIdentities(1)
      if (identity) {
        for (let i = 0; i < plan.subtasks.length; i++) {
          subtaskIdentity.set(i, identity)
        }
      }
    }

    if (animated) {
      resetAnimationState()
      useHistoryStore.getState().startBatch(useDocumentStore.getState().document)
    }

    const isMobile = plan.rootFrame.width <= 480
    const defaultFill: FrameNode['fill'] = (plan.rootFrame.fill as FrameNode['fill']) ?? [
      { type: 'solid', color: plan.styleGuide?.palette?.background ?? '#FFFFFF' },
    ]

    // Track all root frame nodes for result collection
    const rootNodes: FrameNode[] = []

    if (effectiveConcurrency > 1) {
      // Concurrent mode: create one root frame per screen group.
      // Subtasks sharing the same screen insert into the same root frame.
      //
      // IMPORTANT: insertStreamingNode(node, null) has heavy side effects —
      // it may replace the default empty frame (remapping the node ID to
      // DEFAULT_FRAME_ID) and mutates generationRootFrameId. We only call it
      // for the first frame to handle the empty-canvas case. Subsequent frames
      // are inserted with addNode directly to avoid ID remapping and state
      // corruption.
      const { addNode } = useDocumentStore.getState()
      const remappedIds = getGenerationRemappedIds()
      const gap = 100
      let nextX = 0

      for (let g = 0; g < screenGroups.length; g++) {
        const group = screenGroups[g]
        const firstSt = plan.subtasks[group.indices[0]]
        const originalId = `${plan.rootFrame.id}-${group.screen}`

        // Height: sum of all subtask regions in this group (mobile uses fixed viewport)
        const totalRegionHeight = group.indices.reduce(
          (sum, i) => sum + plan.subtasks[i].region.height, 0,
        )
        const frameHeight = isMobile
          ? (plan.rootFrame.height || 812)
          : Math.max(320, totalRegionHeight)

        // Frame name: use screen name if available, else first subtask's short name
        const frameName = firstSt.screen
          ? firstSt.screen
          : (firstSt.label.replace(/\s*[（(].+$/, '').trim() || firstSt.label)

        const rootNode: FrameNode = {
          id: originalId,
          type: 'frame',
          name: frameName,
          x: nextX,
          y: 0,
          width: plan.rootFrame.width,
          height: frameHeight,
          layout: plan.rootFrame.layout ?? 'vertical',
          gap: isMobile ? (plan.rootFrame.gap || 16) : (plan.rootFrame.gap ?? 16),
          ...(plan.rootFrame.padding != null ? { padding: plan.rootFrame.padding } : {}),
          fill: defaultFill,
          children: [],
        }

        if (g === 0) {
          // First frame: use insertStreamingNode to handle empty canvas replacement
          insertStreamingNode(rootNode, null)
          const actualId = remappedIds.get(originalId) ?? originalId
          for (const idx of group.indices) {
            plan.subtasks[idx].parentFrameId = actualId
          }
          rootNode.id = actualId
        } else {
          addNode(null, rootNode)
          for (const idx of group.indices) {
            plan.subtasks[idx].parentFrameId = originalId
          }
        }

        rootNodes.push(rootNode)

        // Register agent badge on the root frame immediately
        const identity = subtaskIdentity.get(group.indices[0])
        if (identity) {
          addAgentFrame(rootNode.id, identity.color, identity.name)
        }

        nextX += plan.rootFrame.width + gap
      }
    } else {
      // Sequential mode: single root frame containing all sections
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
        gap: isMobile ? (plan.rootFrame.gap || 16) : (plan.rootFrame.gap ?? 16),
        ...(plan.rootFrame.padding != null ? { padding: plan.rootFrame.padding } : {}),
        fill: defaultFill,
        children: [],
      }
      insertStreamingNode(rootNode, null)
      // insertStreamingNode may remap ID (e.g. replacing empty frame)
      const actualRootId = getGenerationRootFrameId()
      rootNode.id = actualRootId
      rootNodes.push(rootNode)

      // Register agent badge on the actual root frame
      const firstIdentity = subtaskIdentity.get(0)
      if (firstIdentity) {
        addAgentFrame(actualRootId, firstIdentity.color, firstIdentity.name)
      }
    }

    if (typeof window !== 'undefined') {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => zoomToFitContent())
      })
    }

    // -- Phase 3: Parallel sub-agent execution --
    const progress: OrchestrationProgress = {
      phase: 'generating',
      subtasks: plan.subtasks.map((st, i) => {
        const identity = subtaskIdentity.get(i)
        return {
          id: st.id,
          label: st.label,
          status: 'pending' as const,
          nodeCount: 0,
          ...(identity ? { agentColor: identity.color, agentName: identity.name } : {}),
        }
      }),
      totalNodes: 0,
    }

    let results: SubAgentResult[]
    try {
      results = await executeSubAgents(
        plan,
        request,
        preparedPrompt,
        progress,
        effectiveConcurrency,
        callbacks,
        abortSignal,
      )
      if (animated) {
        if (effectiveConcurrency > 1) {
          for (const rn of rootNodes) {
            adjustRootFrameHeightToContent(rn.id)
          }
        } else {
          adjustRootFrameHeightToContent()
        }
      }
    } finally {
      if (animated) {
        useHistoryStore.getState().endBatch(useDocumentStore.getState().document)
      }
    }

    // -- Phase 4: Collect results --
    const aborted = abortSignal?.aborted ?? false

    if (!aborted) {
      for (const entry of progress.subtasks) {
        if (entry.status !== 'error') {
          entry.status = 'done'
        }
      }
      progress.phase = 'done'
    } else {
      for (const entry of progress.subtasks) {
        if (entry.status === 'streaming') {
          entry.status = 'pending'
        }
      }
      progress.phase = 'done'
    }
    emitProgress(plan, progress, callbacks)

    const allNodes: PenNode[] = [...rootNodes]
    for (const r of results) {
      allNodes.push(...r.nodes)
    }

    const generatedNodeCount = allNodes.length - rootNodes.length
    if (generatedNodeCount === 0 && !aborted) {
      throw new Error('Orchestration produced no nodes beyond root frame')
    }

    if (!animated) {
      if (effectiveConcurrency > 1) {
        for (const rn of rootNodes) {
          adjustRootFrameHeightToContent(rn.id)
        }
      } else {
        adjustRootFrameHeightToContent()
      }
    }
    // Sync heights back to rootNode objects for result
    for (const rn of rootNodes) {
      const adjusted = useDocumentStore.getState().getNodeById(rn.id)
      if (adjusted && adjusted.type === 'frame') {
        rn.height = adjusted.height
      }
    }

    // -- Phase 5: Visual validation (skip if user stopped or disabled) --
    if (!aborted && VALIDATION_ENABLED) {
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
          model: request.model,
          provider: request.provider,
        })
        if (validationResult.applied > 0) {
          validationEntry.nodeCount = validationResult.applied
        }
        validationEntry.status = 'done'
      } catch {
        validationEntry.status = 'done'
        validationEntry.thinking = 'Skipped'
      }
      emitProgress(plan, progress, callbacks)
    }

    // Build final rawResponse that includes step tags so the chat message
    // shows the complete pipeline progress after streaming ends
    const finalStepTags = buildFinalStepTags(plan, progress)

    return { nodes: allNodes, rawResponse: finalStepTags }
  } finally {
    clearAgentIndicators()
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
  model?: string,
  provider?: AIDesignRequest['provider'],
  onThinking?: (thinking: string) => void,
  abortSignal?: AbortSignal,
): Promise<OrchestratorPlan> {
  let rawResponse = ''
  let thinkingContent = ''

  // For basic-tier models, inline system prompt into user message
  // since third-party routers may drop or ignore system prompts.
  const profile = resolveModelProfile(model)
  const inlineSystem = needsSimplifiedPrompt(profile)
  const effectiveSystem = inlineSystem ? '' : ORCHESTRATOR_PROMPT
  const effectiveUserContent = inlineSystem
    ? `${ORCHESTRATOR_PROMPT}\n\n---\n\nUSER REQUEST:\n${prompt}`
    : prompt

  for await (const chunk of streamChat(
    effectiveSystem,
    [{ role: 'user', content: effectiveUserContent }],
    model,
    getOrchestratorTimeouts(timeoutHintLength, model),
    provider,
    abortSignal,
  )) {
    if (chunk.type === 'text') {
      rawResponse += chunk.content
    } else if (chunk.type === 'thinking') {
      thinkingContent += chunk.content
      onThinking?.(thinkingContent)
    } else if (chunk.type === 'error') {
      throw new Error(chunk.content)
    }
  }

  const plan = parseOrchestratorResponse(rawResponse)
  if (!plan) {
    const preview = rawResponse.trim().slice(0, 150)
    const hint = rawResponse.trim().length === 0
      ? 'The model returned an empty response.'
      : `Model output: "${preview}${rawResponse.length > 150 ? '…' : ''}"`
    throw new Error(`Could not parse design plan from model response. ${hint}`)
  }

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

    // Extract styleGuide — required for consistent visual output
    if (obj.styleGuide && typeof obj.styleGuide === 'object') {
      const sg = obj.styleGuide as Record<string, unknown>
      if (sg.palette && typeof sg.palette === 'object' && sg.fonts && typeof sg.fonts === 'object') {
        plan.styleGuide = sg as unknown as import('./ai-types').StyleGuide
      }
    }

    // Fallback: always provide a style guide so sub-agents have consistent styling
    if (!plan.styleGuide) {
      const bg = (plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color ?? '#F8FAFC'
      plan.styleGuide = {
        palette: {
          background: bg,
          surface: '#FFFFFF',
          text: '#0F172A',
          secondary: '#64748B',
          accent: '#6366F1',
          accent2: '#8B5CF6',
          border: '#E2E8F0',
        },
        fonts: { heading: 'Space Grotesk', body: 'Inter' },
        aesthetic: 'clean modern',
      }
    }

    return plan
  } catch {
    return null
  }
}
