// apps/web/src/components/panels/git-panel/git-panel-empty-state.tsx
//
// 3-card chooser for the no-repo state. Per spec lines 91-111:
//   - 新建: initRepo(currentFilePath). Disabled if no file path.
//   - 打开: native folder picker → openRepo(repoPath, currentFilePath).
//   - 克隆: enterCloneWizard() (Phase 6a — was disabled-with-tooltip in 4a).

import { FilePlus, FolderOpen, GitFork, History } from 'lucide-react';
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
      className={`group flex h-[104px] w-[96px] flex-col items-center justify-center gap-2 rounded-xl border bg-card p-3 transition-all ${
        disabled
          ? 'cursor-not-allowed border-border/60 opacity-50'
          : 'cursor-pointer border-border/70 shadow-[0_1px_0_rgba(0,0,0,0.02)] hover:-translate-y-px hover:border-primary/40 hover:bg-accent/30 hover:shadow-[0_4px_12px_-6px_rgba(0,0,0,0.08)]'
      }`}
    >
      <div
        className={`flex h-9 w-9 items-center justify-center rounded-lg transition-colors ${
          disabled
            ? 'bg-muted/50 text-muted-foreground'
            : 'bg-muted/60 text-foreground/80 group-hover:bg-primary/10 group-hover:text-primary'
        }`}
      >
        {icon}
      </div>
      <div className="text-[11px] font-semibold text-foreground">{label}</div>
      <div className="text-center text-[9px] leading-tight text-muted-foreground">
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
    <div className="flex flex-col items-center gap-5 px-4 pb-6 pt-8">
      <div className="flex h-12 w-12 items-center justify-center rounded-2xl bg-gradient-to-br from-muted/60 to-muted/20 ring-1 ring-inset ring-border/60">
        <History size={22} strokeWidth={1.5} className="text-muted-foreground" aria-hidden />
      </div>

      <div className="text-center text-[13px] font-medium text-foreground">
        {t('git.empty.heading')}
      </div>

      <div className="flex items-stretch gap-2">
        <EmptyStateCard
          icon={<FilePlus size={18} strokeWidth={1.75} />}
          label={t('git.empty.newCard')}
          description={t('git.empty.newCardDescription')}
          disabled={newDisabled}
          disabledReason={newDisabled ? t('git.empty.requireSavedFile') : undefined}
          onClick={() => void handleNew()}
        />
        <EmptyStateCard
          icon={<FolderOpen size={18} strokeWidth={1.75} />}
          label={t('git.empty.openCard')}
          description={t('git.empty.openCardDescription')}
          onClick={() => void handleOpen()}
        />
        <EmptyStateCard
          icon={<GitFork size={18} strokeWidth={1.75} />}
          label={t('git.empty.cloneCard')}
          description={t('git.empty.cloneCardDescription')}
          onClick={() => enterCloneWizard()}
        />
      </div>

      <div className="text-center text-[11px] text-muted-foreground/80">
        {t('git.empty.optional')}
      </div>
    </div>
  );
}
