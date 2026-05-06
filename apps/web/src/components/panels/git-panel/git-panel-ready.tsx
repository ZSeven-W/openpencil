// apps/web/src/components/panels/git-panel/git-panel-ready.tsx
//
// Ready-状态身体作曲家。 Orchestrates 四个子组件
// 组成就绪状态：标头、需要保存的警报（有条件）、
// 提交输入和历史列表。 Also 每当我们触发 loadLog
// 进入就绪状态，以便历史列表有内容显示。

import { useGitPanelLogLoader } from './use-git-panel-log-loader';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelSaveRequiredAlert } from './git-panel-save-required-alert';
import { GitPanelCommitInput } from './git-panel-commit-input';
import { GitPanelHistoryList } from './git-panel-history-list';

// Phase 7b：组件外部的稳定常量，因此数组标识在每次渲染时都不会更改，从而避免虚假的 loadLog 重新触发。
const READY_KINDS = ['ready'] as const;

export function GitPanelReady() {
  // Phase 7b：加载当前分支的日志（不是硬编码的“main”）。
  useGitPanelLogLoader(READY_KINDS);

  return (
    <div className="flex h-full flex-col">
      <GitPanelHeader />
      <GitPanelSaveRequiredAlert />
      <GitPanelCommitInput />
      <div className="flex-1 overflow-y-auto">
        <GitPanelHistoryList />
      </div>
    </div>
  );
}
