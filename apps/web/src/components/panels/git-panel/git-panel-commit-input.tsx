// apps/web/src/components/panels/git-panel/git-panel-commit-input.tsx
//
// Commit 输入：文本区域+“保存为里程碑”按钮+懒惰作者表单
// 触发。 Reads commitMessage 来自商店（因此它持续存在
// 面板在会话中重新安装）。 On 提交：
//   1. If commitMessage 为空，不执行任何操作（按钮也被禁用）。
//   2. If authorIdentity 为空，显示内联作者表单和标记
// 提交为待验证后。 The useEffect 下面重新触发
// 表单成功后提交。
//   3. Call commitMilestone；商店的 commitMilestone 操作手柄
// 成功后日志刷新+clearCommitMessage。 If 保存门
// Trips，商店设置父渲染的 saveRequiredFor
// 通过<GitPanelSaveRequiredAlert>。

import { Milestone } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import { GitPanelAuthorForm } from './git-panel-author-form';

export function GitPanelCommitInput() {
  const { t } = useTranslation();
  const commitMessage = useGitStore((s) => s.commitMessage);
  const setCommitMessage = useGitStore((s) => s.setCommitMessage);
  const authorIdentity = useGitStore((s) => s.authorIdentity);
  const authorPromptVisible = useGitStore((s) => s.authorPromptVisible);
  const showAuthorPrompt = useGitStore((s) => s.showAuthorPrompt);
  const commitMilestone = useGitStore((s) => s.commitMilestone);

  const [pendingCommitAfterAuth, setPendingCommitAfterAuth] = useState(false);

  const handleSubmit = async () => {
    const trimmed = commitMessage.trim();
    if (!trimmed) return;

    // Lazy 作者表单触发器 — 显示一次，记住我们想要提交。
    if (authorIdentity === null) {
      setPendingCommitAfterAuth(true);
      showAuthorPrompt();
      return;
    }

    try {
      await commitMilestone(trimmed, authorIdentity);
      // commitMilestone 清除 commitMessage + 刷新登录成功。 If 保存门被触发，存储集
      // saveRequiredFor 是父级通过需要保存的警报呈现的。
    } catch {
      // Swallow — 存储已转换为错误状态 OR 设置 saveRequiredFor。 No 这里需要额外的工作。
    }
  };

  // Re - 在作者表单成功后触发提交。
  useEffect(() => {
    if (pendingCommitAfterAuth && authorIdentity !== null && !authorPromptVisible) {
      setPendingCommitAfterAuth(false);
      void handleSubmit();
    }
    // deps 中故意省略了 handleSubmit。 Because 文本区域
    // 当 authorPromptVisible 为 true 时，被 <GitPanelAuthorForm> 替换，
    // commitMessage 在身份验证流程中无法更改。 The handleSubmit
    // 设置 pendingCommitAfterAuth 时捕获的闭包因此仍然
    // 引用正确的、最新的 commitMessage，并重新调用它
    // 这里很安全。 deps 中的 Including handleSubmit 将重新运行效果
    // 每次击键时都会出现这种情况，这不是我们想要的。
// eslint-disable-next-line react-hooks/exhaustive-deps
  }, [pendingCommitAfterAuth, authorIdentity, authorPromptVisible]);

  const canSubmit = commitMessage.trim().length > 0;

  return (
    <div className="border-b border-border/60">
      {authorPromptVisible ? (
        <GitPanelAuthorForm />
      ) : (
        <div className="p-3">
          <div className="group rounded-lg border border-border/70 bg-card shadow-[0_1px_0_rgba(0,0,0,0.02)] transition-[border-color,box-shadow] focus-within:border-primary/50 focus-within:shadow-[0_0_0_3px_rgba(99,102,241,0.08)]">
            <textarea
              value={commitMessage}
              onChange={(e) => setCommitMessage(e.target.value)}
              onKeyDown={(e) => {
                if ((e.metaKey || e.ctrlKey) && e.key === 'Enter' && canSubmit) {
                  e.preventDefault();
                  void handleSubmit();
                }
              }}
              placeholder={t('git.commit.placeholder')}
              rows={2}
              className="w-full resize-none bg-transparent px-3 pt-2.5 pb-1 text-xs leading-relaxed text-foreground placeholder:text-muted-foreground/70 focus:outline-none"
            />
            <div className="flex items-center justify-end gap-2 px-1.5 pb-1.5">
              <Button
                type="button"
                variant="default"
                size="sm"
                disabled={!canSubmit}
                onClick={() => void handleSubmit()}
                className="h-6 gap-1 rounded-md px-2.5 text-[11px] font-medium shadow-none"
              >
                <Milestone size={11} strokeWidth={2} aria-hidden />
                {t('git.commit.submitButton')}
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
