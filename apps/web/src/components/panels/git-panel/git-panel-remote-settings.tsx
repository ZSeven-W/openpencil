// apps/web/src/components/panels/git-panel/git-panel-remote-settings.tsx
//
// Phase 6c: remote settings subview of the overflow menu. Scope (strict v1):
//   - show the current `origin` URL
//   - edit / save / clear the URL (clear requires an inline confirm step)
//   - fetch-on-demand button
//   - show ahead/behind counts
//   - show the stored-auth mode (token / ssh / none) for the current host
//   - clear the stored auth for the current host
//   - surface SSH transport gating text when the iso engine meets an SSH URL
//
// Everything repo-level is pulled from the store; local state is limited to
// the URL draft, in-flight flags, and the clear-origin confirm switch. The
// subview is mounted inside the header overflow Popover and gets an
// `onBack` prop to return to the menu.

import { ArrowLeft, Loader2 } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { isGitError } from '@/services/git-error';
import type { GitAuthCreds } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import { isSshRemoteUrl } from './git-remote-utils';

const INPUT_CLASS =
  'h-8 w-full rounded-md border border-border/70 bg-card px-2.5 text-xs text-foreground transition-[border-color,box-shadow] placeholder:text-muted-foreground/60 focus:border-primary/50 focus:outline-none focus:ring-[3px] focus:ring-primary/10';

type StoredAuthMode = 'token' | 'ssh' | 'none' | 'unknown';

interface GitPanelRemoteSettingsProps {
  onBack: () => void;
}

export function GitPanelRemoteSettings({ onBack }: GitPanelRemoteSettingsProps) {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const refreshRemote = useGitStore((s) => s.refreshRemote);
  const setRemoteUrl = useGitStore((s) => s.setRemoteUrl);
  const fetchRemote = useGitStore((s) => s.fetchRemote);
  const getAuth = useGitStore((s) => s.getAuth);
  const clearAuth = useGitStore((s) => s.clearAuth);

  const repo = state.kind === 'ready' || state.kind === 'conflict' ? state.repo : null;
  const currentUrl = repo?.remote?.url ?? null;
  const host = repo?.remote?.host ?? null;
  const engineKind = repo?.engineKind ?? null;
  const ahead = repo?.ahead ?? 0;
  const behind = repo?.behind ?? 0;

  const [urlDraft, setUrlDraft] = useState<string>(currentUrl ?? '');
  const [confirmClear, setConfirmClear] = useState(false);
  const [busyAction, setBusyAction] = useState<null | 'save' | 'fetch' | 'clear-auth'>(null);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [storedAuth, setStoredAuth] = useState<StoredAuthMode>('unknown');

  // Refresh the cached `repo.remote` on mount so a stale record (e.g. a
  // terminal-driven git remote set-url) doesn't leak into the UI.
  useEffect(() => {
    void refreshRemote();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Keep the draft in sync with the canonical URL whenever it changes
  // under us (store action, refreshRemote, external edit). The user only
  // loses their typed draft if the authoritative URL itself changes.
  useEffect(() => {
    setUrlDraft(currentUrl ?? '');
  }, [currentUrl]);

  // Load the stored-auth mode for the current host — shown as an at-a-glance
  // badge and gates the "Clear saved auth" button.
  useEffect(() => {
    let cancelled = false;
    if (!host) {
      setStoredAuth('none');
      return () => {
        cancelled = true;
      };
    }
    setStoredAuth('unknown');
    void getAuth(host).then((creds: GitAuthCreds | null) => {
      if (cancelled) return;
      if (creds === null) setStoredAuth('none');
      else if (creds.kind === 'ssh') setStoredAuth('ssh');
      else setStoredAuth('token');
    });
    return () => {
      cancelled = true;
    };
  }, [host, getAuth]);

  if (!repo) return null;

  const trimmed = urlDraft.trim();
  const isDirty = trimmed !== (currentUrl ?? '');
  const sshUrl = isSshRemoteUrl(currentUrl);
  const isoSshBlocked = sshUrl && engineKind === 'iso';

  async function handleSave() {
    setInlineError(null);
    setBusyAction('save');
    try {
      // Empty string normalises to null on the store side, but we never
      // flow an empty save here — clear has its own explicit confirm path.
      await setRemoteUrl(trimmed);
    } catch (err) {
      setInlineError(extractMessage(err));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleConfirmClear() {
    setInlineError(null);
    setBusyAction('save');
    try {
      await setRemoteUrl(null);
      setConfirmClear(false);
    } catch (err) {
      setInlineError(extractMessage(err));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleFetch() {
    setInlineError(null);
    setBusyAction('fetch');
    try {
      await fetchRemote();
    } catch (err) {
      if (isGitError(err) && err.code === 'ssh-not-supported-iso') {
        setInlineError(t('git.remote.sshIsoUnsupported'));
        return;
      }
      setInlineError(extractMessage(err));
    } finally {
      setBusyAction(null);
    }
  }

  async function handleClearAuth() {
    if (!host) return;
    setInlineError(null);
    setBusyAction('clear-auth');
    try {
      await clearAuth(host);
      setStoredAuth('none');
    } catch (err) {
      setInlineError(extractMessage(err));
    } finally {
      setBusyAction(null);
    }
  }

  return (
    <div className="flex flex-col p-1" role="group" aria-label={t('git.remote.settingsLabel')}>
      <div className="flex items-center justify-between gap-2 px-1 pb-1.5 pt-0.5">
        <button
          type="button"
          onClick={onBack}
          aria-label={t('git.remote.back')}
          className="flex items-center gap-1 rounded-md px-1.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
        >
          <ArrowLeft size={12} strokeWidth={1.75} aria-hidden />
          {t('git.remote.back')}
        </button>
      </div>

      <p className="px-2 pb-1 pt-0.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/80">
        {t('git.remote.settingsHeading')}
      </p>

      <div className="flex flex-col gap-2 px-2 py-2">
        {currentUrl === null && (
          <p className="text-[11px] text-muted-foreground">{t('git.remote.emptyNoOrigin')}</p>
        )}

        <label className="flex flex-col gap-1">
          <span className="text-[11px] text-muted-foreground">{t('git.remote.urlLabel')}</span>
          <input
            type="text"
            value={urlDraft}
            onChange={(e) => setUrlDraft(e.target.value)}
            placeholder={t('git.remote.urlPlaceholder')}
            aria-label={t('git.remote.urlLabel')}
            className={INPUT_CLASS}
            autoComplete="off"
            spellCheck={false}
          />
        </label>

        <div className="flex items-center justify-end gap-1.5">
          {currentUrl !== null && !confirmClear && (
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busyAction !== null}
              onClick={() => setConfirmClear(true)}
              className="h-7 rounded-md px-2.5 text-[11px]"
            >
              {t('git.remote.clearButton')}
            </Button>
          )}
          <Button
            type="button"
            size="sm"
            disabled={busyAction !== null || trimmed === '' || !isDirty}
            onClick={() => void handleSave()}
            className="h-7 gap-1 rounded-md px-2.5 text-[11px] shadow-none"
          >
            {busyAction === 'save' && <Loader2 size={12} className="animate-spin" aria-hidden />}
            {t('git.remote.saveButton')}
          </Button>
        </div>

        {confirmClear && (
          <div
            role="alertdialog"
            aria-label={t('git.remote.clearConfirmHeading')}
            className="flex flex-col gap-2 rounded-md border border-destructive/30 bg-destructive/10 p-2.5 text-[11px] text-destructive"
          >
            <div>{t('git.remote.clearConfirmBody')}</div>
            <div className="flex items-center justify-end gap-1.5">
              <Button
                type="button"
                variant="ghost"
                size="sm"
                onClick={() => setConfirmClear(false)}
                className="h-7 rounded-md px-2.5 text-[11px]"
              >
                {t('git.remote.cancel')}
              </Button>
              <Button
                type="button"
                variant="destructive"
                size="sm"
                disabled={busyAction !== null}
                onClick={() => void handleConfirmClear()}
                className="h-7 rounded-md px-2.5 text-[11px]"
              >
                {t('git.remote.clearConfirmAction')}
              </Button>
            </div>
          </div>
        )}
      </div>

      <Separator className="my-1 bg-border/50" />

      <div className="flex flex-col gap-2 px-2 py-2">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">
            {t('git.remote.aheadBehind', { ahead, behind })}
          </span>
          <Button
            type="button"
            variant="ghost"
            size="sm"
            disabled={busyAction !== null || currentUrl === null}
            onClick={() => void handleFetch()}
            className="h-7 gap-1 rounded-md px-2.5 text-[11px]"
          >
            {busyAction === 'fetch' && <Loader2 size={12} className="animate-spin" aria-hidden />}
            {t('git.remote.fetchButton')}
          </Button>
        </div>

        {isoSshBlocked && (
          <p className="text-[11px] text-muted-foreground" role="note">
            {t('git.remote.sshIsoUnsupported')}
          </p>
        )}
      </div>

      <Separator className="my-1 bg-border/50" />

      <div className="flex flex-col gap-2 px-2 py-2">
        <div className="flex items-center justify-between gap-2">
          <span className="text-[11px] text-muted-foreground">
            {t('git.remote.storedAuthLabel')}
          </span>
          <span className="text-[11px] text-foreground">
            {host === null
              ? t('git.remote.storedAuth.noHost')
              : storedAuth === 'unknown'
                ? t('git.remote.storedAuth.loading')
                : t(`git.remote.storedAuth.${storedAuth}`)}
          </span>
        </div>
        {host !== null && storedAuth !== 'none' && storedAuth !== 'unknown' && (
          <div className="flex items-center justify-end">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busyAction !== null}
              onClick={() => void handleClearAuth()}
              className="h-7 gap-1 rounded-md px-2.5 text-[11px]"
            >
              {busyAction === 'clear-auth' && (
                <Loader2 size={12} className="animate-spin" aria-hidden />
              )}
              {t('git.remote.clearAuthButton')}
            </Button>
          </div>
        )}
      </div>

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

function extractMessage(err: unknown): string {
  if (isGitError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
