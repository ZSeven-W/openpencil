// apps/web/src/components/panels/git-panel/git-panel-clone-form.tsx
//
// Phase 6a clone wizard body. Replaces the placeholder text from Phase 4.
// The form:
//   - takes a remote URL (HTTPS or SSH)
//   - lets the user pick a destination folder via window.electronAPI.openDirectory()
//   - infers the auth mode from the URL scheme (HTTPS → token-or-anon, SSH → ssh)
//   - allows anonymous clones for HTTPS by leaving username/token blank
//   - shows inline recoverable errors under the form (set by the store's
//     cloneRepo when a code in CLONE_INLINE_ERROR_CODES escapes)
//   - calls cloneRepo() on submit; success transitions handled by the store
//
// Cancel always returns to no-file via cancelCloneWizard() — git-panel.tsx's
// detect-repo effect rehydrates the correct no-repo / ready state from the
// currently-open document path on the next render.

import { useMemo, useState } from 'react';
import { Loader2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';
import {
  defaultTokenUsername,
  inferCloneAuthMode,
  parseRemoteHost,
  type CloneAuthMode,
} from './git-remote-utils';

const INPUT_CLASS =
  'h-8 bg-secondary border border-input rounded px-2 text-sm text-foreground focus:outline-none focus:border-ring';

interface ValidationErrors {
  url?: string;
  dest?: string;
  token?: string;
}

/**
 * Body of the wizard-clone screen. Reads the inline error and the `busy`
 * flag directly from the wizard-clone state. `busy` MUST live in the store
 * (not a local useState) so the form survives the wizard-clone → busy →
 * wizard-clone round-trip: a recoverable clone failure must leave the URL/
 * dest/token inputs exactly as the user typed them for retry. Keeping the
 * flag local would tie it to a component that, under the previous design,
 * unmounted mid-clone and wiped all the fields.
 */
export function GitPanelCloneForm() {
  const { t } = useTranslation();
  const cloneRepo = useGitStore((s) => s.cloneRepo);
  const cancelCloneWizard = useGitStore((s) => s.cancelCloneWizard);
  const wizardError = useGitStore((s) => (s.state.kind === 'wizard-clone' ? s.state.error : null));
  const busy = useGitStore((s) => (s.state.kind === 'wizard-clone' ? s.state.busy : false));

  const [url, setUrl] = useState('');
  const [dest, setDest] = useState('');
  const [tokenUsername, setTokenUsername] = useState('');
  const [token, setToken] = useState('');
  const [errors, setErrors] = useState<ValidationErrors>({});

  const authMode: CloneAuthMode = useMemo(() => inferCloneAuthMode(url), [url]);
  const host = useMemo(() => parseRemoteHost(url), [url]);

  const handlePickDest = async () => {
    if (typeof window === 'undefined' || !window.electronAPI) return;
    const picked = await window.electronAPI.openDirectory();
    if (!picked) return;
    setDest(picked);
  };

  const validate = (): boolean => {
    const next: ValidationErrors = {};
    if (!url.trim()) next.url = t('git.wizard.clone.validationUrl');
    if (!dest.trim()) next.dest = t('git.wizard.clone.validationDest');
    if (authMode === 'token-or-anon' && token.trim() && !tokenUsername.trim()) {
      next.token = t('git.wizard.clone.validationTokenUsername');
    }
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const handleSubmit = async () => {
    if (busy) return;
    if (!validate()) return;
    // No local busy tracking — the store flips wizard-clone.busy inside
    // cloneRepo and flips it back on recoverable failure. On success the
    // store transitions out of wizard-clone entirely, which unmounts this
    // component; any `finally { setBusy(false) }` would hit an unmounted
    // component and leak a state-update warning.
    const auth =
      authMode === 'token-or-anon' && token.trim()
        ? {
            kind: 'token' as const,
            username: tokenUsername.trim() || defaultTokenUsername(host),
            token: token.trim(),
          }
        : undefined;
    await cloneRepo({ url: url.trim(), dest: dest.trim(), auth });
  };

  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="text-sm font-medium text-foreground">{t('git.wizard.clone.heading')}</div>
      <div className="text-xs text-muted-foreground">{t('git.wizard.clone.subheading')}</div>

      <CloneFormUrlField
        url={url}
        onChange={setUrl}
        error={errors.url}
        host={host}
        authMode={authMode}
      />
      <CloneFormDestField
        dest={dest}
        onChange={setDest}
        onPick={() => void handlePickDest()}
        error={errors.dest}
      />
      {authMode === 'token-or-anon' && (
        <CloneFormTokenFields
          username={tokenUsername}
          token={token}
          onUsernameChange={setTokenUsername}
          onTokenChange={setToken}
          host={host}
          tokenError={errors.token}
        />
      )}
      {authMode === 'ssh' && (
        <div className="text-[11px] text-muted-foreground">{t('git.wizard.clone.sshHint')}</div>
      )}

      {wizardError && (
        <div
          role="alert"
          className="text-[11px] text-destructive border border-destructive/40 bg-destructive/10 rounded px-2 py-1.5"
        >
          {t(`git.wizard.clone.error.${wizardError.code}`, { defaultValue: wizardError.message })}
        </div>
      )}

      <div className="flex items-center justify-end gap-2 pt-1">
        <Button type="button" variant="ghost" size="sm" onClick={() => cancelCloneWizard()}>
          {t('git.wizard.clone.cancel')}
        </Button>
        <Button
          type="button"
          variant="default"
          size="sm"
          disabled={busy}
          onClick={() => void handleSubmit()}
        >
          {busy && <Loader2 size={14} className="mr-1 animate-spin" aria-hidden />}
          {t('git.wizard.clone.submit')}
        </Button>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Subcomponents — kept in the same file to stay under the LoC budget.
// ---------------------------------------------------------------------------

function CloneFormUrlField({
  url,
  onChange,
  error,
  host,
  authMode,
}: {
  url: string;
  onChange: (next: string) => void;
  error?: string;
  host: string | null;
  authMode: CloneAuthMode;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-muted-foreground" htmlFor="git-clone-url">
        {t('git.wizard.clone.urlLabel')}
      </label>
      <input
        id="git-clone-url"
        type="text"
        value={url}
        onChange={(e) => onChange(e.target.value)}
        placeholder={t('git.wizard.clone.urlPlaceholder')}
        className={INPUT_CLASS}
        autoComplete="off"
        spellCheck={false}
      />
      {error && <div className="text-[11px] text-destructive">{error}</div>}
      {!error && host && (
        <div className="text-[10px] text-muted-foreground">
          {t('git.wizard.clone.hostDetected', {
            host,
            mode: t(`git.wizard.clone.authMode.${authMode}`),
          })}
        </div>
      )}
    </div>
  );
}

function CloneFormDestField({
  dest,
  onChange,
  onPick,
  error,
}: {
  dest: string;
  onChange: (next: string) => void;
  onPick: () => void;
  error?: string;
}) {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-1">
      <label className="text-xs text-muted-foreground" htmlFor="git-clone-dest">
        {t('git.wizard.clone.destLabel')}
      </label>
      <div className="flex items-center gap-2">
        <input
          id="git-clone-dest"
          type="text"
          value={dest}
          onChange={(e) => onChange(e.target.value)}
          placeholder={t('git.wizard.clone.destPlaceholder')}
          className={`${INPUT_CLASS} flex-1`}
          autoComplete="off"
          spellCheck={false}
        />
        <Button type="button" variant="outline" size="sm" onClick={onPick}>
          {t('git.wizard.clone.destPickButton')}
        </Button>
      </div>
      {error && <div className="text-[11px] text-destructive">{error}</div>}
    </div>
  );
}

function CloneFormTokenFields({
  username,
  token,
  onUsernameChange,
  onTokenChange,
  host,
  tokenError,
}: {
  username: string;
  token: string;
  onUsernameChange: (next: string) => void;
  onTokenChange: (next: string) => void;
  host: string | null;
  tokenError?: string;
}) {
  const { t } = useTranslation();
  return (
    <>
      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground" htmlFor="git-clone-username">
          {t('git.wizard.clone.usernameLabel')}
        </label>
        <input
          id="git-clone-username"
          type="text"
          value={username}
          onChange={(e) => onUsernameChange(e.target.value)}
          placeholder={defaultTokenUsername(host)}
          className={INPUT_CLASS}
          autoComplete="off"
          spellCheck={false}
        />
      </div>
      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground" htmlFor="git-clone-token">
          {t('git.wizard.clone.tokenLabel')}
        </label>
        <input
          id="git-clone-token"
          type="password"
          value={token}
          onChange={(e) => onTokenChange(e.target.value)}
          placeholder={t('git.wizard.clone.tokenPlaceholder')}
          className={INPUT_CLASS}
          autoComplete="off"
          spellCheck={false}
        />
        {tokenError && <div className="text-[11px] text-destructive">{tokenError}</div>}
        <div className="text-[10px] text-muted-foreground">
          {t('git.wizard.clone.anonymousHint')}
        </div>
      </div>
    </>
  );
}
