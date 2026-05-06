// apps/web/src/components/panels/git-panel/git-panel-conflict.tsx
//
// 当 state.kind === 'conflict' 时显示 Body。 Mirrors 如何 GitPanelReady
// 组成就绪状态：面板标题位于顶部（带有分支
// 在合并中切换禁用），破坏性冲突横幅是
// next，历史列表以剩余可滚动空间为
// 只读上下文。 There 故意不提交输入 — 提交
// 在冲突期间，在 Phase 7 登陆之前，不属于法律诉讼
// 分辨率。
//
// Phase 7b：对 non-.op 未解析文件的轮询位于此处，因此仅
// 在冲突工作区可见时运行。

import { useEffect, useRef, useState } from 'react';
import { AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useGitStore } from '@/stores/git-store';
import { useGitPanelLogLoader } from './use-git-panel-log-loader';
import { GitPanelHeader } from './git-panel-header';
import { GitPanelConflictBanner } from './git-panel-conflict-banner';
import { GitPanelConflictList } from './git-panel-conflict-list';
import { GitPanelHistoryList } from './git-panel-history-list';

const CONFLICT_KINDS = ['conflict'] as const;
const POLL_INTERVAL_MS = 3000;

export function GitPanelConflict() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const refreshStatus = useGitStore((s) => s.refreshStatus);

  // Phase 7b：加载当前分支的日志（不是硬编码的“main”）。
  useGitPanelLogLoader(CONFLICT_KINDS);

  // Phase 7b：每 3 秒轮询一次 refreshStatus，当有未解决的问题时
  // non-.op 文件。 This 允许在用户解决问题时更新横幅
  // 它们可以在外部编辑器中使用，而无需重新安装面板。
//
  // Lifecycle 规则：
  //   - 仅当 state.kind === 'conflict' AND unresolvedFiles.length > 0 时才进行轮询
  //   - 通过飞行参考跳过重叠的民意调查
  //   - 停止轮询第一个错误（出现一次错误，然后停止）
  //   - 卸载时清理
  const inFlightRef = useRef<boolean>(false);
  const pollStoppedRef = useRef<boolean>(false);
  const [pollError, setPollError] = useState<string | null>(null);

  const unresolvedCount = state.kind === 'conflict' ? state.unresolvedFiles.length : 0;
  const shouldPoll = state.kind === 'conflict' && unresolvedCount > 0;

  useEffect(() => {
    if (!shouldPoll) return;
    // 每个新轮询会话上的 Reset 错误状态（例如，状态刷新后 unresolvedCount 变为 0 → 非零）。
    setPollError(null);
    pollStoppedRef.current = false;
    let cancelled = false;

    const id = setInterval(async () => {
      // Skip 如果刷新已在进行中。
      if (inFlightRef.current || pollStoppedRef.current) return;

      inFlightRef.current = true;
      try {
        await refreshStatus();
      } catch (err) {
        if (cancelled) return;
        pollStoppedRef.current = true;
        setPollError(err instanceof Error ? err.message : String(err));
      } finally {
        inFlightRef.current = false;
      }
    }, POLL_INTERVAL_MS);

    return () => {
      cancelled = true;
      clearInterval(id);
      inFlightRef.current = false;
    };
  }, [shouldPoll, refreshStatus]);

  return (
    <div className="flex h-full flex-col">
      <GitPanelHeader />
      <GitPanelConflictBanner />
      {pollError !== null && (
        <div className="mx-3 mb-2 flex items-start gap-1.5 rounded border border-destructive/20 bg-destructive/10 px-2 py-1.5 text-xs text-destructive">
          <AlertCircle className="mt-px size-3 shrink-0" />
          <span>{t('git.conflict.banner.pollError', { message: pollError })}</span>
        </div>
      )}
      {/* Phase 7c：冲突解决列表 — 安装在横幅和历史记录之间 */}
      <GitPanelConflictList />
      <div className="flex-1 overflow-y-auto">
        <GitPanelHistoryList readOnly />
      </div>
    </div>
  );
}
