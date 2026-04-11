// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-ssh-keys.test.tsx
//
// Phase 6c: SSH keys subview. Coverage matches the plan's verification list:
//   - refreshSshKeys on open
//   - list keys (with current-host float)
//   - generate / import / delete / copy public key
//   - provider link visibility for github / gitlab, generic guidance otherwise
//   - SSH iso gating banner
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { GitError } from '@/services/git-error';
import type { GitPublicSshKeyInfo, GitRemoteInfo } from '@/services/git-types';

interface ReadyStateMock {
  kind: 'ready';
  repo: {
    repoId: string;
    engineKind: 'iso' | 'sys';
    remote: GitRemoteInfo | null;
  };
}

const fx = {
  state: {
    kind: 'ready',
    repo: { repoId: 'r1', engineKind: 'sys' as const, remote: null },
  } as ReadyStateMock,
  sshKeys: [] as GitPublicSshKeyInfo[],
  refreshSshKeys: vi.fn(async () => {}),
  generateSshKey: vi.fn(
    async (_opts: { host: string; comment: string }): Promise<GitPublicSshKeyInfo> => ({
      id: 'new-key',
      host: 'github.com',
      publicKey: 'ssh-ed25519 NEW',
      fingerprint: 'SHA256:new',
      comment: 'new',
    }),
  ),
  importSshKey: vi.fn(
    async (_opts: { privateKeyPath: string; host: string }): Promise<GitPublicSshKeyInfo> => ({
      id: 'imported-key',
      host: 'github.com',
      publicKey: 'ssh-ed25519 IMP',
      fingerprint: 'SHA256:imp',
      comment: 'imp',
    }),
  ),
  deleteSshKey: vi.fn(async (_keyId: string) => {}),
};

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof fx) => unknown) => selector(fx),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

// Shared clipboard mock — navigator.clipboard is not present by default in
// jsdom. The writeText spy is returned so tests can assert on it.
let clipboardWriteSpy: ReturnType<typeof vi.fn>;

// Electron API mock — openFile returns a realistic payload shape.
const openFileMock = vi.fn(async () => null as { filePath: string; content: string } | null);

import { GitPanelSshKeys } from '@/components/panels/git-panel/git-panel-ssh-keys';

function setReady(patch: Partial<ReadyStateMock['repo']> = {}) {
  fx.state = {
    kind: 'ready',
    repo: { repoId: 'r1', engineKind: 'sys', remote: null, ...patch },
  };
}

describe('GitPanelSshKeys', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    fx.sshKeys = [];
    setReady();
    clipboardWriteSpy = vi.fn(async () => {});
    Object.defineProperty(global.navigator, 'clipboard', {
      configurable: true,
      value: { writeText: clipboardWriteSpy },
    });
    // Electron preload bridge
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (global as any).window.electronAPI = { openFile: openFileMock };
  });

  afterEach(() => {
    cleanup();
  });

  // ---- Refresh on mount -------------------------------------------------

  it('calls refreshSshKeys on mount', async () => {
    render(<GitPanelSshKeys onBack={() => {}} />);
    await waitFor(() => expect(fx.refreshSshKeys).toHaveBeenCalledTimes(1));
  });

  // ---- Empty state ------------------------------------------------------

  it('shows the empty state hint and two entry buttons when no keys exist', () => {
    fx.sshKeys = [];
    render(<GitPanelSshKeys onBack={() => {}} />);
    expect(screen.getByTestId('ssh-keys-empty')).toBeTruthy();
    expect(screen.getByText('git.ssh.generateAction')).toBeTruthy();
    expect(screen.getByText('git.ssh.importAction')).toBeTruthy();
  });

  // ---- List rendering + ordering ---------------------------------------

  it('floats keys matching the current remote host to the top of the list', () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.sshKeys = [
      {
        id: 'other',
        host: 'gitlab.com',
        publicKey: 'ssh A',
        fingerprint: 'SHA256:a',
        comment: 'other',
      },
      {
        id: 'gh',
        host: 'github.com',
        publicKey: 'ssh B',
        fingerprint: 'SHA256:b',
        comment: 'gh',
      },
    ];
    const { container } = render(<GitPanelSshKeys onBack={() => {}} />);
    const rows = container.querySelectorAll('[data-testid^="ssh-key-row-"]');
    expect(rows[0].getAttribute('data-testid')).toBe('ssh-key-row-gh');
    expect(rows[1].getAttribute('data-testid')).toBe('ssh-key-row-other');
  });

  // ---- Copy public key --------------------------------------------------

  it('copy button writes the publicKey to the clipboard', async () => {
    fx.sshKeys = [
      {
        id: 'gh',
        host: 'github.com',
        publicKey: 'ssh-ed25519 AAAA',
        fingerprint: 'SHA256:aaa',
        comment: 'gh',
      },
    ];
    render(<GitPanelSshKeys onBack={() => {}} />);
    const copyBtn = screen.getByLabelText('git.ssh.copyPublicKey');
    fireEvent.click(copyBtn);
    await waitFor(() => expect(clipboardWriteSpy).toHaveBeenCalledWith('ssh-ed25519 AAAA'));
    // Success hint renders and is announced to assistive tech.
    const hint = await waitFor(() => screen.getByText('git.ssh.copiedHint'));
    expect(hint.getAttribute('aria-live')).toBe('polite');
    expect(hint.getAttribute('role')).toBe('status');
  });

  it('copy falls back to copyUnsupported inline error when navigator.clipboard is undefined', async () => {
    fx.sshKeys = [
      {
        id: 'gh',
        host: 'github.com',
        publicKey: 'ssh-ed25519 AAAA',
        fingerprint: 'SHA256:aaa',
        comment: 'gh',
      },
    ];
    // Simulate a non-secure context (http:// or file://) where the
    // Clipboard API is gated off and `navigator.clipboard` is undefined.
    Object.defineProperty(global.navigator, 'clipboard', {
      configurable: true,
      value: undefined,
    });
    render(<GitPanelSshKeys onBack={() => {}} />);
    const copyBtn = screen.getByLabelText('git.ssh.copyPublicKey');
    fireEvent.click(copyBtn);
    await waitFor(() => expect(screen.getByText('git.ssh.copyUnsupported')).toBeTruthy());
    // The copied flash must NOT appear when clipboard is unavailable.
    expect(screen.queryByText('git.ssh.copiedHint')).toBeNull();
  });

  // ---- Provider link visibility ----------------------------------------

  it('shows the provider link for github.com', () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.sshKeys = [
      {
        id: 'gh',
        host: 'github.com',
        publicKey: 'ssh',
        fingerprint: 'SHA256:a',
        comment: 'gh',
      },
    ];
    render(<GitPanelSshKeys onBack={() => {}} />);
    const link = screen.getByTestId('ssh-provider-link') as HTMLAnchorElement;
    expect(link.href).toBe('https://github.com/settings/keys');
  });

  it('shows the provider link for gitlab.com', () => {
    setReady({
      remote: { name: 'origin', url: 'https://gitlab.com/foo/bar.git', host: 'gitlab.com' },
    });
    fx.sshKeys = [
      {
        id: 'gl',
        host: 'gitlab.com',
        publicKey: 'ssh',
        fingerprint: 'SHA256:a',
        comment: 'gl',
      },
    ];
    render(<GitPanelSshKeys onBack={() => {}} />);
    const link = screen.getByTestId('ssh-provider-link') as HTMLAnchorElement;
    expect(link.href).toBe('https://gitlab.com/-/profile/keys');
  });

  it('shows generic guidance (no provider link) for unknown hosts', () => {
    setReady({
      remote: { name: 'origin', url: 'https://gitea.local/foo/bar.git', host: 'gitea.local' },
    });
    fx.sshKeys = [
      {
        id: 'k',
        host: 'gitea.local',
        publicKey: 'ssh',
        fingerprint: 'SHA256:a',
        comment: 'k',
      },
    ];
    render(<GitPanelSshKeys onBack={() => {}} />);
    expect(screen.queryByTestId('ssh-provider-link')).toBeNull();
    expect(screen.getByText('git.ssh.genericGuidance')).toBeTruthy();
  });

  // ---- Generate flow ----------------------------------------------------

  it('generate submits host + comment and returns to list on success', async () => {
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.ssh.generateAction'));
    fireEvent.change(screen.getByLabelText('git.ssh.hostLabel'), {
      target: { value: 'github.com' },
    });
    fireEvent.change(screen.getByLabelText('git.ssh.commentLabel'), {
      target: { value: 'laptop' },
    });
    fireEvent.click(screen.getByText('git.ssh.generateSubmit'));
    await waitFor(() =>
      expect(fx.generateSshKey).toHaveBeenCalledWith({ host: 'github.com', comment: 'laptop' }),
    );
  });

  it('generate validation blocks submit when host or comment is blank', async () => {
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.ssh.generateAction'));
    // Host blank → validationHost
    fireEvent.click(screen.getByText('git.ssh.generateSubmit'));
    await waitFor(() => expect(screen.getByText('git.ssh.validationHost')).toBeTruthy());
    expect(fx.generateSshKey).not.toHaveBeenCalled();
  });

  it('maps ssh-not-supported-iso from generateSshKey to the localized hint', async () => {
    // Mirror the remote-settings pattern: a GitError with the iso gate code
    // must surface the localized `git.ssh.isoUnsupported` string, NOT the
    // raw engine-level error message.
    fx.generateSshKey.mockRejectedValueOnce(
      new GitError('ssh-not-supported-iso', 'raw engine error text'),
    );
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.ssh.generateAction'));
    fireEvent.change(screen.getByLabelText('git.ssh.hostLabel'), {
      target: { value: 'github.com' },
    });
    fireEvent.change(screen.getByLabelText('git.ssh.commentLabel'), {
      target: { value: 'laptop' },
    });
    fireEvent.click(screen.getByText('git.ssh.generateSubmit'));
    await waitFor(() => expect(screen.getByText('git.ssh.isoUnsupported')).toBeTruthy());
    expect(screen.queryByText('raw engine error text')).toBeNull();
  });

  // ---- Import flow ------------------------------------------------------

  it('import passes the picked filePath into importSshKey', async () => {
    openFileMock.mockResolvedValueOnce({ filePath: '/tmp/id_ed25519', content: '' });
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.ssh.importAction'));
    fireEvent.change(screen.getByLabelText('git.ssh.hostLabel'), {
      target: { value: 'github.com' },
    });
    fireEvent.click(screen.getByText('git.ssh.importBrowse'));
    await waitFor(() => {
      const input = screen.getByLabelText('git.ssh.importPathLabel') as HTMLInputElement;
      expect(input.value).toBe('/tmp/id_ed25519');
    });
    fireEvent.click(screen.getByText('git.ssh.importSubmit'));
    await waitFor(() =>
      expect(fx.importSshKey).toHaveBeenCalledWith({
        privateKeyPath: '/tmp/id_ed25519',
        host: 'github.com',
      }),
    );
  });

  it('import validation blocks submit when path is blank', async () => {
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.ssh.importAction'));
    fireEvent.change(screen.getByLabelText('git.ssh.hostLabel'), {
      target: { value: 'github.com' },
    });
    fireEvent.click(screen.getByText('git.ssh.importSubmit'));
    await waitFor(() => expect(screen.getByText('git.ssh.validationImportPath')).toBeTruthy());
    expect(fx.importSshKey).not.toHaveBeenCalled();
  });

  // ---- Delete flow ------------------------------------------------------

  it('delete flow fires deleteSshKey after confirm', async () => {
    fx.sshKeys = [
      {
        id: 'gh',
        host: 'github.com',
        publicKey: 'ssh',
        fingerprint: 'SHA256:a',
        comment: 'gh',
      },
    ];
    render(<GitPanelSshKeys onBack={() => {}} />);
    fireEvent.click(screen.getByLabelText(/git\.ssh\.deleteKey/));
    // Confirm shown
    expect(screen.getByText(/git\.ssh\.deletePrompt/)).toBeTruthy();
    fireEvent.click(screen.getByText('git.ssh.deleteConfirm'));
    await waitFor(() => expect(fx.deleteSshKey).toHaveBeenCalledWith('gh'));
  });

  // ---- SSH iso gating banner -------------------------------------------

  it('shows the iso-unsupported banner when engineKind=iso and URL is SSH', () => {
    setReady({
      engineKind: 'iso',
      remote: { name: 'origin', url: 'git@github.com:foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelSshKeys onBack={() => {}} />);
    expect(screen.getByRole('note').textContent).toContain('git.ssh.isoUnsupported');
  });

  // ---- Back navigation --------------------------------------------------

  it('back button at the list view calls onBack', () => {
    const onBack = vi.fn();
    render(<GitPanelSshKeys onBack={onBack} />);
    fireEvent.click(screen.getByLabelText('git.ssh.back'));
    expect(onBack).toHaveBeenCalledTimes(1);
  });

  it('back button inside generate subview returns to list (does NOT call onBack)', () => {
    const onBack = vi.fn();
    render(<GitPanelSshKeys onBack={onBack} />);
    fireEvent.click(screen.getByText('git.ssh.generateAction'));
    fireEvent.click(screen.getByLabelText('git.ssh.back'));
    expect(onBack).not.toHaveBeenCalled();
    expect(screen.getByTestId('ssh-keys-empty')).toBeTruthy();
  });
});
