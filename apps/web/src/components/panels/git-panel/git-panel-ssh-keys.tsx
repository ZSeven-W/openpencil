// apps/web/src/components/panels/git-panel/git-panel-ssh-keys.tsx
//
// Phase 6c: SSH key manager subview of the overflow menu. Responsibilities:
//   - refreshSshKeys() on open
//   - list keys, with the keys for the current remote host floated to top
//   - generate key (host, comment)
//   - import key via window.electronAPI.openFile(), pass filePath into importSshKey
//   - delete key (with inline confirm)
//   - copy public key with a transient success hint
//   - provider link for github.com / gitlab.com, else generic copy guidance
//   - SSH transport gating banner when iso engine meets an SSH-ish URL
//
// Local state machine:
//   view = 'list' | 'generate' | 'import' | 'delete-confirm'
// View state is local to this subview since it's purely a submenu of the
// overflow popover and doesn't need to survive across opens.

import { ArrowLeft, Copy, ExternalLink, Loader2, Plus, Trash2, Upload } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { isGitError } from '@/services/git-error';
import type { GitPublicSshKeyInfo } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import { getProviderSshSettingsUrl, isSshRemoteUrl } from './git-remote-utils';

const INPUT_CLASS =
  'h-8 w-full rounded-md border border-border/70 bg-card px-2.5 text-xs text-foreground transition-[border-color,box-shadow] placeholder:text-muted-foreground/60 focus:border-primary/50 focus:outline-none focus:ring-[3px] focus:ring-primary/10';

type SshKeysView = 'list' | 'generate' | 'import' | 'delete-confirm';

interface GitPanelSshKeysProps {
  onBack: () => void;
}

export function GitPanelSshKeys({ onBack }: GitPanelSshKeysProps) {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const sshKeys = useGitStore((s) => s.sshKeys);
  const refreshSshKeys = useGitStore((s) => s.refreshSshKeys);
  const generateSshKey = useGitStore((s) => s.generateSshKey);
  const importSshKey = useGitStore((s) => s.importSshKey);
  const deleteSshKey = useGitStore((s) => s.deleteSshKey);

  const repo = state.kind === 'ready' || state.kind === 'conflict' ? state.repo : null;
  const currentHost = repo?.remote?.host ?? null;
  const remoteUrl = repo?.remote?.url ?? null;
  const engineKind = repo?.engineKind ?? null;
  const isoSshBlocked = engineKind === 'iso' && isSshRemoteUrl(remoteUrl);

  const [view, setView] = useState<SshKeysView>('list');
  const [busy, setBusy] = useState(false);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [copiedKeyId, setCopiedKeyId] = useState<string | null>(null);
  // Track the pending "copied" flash timer so it can be cleared on a second
  // copy (debounces the flash) and, critically, on unmount — otherwise the
  // timeout fires after the popover closes and setCopiedKeyId runs on a
  // stale component.
  const copyTimerRef = useRef<number | null>(null);

  // Generate form fields
  const [genHost, setGenHost] = useState<string>(currentHost ?? '');
  const [genComment, setGenComment] = useState<string>('');

  // Import form fields
  const [importHost, setImportHost] = useState<string>(currentHost ?? '');
  const [importPath, setImportPath] = useState<string>('');

  // Delete confirm target
  const [deleteTarget, setDeleteTarget] = useState<GitPublicSshKeyInfo | null>(null);

  // On mount: refresh the key list so the subview never opens against a
  // stale cache. Fire-and-forget; the store populates `sshKeys` when done.
  useEffect(() => {
    void refreshSshKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Clean up any in-flight "copied" flash timer on unmount so it can't call
  // setCopiedKeyId after the subview is gone.
  useEffect(() => {
    return () => {
      if (copyTimerRef.current !== null) {
        window.clearTimeout(copyTimerRef.current);
        copyTimerRef.current = null;
      }
    };
  }, []);

  // Float keys bound to the current remote host to the top of the list.
  const sortedKeys = useMemo(() => {
    if (!currentHost) return sshKeys;
    const matching: GitPublicSshKeyInfo[] = [];
    const others: GitPublicSshKeyInfo[] = [];
    for (const k of sshKeys) {
      if (k.host === currentHost) matching.push(k);
      else others.push(k);
    }
    return [...matching, ...others];
  }, [sshKeys, currentHost]);

  const resetForms = () => {
    setGenHost(currentHost ?? '');
    setGenComment('');
    setImportHost(currentHost ?? '');
    setImportPath('');
    setInlineError(null);
  };

  async function handleGenerate() {
    const host = genHost.trim();
    const comment = genComment.trim();
    if (!host) {
      setInlineError(t('git.ssh.validationHost'));
      return;
    }
    if (!comment) {
      setInlineError(t('git.ssh.validationComment'));
      return;
    }
    setInlineError(null);
    setBusy(true);
    try {
      await generateSshKey({ host, comment });
      setView('list');
      resetForms();
    } catch (err) {
      // Symmetric with git-panel-remote-settings: translate the iso SSH
      // transport gate to its localized hint so the user doesn't see a raw
      // engine-level error string.
      if (isGitError(err) && err.code === 'ssh-not-supported-iso') {
        setInlineError(t('git.ssh.isoUnsupported'));
      } else {
        setInlineError(extractMessage(err));
      }
    } finally {
      setBusy(false);
    }
  }

  async function handlePickImportPath() {
    if (typeof window === 'undefined' || !window.electronAPI) return;
    const picked = await window.electronAPI.openFile();
    if (picked === null) return;
    // openFile() returns { filePath, content }; importSshKey takes the
    // path so the desktop side can copy + chmod the file out of band.
    setImportPath(picked.filePath);
  }

  async function handleImport() {
    const host = importHost.trim();
    const path = importPath.trim();
    if (!host) {
      setInlineError(t('git.ssh.validationHost'));
      return;
    }
    if (!path) {
      setInlineError(t('git.ssh.validationImportPath'));
      return;
    }
    setInlineError(null);
    setBusy(true);
    try {
      await importSshKey({ privateKeyPath: path, host });
      setView('list');
      resetForms();
    } catch (err) {
      if (isGitError(err) && err.code === 'ssh-not-supported-iso') {
        setInlineError(t('git.ssh.isoUnsupported'));
      } else {
        setInlineError(extractMessage(err));
      }
    } finally {
      setBusy(false);
    }
  }

  async function handleConfirmDelete() {
    if (!deleteTarget) return;
    setInlineError(null);
    setBusy(true);
    try {
      await deleteSshKey(deleteTarget.id);
      setDeleteTarget(null);
      setView('list');
    } catch (err) {
      setInlineError(extractMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleCopyPublicKey(key: GitPublicSshKeyInfo) {
    // Feature-gate: navigator.clipboard is undefined in non-secure contexts
    // (http://, file://, some jsdom test environments). Blindly reading
    // `.writeText` would throw a raw TypeError with a confusing message.
    if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
      setInlineError(t('git.ssh.copyUnsupported'));
      return;
    }
    try {
      await navigator.clipboard.writeText(key.publicKey);
      setCopiedKeyId(key.id);
      // Clear the copied hint after a moment so a later copy still flashes.
      // Cancel any previous pending clear so rapid successive copies on
      // different rows don't race each other, and store the handle so the
      // unmount effect can cancel it if the subview closes first.
      if (copyTimerRef.current !== null) {
        window.clearTimeout(copyTimerRef.current);
      }
      copyTimerRef.current = window.setTimeout(() => {
        copyTimerRef.current = null;
        setCopiedKeyId((prev) => (prev === key.id ? null : prev));
      }, 1600);
    } catch (err) {
      setInlineError(extractMessage(err));
    }
  }

  if (!repo) return null;

  return (
    <div className="flex flex-col p-1" role="group" aria-label={t('git.ssh.label')}>
      <div className="flex items-center justify-between gap-2 px-1 pb-1.5 pt-0.5">
        <button
          type="button"
          onClick={() => {
            if (view === 'list') {
              onBack();
              return;
            }
            setView('list');
            resetForms();
            setDeleteTarget(null);
          }}
          aria-label={t('git.ssh.back')}
          className="flex items-center gap-1 rounded-md px-1.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
        >
          <ArrowLeft size={12} strokeWidth={1.75} aria-hidden />
          {t('git.ssh.back')}
        </button>
      </div>

      <p className="px-2 pb-1 pt-0.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/80">
        {t('git.ssh.heading')}
      </p>

      {isoSshBlocked && (
        <div
          role="note"
          className="mx-2 mb-2 rounded-md border border-border/60 bg-muted/40 px-2.5 py-2 text-[11px] text-muted-foreground"
        >
          {t('git.ssh.isoUnsupported')}
        </div>
      )}

      {view === 'list' && (
        <ListView
          keys={sortedKeys}
          currentHost={currentHost}
          copiedKeyId={copiedKeyId}
          onGenerate={() => {
            resetForms();
            setView('generate');
          }}
          onImport={() => {
            resetForms();
            setView('import');
          }}
          onCopy={(k) => void handleCopyPublicKey(k)}
          onDelete={(k) => {
            setDeleteTarget(k);
            setView('delete-confirm');
            setInlineError(null);
          }}
        />
      )}

      {view === 'generate' && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.hostLabel')}</span>
            <input
              type="text"
              value={genHost}
              onChange={(e) => setGenHost(e.target.value)}
              placeholder={t('git.ssh.hostPlaceholder')}
              aria-label={t('git.ssh.hostLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.commentLabel')}</span>
            <input
              type="text"
              value={genComment}
              onChange={(e) => setGenComment(e.target.value)}
              placeholder={t('git.ssh.commentPlaceholder')}
              aria-label={t('git.ssh.commentLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setView('list');
                resetForms();
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button type="button" size="sm" disabled={busy} onClick={() => void handleGenerate()}>
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.generateSubmit')}
            </Button>
          </div>
        </div>
      )}

      {view === 'import' && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.hostLabel')}</span>
            <input
              type="text"
              value={importHost}
              onChange={(e) => setImportHost(e.target.value)}
              placeholder={t('git.ssh.hostPlaceholder')}
              aria-label={t('git.ssh.hostLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <div className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">
              {t('git.ssh.importPathLabel')}
            </span>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={importPath}
                onChange={(e) => setImportPath(e.target.value)}
                placeholder={t('git.ssh.importPathPlaceholder')}
                aria-label={t('git.ssh.importPathLabel')}
                className={INPUT_CLASS}
                autoComplete="off"
                spellCheck={false}
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={() => void handlePickImportPath()}
              >
                {t('git.ssh.importBrowse')}
              </Button>
            </div>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setView('list');
                resetForms();
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button type="button" size="sm" disabled={busy} onClick={() => void handleImport()}>
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.importSubmit')}
            </Button>
          </div>
        </div>
      )}

      {view === 'delete-confirm' && deleteTarget && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <p className="text-xs text-foreground">
            {t('git.ssh.deletePrompt', {
              name: deleteTarget.comment || deleteTarget.fingerprint,
            })}
          </p>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setDeleteTarget(null);
                setView('list');
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              disabled={busy}
              onClick={() => void handleConfirmDelete()}
            >
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.deleteConfirm')}
            </Button>
          </div>
        </div>
      )}

      {inlineError && (
        <div
          role="alert"
          className="mx-2 mb-2 rounded border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-[11px] text-destructive"
        >
          {inlineError}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// List subview — extracted so the key-row map stays readable without a
// giant cascading conditional in the parent.
// ---------------------------------------------------------------------------

function ListView({
  keys,
  currentHost,
  copiedKeyId,
  onGenerate,
  onImport,
  onCopy,
  onDelete,
}: {
  keys: GitPublicSshKeyInfo[];
  currentHost: string | null;
  copiedKeyId: string | null;
  onGenerate: () => void;
  onImport: () => void;
  onCopy: (k: GitPublicSshKeyInfo) => void;
  onDelete: (k: GitPublicSshKeyInfo) => void;
}) {
  const { t } = useTranslation();
  const providerUrl = getProviderSshSettingsUrl(currentHost);

  if (keys.length === 0) {
    return (
      <div className="flex flex-col gap-2 px-2 py-2" data-testid="ssh-keys-empty">
        <p className="text-[11px] text-muted-foreground">{t('git.ssh.emptyList')}</p>
        <div className="flex items-center justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" onClick={onImport}>
            <Upload size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
            {t('git.ssh.importAction')}
          </Button>
          <Button type="button" size="sm" onClick={onGenerate}>
            <Plus size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
            {t('git.ssh.generateAction')}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1 px-1 py-1">
      {keys.map((k) => {
        const isForCurrent = currentHost !== null && k.host === currentHost;
        return (
          <div
            key={k.id}
            data-testid={`ssh-key-row-${k.id}`}
            className="flex flex-col gap-1 rounded-sm border border-transparent px-2 py-1.5 hover:border-border"
          >
            <div className="flex items-center justify-between gap-2">
              <div className="flex min-w-0 flex-col">
                <span className="truncate text-xs text-foreground">
                  {k.comment || k.fingerprint}
                </span>
                <span className="truncate text-[10px] text-muted-foreground">
                  {k.host}
                  {isForCurrent && ` · ${t('git.ssh.currentHostBadge')}`}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t('git.ssh.copyPublicKey')}
                  onClick={() => onCopy(k)}
                >
                  <Copy size={12} strokeWidth={1.5} aria-hidden />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t('git.ssh.deleteKey', { name: k.comment || k.fingerprint })}
                  onClick={() => onDelete(k)}
                >
                  <Trash2 size={12} strokeWidth={1.5} aria-hidden />
                </Button>
              </div>
            </div>
            {copiedKeyId === k.id && (
              <span className="text-[10px] text-muted-foreground" role="status" aria-live="polite">
                {t('git.ssh.copiedHint')}
              </span>
            )}
          </div>
        );
      })}

      <Separator className="my-1" />

      {providerUrl ? (
        <a
          href={providerUrl}
          target="_blank"
          rel="noopener noreferrer"
          data-testid="ssh-provider-link"
          className="mx-1 flex items-center gap-1 rounded-sm px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent"
        >
          <ExternalLink size={12} strokeWidth={1.5} aria-hidden />
          {t('git.ssh.providerLink', { host: currentHost ?? '' })}
        </a>
      ) : (
        <p className="mx-2 py-1 text-[11px] text-muted-foreground">
          {t('git.ssh.genericGuidance')}
        </p>
      )}

      <div className="flex items-center justify-end gap-2 px-2 py-1">
        <Button type="button" variant="ghost" size="sm" onClick={onImport}>
          <Upload size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
          {t('git.ssh.importAction')}
        </Button>
        <Button type="button" size="sm" onClick={onGenerate}>
          <Plus size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
          {t('git.ssh.generateAction')}
        </Button>
      </div>
    </div>
  );
}

function extractMessage(err: unknown): string {
  if (isGitError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
