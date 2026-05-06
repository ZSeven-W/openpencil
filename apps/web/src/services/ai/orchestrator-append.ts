import type { AppendContext, OrchestratorPlan } from './ai-types';

const STATUS_BAR_SUBTASK_RE = /(status\s*bar|status_bar|status-bar|system chrome|系统栏|状态栏)/i;

export interface AppendPlanResult {
  plan: OrchestratorPlan;
  skipRootInsertion: boolean;
  skipStatusBar: boolean;
}

/**
 * Mutates/retu
 * rns 根据 AppendContext 进行计划： - Repoints `rootFrame.id` 到
 * `targetParentId` 因此子代理部分将作为现有页面内容根的子项插入。 - Drops
 * 任何规划器发出的状态栏子任务（现有页面已经有一个）。 - Carries
 * `existingSectionLabels` 贯穿每个剩余的子任务，以便子代理提示可以指示模型不要重新生成它们。
 *
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
