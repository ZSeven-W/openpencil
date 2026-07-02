// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-remote-controls.test.tsx
//
// Phase 6b: the pull/push header controls. Coverage matches the plan's
// verification list:
//   - disabled-without-remote
//   - pull success
//   - auth prompt path (pull)
//   - pull conflict-non-op
//   - push-rejected path
//   - push-auth-failed path
//   - loading state
import React from 'react';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import { TooltipProvider } from '@/components/ui/tooltip';
import { GitError } from '@/services/git-error';
import type { GitAuthCreds, GitRemoteInfo } from '@/services/git-types';

// Mutable fixtures so each test can swap the active store state and
// mock responses without re-wiring the mock module.
interface ReadyStateMock {
  kind: 'ready';
  repo: {
    repoId: string;
    ahead: number;
    remote: GitRemoteInfo | null;
  };
}
interface ConflictStateMock {
  kind: 'conflict';
  repo: {
    repoId: string;
    ahead: number;
    remote: GitRemoteInfo | null;
  };
  conflicts: { nodeConflicts: Map<string, unknown>; docFieldConflicts: Map<string, unknown> };
  unresolvedFiles: string[];
}

const fx = {
  state: {
    kind: 'ready',
    repo: {
      repoId: 'r1',
      ahead: 0,
      remote: null,
    },
  } as ReadyStateMock | ConflictStateMock,
  sshKeys: [] as Array<unknown>,
  // The embedded auth form reads refreshSshKeys on mount (Phase 6b fix
  // for first-time SSH pulls). Provide a no-op so the effect doesn't
  // crash the render.
  refreshSshKeys: vi.fn(async () => {}),
  pull: vi.fn(async (_auth?: unknown) => {}),
  push: vi.fn(async (_auth?: unknown) => {}),
  // Explicit return type so mockResolvedValueOnce can return either null
  // or a stored credential record without fighting vitest's inference.
  getAuth: vi.fn(async (_host: string): Promise<GitAuthCreds | null> => null),
  storeAuth: vi.fn(async (_host: string, _creds: unknown) => {}),
};

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof fx) => unknown) => selector(fx),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelRemoteControls } from '@/components/panels/git-panel/git-panel-remote-controls';

function renderWithProvider(ui: React.ReactElement) {
  return render(<TooltipProvider>{ui}</TooltipProvider>);
}

function setReady(repo: Partial<ReadyStateMock['repo']> = {}) {
  fx.state = {
    kind: 'ready',
    repo: {
      repoId: 'r1',
      ahead: 0,
      remote: null,
      ...repo,
    },
  };
}

function setConflict(repo: Partial<ConflictStateMock['repo']> = {}) {
  fx.state = {
    kind: 'conflict',
    repo: {
      repoId: 'r1',
      ahead: 0,
      remote: null,
      ...repo,
    },
    conflicts: { nodeConflicts: new Map(), docFieldConflicts: new Map() },
    unresolvedFiles: [],
  };
}

describe('GitPanelRemoteControls', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    setReady();
  });

  afterEach(() => {
    cleanup();
  });

  // ---- Conflict gating --------------------------------------------------

  it('renders nothing while the repo is in conflict state (the conflict banner owns recovery)', () => {
    // Pull/push during an in-flight merge is a footgun — pull fails
    // deterministically and push would try to push a half-merged tree.
    // The component must bail out cleanly so the conflict banner can
    // drive recovery without racing a stray button click.
    setConflict({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
      ahead: 2,
    });
    const { container } = renderWithProvider(<GitPanelRemoteControls />);
    // TooltipProvider wraps the component; the component itself renders
    // null, so the provider's children set must be empty.
    expect(container.firstChild?.childNodes.length ?? 0).toBe(0);
    expect(screen.queryByLabelText('git.pull.label')).toBeNull();
    expect(screen.queryByLabelText('git.push.label')).toBeNull();
  });

  // ---- Disabled / no-remote --------------------------------------------

  it('renders pull + push disabled and pointer-events-none when no remote is configured', () => {
    setReady({ remote: null });
    renderWithProvider(<GitPanelRemoteControls />);
    const pull = screen.getByLabelText('git.pull.label').closest('button')!;
    const push = screen.getByLabelText('git.push.label').closest('button')!;
    expect(pull.disabled).toBe(true);
    expect(push.disabled).toBe(true);
    expect(pull.className).toContain('pointer-events-none');
  });

  it('renders push disabled when ahead=0 even if the remote is configured', () => {
    setReady({
      ahead: 0,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    renderWithProvider(<GitPanelRemoteControls />);
    const push = screen.getByLabelText('git.push.label').closest('button')!;
    expect(push.disabled).toBe(true);
    // Pull stays enabled — you can still pull at ahead=0.
    const pull = screen.getByLabelText('git.pull.label').closest('button')!;
    expect(pull.disabled).toBe(false);
  });

  // ---- Pull success -----------------------------------------------------

  it('pull success: fires the pull action once and leaves no inline UI behind', async () => {
    setReady({
      ahead: 2,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.pull.mockResolvedValueOnce(undefined);

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));
    await waitFor(() => expect(fx.pull).toHaveBeenCalledTimes(1));

    // No auth form, no push-rejected strip, no error card
    expect(screen.queryByLabelText('git.auth.formLabel')).toBeNull();
    expect(screen.queryByText('git.push.rejectedBody')).toBeNull();
  });

  // ---- Pull auth prompt -------------------------------------------------

  it('pull auth-required: opens the shared auth form inline', async () => {
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.pull.mockRejectedValueOnce(new GitError('auth-required', 'HTTP 401'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    await waitFor(() => expect(screen.getByLabelText('git.auth.formLabel')).toBeTruthy());
    // Default retry label uses the pull variant.
    expect(screen.getByText('git.pull.retry')).toBeTruthy();
    // getAuth was consulted so the mode can be preseeded from stored creds.
    expect(fx.getAuth).toHaveBeenCalledWith('github.com');
  });

  it('pull auth-failed: flips the stored-auth mode to the matching tab', async () => {
    setReady({
      remote: { name: 'origin', url: 'git@github.com:foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce({ kind: 'ssh', keyId: 'key-1' });
    fx.pull.mockRejectedValueOnce(new GitError('auth-failed', 'HTTP 403'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    await waitFor(() => expect(screen.getByLabelText('git.auth.formLabel')).toBeTruthy());
    // ssh tab should be pre-selected because the last stored credential
    // for this host was an SSH key.
    expect(
      screen.getByRole('tab', { name: 'git.auth.modeSsh' }).getAttribute('aria-selected'),
    ).toBe('true');
  });

  it('pull auth-required: preseeds SSH tab for SSH remote URLs even without stored auth', async () => {
    // The regression this guards against: a user opens a repo with an
    // SSH remote (git@...) and never visits the SSH Keys subview, so
    // the store has no cached auth. Previously `preseedAuthMode` would
    // default to 'token' here, dead-ending the user on the wrong tab.
    setReady({
      remote: { name: 'origin', url: 'git@github.com:foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce(null);
    fx.pull.mockRejectedValueOnce(new GitError('auth-required', 'HTTP 401'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    await waitFor(() => expect(screen.getByLabelText('git.auth.formLabel')).toBeTruthy());
    expect(
      screen.getByRole('tab', { name: 'git.auth.modeSsh' }).getAttribute('aria-selected'),
    ).toBe('true');
  });

  it('pull auth-required: preseeds token tab for HTTPS remote URLs without stored auth', async () => {
    // Counter-test to the SSH case above: an HTTPS remote with no
    // stored auth should still land on the token tab — we only switch
    // to SSH when the URL scheme calls for it.
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.getAuth.mockResolvedValueOnce(null);
    fx.pull.mockRejectedValueOnce(new GitError('auth-required', 'HTTP 401'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    await waitFor(() => expect(screen.getByLabelText('git.auth.formLabel')).toBeTruthy());
    expect(
      screen.getByRole('tab', { name: 'git.auth.modeToken' }).getAttribute('aria-selected'),
    ).toBe('true');
  });

  // ---- Pull conflict-non-op path ---------------------------------------

  it('pull throwing a normal error (not auth, not push-rejected) surfaces a compact inline error', async () => {
    setReady({
      ahead: 0,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.pull.mockRejectedValueOnce(new Error('network unreachable'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    await waitFor(() => expect(screen.getByRole('alert')).toBeTruthy());
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('network unreachable');
  });

  it('pull conflict-non-op: store transitions owned by the store — button just succeeds and renders no stale UI', async () => {
    // The store transitions ready → conflict on 'conflict-non-op'. The
    // button does NOT try to read that — its job is to call pull() and
    // not render stale UI when the action resolves cleanly. The real
    // store test covers the state transition; this one guards the button
    // against double-firing or crashing when the resolved promise doesn't
    // throw.
    setReady({
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.pull.mockResolvedValueOnce(undefined);

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));
    await waitFor(() => expect(fx.pull).toHaveBeenCalledTimes(1));
    expect(screen.queryByLabelText('git.auth.formLabel')).toBeNull();
    expect(screen.queryByRole('alert')).toBeNull();
  });

  // ---- Push-rejected path ----------------------------------------------

  it('push-rejected: surfaces the "pull first" recovery strip with a one-click retry action', async () => {
    setReady({
      ahead: 3,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.push.mockRejectedValueOnce(new GitError('push-rejected', 'non-fast-forward'));
    fx.pull.mockResolvedValueOnce(undefined);

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.push.label'));

    await waitFor(() => expect(screen.getByText('git.push.rejectedBody')).toBeTruthy());
    // One-click retry kicks off pull()
    fireEvent.click(screen.getByText('git.push.rejectedPull'));
    await waitFor(() => expect(fx.pull).toHaveBeenCalledTimes(1));
  });

  // ---- Push auth-failed -------------------------------------------------

  it('push auth-failed: opens the shared auth form with the push retry label', async () => {
    setReady({
      ahead: 2,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    fx.push.mockRejectedValueOnce(new GitError('auth-failed', 'HTTP 403'));

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.push.label'));

    await waitFor(() => expect(screen.getByLabelText('git.auth.formLabel')).toBeTruthy());
    expect(screen.getByText('git.push.retry')).toBeTruthy();
  });

  // ---- Loading state ----------------------------------------------------

  it('loading state: disables both buttons while a pull is in flight', async () => {
    setReady({
      ahead: 1,
      remote: { name: 'origin', url: 'https://github.com/foo/bar.git', host: 'github.com' },
    });
    // Hold the pull promise open so the button sits in the "pulling" step.
    let resolvePull!: () => void;
    fx.pull.mockImplementationOnce(
      () =>
        new Promise<void>((r) => {
          resolvePull = r;
        }),
    );

    renderWithProvider(<GitPanelRemoteControls />);
    fireEvent.click(screen.getByLabelText('git.pull.label'));

    // Both buttons should be disabled mid-flight.
    await waitFor(() => {
      const pull = screen.getByLabelText('git.pull.label').closest('button')!;
      const push = screen.getByLabelText('git.push.label').closest('button')!;
      expect(pull.disabled).toBe(true);
      expect(push.disabled).toBe(true);
    });

    resolvePull();
  });
});
