// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-remote-settings.test.tsx
//
// Phase 6c: remote settings subview. Coverage matches the plan's
// verification list:
//   - load / edit / save / clear remote URL
//   - fetch button
//   - clear auth button visibility + action
//   - SSH iso gating text
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import type { GitAuthCreds, GitRemoteInfo } from '@/services/git-types';

interface ReadyStateMock {
  kind: 'ready';
  repo: {
    repoId: string;
    engineKind: 'iso' | 'sys';
    ahead: number;
    behind: number;
    remote: GitRemoteInfo | null;
  };
}

const fx = {
  state: {
    kind: 'ready',
    repo: {
      repoId: 'r1',
      engineKind: 'sys' as const,
      ahead: 0,
      behind: 0,
      remote: null,
    },
  } as ReadyStateMock,
  refreshRemote: vi.fn(async () => {}),
  setRemoteUrl: vi.fn(async (_url: string | null) => {}),
  fetchRemote: vi.fn(async (_auth?: unknown) => {}),
  getAuth: vi.fn(async (_host: string): Promise<GitAuthCreds | null> => null),
  clearAuth: vi.fn(async (_host: string) => {}),
};

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof fx) => unknown) => selector(fx),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelRemoteSettings } from '@/components/panels/git-panel/git-panel-remote-settings';

function setReady(patch: Partial<ReadyStateMock['repo']> = {}) {
  fx.state = {
    kind: 'ready',
    repo: {
      repoId: 'r1',
      engineKind: 'sys',
      ahead: 0,
      behind: 0,
      remote: null,
      ...patch,
    },
  };
}

describe('GitPanelRemoteSettings', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setReady();
  });

  afterEach(() => {
    cleanup();
  });

  it('refreshes the cached remote on mount', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    await waitFor(() => expect(fx.refreshRemote).toHaveBeenCalledTimes(1));
  });

  it('shows the empty-no-origin hint when no remote is configured', () => {
    setReady({ remote: null });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    expect(screen.getByText('git.remote.emptyNoOrigin')).toBeTruthy();
  });

  it('hydrates the input from repo.remote.url', () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    const input = screen.getByLabelText('git.remote.urlLabel') as HTMLInputElement;
    expect(input.value).toBe('https://github.com/foo/bar.git');
  });

  it('save button is disabled when the draft matches the current URL', () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    const save = screen.getByText('git.remote.saveButton').closest('button')!;
    expect(save.disabled).toBe(true);
  });

  it('edits + saves the URL via setRemoteUrl', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    const input = screen.getByLabelText('git.remote.urlLabel') as HTMLInputElement;
    fireEvent.change(input, { target: { value: 'https://github.com/foo/baz.git' } });
    fireEvent.click(screen.getByText('git.remote.saveButton'));
    await waitFor(() =>
      expect(fx.setRemoteUrl).toHaveBeenCalledWith('https://github.com/foo/baz.git'),
    );
  });

  it('clearing the URL requires a confirm step', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.remote.clearButton'));
    // Confirm UI appears
    expect(screen.getByText('git.remote.clearConfirmBody')).toBeTruthy();
    // setRemoteUrl NOT called yet
    expect(fx.setRemoteUrl).not.toHaveBeenCalled();
    // Confirm the clear
    fireEvent.click(screen.getByText('git.remote.clearConfirmAction'));
    await waitFor(() => expect(fx.setRemoteUrl).toHaveBeenCalledWith(null));
  });

  it('cancel on the clear-confirm abandons without calling setRemoteUrl', () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.remote.clearButton'));
    fireEvent.click(screen.getByText('git.remote.cancel'));
    expect(screen.queryByText('git.remote.clearConfirmBody')).toBeNull();
    expect(fx.setRemoteUrl).not.toHaveBeenCalled();
  });

  it('fetch button calls fetchRemote without auth', async () => {
    setReady({
      ahead: 1,
      behind: 2,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    fireEvent.click(screen.getByText('git.remote.fetchButton'));
    await waitFor(() => expect(fx.fetchRemote).toHaveBeenCalledTimes(1));
  });

  it('fetch button is disabled when no remote URL is configured', () => {
    setReady({ remote: null });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    const fetchBtn = screen.getByText('git.remote.fetchButton').closest('button')!;
    expect(fetchBtn.disabled).toBe(true);
  });

  it('renders the ahead/behind counts from repo', () => {
    setReady({
      ahead: 4,
      behind: 7,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    expect(screen.getByText(/git.remote.aheadBehind:/)).toBeTruthy();
    expect(screen.getByText(/"ahead":4/)).toBeTruthy();
    expect(screen.getByText(/"behind":7/)).toBeTruthy();
  });

  it('loads the stored-auth mode via getAuth and renders the label', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce({ kind: 'token', username: 'git', token: 'ghp_xyz' });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    await waitFor(() => expect(screen.getByText('git.remote.storedAuth.token')).toBeTruthy());
  });

  it('renders the "none" stored-auth label when getAuth returns null', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce(null);
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    await waitFor(() => expect(screen.getByText('git.remote.storedAuth.none')).toBeTruthy());
    // Clear button NOT rendered when nothing is stored
    expect(screen.queryByText('git.remote.clearAuthButton')).toBeNull();
  });

  it('clear-auth button calls clearAuth(host) and hides itself afterwards', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce({ kind: 'ssh', keyId: 'key-1' });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    await waitFor(() => expect(screen.getByText('git.remote.storedAuth.ssh')).toBeTruthy());
    fireEvent.click(screen.getByText('git.remote.clearAuthButton'));
    await waitFor(() => expect(fx.clearAuth).toHaveBeenCalledWith('github.com'));
    // After clearing, the UI reflects none
    await waitFor(() => expect(screen.getByText('git.remote.storedAuth.none')).toBeTruthy());
  });

  it('surfaces SSH iso-unsupported guidance when engineKind=iso and URL is SSH', () => {
    setReady({
      engineKind: 'iso',
      remote: { name: 'origin', url: 'git@github.com:foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    expect(screen.getByRole('note').textContent).toContain('git.remote.sshIsoUnsupported');
  });

  it('does not show SSH gating text when engineKind=sys even on an SSH URL', () => {
    setReady({
      engineKind: 'sys',
      remote: { name: 'origin', url: 'git@github.com:foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={() => {}} />);
    expect(screen.queryByRole('note')).toBeNull();
  });

  it('back button calls onBack', () => {
    const onBack = vi.fn();
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    render(<GitPanelRemoteSettings onBack={onBack} />);
    fireEvent.click(screen.getByLabelText('git.remote.back'));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
