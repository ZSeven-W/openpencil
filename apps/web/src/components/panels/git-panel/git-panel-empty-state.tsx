// apps/web/src/components/panels/git-panel/git-panel-empty-state.tsx
//
// 3-card chooser for the no-repo state. Per spec lines 91-111:
//   - 新建: initRepo(currentFilePath). Disabled if no file path.
//   - 打开: native folder picker → openRepo(repoPath, currentFilePath).
//   - 克隆: enterCloneWizard() (Phase 6a — was disabled-with-tooltip in 4a).

import { FilePlus, FolderOpen, GitFork } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { useGitStore } from '@/stores/git-store';
import { useDocumentStore } from '@/stores/document-store';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';

interface EmptyStateCardProps {
  icon: React.ReactNode;
  label: string;
  description: string;
  disabled?: boolean;
  disabledReason?: string;
  onClick?: () => void;
}

function EmptyStateCard({
  icon,
  label,
  description,
  disabled,
  disabledReason,
  onClick,
}: EmptyStateCardProps) {
  const card = (
    <button
      type="button"
      disabled={disabled}
      onClick={onClick}
      className={`flex flex-col items-center gap-1.5 p-3 rounded-lg border border-border bg-card transition-colors w-[90px] h-[88px] ${
        disabled
          ? 'opacity-50 cursor-not-allowed'
          : 'hover:bg-accent hover:border-accent-foreground/20 cursor-pointer'
      }`}
    >
      <div className="text-foreground">{icon}</div>
      <div className="text-xs font-medium text-foreground">{label}</div>
      <div className="text-[10px] text-muted-foreground text-center leading-tight">
        {description}
      </div>
    </button>
  );

  if (disabled && disabledReason) {
    return (
      <Tooltip>
        <TooltipTrigger asChild>{card}</TooltipTrigger>
        <TooltipContent side="bottom">{disabledReason}</TooltipContent>
      </Tooltip>
    );
  }
  return card;
}

export function GitPanelEmptyState() {
  const { t } = useTranslation();
  const initRepo = useGitStore((s) => s.initRepo);
  const openRepo = useGitStore((s) => s.openRepo);
  const enterCloneWizard = useGitStore((s) => s.enterCloneWizard);

  // Reactive read — re-renders when the user saves a previously-unsaved
  // document, opens a different file via the file menu, or any other
  // path that mutates document-store filePath while the panel is open.
  // The 新建 card's enabled-state and the openRepo currentFilePath
  // argument both depend on this being current.
  const filePath = useDocumentStore((s) => s.filePath);
  const newDisabled = !filePath;

  const handleNew = async () => {
    if (!filePath) return;
    await initRepo(filePath);
  };

  const handleOpen = async () => {
    // Open a native folder picker via the dialog:openDirectory IPC added
    // in Task 1. The store's openRepo accepts the directory path and the
    // desktop side detects the rootPath via repo-detector. If the user
    // cancels, openDirectory returns null and we no-op.
    if (typeof window === 'undefined' || !window.electronAPI) return;
    const dirPath = await window.electronAPI.openDirectory();
    if (!dirPath) return;
    await openRepo(dirPath, filePath ?? undefined);
  };

  return (
    <div className="flex flex-col items-center gap-4 px-4 py-6">
      <div className="text-sm text-foreground text-center">{t('git.empty.heading')}</div>

      <div className="flex items-center gap-2">
        <EmptyStateCard
          icon={<FilePlus size={20} strokeWidth={1.5} />}
          label={t('git.empty.newCard')}
          description={t('git.empty.newCardDescription')}
          disabled={newDisabled}
          disabledReason={newDisabled ? t('git.empty.requireSavedFile') : undefined}
          onClick={() => void handleNew()}
        />
        <EmptyStateCard
          icon={<FolderOpen size={20} strokeWidth={1.5} />}
          label={t('git.empty.openCard')}
          description={t('git.empty.openCardDescription')}
          onClick={() => void handleOpen()}
        />
        <EmptyStateCard
          icon={<GitFork size={20} strokeWidth={1.5} />}
          label={t('git.empty.cloneCard')}
          description={t('git.empty.cloneCardDescription')}
          onClick={() => enterCloneWizard()}
        />
      </div>

      <div className="text-[11px] text-muted-foreground text-center">{t('git.empty.optional')}</div>
    </div>
  );
}
