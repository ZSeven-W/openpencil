// apps/web/src/components/editor/git-button.tsx
//
// Top-bar entry point to the Git panel. Phase 4a.1 converts this from
// a plain toggle button into a Popover trigger — the Git panel is now a
// dropdown anchored below this button (side="bottom", align="end"). The
// panel's open/close state is still tracked in useGitStore.panelOpen so
// other components (keyboard shortcuts, etc.) can observe it.
//
// Rendering is gated by isGitApiAvailable() — the button returns null
// in browser mode because git operations require the desktop process.

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

  // Browser hide — git operations require the desktop process.
  if (!isGitApiAvailable()) return null;

  return (
    <Popover open={panelOpen} onOpenChange={(open) => (open ? openPanel() : closePanel())}>
      <Tooltip>
        <TooltipTrigger asChild>
          <PopoverTrigger asChild>
            <Button
              type="button"
              variant="ghost"
              size="icon-sm"
              aria-label="Git panel"
              aria-pressed={panelOpen}
              className={panelOpen ? 'text-foreground' : 'text-muted-foreground'}
            >
              <GitBranch size={15} strokeWidth={1.5} aria-hidden />
            </Button>
          </PopoverTrigger>
        </TooltipTrigger>
        <TooltipContent side="bottom">Git</TooltipContent>
      </Tooltip>
      <PopoverContent
        side="bottom"
        align="end"
        sideOffset={8}
        className="w-[420px] h-[480px] p-0 overflow-hidden flex flex-col"
      >
        <GitPanel />
      </PopoverContent>
    </Popover>
  );
}
