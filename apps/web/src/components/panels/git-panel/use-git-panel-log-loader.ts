// apps/web/src/components/panels/git-panel/use-git-panel-log-loader.ts
//
// Phase 7b：加载当前分支的提交日志的共享钩子
// 每当 state.kind 与 active-repo 类型之一匹配时。 Replaces 的
// GitPanelReady 和 GitPanelConflict 用来硬编码的 `ref: 'main'`
// pass — 日志应始终遵循 state.repo.currentBranch。
//
// Callers 只是调用钩子；它会在安装时触发 loadLog
// state.kind 或 state.repo.currentBranch 更改。

import { useEffect } from 'react';
import { useGitStore } from '@/stores/git-store';

/**
 * Fires `loadL
 * og({ ref: state.repo.currentBranch, limit: 50 })` 每当面板位于给定的 `kinds`
 * 之一时。 Re-当 state.kind 或 currentBranch 更改时触发，因此分支切换和冲突 → 就绪转换始终显示正确的日志。
 *
 */
export function useGitPanelLogLoader(kinds: ReadonlyArray<string>): void {
  const stateKind = useGitStore((s) => s.state.kind);
  const currentBranch = useGitStore((s) =>
    s.state.kind === 'ready' || s.state.kind === 'conflict' || s.state.kind === 'needs-tracked-file'
      ? s.state.repo.currentBranch
      : null,
  );
  const loadLog = useGitStore((s) => s.loadLog);

  useEffect(() => {
    if (!kinds.includes(stateKind)) return;
    if (currentBranch === null) return;
    void loadLog({ ref: currentBranch, limit: 50 });
  }, [stateKind, currentBranch, loadLog, kinds]);
}
