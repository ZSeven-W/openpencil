// apps/web/src/components/panels/git-panel/git-panel-remote-controls.tsx
//
// Phase 6b: pull / push controls for the git panel header. Owns only
// remote-action local state — everything repo-level comes from the store.
// We keep this as a separate subtree (not inlined into git-panel-header)
// so the header stays a thin compositor: branch picker + remote controls
// + overflow menu.
//
// Local state machine:
//   { step: 'idle' }
//   { step: 'pulling', auth?: creds }
//   { step: 'pull-auth', host, mode, error? }       ← shared auth form
//   { step: 'pushing', auth?: creds }
//   { step: 'push-auth', host, mode, error? }       ← shared auth form
//   { step: 'push-rejected' }                        ← one-click retry-to-pull strip
//   { step: 'error', message }                       ← compact inline error
//
// The store is the only authority for IPC + dirty-tree gating. This
// component never bypasses it — it calls `pull()` / `push()` and catches
// the returned GitError, branching on err.code to pick the next step.

import { ArrowDown, ArrowUp } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { GitError, isGitError } from '@/services/git-error';
import type { GitAuthCreds } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import {
  PULL_AUTH_ERROR_CODES,
  PUSH_AUTH_ERROR_CODES,
  type PullAuthErrorCode,
  type PushAuthErrorCode,
} from '@/stores/git-store-types';
import { GitPanelAuthForm, type GitPanelAuthFormMode } from './git-panel-auth-form';

type RemoteControlsStep =
  | { step: 'idle' }
  | { step: 'pulling' }
  | {
      step: 'pull-auth';
      host: string | null;
      mode: GitPanelAuthFormMode;
      error: string | null;
      busy: boolean;
    }
  | { step: 'pushing' }
  | {
      step: 'push-auth';
      host: string | null;
      mode: GitPanelAuthFormMode;
      error: string | null;
      busy: boolean;
    }
  | { step: 'push-rejected' }
  | { step: 'error'; message: string };

function isPullAuthCode(code: string): code is PullAuthErrorCode {
  return (PULL_AUTH_ERROR_CODES as readonly string[]).includes(code);
}

function isPushAuthCode(code: string): code is PushAuthErrorCode {
  return (PUSH_AUTH_ERROR_CODES as readonly string[]).includes(code);
}

export function GitPanelRemoteControls() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const pull = useGitStore((s) => s.pull);
  const push = useGitStore((s) => s.push);
  const getAuth = useGitStore((s) => s.getAuth);
  const storeAuth = useGitStore((s) => s.storeAuth);

  const [step, setStep] = useState<RemoteControlsStep>({ step: 'idle' });

  // Reset local state whenever the repo/remote changes so a stale pull-auth
  // panel can't leak between repositories. `remote` may be absent in test
  // fixtures that predate Phase 6a; treat undefined like null.
  const remoteUrl =
    state.kind === 'ready' || state.kind === 'conflict' ? (state.repo.remote?.url ?? null) : null;
  useEffect(() => {
    setStep({ step: 'idle' });
  }, [remoteUrl]);

  if (state.kind !== 'ready' && state.kind !== 'conflict') return null;
  const repo = state.repo;
  // Defensive: `remote` is `GitRemoteInfo | null` per the contract, but
  // tests that stub repo without the field still land here — treat a
  // missing property the same as an explicit null.
  const hasRemote = repo.remote != null && repo.remote.url != null;
  const host = repo.remote?.host ?? null;
  const ahead = repo.ahead;
  const busy = step.step === 'pulling' || step.step === 'pushing';

  // ---- Pull -------------------------------------------------------------
  const runPull = async (auth?: GitAuthCreds): Promise<'ok' | 'handled'> => {
    setStep({ step: 'pulling' });
    try {
      await pull(auth);
      setStep({ step: 'idle' });
      return 'ok';
    } catch (err) {
      if (isGitError(err) && isPullAuthCode(err.code)) {
        // Surface the shared auth form. Seed the mode from any previously
        // stored credential so the user lands on the right tab without a
        // click.
        const mode = await preseedAuthMode(host, getAuth);
        setStep({
          step: 'pull-auth',
          host,
          mode,
          error: t(`git.auth.error.${err.code}`, { defaultValue: err.message }),
          busy: false,
        });
        return 'handled';
      }
      if (isGitError(err) && err.code === 'save-required') {
        // The store's withCleanWorkingTree gate caught a dirty tree and
        // set saveRequiredFor on the repo state. The save-required alert
        // is already rendered elsewhere; we just bounce back to idle so
        // the button re-enables after the user saves.
        setStep({ step: 'idle' });
        return 'handled';
      }
      setStep({ step: 'error', message: extractMessage(err) });
      return 'handled';
    }
  };

  // ---- Push -------------------------------------------------------------
  const runPush = async (auth?: GitAuthCreds): Promise<'ok' | 'handled'> => {
    setStep({ step: 'pushing' });
    try {
      await push(auth);
      setStep({ step: 'idle' });
      return 'ok';
    } catch (err) {
      if (isGitError(err)) {
        if (isPushAuthCode(err.code)) {
          const mode = await preseedAuthMode(host, getAuth);
          setStep({
            step: 'push-auth',
            host,
            mode,
            error: t(`git.auth.error.${err.code}`, { defaultValue: err.message }),
            busy: false,
          });
          return 'handled';
        }
        if (err.code === 'push-rejected') {
          setStep({ step: 'push-rejected' });
          return 'handled';
        }
        if (err.code === 'save-required') {
          setStep({ step: 'idle' });
          return 'handled';
        }
      }
      setStep({ step: 'error', message: extractMessage(err) });
      return 'handled';
    }
  };

  // ---- Auth-form submit handlers ----------------------------------------
  const submitAuthAndRetry = async (
    which: 'pull' | 'push',
    creds: GitAuthCreds,
    remember: boolean,
  ) => {
    // Remember the credential for this host BEFORE the retry so a repeat
    // failure doesn't have to re-store. If the host is unknown, skip the
    // store step — there's nothing to key on.
    if (remember && host) {
      try {
        await storeAuth(host, creds);
      } catch {
        // Swallow — the retry still has the in-memory creds as a fallback.
      }
    }
    // Flip the embedded auth form into busy state so the submit spinner
    // renders while the retry IPC is in flight.
    setStep((prev) =>
      prev.step === 'pull-auth' || prev.step === 'push-auth'
        ? { ...prev, busy: true, error: null }
        : prev,
    );
    if (which === 'pull') await runPull(creds);
    else await runPush(creds);
  };

  // ---- Render -----------------------------------------------------------
  const pullDisabled = !hasRemote || busy;
  const pushDisabled = !hasRemote || busy;
  const pushHint = !hasRemote
    ? t('git.push.noRemote')
    : ahead === 0
      ? t('git.push.upToDate')
      : t('git.push.tooltip', { count: ahead });
  const pullHint = hasRemote ? t('git.pull.tooltip') : t('git.pull.noRemote');

  return (
    <div className="flex flex-col gap-1">
      <div className="flex items-center gap-0.5">
        <Tooltip>
          <TooltipTrigger asChild>
            <span tabIndex={0} className="inline-flex">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                disabled={pullDisabled}
                aria-label={t('git.pull.label')}
                onClick={() => void runPull()}
                className={pullDisabled ? 'pointer-events-none text-muted-foreground' : ''}
              >
                <ArrowDown size={12} strokeWidth={1.5} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">{pullHint}</TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <span tabIndex={0} className="inline-flex">
              <Button
                type="button"
                variant="ghost"
                size="icon-sm"
                disabled={pushDisabled || ahead === 0}
                aria-label={t('git.push.label')}
                onClick={() => void runPush()}
                className={
                  pushDisabled || ahead === 0 ? 'pointer-events-none text-muted-foreground' : ''
                }
              >
                <ArrowUp size={12} strokeWidth={1.5} aria-hidden />
              </Button>
            </span>
          </TooltipTrigger>
          <TooltipContent side="bottom">{pushHint}</TooltipContent>
        </Tooltip>
      </div>

      {(step.step === 'pull-auth' || step.step === 'push-auth') && (
        <div className="px-2 pb-2">
          <GitPanelAuthForm
            mode={step.mode}
            host={step.host}
            retryLabel={step.step === 'pull-auth' ? t('git.pull.retry') : t('git.push.retry')}
            busy={step.busy}
            error={step.error}
            onCancel={() => setStep({ step: 'idle' })}
            onSubmit={(creds, remember) =>
              void submitAuthAndRetry(step.step === 'pull-auth' ? 'pull' : 'push', creds, remember)
            }
          />
        </div>
      )}

      {step.step === 'push-rejected' && (
        <div
          role="alert"
          className="mx-2 mb-2 flex flex-col gap-1 rounded border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-[11px] text-destructive"
        >
          <div>{t('git.push.rejectedBody')}</div>
          <div className="flex items-center justify-end gap-1">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setStep({ step: 'idle' })}
            >
              {t('git.push.rejectedDismiss')}
            </Button>
            <Button type="button" variant="default" size="sm" onClick={() => void runPull()}>
              {t('git.push.rejectedPull')}
            </Button>
          </div>
        </div>
      )}

      {step.step === 'error' && (
        <div
          role="alert"
          className="mx-2 mb-2 rounded border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-[11px] text-destructive"
        >
          <div className="flex items-start justify-between gap-2">
            <span className="flex-1 break-words">{step.message}</span>
            <button
              type="button"
              onClick={() => setStep({ step: 'idle' })}
              className="text-[10px] underline"
            >
              {t('git.remote.dismissError')}
            </button>
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Local helpers
// ---------------------------------------------------------------------------

async function preseedAuthMode(
  host: string | null,
  getAuth: (h: string) => Promise<GitAuthCreds | null>,
): Promise<GitPanelAuthFormMode> {
  if (!host) return 'token';
  try {
    const stored = await getAuth(host);
    if (stored?.kind === 'ssh') return 'ssh';
    return 'token';
  } catch {
    return 'token';
  }
}

function extractMessage(err: unknown): string {
  if (isGitError(err)) return err.message;
  if (err instanceof GitError) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
