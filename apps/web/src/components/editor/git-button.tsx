// apps/web/src/components/editor/git-button.tsx
//
// Top-bar entry point to the Git panel. Phase 4a.1 converts this from
// a plain toggle button into a Popover trigger — the Git panel is now a
// dropdown anchored below this button. The panel's open/close state is
// still tracked in useGitStore.panelOpen so other components (keyboard
// shortcuts, etc.) can observe it.
//
// When a repo is connected we render a subtle pill showing the current
// branch next to the file name; otherwise we render the icon-only
// trigger. Rendering is gated by isGitApiAvailable() — the button
// returns null in browser mode because git operations require the
// desktop process.

import { GitBranch } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Popover, PopoverContent, PopoverTrigger } from '@/components/ui/popover';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';
import { isGitApiAvailable } from '@/services/git-client';
import { GitPanel } from '@/components/panels/git-panel/git-panel';

// TODO(phase-4): i18n — replace the hardcoded 'Git' / 'Git panel' strings
// with t('topbar.git*') keys when the 15-language Phase 4 pass lands.
export function GitButton(): React.ReactElement | null {
  const panelOpen = useGitStore((s) => s.panelOpen);
  const openPanel = useGitStore((s) => s.openPanel);
  const closePanel = useGitStore((s) => s.closePanel);
  const state = useGitStore((s) => s.state);

  // Browser hide — git operations require the desktop process.
  if (!isGitApiAvailable()) return null;

  const branchName =
    state.kind === 'ready' || state.kind === 'conflict' ? state.repo.currentBranch : null;

  return (
    <Popover open={panelOpen} onOpenChange={(open) => (open ? openPanel() : closePanel())}>
      <Tooltip>
        <TooltipTrigger asChild>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              aria-label="Git panel"
              aria-pressed={panelOpen}
              className={`flex h-6 items-center gap-1 rounded-full border border-transparent text-[11px] font-medium leading-none transition-colors ${
                branchName ? 'px-2' : 'w-6 justify-center p-0'
              } ${
                panelOpen
                  ? 'border-border/70 bg-accent/60 text-foreground shadow-[0_1px_0_rgba(0,0,0,0.02)]'
                  : 'text-muted-foreground hover:border-border/60 hover:bg-accent/40 hover:text-foreground'
              }`}
            >
              <GitBranch size={12} strokeWidth={1.75} aria-hidden />
              {branchName && (
                <span className="max-w-[96px] truncate leading-none">{branchName}</span>
              )}
            </Button>
          </PopoverTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom">Git</TooltipContent>
      </Tooltip>
      <PopoverContent
        side="bottom"
        align="center"
        sideOffset={10}
        className="flex h-[480px] w-[420px] flex-col overflow-visible p-0"
      >
        <div className="flex h-full w-full flex-col overflow-hidden rounded-[inherit]">
          <GitPanel />
        </div>
      </PopoverContent>
    </Popover>
  );
}
