// apps/web/src/components/panels/git-panel/git-panel-auth-form.tsx
//
// Shared inline auth form used by the Phase 6b pull/push retry flows and
// (later) Phase 6c remote settings. The form is stateless from the store's
// perspective — the only store touchpoint is `sshKeys` for the SSH picker.
// Everything else is passed in via props so callers can reuse it for
// token-shaped auth, SSH-shaped auth, and any future hybrid mode without
// reinventing the validation + "remember this host" toggle.
//
// Callers provide:
//   - `mode` (seeded from a previous authGet(host) lookup)
//   - `host` (for the default token-username and header label)
//   - `retryLabel` / `onSubmit(creds, remember)` / `onCancel`
//   - `busy` / `error` so the parent owns the retry loop

import { Loader2 } from 'lucide-react';
import { useEffect, useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import type { GitAuthCreds } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import { defaultTokenUsername } from './git-remote-utils';

const INPUT_CLASS =
  'h-8 bg-secondary border border-input rounded px-2 text-sm text-foreground focus:outline-none focus:border-ring';

export type GitPanelAuthFormMode = 'token' | 'ssh';

export interface GitPanelAuthFormProps {
  mode: GitPanelAuthFormMode;
  host: string | null;
  retryLabel: string;
  onSubmit: (creds: GitAuthCreds, remember: boolean) => void | Promise<void>;
  onCancel: () => void;
  busy?: boolean;
  error?: string | null;
}

interface ValidationErrors {
  token?: string;
  keyId?: string;
}

export function GitPanelAuthForm({
  mode: initialMode,
  host,
  retryLabel,
  onSubmit,
  onCancel,
  busy = false,
  error = null,
}: GitPanelAuthFormProps) {
  const { t } = useTranslation();
  const sshKeys = useGitStore((s) => s.sshKeys);
  const refreshSshKeys = useGitStore((s) => s.refreshSshKeys);

  const [mode, setMode] = useState<GitPanelAuthFormMode>(initialMode);
  const [tokenUsername, setTokenUsername] = useState('');
  const [token, setToken] = useState('');
  const [sshKeyId, setSshKeyId] = useState<string>(() => sshKeys[0]?.id ?? '');
  const [remember, setRemember] = useState(true);
  const [errors, setErrors] = useState<ValidationErrors>({});

  // Fire-and-forget refresh on mount so a first-time pull that lands here
  // without ever visiting the SSH Keys subview still sees the keys on
  // disk. Without this, a user who opened a repo with an SSH remote and
  // never navigated to overflow → SSH Keys would see an empty picker and
  // a "no keys" hint — a hard dead-end they can only escape by leaving
  // the auth form, opening the keys view, and retrying the pull.
  //
  // refreshSshKeys is a stable zustand action reference but we keep it
  // in deps for correctness; the effect still fires only once per mount.
  useEffect(() => {
    void refreshSshKeys();
  }, [refreshSshKeys]);

  // Scope the SSH picker to keys bound to this host when the host is
  // known; fall back to the full list if the filter would be empty or
  // the host itself is null (unparseable remote URL).
  const hostKeys = useMemo(() => {
    if (!host) return sshKeys;
    const scoped = sshKeys.filter((k) => k.host === host);
    return scoped.length > 0 ? scoped : sshKeys;
  }, [sshKeys, host]);

  // Re-seed the SSH key selection once keys arrive from the async
  // refresh. The `useState` lazy initializer runs exactly once, so if
  // `sshKeys` was empty on mount, `sshKeyId` would stay '' and submit
  // would fail validation with "select a key" even though the <select>
  // visibly has an option. Only fire while `sshKeyId === ''` so we
  // don't fight an explicit user selection.
  useEffect(() => {
    if (sshKeyId === '' && hostKeys[0]) {
      setSshKeyId(hostKeys[0].id);
    }
  }, [hostKeys, sshKeyId]);

  const handleSubmit = async () => {
    if (busy) return;
    const next: ValidationErrors = {};
    if (mode === 'token') {
      if (!token.trim()) next.token = t('git.auth.validationToken');
    } else if (!sshKeyId) {
      next.keyId = t('git.auth.validationSshKey');
    }
    setErrors(next);
    if (Object.keys(next).length > 0) return;

    const creds: GitAuthCreds =
      mode === 'token'
        ? {
            kind: 'token',
            username: tokenUsername.trim() || defaultTokenUsername(host),
            token: token.trim(),
          }
        : { kind: 'ssh', keyId: sshKeyId };
    await onSubmit(creds, remember);
  };

  return (
    <div
      role="group"
      aria-label={t('git.auth.formLabel')}
      className="flex flex-col gap-2 rounded border border-border bg-card p-3"
    >
      <div className="flex items-center justify-between gap-2">
        <div className="text-xs font-medium text-foreground">
          {host ? t('git.auth.heading', { host }) : t('git.auth.headingUnknown')}
        </div>
        <ModeToggle mode={mode} onChange={setMode} />
      </div>

      {mode === 'token' ? (
        <>
          <LabeledInput
            id="git-auth-username"
            label={t('git.auth.usernameLabel')}
            value={tokenUsername}
            onChange={setTokenUsername}
            placeholder={defaultTokenUsername(host)}
          />
          <LabeledInput
            id="git-auth-token"
            label={t('git.auth.tokenLabel')}
            value={token}
            onChange={setToken}
            placeholder={t('git.auth.tokenPlaceholder')}
            type="password"
            fieldError={errors.token}
          />
        </>
      ) : hostKeys.length === 0 ? (
        <div className="text-[11px] text-muted-foreground">{t('git.auth.sshNoKeys')}</div>
      ) : (
        <div className="flex flex-col gap-1">
          <label className="text-[11px] text-muted-foreground" htmlFor="git-auth-ssh-key">
            {t('git.auth.sshKeyLabel')}
          </label>
          <select
            id="git-auth-ssh-key"
            value={sshKeyId}
            onChange={(e) => setSshKeyId(e.target.value)}
            className={INPUT_CLASS}
          >
            {hostKeys.map((k) => (
              <option key={k.id} value={k.id}>
                {k.comment || k.fingerprint} · {k.host}
              </option>
            ))}
          </select>
          {errors.keyId && <div className="text-[11px] text-destructive">{errors.keyId}</div>}
        </div>
      )}

      <label className="flex items-center gap-2 text-[11px] text-muted-foreground">
        <input
          type="checkbox"
          checked={remember}
          onChange={(e) => setRemember(e.target.checked)}
          aria-label={t('git.auth.rememberLabel')}
          className="h-3 w-3 accent-primary"
        />
        {t('git.auth.rememberHint')}
      </label>

      {error && (
        <div
          role="alert"
          className="text-[11px] text-destructive border border-destructive/40 bg-destructive/10 rounded px-2 py-1.5"
        >
          {error}
        </div>
      )}

      <div className="flex items-center justify-end gap-2 pt-1">
        <Button type="button" variant="ghost" size="sm" onClick={onCancel}>
          {t('git.auth.cancel')}
        </Button>
        <Button
          type="button"
          variant="default"
          size="sm"
          disabled={busy}
          onClick={() => void handleSubmit()}
        >
          {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
          {retryLabel}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Subcomponents — inline so the form stays in one file.
// ---------------------------------------------------------------------------

function ModeToggle({
  mode,
  onChange,
}: {
  mode: GitPanelAuthFormMode;
  onChange: (next: GitPanelAuthFormMode) => void;
}) {
  const { t } = useTranslation();
  const tabClass = (active: boolean) =>
    `h-5 rounded px-2 text-[10px] ${
      active ? 'bg-background text-foreground shadow-sm' : 'text-muted-foreground'
    }`;
  return (
    <div
      role="tablist"
      aria-label={t('git.auth.modeToggleLabel')}
      className="inline-flex h-6 items-center rounded border border-border bg-secondary p-0.5"
    >
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'token'}
        onClick={() => onChange('token')}
        className={tabClass(mode === 'token')}
      >
        {t('git.auth.modeToken')}
      </button>
      <button
        type="button"
        role="tab"
        aria-selected={mode === 'ssh'}
        onClick={() => onChange('ssh')}
        className={tabClass(mode === 'ssh')}
      >
        {t('git.auth.modeSsh')}
      </button>
    </div>
  );
}

function LabeledInput({
  id,
  label,
  value,
  onChange,
  placeholder,
  type = 'text',
  fieldError,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (next: string) => void;
  placeholder?: string;
  type?: 'text' | 'password';
  fieldError?: string;
}) {
  return (
    <div className="flex flex-col gap-1">
      <label className="text-[11px] text-muted-foreground" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        type={type}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        placeholder={placeholder}
        className={INPUT_CLASS}
        autoComplete="off"
        spellCheck={false}
      />
      {fieldError && <div className="text-[11px] text-destructive">{fieldError}</div>}
    </div>
  );
}
