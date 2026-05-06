// apps/web/src/components/panels/git-panel/git-panel-auth-form.tsx
//
// Shared inline auth form used by the Phase 6b pull/push retry flows and
// （稍后）Phase 6c 远程设置。 The 表单对于商店来说是无状态的
// perspective — the only store touchpoint is `sshKeys` for the SSH picker.
// Everything else 通过 props 传入，因此调用者可以重复使用它
// 令牌形身份验证、SSH 形身份验证以及任何未来的混合模式，无需
// 重新发明验证+“记住此主机”切换。
//
// Callers 提供：
//   - `mode`（来自之前的 authGet(host) 查找）
//   - `host`（用于默认令牌用户名和标头标签）
//   - `retryLabel` / `onSubmit(creds, remember)` / `onCancel`
//   - `busy` / `error` 因此父级拥有重试循环

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

  // Fire-然后忘记挂载刷新，因此第一次拉动会落在此处
  // 无需访问 SSH Keys 子视图仍然可以看到按键
  // 磁盘。 Without 这是一个使用 SSH 遥控器打开存储库的用户，
  // 从未导航到溢出 → SSH Keys 会看到一个空的选择器并且
  // “没有钥匙”的提示——他们只能离开才能逃脱的死胡同
  // auth 表单，打开密钥视图，然后重试拉取。
//
  // refreshSshKeys 是一个稳定的 zustand 动作参考，但我们保留它
  // 依赖于正确性；每个坐骑仍然只能触发一次该效果。
  useEffect(() => {
    void refreshSshKeys();
  }, [refreshSshKeys]);

  // Scope 当主机已知时，SSH 选择器绑定到该主机的键；如果过滤器为空或主机本身为空（无法解析远程
  // URL），则返回到完整列表。
  const hostKeys = useMemo(() => {
    if (!host) return sshKeys;
    const scoped = sshKeys.filter((k) => k.host === host);
    return scoped.length > 0 ? scoped : sshKeys;
  }, [sshKeys, host]);

  // 一旦密钥从异步刷新到达，Re-seed SSH 密钥选择。 The `useState`
  // 惰性初始化程序仅运行一次，因此如果 `sshKeys` 在挂载时为空，则 `sshKeyId` 将保留“”，并且即使
  // <select> 明显有一个选项，提交也会因“选择一个键”而失败验证。 Only 会在 `sshKeyId === ''`
  // 时触发，因此我们不会与明确的用户选择作斗争。
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
// Subcomponents — 内联，因此表单保留在一个文件中。
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
