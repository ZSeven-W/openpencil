import type { AppendContext, OrchestratorPlan } from './ai-types';

const STATUS_BAR_SUBTASK_RE = /(status\s*bar|status_bar|status-bar|system chrome|系统栏|状态栏)/i;

export interface AppendPlanResult {
  plan: OrchestratorPlan;
  skipRootInsertion: boolean;
  skipStatusBar: boolean;
}

/**
 * Mutates/returns the plan according to an AppendContext:
 * - Repoints `rootFrame.id` to `targetParentId` so sub-agent sections are
 *   inserted as children of the existing page content root.
 * - Drops any planner-emitted status-bar subtasks (existing page already has one).
 * - Carries `existingSectionLabels` through to each remaining subtask so the
 *   sub-agent prompt can instruct the model not to regenerate them.
 */
export function applyAppendContextToPlan(
  plan: OrchestratorPlan,
  append: AppendContext | undefined,
): AppendPlanResult {
  if (!append) {
    return { plan, skipRootInsertion: false, skipStatusBar: false };
  }
  plan.rootFrame.id = append.targetParentId;
  plan.rootFrame.width = append.targetWidth;
  plan.subtasks = plan.subtasks
    .filter((st) => !STATUS_BAR_SUBTASK_RE.test(`${st.id} ${st.label}`))
    .map((st) => ({ ...st, existingSectionLabels: append.existingSectionLabels }));
  return { plan, skipRootInsertion: true, skipStatusBar: true };
}
