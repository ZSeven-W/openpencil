// apps/web/src/components/panels/git-panel/git-panel.tsx
//
// Git 面板主体 — 在锚定到的 <PopoverContent> 内呈现
// 顶栏 GitButton。 Phase 4a.1 删除了 floating/draggable/minimized
// chrome 支持下拉表单。 The 弹出窗口本身拥有可见性
// （open/close 通过 Popover 的 open + onOpenChange 道具连接
// git-button.tsx)，因此 GitPanel 不再检查 panelOpen 或
// panelMinimized — 当该组件渲染时，弹出窗口是
// 已经开放了。
//
// The 主体开启 `state.kind` 并委托给同级
// 组件（空状态、错误卡等）。 Author 身份已加载
// 第一次安装时。 Repo 被跟踪时触发检测
// 当面板可见时文件路径会发生变化。

import { useEffect } from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useGitStore } from '@/stores/git-store';
import { useDocumentStore } from '@/stores/document-store';
import { GitPanelCloneForm } from './git-panel-clone-form';
import { GitPanelConflict } from './git-panel-conflict';
import { GitPanelEmptyState } from './git-panel-empty-state';
import { GitPanelErrorCard } from './git-panel-error-card';
import { GitPanelReady } from './git-panel-ready';
import { GitPanelTrackedPicker } from './git-panel-tracked-picker';
import { Button } from '@/components/ui/button';

/**
 * Git 弹出窗口的
 * Body。 Assumes Popover 祖先已经打开（Radix 在关闭时卸载
 * PopoverContent，因此该组件仅在可见时呈现）。
 */
export function GitPanel() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const loadAuthorIdentity = useGitStore((s) => s.loadAuthorIdentity);
  const detectRepo = useGitStore((s) => s.detectRepo);
  const lastAutoBindedPath = useGitStore((s) => s.lastAutoBindedPath);
  const acknowledgeAutoBind = useGitStore((s) => s.acknowledgeAutoBind);
  const acknowledgeAutoBindAndOpen = useGitStore((s) => s.acknowledgeAutoBindAndOpen);

  // Reactive 文档存储读取 — 当用户在下拉列表打开时保存或打开不同的文件时重新呈现。
  const docFilePath = useDocumentStore((s) => s.filePath);

  // Load 首次安装时显示作者身份。 loadAuthorIdentity 本身是幂等的，并且通过某种类型的窗口防护对 SSR
  // 进行短路。
  useEffect(() => {
    void loadAuthorIdentity();
  }, [loadAuthorIdentity]);

  // 当面板在无文件状态下打开时，每当当前文件路径发生更改时，Trigger 都会进行存储库检测。 Covers
  // “用户在保存之前打开下拉列表，然后保存”流程。
  useEffect(() => {
    if (state.kind !== 'no-file') return;
    if (docFilePath) {
      void detectRepo(docFilePath);
    }
  }, [state.kind, detectRepo, docFilePath]);

  return (
    <div className="flex-1 overflow-y-auto">
      {lastAutoBindedPath && (
        <AutoBindBanner
          path={lastAutoBindedPath}
          onOpen={() => void acknowledgeAutoBindAndOpen()}
          onDismiss={() => acknowledgeAutoBind()}
        />
      )}
      {(state.kind === 'no-file' || state.kind === 'no-repo') && <GitPanelEmptyState />}
      {state.kind === 'wizard-clone' && <GitPanelCloneForm />}
      {state.kind === 'initializing' && (
        <div className="flex flex-col items-center justify-center gap-2 p-6 text-muted-foreground">
          <Loader2 size={20} className="animate-spin" aria-hidden />
          <span className="text-xs">{t('git.initializing')}</span>
        </div>
      )}
      {state.kind === 'needs-tracked-file' && <GitPanelTrackedPicker />}
      {state.kind === 'ready' && <GitPanelReady />}
      {state.kind === 'conflict' && <GitPanelConflict />}
      {state.kind === 'error' && (
        <GitPanelErrorCard message={state.message} recoverable={state.recoverable} />
      )}
    </div>
  );
}

function AutoBindBanner({
  path,
  onOpen,
  onDismiss,
}: {
  path: string;
  onOpen: () => void;
  onDismiss: () => void;
}) {
  const { t } = useTranslation();
  const fileName = path.split(/[/\\]/).pop() || path;
  return (
    <div className="border-b border-border bg-muted/40 px-4 py-3 flex flex-col gap-2">
      <div className="text-xs text-foreground">
        {t('git.autoBind.confirmHeading', { fileName })}
      </div>
      <div className="flex items-center justify-end gap-2">
        <Button type="button" variant="ghost" size="sm" onClick={onDismiss}>
          {t('git.autoBind.dismissButton')}
        </Button>
        <Button type="button" variant="default" size="sm" onClick={onOpen}>
          {t('git.autoBind.openButton')}
        </Button>
      </div>
    </div>
  );
}
