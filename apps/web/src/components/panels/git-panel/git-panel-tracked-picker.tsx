// apps/web/src/components/panels/git-panel/git-panel-tracked-picker.tsx
//
// Phase 4b：跟踪文件选择器。当 openRepo 或 cloneRepo 时为 Shown
// 返回包含多个 .op 文件的文件夹模式存储库。 The 用户精选
// Git 面板应跟踪哪个文件。 Two 操作按钮：
//   - 跟踪此文件（仅跟踪）：bindTrackedFile，面板转换为就绪状态
//   - 跟踪并打开（跟踪并打开）：bindTrackedFile + 将文件加载到
// 编辑器通过 loadOpFileFromPath 助手
//
// The 零候选边缘情况渲染一个小空卡提示
// 用户关闭面板（以及底层存储库会话）。
//
// The 恰好一个候选路径在商店的 openRepo / 中处理
// cloneRepo 操作（自动绑定），因此该组件永远不必显示
// 单行选择器。

import { Check, FileText } from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import { loadOpFileFromPath } from '@/utils/load-op-file';
import type { GitCandidateFileInfo } from '@/services/git-types';

export function GitPanelTrackedPicker() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const bindTrackedFile = useGitStore((s) => s.bindTrackedFile);
  const closePanel = useGitStore((s) => s.closePanel);
  const closeRepo = useGitStore((s) => s.closeRepo);
  // Phase 7b：exitTrackedFilePicker 驱动 back/cancel 导航规则（重新绑定时返回 →
  // 就绪，首次打开时取消 → 无文件）。
  const exitTrackedFilePicker = useGitStore((s) => s.exitTrackedFilePicker);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

  // Defensive 防护 — git-panel.tsx 的主体开关仅将我们安装在需求跟踪文件分支中，但如果状态转换与我们竞争，我们将
  // 渲染 null 而不是崩溃。
  if (state.kind !== 'needs-tracked-file') return null;

  const candidates = state.repo.candidateFiles;
  // Phase 7b：根据跟踪文件是否已绑定来确定 back/cancel 标签。 isRebind=true → 从就绪 → 后标签输入。
  // isRebind=false → 第一个 post-open/clone 屏幕 → 取消标签。
  const isRebind = state.repo.trackedFilePath !== null;
  const backLabel = isRebind ? t('git.picker.back') : t('git.picker.backClose');

  // Edge 案例：零个候选者
  if (candidates.length === 0) {
    return (
      <div className="flex flex-col items-center gap-3 p-6 text-center">
        <div className="text-sm font-medium text-foreground">{t('git.picker.empty.heading')}</div>
        <div className="text-xs text-muted-foreground max-w-[280px]">
          {t('git.picker.empty.body')}
        </div>
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={async () => {
            await closeRepo();
            closePanel();
          }}
        >
          {t('git.picker.empty.close')}
        </Button>
      </div>
    );
  }

  // Sort by lastCommitAt desc，最后为 null。 Tiebreak 在 relativePath 上升序，对于
  // ANY 相等的主键（两个空 OR 两个相等的非空时间戳），因此排序是完整且稳定的。
  const sorted = [...candidates].sort((a, b) => {
    // Primary 键：lastCommitAt desc，最后为空
    if (a.lastCommitAt !== b.lastCommitAt) {
      if (a.lastCommitAt === null) return 1;
      if (b.lastCommitAt === null) return -1;
      return b.lastCommitAt - a.lastCommitAt;
    }
    // Equal 主键（均为 null OR 均为相同的非空时间戳）：回退到 relativePath asc 作为决胜局。
    return a.relativePath.localeCompare(b.relativePath);
  });

  const handleBindOnly = async () => {
    if (!selectedPath) return;
    await bindTrackedFile(selectedPath);
  };
  const handleBindAndOpen = async () => {
    if (!selectedPath) return;
    await bindTrackedFile(selectedPath);
    const ok = await loadOpFileFromPath(selectedPath);
    void ok;
  };

  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
        {t('git.picker.heading', { count: candidates.length })}
      </div>
      <div className="flex flex-col gap-1.5">
        {sorted.map((c) => (
          <TrackedPickerRow
            key={c.path}
            candidate={c}
            selected={selectedPath === c.path}
            onSelect={() => setSelectedPath(c.path)}
            t={t}
          />
        ))}
      </div>
      <div className="flex items-center justify-between gap-2 pt-1">
        {/* Phase 7b：back/cancel 可供性 — 重新绑定时导航回就绪状态，或在首次打开时关闭临时会话。
             */}
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => void exitTrackedFilePicker()}
          className="h-7 rounded-md px-2.5 text-[11px]"
        >
          {backLabel}
        </Button>
        <div className="flex items-center gap-1.5">
          <Button
            type="button"
            variant="outline"
            size="sm"
            disabled={!selectedPath}
            onClick={() => void handleBindOnly()}
            className="h-7 rounded-md px-2.5 text-[11px]"
          >
            {t('git.picker.bindButton')}
          </Button>
          <Button
            type="button"
            variant="default"
            size="sm"
            disabled={!selectedPath}
            onClick={() => void handleBindAndOpen()}
            className="h-7 rounded-md px-2.5 text-[11px]"
          >
            {t('git.picker.bindAndOpenButton')}
          </Button>
        </div>
      </div>
    </div>
  );
}

interface TrackedPickerRowProps {
  candidate: GitCandidateFileInfo;
  selected: boolean;
  onSelect: () => void;
  t: (key: string, opts?: Record<string, unknown>) => string;
}

function TrackedPickerRow({ candidate, selected, onSelect, t }: TrackedPickerRowProps) {
  const milestoneLabel =
    candidate.milestoneCount === 0
      ? t('git.picker.noHistory')
      : t('git.picker.milestoneCount', { count: candidate.milestoneCount });

  return (
    <button
      type="button"
      onClick={onSelect}
      aria-pressed={selected}
      className={`group relative flex items-start gap-2.5 rounded-lg border p-2.5 text-left transition-all ${
        selected
          ? 'border-primary/60 bg-primary/5 shadow-[0_0_0_3px_hsl(var(--primary)/0.08)]'
          : 'border-border/70 bg-card hover:border-border hover:bg-accent/40'
      }`}
    >
      <span
        className={`mt-0.5 flex h-6 w-6 shrink-0 items-center justify-center rounded-md transition-colors ${
          selected ? 'bg-primary/15 text-primary' : 'bg-muted text-muted-foreground'
        }`}
        aria-hidden
      >
        {selected ? (
          <Check size={13} strokeWidth={2.25} />
        ) : (
          <FileText size={13} strokeWidth={1.75} />
        )}
      </span>
      <div className="flex min-w-0 flex-1 flex-col gap-0.5">
        <div className="flex w-full items-center justify-between gap-2">
          <span className="truncate text-[12px] font-medium text-foreground">
            {candidate.relativePath}
          </span>
          <span className="shrink-0 text-[10px] text-muted-foreground">{milestoneLabel}</span>
        </div>
        {candidate.lastCommitMessage && (
          <div className="w-full truncate text-[10px] text-muted-foreground/80">
            {t('git.picker.lastCommit', {
              message: candidate.lastCommitMessage,
              time: formatRelativeTime(candidate.lastCommitAt, t),
            })}
          </div>
        )}
      </div>
    </button>
  );
}

/**
 * Format a unix timestamp (seconds OR milliseconds) as a localized
 * relative time string. Returns an empty string for null timestamps.
 */
function formatRelativeTime(
  ts: number | null,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  if (ts === null) return '';
  const tsMs = ts < 1e12 ? ts * 1000 : ts;
  const diffMin = Math.floor((Date.now() - tsMs) / 60000);
  if (diffMin < 1) return t('git.relativeTime.justNow');
  if (diffMin < 60) return t('git.relativeTime.minutesAgo', { count: diffMin });
  const diffHr = Math.floor(diffMin / 60);
  if (diffHr < 24) return t('git.relativeTime.hoursAgo', { count: diffHr });
  const diffDay = Math.floor(diffHr / 24);
  return t('git.relativeTime.daysAgo', { count: diffDay });
}
