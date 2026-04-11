// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-clone-form.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';

const cloneRepoMock = vi.fn(async () => {});
const cancelCloneWizardMock = vi.fn(() => {});

// The wizard error is mutable so individual tests can flip it before render.
let mockedWizardError: { code: string; message: string } | null = null;

vi.mock('@/stores/git-store', () => {
  // The selector receives a partial GitStore-like shape. We construct it to
  // match exactly what GitPanelCloneForm reads: cloneRepo, cancelCloneWizard,
  // and `state` (so the wizardError selector can read state.error).
  const useGitStore = (
    selector: (s: {
      cloneRepo: typeof cloneRepoMock;
      cancelCloneWizard: typeof cancelCloneWizardMock;
      state: { kind: 'wizard-clone'; error: typeof mockedWizardError };
    }) => unknown,
  ) =>
    selector({
      cloneRepo: cloneRepoMock,
      cancelCloneWizard: cancelCloneWizardMock,
      state: { kind: 'wizard-clone', error: mockedWizardError },
    });
  return { useGitStore };
});

vi.mock('react-i18next', () => ({
  // The error rendering uses t(`git.wizard.clone.error.${code}`, { defaultValue }).
  // We honour the defaultValue so the second-arg fallback is testable.
  useTranslation: () => ({
    t: (k: string, opts?: { defaultValue?: string }) => {
      if (opts?.defaultValue !== undefined) return opts.defaultValue;
      return k;
    },
  }),
}));

import { GitPanelCloneForm } from '@/components/panels/git-panel/git-panel-clone-form';

describe('GitPanelCloneForm', () => {
  beforeEach(() => {
    mockedWizardError = null;
    vi.clearAllMocks();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).electronAPI = {
      openDirectory: vi.fn(async () => '/picked/dest'),
    };
  });

  afterEach(() => {
    cleanup();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (window as any).electronAPI;
  });

  it('cancel button calls cancelCloneWizard without cloning', () => {
    render(<GitPanelCloneForm />);
    fireEvent.click(screen.getByText('git.wizard.clone.cancel'));
    expect(cancelCloneWizardMock).toHaveBeenCalledTimes(1);
    expect(cloneRepoMock).not.toHaveBeenCalled();
  });

  it('destination picker calls openDirectory and updates the input', async () => {
    render(<GitPanelCloneForm />);
    fireEvent.click(screen.getByText('git.wizard.clone.destPickButton'));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await waitFor(() => expect((window as any).electronAPI.openDirectory).toHaveBeenCalledTimes(1));
    // The setState happens after the async openDirectory resolves; waitFor
    // polls until the controlled input reflects the picked path.
    await waitFor(() => {
      const destInput = screen.getByLabelText('git.wizard.clone.destLabel') as HTMLInputElement;
      expect(destInput.value).toBe('/picked/dest');
    });
  });

  it('submit with HTTPS URL + token sends a token-shaped auth', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText('git.wizard.clone.urlLabel'), {
      target: { value: 'https://github.com/foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText('git.wizard.clone.destLabel'), {
      target: { value: '/tmp/clone' },
    });
    fireEvent.change(screen.getByLabelText('git.wizard.clone.usernameLabel'), {
      target: { value: 'alice' },
    });
    fireEvent.change(screen.getByLabelText('git.wizard.clone.tokenLabel'), {
      target: { value: 'ghp_abc123' },
    });

    fireEvent.click(screen.getByText('git.wizard.clone.submit'));
    // Allow the async handler chain to settle.
    await Promise.resolve();
    await Promise.resolve();

    expect(cloneRepoMock).toHaveBeenCalledTimes(1);
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
      auth: { kind: 'token', username: 'alice', token: 'ghp_abc123' },
    });
  });

  it('submit with HTTPS URL and blank token clones anonymously (no auth)', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText('git.wizard.clone.urlLabel'), {
      target: { value: 'https://github.com/foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText('git.wizard.clone.destLabel'), {
      target: { value: '/tmp/clone' },
    });

    fireEvent.click(screen.getByText('git.wizard.clone.submit'));
    await Promise.resolve();
    await Promise.resolve();

    expect(cloneRepoMock).toHaveBeenCalledTimes(1);
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
      auth: undefined,
    });
  });

  it('SSH URL hides token fields and sends no auth (Phase 6c picks the key)', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText('git.wizard.clone.urlLabel'), {
      target: { value: 'git@github.com:foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText('git.wizard.clone.destLabel'), {
      target: { value: '/tmp/clone' },
    });

    // Token fields are gone in SSH mode.
    expect(screen.queryByLabelText('git.wizard.clone.tokenLabel')).toBeNull();
    expect(screen.queryByLabelText('git.wizard.clone.usernameLabel')).toBeNull();
    // SSH hint is shown.
    expect(screen.getByText('git.wizard.clone.sshHint')).toBeTruthy();

    fireEvent.click(screen.getByText('git.wizard.clone.submit'));
    await Promise.resolve();
    await Promise.resolve();

    expect(cloneRepoMock).toHaveBeenCalledTimes(1);
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'git@github.com:foo/bar.git',
      dest: '/tmp/clone',
      auth: undefined,
    });
  });

  it('renders an inline recoverable error when state.error is set', () => {
    mockedWizardError = { code: 'auth-failed', message: 'Bad PAT' };
    render(<GitPanelCloneForm />);
    // The error block uses role=alert with the resolved t() value (we mock
    // t to return defaultValue when present, so we see the GitError message).
    const alert = screen.getByRole('alert');
    expect(alert.textContent).toContain('Bad PAT');
  });

  it('blocks submit and surfaces validation errors when fields are blank', async () => {
    render(<GitPanelCloneForm />);
    fireEvent.click(screen.getByText('git.wizard.clone.submit'));
    await Promise.resolve();
    expect(cloneRepoMock).not.toHaveBeenCalled();
    expect(screen.getByText('git.wizard.clone.validationUrl')).toBeTruthy();
    expect(screen.getByText('git.wizard.clone.validationDest')).toBeTruthy();
  });
});
