// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-auth-form.test.tsx
//
// Shared auth form — used by BOTH pull and push retry flows. The tests
// here cover the three acceptance criteria for Phase 6b:
//   1. token-mode validation (token required) + username default
//   2. SSH-mode selection from sshKeys
//   3. remember toggle flows through to onSubmit
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';
import type { GitPublicSshKeyInfo } from '@/services/git-types';

// Mutable sshKeys so individual tests can seed the picker. The store mock
// below re-reads the live value on every selector call.
let mockedSshKeys: GitPublicSshKeyInfo[] = [];

vi.mock('@/stores/git-store', () => {
  const useGitStore = (selector: (s: { sshKeys: GitPublicSshKeyInfo[] }) => unknown) =>
    selector({ sshKeys: mockedSshKeys });
  return { useGitStore };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelAuthForm } from '@/components/panels/git-panel/git-panel-auth-form';

describe('GitPanelAuthForm', () => {
  beforeEach(() => {
    mockedSshKeys = [];
  });

  afterEach(() => {
    cleanup();
  });

  // ---- Token mode -------------------------------------------------------

  it('token mode: blocks submit with a validation error when the token is blank', () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );
    fireEvent.click(screen.getByText('retry'));
    expect(screen.getByText('git.auth.validationToken')).toBeTruthy();
    expect(onSubmit).not.toHaveBeenCalled();
  });

  it('token mode: fills defaultTokenUsername for GitHub when username is blank', async () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );
    fireEvent.change(screen.getByLabelText('git.auth.tokenLabel'), {
      target: { value: 'ghp_xyz' },
    });
    fireEvent.click(screen.getByText('retry'));
    // Allow the async submit to settle.
    await Promise.resolve();
    expect(onSubmit).toHaveBeenCalledWith(
      { kind: 'token', username: 'git', token: 'ghp_xyz' },
      true,
    );
  });

  it('token mode: explicit username overrides the default', async () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );
    fireEvent.change(screen.getByLabelText('git.auth.usernameLabel'), {
      target: { value: 'alice' },
    });
    fireEvent.change(screen.getByLabelText('git.auth.tokenLabel'), {
      target: { value: 'ghp_xyz' },
    });
    fireEvent.click(screen.getByText('retry'));
    await Promise.resolve();
    expect(onSubmit).toHaveBeenCalledWith(
      { kind: 'token', username: 'alice', token: 'ghp_xyz' },
      true,
    );
  });

  // ---- SSH mode ---------------------------------------------------------

  it('ssh mode: surfaces a no-keys hint when the picker would be empty', () => {
    render(
      <GitPanelAuthForm
        mode="ssh"
        host="github.com"
        retryLabel="retry"
        onSubmit={() => {}}
        onCancel={() => {}}
      />,
    );
    expect(screen.getByText('git.auth.sshNoKeys')).toBeTruthy();
  });

  it('ssh mode: selects the first key and passes its id to onSubmit', async () => {
    mockedSshKeys = [
      {
        id: 'key-1',
        host: 'github.com',
        publicKey: 'ssh-ed25519 AAA…',
        fingerprint: 'SHA256:aaa',
        comment: 'gh',
      },
      {
        id: 'key-2',
        host: 'github.com',
        publicKey: 'ssh-ed25519 BBB…',
        fingerprint: 'SHA256:bbb',
        comment: 'gh-backup',
      },
    ];

    const onSubmit = vi.fn();
    render(
      <GitPanelAuthForm
        mode="ssh"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );
    // Default selection is the first key. Switch to the second and submit.
    fireEvent.change(screen.getByLabelText('git.auth.sshKeyLabel'), {
      target: { value: 'key-2' },
    });
    fireEvent.click(screen.getByText('retry'));
    await Promise.resolve();
    expect(onSubmit).toHaveBeenCalledWith({ kind: 'ssh', keyId: 'key-2' }, true);
  });

  it('ssh mode: falls back to showing all keys when no key is bound to this host', () => {
    mockedSshKeys = [
      {
        id: 'key-1',
        host: 'other.com',
        publicKey: 'ssh-ed25519 CCC…',
        fingerprint: 'SHA256:ccc',
        comment: 'other',
      },
    ];
    render(
      <GitPanelAuthForm
        mode="ssh"
        host="github.com"
        retryLabel="retry"
        onSubmit={() => {}}
        onCancel={() => {}}
      />,
    );
    // The picker is rendered even though no key is scoped to github.com —
    // the component falls back to the full list so the user still has a
    // way to proceed.
    const select = screen.getByLabelText('git.auth.sshKeyLabel') as HTMLSelectElement;
    expect(select.options).toHaveLength(1);
    expect(select.options[0].value).toBe('key-1');
  });

  // ---- Remember toggle --------------------------------------------------

  it('remember toggle: defaults to on and propagates through onSubmit', async () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={() => {}}
      />,
    );
    // Checkbox default is `true`.
    const checkbox = screen.getByLabelText('git.auth.rememberLabel') as HTMLInputElement;
    expect(checkbox.checked).toBe(true);

    // Toggle off and submit.
    fireEvent.click(checkbox);
    fireEvent.change(screen.getByLabelText('git.auth.tokenLabel'), {
      target: { value: 'ghp_xyz' },
    });
    fireEvent.click(screen.getByText('retry'));
    await Promise.resolve();

    expect(onSubmit).toHaveBeenCalledWith(
      { kind: 'token', username: 'git', token: 'ghp_xyz' },
      false,
    );
  });

  // ---- Cancel -----------------------------------------------------------

  it('cancel button calls onCancel without submitting', () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );
    fireEvent.click(screen.getByText('git.auth.cancel'));
    expect(onCancel).toHaveBeenCalledTimes(1);
    expect(onSubmit).not.toHaveBeenCalled();
  });

  // ---- Busy + error ----------------------------------------------------

  it('busy=true disables the retry button', () => {
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={() => {}}
        onCancel={() => {}}
        busy
      />,
    );
    const retry = screen.getByText('retry').closest('button')!;
    expect(retry.disabled).toBe(true);
  });

  it('error prop renders a role=alert banner', () => {
    render(
      <GitPanelAuthForm
        mode="token"
        host="github.com"
        retryLabel="retry"
        onSubmit={() => {}}
        onCancel={() => {}}
        error="token rejected"
      />,
    );
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('token rejected');
  });
});
