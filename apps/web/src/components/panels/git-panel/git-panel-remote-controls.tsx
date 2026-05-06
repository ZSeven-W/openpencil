// apps/web/src/components/panels/git-panel/git-panel-remote-controls.tsx
//
// Phase 6b：git 面板标题的拉/推控件。仅限 Owns
// 远程操作本地状态——回购级别的所有内容都来自商店。
// We 将其保留为单独的子树（未内联到 git-panel-header 中）
// 所以标题保持一个薄合成器：分支选择器+远程控制
// + 溢出菜单。
//
// Local 状态机：
//   { step: 'idle' }
//   { step: 'pulling', auth?: creds }
//   { step: 'pull-auth', host, mode, error? }       ← shared auth form
//   { step: 'pushing', auth?: creds }
//   { step: 'push-auth', host, mode, error? }       ← shared auth form
//   { step: 'push-rejected' }                        ← one-click retry-to-pull strip
//   { step: 'error', message }                       ← compact inline error
//
// The 存储是 IPC + 脏树门控的唯一权限。 This
// 组件永远不会绕过它——它调用 `pull()` / `push()` 并捕获
// 返回的 GitError，在 err.code 上分支以选择下一步。

import { ArrowDown, ArrowUp } from 'lucide-react';
import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipContent, TooltipTrigger } from '@/components/ui/tooltip';
import { isGitError } from '@/services/git-error';
import type { GitAuthCreds } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import { classifyRemoteAuthError } from '@/stores/git-store-helpers';
import { GitPanelAuthForm, type GitPanelAuthFormMode } from './git-panel-auth-form';
import { isSshRemoteUrl } from './git-remote-utils';

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

export function GitPanelRemoteControls() {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const pull = useGitStore((s) => s.pull);
  const push = useGitStore((s) => s.push);
  const getAuth = useGitStore((s) => s.getAuth);
  const storeAuth = useGitStore((s) => s.storeAuth);

  const [step, setStep] = useState<RemoteControlsStep>({ step: 'idle' });

  // 每当 repo/remote 发生更改时，Reset 本地状态都会发生变化，因此过时的 pull-auth 面板不会在存储库之间泄漏。
  // `remote` 在 Phase 6a 之前的测试装置中可能不存在；将未定义视为 null。
  const remoteUrl = state.kind === 'ready' ? (state.repo.remote?.url ?? null) : null;
  useEffect(() => {
    setStep({ step: 'idle' });
  }, [remoteUrl]);

  // Only 在仓库处于干净的 `ready` 状态时渲染。 During 和 `conflict`
  // 用户必须先完成正在进行的合并，然后才能再次执行 pull/push — 冲突横幅拥有恢复 UI，并且对于半合并树，两个 IPCs
  // 将确定性失败。
  if (state.kind !== 'ready') return null;
  const repo = state.repo;
  // Defensive：根据合同，`remote` 是 `GitRemoteInfo |
  // null`，但测试没有该字段的存根存储库仍然落在此处 - 将丢失的属性与显式空值相同。
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
      const classification = classifyRemoteAuthError(err, 'pull');
      if (classification.kind === 'auth') {
        // Surface 共享身份验证表单。 Seed 来自任何先前存储的凭据的模式，以便用户无需单击即可登陆右侧选项卡。 When
        // 没有存储的凭据，请回退到 URL 方案，以便 SSH 远程在 SSH 选项卡（不是令牌）上打开。
        const mode = await preseedAuthMode(host, remoteUrl, getAuth);
        setStep({
          step: 'pull-auth',
          host,
          mode,
          error: t(`git.auth.error.${classification.code}`, {
            defaultValue: classification.message,
          }),
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
      const classification = classifyRemoteAuthError(err, 'push');
      if (classification.kind === 'auth') {
        const mode = await preseedAuthMode(host, remoteUrl, getAuth);
        setStep({
          step: 'push-auth',
          host,
          mode,
          error: t(`git.auth.error.${classification.code}`, {
            defaultValue: classification.message,
          }),
          busy: false,
        });
        return 'handled';
      }
      if (isGitError(err)) {
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
  remoteUrl: string | null,
  getAuth: (h: string) => Promise<GitAuthCreds | null>,
): Promise<GitPanelAuthFormMode> {
  if (host) {
    try {
      const stored = await getAuth(host);
      if (stored) return stored.kind === 'ssh' ? 'ssh' : 'token';
    } catch {
      // Fall through to URL-scheme inference on lookup failure.
    }
  }
  // No stored credential for this host — infer from the remote URL so
  // SSH remotes open on the SSH tab. Without this, a user with an SSH
  // remote but no stored creds would land on the token tab, a hard
  // dead-end for first-time pull/push attempts.
  return remoteUrl && isSshRemoteUrl(remoteUrl) ? 'ssh' : 'token';
}

function extractMessage(err: unknown): string {
  if (isGitError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
