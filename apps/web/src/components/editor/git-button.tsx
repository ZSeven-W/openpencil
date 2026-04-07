// apps/web/src/components/editor/git-button.tsx
//
// Top-bar entry point to the Git panel. Phase 3 only ships the button —
// the panel itself is Phase 4. Clicking the button toggles
// useGitStore.panelOpen; Phase 4's panel component will read that flag.
//
// Renders null in a browser (not Electron) — Git requires the desktop
// process because all operations go through ipcMain.

import { GitBranch } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { useGitStore } from '@/stores/git-store';
import { isGitApiAvailable } from '@/services/git-client';

// TODO(phase-4): i18n — replace the hardcoded 'Git' / 'Git panel' strings
// with t('topbar.git*') keys when the 15-language Phase 4 pass lands.
export function GitButton(): React.ReactElement | null {
  const panelOpen = useGitStore((s) => s.panelOpen);
  const togglePanel = useGitStore((s) => s.togglePanel);

  // Browser hide — per spec "Phase 3: ... top bar button + browser hide".
  if (!isGitApiAvailable()) return null;

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          aria-label="Git panel"
          aria-pressed={panelOpen}
          onClick={togglePanel}
          className={panelOpen ? 'text-foreground' : 'text-muted-foreground'}
        >
          <GitBranch size={15} strokeWidth={1.5} aria-hidden />
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">Git</TooltipContent>
    </Tooltip>
  );
}
