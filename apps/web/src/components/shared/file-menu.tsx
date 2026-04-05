import { useRef, useEffect } from 'react';
import { Plus, Folder, Save, FileText } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { getRecentFiles, clearRecentFiles, relativeTime } from '@/utils/recent-files';

interface FileMenuProps {
  open: boolean;
  onClose: () => void;
  onNew: () => void;
  onOpen: () => void;
  onSave: () => void;
  onSaveAs: () => void;
  onOpenRecent: (filePath: string) => void;
}

export default function FileMenu({
  open,
  onClose,
  onNew,
  onOpen,
  onSave,
  onSaveAs,
  onOpenRecent,
}: FileMenuProps) {
  const { t } = useTranslation();
  const menuRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (!open) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    const keyHandler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose();
    };
    document.addEventListener('mousedown', handler);
    document.addEventListener('keydown', keyHandler);
    return () => {
      document.removeEventListener('mousedown', handler);
      document.removeEventListener('keydown', keyHandler);
    };
  }, [open, onClose]);

  if (!open) return null;

  const recentFiles = getRecentFiles();
  const isMac = typeof navigator !== 'undefined' && /Mac/i.test(navigator.userAgent);
  const mod = isMac ? '\u2318' : 'Ctrl+';

  const MenuItem = ({
    icon: Icon,
    label,
    shortcut,
    onClick,
    className,
  }: {
    icon?: typeof Plus;
    label: string;
    shortcut?: string;
    onClick: () => void;
    className?: string;
  }) => (
    <button
      type="button"
      onClick={() => { onClick(); onClose(); }}
      className={`w-full flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground transition-colors ${className ?? ''}`}
    >
      {Icon && <Icon size={13} className="shrink-0" />}
      {!Icon && <span className="w-[13px] shrink-0" />}
      <span className="flex-1 text-left">{label}</span>
      {shortcut && <span className="text-[10px] text-muted-foreground/60">{shortcut}</span>}
    </button>
  );

  return (
    <div
      ref={menuRef}
      className="absolute top-full left-1/2 -translate-x-1/2 mt-1 z-50 w-56 rounded-lg border border-border bg-card shadow-xl py-1"
    >
      <MenuItem icon={Plus} label={t('fileMenu.newFile')} shortcut={`${mod}N`} onClick={onNew} />
      <MenuItem icon={Folder} label={t('fileMenu.openFile')} shortcut={`${mod}O`} onClick={onOpen} />
      <div className="h-px bg-border mx-2 my-1" />
      <MenuItem icon={Save} label={t('fileMenu.save')} shortcut={`${mod}S`} onClick={onSave} />
      <MenuItem label={t('fileMenu.saveAs')} shortcut={`${mod}\u21E7S`} onClick={onSaveAs} />

      {recentFiles.length > 0 && (
        <>
          <div className="h-px bg-border mx-2 my-1" />
          <div className="px-3 py-1">
            <span className="text-[10px] font-semibold text-muted-foreground/60 uppercase tracking-wider">
              {t('fileMenu.recentFiles')}
            </span>
          </div>
          {recentFiles.map((file, i) => {
            const time = relativeTime(file.lastOpened);
            return (
              <button
                key={`${file.fileName}-${i}`}
                type="button"
                onClick={() => {
                  if (file.filePath) onOpenRecent(file.filePath);
                  onClose();
                }}
                disabled={!file.filePath}
                className="w-full flex items-center gap-2 px-3 py-1.5 text-xs text-muted-foreground hover:bg-accent hover:text-foreground transition-colors disabled:opacity-50 disabled:cursor-default"
              >
                <FileText size={12} className="shrink-0" />
                <span className="flex-1 text-left truncate">{file.fileName}</span>
                <span className="text-[10px] text-muted-foreground/50 shrink-0">
                  {t(time.key, time.params)}
                </span>
              </button>
            );
          })}
          <div className="h-px bg-border mx-2 my-1" />
          <button
            type="button"
            onClick={() => { clearRecentFiles(); onClose(); }}
            className="w-full px-3 py-1.5 text-[11px] text-muted-foreground/60 hover:text-foreground hover:bg-accent transition-colors text-left"
          >
            {t('fileMenu.clearHistory')}
          </button>
        </>
      )}

      {recentFiles.length === 0 && (
        <>
          <div className="h-px bg-border mx-2 my-1" />
          <div className="px-3 py-2 text-[11px] text-muted-foreground/50 text-center">
            {t('fileMenu.noRecentFiles')}
          </div>
        </>
      )}
    </div>
  );
}
