/**
 * Progress emission utilities for the orchestrator.
 *
 * Formats orchestration progress as `<step>` tags so the chat panel can
 * render a live pipeline view during design generation.
 */

import type { OrchestratorPlan, OrchestrationProgress } from './ai-types'

// ---------------------------------------------------------------------------
// Progress emission — updates UI via <step> tags
// ---------------------------------------------------------------------------

export function emitProgress(
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
      const thinkingContent = entry.thinking ?? ''
      return `<step title="${st.label}${nodeInfo}" status="${status}">${thinkingContent}</step>`
    })
    .join('\n')

  let output = `${planningStep}\n${subtaskSteps}`
  if (streamingText) {
    output += '\n\n' + streamingText
  }
  callbacks.onTextUpdate(output)
}

/** Build step tags for the final rawResponse (shown in message after streaming ends) */
export function buildFinalStepTags(
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
