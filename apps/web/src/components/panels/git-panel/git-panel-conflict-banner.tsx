// apps/web/src/components/panels/git-panel/git-panel-conflict-banner.tsx
//
// Phase 7b：升级冲突横幅。 Replaces Phase 5 仅中止 shell
// 以及 Phase 6b 非操作条，具有统一的状态标头：
//   - 显示标题 + resolved/total 进度计数
//   - 列出 non-.op 存在的未解析文件
//   - 动态主按钮：
//       * 当未解决的 .op 冲突仍然存在时应用合并（“Apply merge”）
//       * 继续 ("Continue") 当 .op 冲突全部解决且仅
// 终端解析的 non-.op 文件仍待处理
//   - 始终可见的中止合并（“Abort merge”）按钮
//   - 来自 applyMerge() 的内联 finalizeError 抛出合并仍然冲突
//
// The 横幅位于面板标题下方的 <GitPanelConflict /> 内。

import { AlertTriangle, AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

export function GitPanelConflictBanner() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const abortMerge = useGitStore((s) => s.abortMerge);
  const applyMerge = useGitStore((s) => s.applyMerge);

  // The 横幅仅由 GitPanelConflict 挂载，因此 state.kind 在这里始终是“冲突”。 The 缩小为
  // TypeScript。
  const unresolvedFiles = state.kind === 'conflict' ? state.unresolvedFiles : [];
  const hasNonOpConflict = unresolvedFiles.length > 0;
  const finalizeError = state.kind === 'conflict' ? state.finalizeError : null;
  // I2：面板在合并中重新打开 - 内存中冲突状态丢失。
  const reopenedMidMerge = state.kind === 'conflict' ? state.reopenedMidMerge : false;

  // Count 已解决进度显示与总 .op 冲突。
  let resolvedCount = 0;
  let totalCount = 0;
  if (state.kind === 'conflict') {
    const { nodeConflicts, docFieldConflicts } = state.conflicts;
    for (const c of nodeConflicts.values()) {
      totalCount++;
      if (c.resolution != null) resolvedCount++;
    }
    for (const c of docFieldConflicts.values()) {
      totalCount++;
      if (c.resolution != null) resolvedCount++;
    }
  }

  // Determine 主要操作标签。 Rule： - 未解决的 .op
  // 冲突仍然存在 →“Apply 合并”（或“Apply”，当还有 non-.op 文件时，表示我们已经过了 .op 阶段） - 所有
  // .op 已解决（或零 .op 冲突）+ non-.op 文件待处理 →“Continue”
  const opUnresolved = totalCount - resolvedCount;
  const useApplyLabel = opUnresolved > 0 || (totalCount === 0 && !hasNonOpConflict);
  const primaryLabel = useApplyLabel
    ? t('git.conflict.banner.apply')
    : t('git.conflict.banner.continue');

  const showProgress = totalCount > 0;
  // I2：在面板重新打开降级模式下，完全隐藏主按钮 - 只有中止按钮可操作。
  const showPrimaryButton = !reopenedMidMerge && (useApplyLabel || hasNonOpConflict);

  return (
    <div
      role="alert"
      className="flex flex-col gap-2 border-b border-destructive/30 bg-destructive/10 px-4 py-3 text-destructive"
    >
      {/* Title + 进度 */}
      <div className="flex items-start gap-2">
        <AlertTriangle size={14} className="mt-0.5 shrink-0" aria-hidden />
        <div className="flex flex-col gap-0.5 flex-1">
          <div className="flex items-center justify-between gap-2">
            <p className="text-xs font-medium">
              {hasNonOpConflict ? t('git.conflict.nonOp.title') : t('git.conflict.title')}
            </p>
            {showProgress && (
              <span className="text-[10px] tabular-nums shrink-0">
                {t('git.conflict.banner.progress', {
                  resolved: resolvedCount,
                  total: totalCount,
                })}
              </span>
            )}
          </div>
          <p className="text-xs opacity-80">
            {hasNonOpConflict ? t('git.conflict.nonOp.description') : t('git.conflict.description')}
          </p>
        </div>
      </div>

      {/* I2：面板重新打开警告 — 显示而不是正常的主要操作 */}
      {reopenedMidMerge && (
        <div className="flex items-start gap-1.5 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <AlertCircle size={11} className="mt-0.5 shrink-0" aria-hidden />
          <p className="text-[11px]">{t('git.conflict.banner.reopenMessage')}</p>
        </div>
      )}

      {/* Non-.op 未解析的文件列表 */}
      {hasNonOpConflict && (
        <div className="flex flex-col gap-1 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <div className="text-[11px] font-medium text-destructive">
            {t('git.conflict.nonOp.unresolvedHeading', { count: unresolvedFiles.length })}
          </div>
          <ul className="flex flex-col gap-0.5">
            {unresolvedFiles.map((path) => (
              <li key={path} className="text-[11px] text-foreground font-mono">
                {path}
              </li>
            ))}
          </ul>
        </div>
      )}

      {/* Inline 最终确定来自合并仍然冲突的错误 */}
      {finalizeError != null && (
        <div className="flex items-start gap-1.5 rounded border border-destructive/30 bg-background/40 px-2 py-1.5">
          <AlertCircle size={11} className="mt-0.5 shrink-0" aria-hidden />
          <p className="text-[11px]">
            {t('git.conflict.banner.finalizeError', { message: finalizeError })}
          </p>
        </div>
      )}

      {/* Action 按钮 */}
      <div className="flex justify-end gap-2">
        <Button type="button" variant="outline" size="sm" onClick={() => void abortMerge()}>
          {t('git.conflict.abort')}
        </Button>
        {showPrimaryButton && (
          <Button type="button" variant="default" size="sm" onClick={() => void applyMerge()}>
            {primaryLabel}
          </Button>
        )}
      </div>
    </div>
  );
}
