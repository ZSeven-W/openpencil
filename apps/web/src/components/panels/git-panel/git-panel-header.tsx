// apps/web/src/components/panels/git-panel/git-panel-header.tsx
//
// Git 面板 ready/conflict 的 Header 行表示 (Phase 4c → 6c)。
// Renders 具有两组的弹性行：
// Left：分支选择器 (Phase 5) + pull/push 遥控器 (Phase 6b)
// Right：自动保存错误点 + 作者缺失点 + 溢出弹出菜单
//
// Phase 6c 将溢出弹出框扩展为 LOCAL 状态机镜像
// Phase 5 分支选择器模式：
//   { view: 'menu' | 'remote-settings' | 'ssh-keys' }
// The 菜单视图显示现有的三个条目以及两个新条目
// 将弹出窗口内容交换到 git-panel-remote- 中定义的子视图中
// settings.tsx / git-panel-ssh-keys.tsx。 Subview 状态是 NOT 持续到
// 商店 - 它完全存在于这个文件中，并在弹出窗口关闭时重置。
//
// The 组件返回 null，除非 state.kind 为“就绪”或“冲突”。

import {
  ChevronRight,
  FileSearch,
  Key,
  LogOut,
  MoreHorizontal,
  Settings2,
  UserX,
} from 'lucide-react';
import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Separator } from '@/components/ui/separator';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';
import { GitPanelBranchPicker } from './git-panel-branch-picker';
import { GitPanelRemoteControls } from './git-panel-remote-controls';
import { GitPanelRemoteSettings } from './git-panel-remote-settings';
import { GitPanelSshKeys } from './git-panel-ssh-keys';

type OverflowView = 'menu' | 'remote-settings' | 'ssh-keys';

export function GitPanelHeader() {
  const { t } = useTranslation();
  const [overflowOpen, setOverflowOpen] = useState(false);
  const [overflowView, setOverflowView] = useState<OverflowView>('menu');

  const state = useGitStore((s) => s.state);
  const autosaveError = useGitStore((s) => s.autosaveError);
  const clearAutosaveError = useGitStore((s) => s.clearAutosaveError);
  const enterTrackedFilePicker = useGitStore((s) => s.enterTrackedFilePicker);
  const clearAuthorIdentity = useGitStore((s) => s.clearAuthorIdentity);
  const closeRepo = useGitStore((s) => s.closeRepo);
  const authorIdentity = useGitStore((s) => s.authorIdentity);

  if (state.kind !== 'ready' && state.kind !== 'conflict') return null;

  const popoverWidth =
    overflowView === 'menu' ? 'w-56' : overflowView === 'remote-settings' ? 'w-[300px]' : 'w-80';

  return (
    <div className="flex items-center justify-between gap-1 border-b border-border/60 bg-card/40 px-2.5 py-1.5 backdrop-blur-sm">
      {/* ── Left 群组：分支+遥控器── */}
      <div className="flex items-center gap-0.5">
        <GitPanelBranchPicker />
        <GitPanelRemoteControls />
      </div>

      {/* ── Right 组：状态点+溢出菜单── */}
      <div className="flex items-center gap-1">
        {/* Autosave 错误点 — 仅在存在错误时呈现 */}
        {autosaveError !== null && (
          <Tooltip>
            <TooltipTrigger asChild>
              <button
                type="button"
                onClick={() => clearAutosaveError()}
                className="flex h-6 w-6 items-center justify-center rounded-full"
                aria-label={t('git.header.autosaveError')}
              >
                <span className="block h-2 w-2 rounded-full bg-destructive" aria-hidden />
              </button>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-[200px]">
              <p className="font-medium">{t('git.header.autosaveErrorTitle')}</p>
              <p className="text-xs opacity-80">{autosaveError}</p>
            </TooltipContent>
          </Tooltip>
        )}

        {/* Author-missing dot — 仅在未设置作者身份时呈现 */}
        {authorIdentity === null && (
          <Tooltip>
            <TooltipTrigger asChild>
              {/* Not clickable — 仅工具提示提示 */}
              <span
                className="flex h-5 w-5 cursor-default items-center justify-center rounded-full"
                role="status"
                aria-label={t('git.header.authorMissingWarning')}
              >
                {/* bg-yellow-500 是故意的 — 没有用于“警告”的 shadcn 令牌 */}
                <span className="block h-2 w-2 rounded-full bg-yellow-500" aria-hidden />
              </span>
            </TooltipTrigger>
            <TooltipContent side="bottom" className="max-w-[200px]">
              {t('git.header.authorMissingWarning')}
            </TooltipContent>
          </Tooltip>
        )}

        {/* Overflow 菜单 */}
        <Popover
          open={overflowOpen}
          onOpenChange={(next) => {
            setOverflowOpen(next);
            if (next) {
              // Always 在菜单视图上打开 - 当用户重新打开弹出窗口时，前一个会话的子视图不应该泄漏回来。
              setOverflowView('menu');
            }
          }}
        >
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label={t('git.header.overflowMoreActions')}
              className="text-muted-foreground"
            >
              <MoreHorizontal size={13} strokeWidth={1.5} aria-hidden />
            </Button>
          </PopoverTrigger>
          <PopoverContent
            align="end"
            className={`${popoverWidth} rounded-lg border-border/70 p-1 shadow-lg`}
            role="menu"
          >
            {overflowView === 'menu' && (
              <>
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    enterTrackedFilePicker();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <FileSearch
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowSwitchTracked')}
                </button>
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    void clearAuthorIdentity();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <UserX
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowClearAuthor')}
                </button>
                <Separator className="my-1 bg-border/50" />
                <button
                  type="button"
                  role="menuitem"
                  data-testid="overflow-open-remote-settings"
                  onClick={() => setOverflowView('remote-settings')}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <Settings2
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  <span className="flex-1 text-left">{t('git.header.overflowRemoteSettings')}</span>
                  <ChevronRight
                    size={12}
                    strokeWidth={1.5}
                    className="text-muted-foreground/70"
                    aria-hidden
                  />
                </button>
                <button
                  type="button"
                  role="menuitem"
                  data-testid="overflow-open-ssh-keys"
                  onClick={() => setOverflowView('ssh-keys')}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <Key size={13} strokeWidth={1.75} className="text-muted-foreground" aria-hidden />
                  <span className="flex-1 text-left">{t('git.header.overflowSshKeys')}</span>
                  <ChevronRight
                    size={12}
                    strokeWidth={1.5}
                    className="text-muted-foreground/70"
                    aria-hidden
                  />
                </button>
                <Separator className="my-1 bg-border/50" />
                <button
                  type="button"
                  role="menuitem"
                  onClick={() => {
                    setOverflowOpen(false);
                    void closeRepo();
                  }}
                  className="flex w-full items-center gap-2.5 rounded-md px-2 py-1.5 text-[12px] text-foreground transition-colors hover:bg-accent/60"
                >
                  <LogOut
                    size={13}
                    strokeWidth={1.75}
                    className="text-muted-foreground"
                    aria-hidden
                  />
                  {t('git.header.overflowCloseRepo')}
                </button>
              </>
            )}
            {overflowView === 'remote-settings' && (
              <GitPanelRemoteSettings onBack={() => setOverflowView('menu')} />
            )}
            {overflowView === 'ssh-keys' && (
              <GitPanelSshKeys onBack={() => setOverflowView('menu')} />
            )}
          </PopoverContent>
        </Popover>
      </div>
    </div>
  );
}
