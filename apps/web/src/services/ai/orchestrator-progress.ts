/**
 * Progress
 *
 * 协调器的发射实用程序。 Formats 编排进度作为 `<step>`
 * 标签，以便聊天面板可以在
 设计生成期间呈现实时管道视图。
 */

import type { OrchestratorPlan, OrchestrationProgress } from './ai-types';

// ---------------------------------------------------------------------------
// Progress 发射 — 通过 <step> 标签更新 UI
// ---------------------------------------------------------------------------

export function emitProgress(
  plan: OrchestratorPlan,
  progress: OrchestrationProgress,
  callbacks?: {
    onTextUpdate?: (text: string) => void;
  },
  streamingText?: string,
): void {
  if (!callbacks?.onTextUpdate) return;

  // Always 显示“Planning 布局”，如首先完成的那样
  const planningStep =
    '<step title="Planning layout" status="done">Analyzing design structure...</step>';

  const subtaskSteps = plan.subtasks
    .map((st, i) => {
      const entry = progress.subtasks[i];
      const status =
        entry.status === 'streaming'
          ? 'streaming'
          : entry.status === 'done'
            ? 'done'
            : entry.status === 'error'
              ? 'error'
              : 'pending';
      const nodeInfo = entry.nodeCount > 0 ? ` (${entry.nodeCount} elements)` : '';
      return `<step title="${st.label}${nodeInfo}" status="${status}"></step>`;
    })
    .join('\n');

  let output = `${planningStep}\n${subtaskSteps}`;
  if (streamingText) {
    output += '\n\n' + streamingText;
  }
  callbacks.onTextUpdate(output);
}

/** Build step tags for the final rawResponse (shown in message after streaming ends) */
export function buildFinalStepTags(
  plan: OrchestratorPlan,
  progress: OrchestrationProgress,
): string {
  const planningStep =
    '<step title="Planning layout" status="done">Analyzing design structure...</step>';
  const subtaskSteps = plan.subtasks
    .map((st, i) => {
      const entry = progress.subtasks[i];
      const status = entry.status;
      const nodeInfo = entry.nodeCount > 0 ? ` (${entry.nodeCount} elements)` : '';
      // Preserve thinking content so validation details remain visible after streaming
      const thinkingContent = entry.thinking ?? '';
      return `<step title="${st.label}${nodeInfo}" status="${status}">${thinkingContent}</step>`;
    })
    .join('\n');
  return `${planningStep}\n${subtaskSteps}`;
}
