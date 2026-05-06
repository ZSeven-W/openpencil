/**
 * 预校验：一组不依赖 LLM 的纯代码检查。
 *
 * 这个文件现在只是 `@zseven-w/pen-ai-skills` 诊断能力的一层薄包装。
 * 真正的检测逻辑都以纯函数形式放在
 * `packages/pen-ai-skills/src/diagnostics/detectors.ts` 里，
 * 这样调试工具和业务代码都能复用它们，而不会引入副作用。
 */

import { detectAllIssues, type Issue } from '@zseven-w/pen-ai-skills';
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

/**
 * 对当前实时文档运行预校验检测器，并应用建议的自动修复。
 *
 * 返回值表示“实际应用了多少个修复”，而不是检测到多少个问题。
 * 被跳过的问题不会计数，比如：
 * - `info` 级别的问题（只检测，不自动修）
 * - 被保护节点上的删除动作（例如状态栏）
 *
 * 因此调用方可以把返回值当作“文档是否真的被改动过”的可靠信号。
 */
export function runPreValidationFixes(): number {
  const store = useDocumentStore.getState();
  const root = store.getNodeById(DEFAULT_FRAME_ID);
  if (!root) return 0;

  const issues = detectAllIssues(root, store.document);
  return applyFixes(issues);
}

function applyFixes(issues: Issue[]): number {
  const store = useDocumentStore.getState();
  let applied = 0;
  for (const issue of issues) {
    // `info` 级别只做检测，不自动修复。
    // 这类问题通常更偏“提示”而不是“确定性错误”，
    // 贸然改写可能会误伤一些结构上本来就特殊的兄弟节点。
    if (issue.severity === 'info') continue;

    if (issue.property === '__remove') {
      // 不允许删除预注入的界面 chrome，例如 iPhone 状态栏。
      const target = store.getNodeById(issue.nodeId);
      if (target && 'role' in target && (target as { role?: string }).role === 'status-bar') {
        console.log(`[Pre-validation] ${issue.nodeId}: skipped removal (protected status-bar)`);
        continue;
      }
      store.removeNode(issue.nodeId);
      applied++;
      console.log(`[Pre-validation] ${issue.nodeId}: removed (${issue.reason})`);
    } else {
      store.updateNode(issue.nodeId, {
        [issue.property]: issue.suggestedValue,
      } as Partial<PenNode>);
      applied++;
      console.log(
        `[Pre-validation] ${issue.nodeId}: ${issue.property} → ${JSON.stringify(issue.suggestedValue)} (${issue.reason})`,
      );
    }
  }
  return applied;
}
