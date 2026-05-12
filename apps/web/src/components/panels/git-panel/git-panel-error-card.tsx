// apps/web/src/components/panels/git-panel/git-panel-error-card.tsx
//
// Generic state.kind === 'error' 的错误显示。 Shows 错误消息
// 来自 GitError、一个可选的重试按钮（可恢复时）和一个
// 关闭按钮（调用 closeRepo 重置为无文件）。
//
// Phase 4a 将此用于 init/open/clone 错误路径。 Phase 4c 将
// 通过面板标题指示器重用它来处理自动保存错误。

import { AlertCircle } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import { cn } from '@/lib/utils';

interface GitPanelErrorCardProps {
  message: string;
  recoverable: boolean;
  /** Optional 重试处理程序。 If 省略，重试按钮调用 closePanel。 */
  onRetry?: () => void;
  onDismiss?: () => void;
  className?: string;
}

export function GitPanelErrorCard({
  message,
  recoverable,
  onRetry,
  onDismiss,
  className,
}: GitPanelErrorCardProps) {
  const { t } = useTranslation();
  const closeRepo = useGitStore((s) => s.closeRepo);
  const dismiss = onDismiss ?? (() => void closeRepo());

  return (
    <div className={cn('flex flex-col items-center justify-center gap-3 p-6 text-center', className)}>
      <AlertCircle size={28} className="text-destructive" aria-hidden />
      <div className="text-sm font-medium text-foreground">{t('git.error.title')}</div>
      <div className="text-xs text-muted-foreground max-w-[280px] break-words">{message}</div>
      <div className="flex items-center gap-2 pt-1">
        {recoverable && (
          <Button
            type="button"
            variant="default"
            size="sm"
            onClick={() => {
              if (onRetry) {
                onRetry();
              } else {
                dismiss();
              }
            }}
          >
            {t('git.error.retry')}
          </Button>
        )}
        <Button type="button" variant="ghost" size="sm" onClick={dismiss}>
          {t('git.error.dismiss')}
        </Button>
      </div>
    </div>
  );
}
