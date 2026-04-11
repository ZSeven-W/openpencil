// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-clone-form.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import en from '@/i18n/locales/en';
import { CLONE_INLINE_ERROR_CODES } from '@/stores/git-store-types';

const cloneRepoMock = vi.fn(async () => {});
const cancelCloneWizardMock = vi.fn(() => {});

// The wizard error/busy are mutable so individual tests can flip them
// before render. The store mock below reads the live values on every
// selector call so a test can mutate these in a `cloneRepo` side effect
// (e.g. to simulate the store transitioning back to `wizard-clone` with
// an error after a failed clone).
let mockedWizardError: { code: string; message: string } | null = null;
let mockedBusy = false;

vi.mock('@/stores/git-store', () => {
  // The selector receives a partial GitStore-like shape. We construct it to
  // match exactly what GitPanelCloneForm reads: cloneRepo, cancelCloneWizard,
  // and `state` (so the wizardError + busy selectors can read state.error /
  // state.busy).
  const useGitStore = (
    selector: (s: {
      cloneRepo: typeof cloneRepoMock;
      cancelCloneWizard: typeof cancelCloneWizardMock;
      state: { kind: 'wizard-clone'; busy: boolean; error: typeof mockedWizardError };
    }) => unknown,
  ) =>
    selector({
      cloneRepo: cloneRepoMock,
      cancelCloneWizard: cancelCloneWizardMock,
      state: { kind: 'wizard-clone', busy: mockedBusy, error: mockedWizardError },
    });
  return { useGitStore };
});

vi.mock('react-i18next', () => ({
  // We honour the defaultValue fallback for tests that don't care about the
  // specific localized string, and honour real key lookups into the bundled
  // `en` locale for tests that DO care (the parametrized locale test below).
  useTranslation: () => ({
    t: (k: string, opts?: { defaultValue?: string }) => {
      const localized = (en as Record<string, string | undefined>)[k];
      if (localized !== undefined) return localized;
      if (opts?.defaultValue !== undefined) return opts.defaultValue;
      return k;
    },
  }),
}));

import { GitPanelCloneForm } from '@/components/panels/git-panel/git-panel-clone-form';

// Shortcut to the localized strings we assert on repeatedly.
const L = en as Record<string, string>;

describe('GitPanelCloneForm', () => {
  beforeEach(() => {
    mockedWizardError = null;
    mockedBusy = false;
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
    fireEvent.click(screen.getByText(L['git.wizard.clone.cancel']));
    expect(cancelCloneWizardMock).toHaveBeenCalledTimes(1);
    expect(cloneRepoMock).not.toHaveBeenCalled();
  });

  it('destination picker calls openDirectory and updates the input', async () => {
    render(<GitPanelCloneForm />);
    fireEvent.click(screen.getByText(L['git.wizard.clone.destPickButton']));
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    await waitFor(() => expect((window as any).electronAPI.openDirectory).toHaveBeenCalledTimes(1));
    // The setState happens after the async openDirectory resolves; waitFor
    // polls until the controlled input reflects the picked path.
    await waitFor(() => {
      const destInput = screen.getByLabelText(L['git.wizard.clone.destLabel']) as HTMLInputElement;
      expect(destInput.value).toBe('/picked/dest');
    });
  });

  it('submit with HTTPS URL + token sends a token-shaped auth', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.urlLabel']), {
      target: { value: 'https://github.com/foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.destLabel']), {
      target: { value: '/tmp/clone' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.usernameLabel']), {
      target: { value: 'alice' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.tokenLabel']), {
      target: { value: 'ghp_abc123' },
    });

    fireEvent.click(screen.getByText(L['git.wizard.clone.submit']));

    await waitFor(() => expect(cloneRepoMock).toHaveBeenCalledTimes(1));
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
      auth: { kind: 'token', username: 'alice', token: 'ghp_abc123' },
    });
  });

  it('submit with HTTPS URL and blank token clones anonymously (no auth)', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.urlLabel']), {
      target: { value: 'https://github.com/foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.destLabel']), {
      target: { value: '/tmp/clone' },
    });

    fireEvent.click(screen.getByText(L['git.wizard.clone.submit']));

    await waitFor(() => expect(cloneRepoMock).toHaveBeenCalledTimes(1));
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'https://github.com/foo/bar.git',
      dest: '/tmp/clone',
      auth: undefined,
    });
  });

  it('SSH URL hides token fields and sends no auth (Phase 6c picks the key)', async () => {
    render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.urlLabel']), {
      target: { value: 'git@github.com:foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.destLabel']), {
      target: { value: '/tmp/clone' },
    });

    // Token fields are gone in SSH mode.
    expect(screen.queryByLabelText(L['git.wizard.clone.tokenLabel'])).toBeNull();
    expect(screen.queryByLabelText(L['git.wizard.clone.usernameLabel'])).toBeNull();
    // SSH hint is shown.
    expect(screen.getByText(L['git.wizard.clone.sshHint'])).toBeTruthy();

    fireEvent.click(screen.getByText(L['git.wizard.clone.submit']));

    await waitFor(() => expect(cloneRepoMock).toHaveBeenCalledTimes(1));
    expect(cloneRepoMock).toHaveBeenCalledWith({
      url: 'git@github.com:foo/bar.git',
      dest: '/tmp/clone',
      auth: undefined,
    });
  });

  it('blocks submit and surfaces validation errors when fields are blank', async () => {
    render(<GitPanelCloneForm />);
    fireEvent.click(screen.getByText(L['git.wizard.clone.submit']));
    await waitFor(() => {
      expect(screen.getByText(L['git.wizard.clone.validationUrl'])).toBeTruthy();
      expect(screen.getByText(L['git.wizard.clone.validationDest'])).toBeTruthy();
    });
    expect(cloneRepoMock).not.toHaveBeenCalled();
  });

  // ----- Issue 3: parametrized locale coverage for every inline error code -
  //
  // CLONE_INLINE_ERROR_CODES enumerates all recoverable codes the store
  // surfaces inline. The i18n layer has a localized string per code under
  // `git.wizard.clone.error.<code>` in all 15 locales. If any locale key is
  // mistyped (e.g. `auth_token_invalid` instead of `auth-token-invalid`) the
  // t() defaultValue fallback hides the miss. This test proves the rendered
  // banner text for every code matches the bundled `en` locale string, NOT
  // the raw GitError message — so any locale-key typo shows up as a
  // loud test failure.
  describe('inline error banner for every recoverable code', () => {
    it.each(CLONE_INLINE_ERROR_CODES)('renders the localized banner for "%s"', (code) => {
      mockedWizardError = { code, message: `raw-error-for-${code}` };
      const key = `git.wizard.clone.error.${code}`;
      const expected = L[key];
      // Sanity: the locale key itself must exist. If this throws the fix
      // is to add the missing key to en.ts (and the other 14 locales).
      expect(expected, `missing locale key ${key}`).toBeTruthy();

      render(<GitPanelCloneForm />);
      const alert = screen.getByRole('alert');
      expect(alert.textContent).toContain(expected);
      // And crucially NOT the raw GitError message — that would mean the
      // t() defaultValue fallback fired, which indicates a missing or
      // mistyped locale key.
      expect(alert.textContent).not.toContain(`raw-error-for-${code}`);
    });
  });

  // ----- Issue 1 regression: form state survives a recoverable clone failure
  //
  // Before the fix, cloneRepo transitioned through `initializing`, which
  // unmounted GitPanelCloneForm and wiped the URL/dest/token inputs. On a
  // recoverable failure the user would see the inline banner over an empty
  // form and have to re-type everything. The fix keeps the wizard mounted
  // across the entire clone attempt by flipping `state.busy` in place.
  //
  // We simulate the store's behaviour by mutating `mockedBusy` and
  // `mockedWizardError` inside the cloneRepo mock. The component re-reads
  // both via selectors on every store update, matching the real store.
  it('preserves URL/dest/token inputs across a recoverable clone failure', async () => {
    cloneRepoMock.mockImplementationOnce(async () => {
      // Real store would transition wizard-clone → wizard-clone with busy=true
      // → wizard-clone with busy=false + error. We only need the final state:
      // a recoverable inline error without unmounting the form.
      mockedWizardError = { code: 'auth-failed', message: 'Bad PAT' };
      mockedBusy = false;
    });

    const { rerender } = render(<GitPanelCloneForm />);

    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.urlLabel']), {
      target: { value: 'https://github.com/foo/bar.git' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.destLabel']), {
      target: { value: '/tmp/clone' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.usernameLabel']), {
      target: { value: 'alice' },
    });
    fireEvent.change(screen.getByLabelText(L['git.wizard.clone.tokenLabel']), {
      target: { value: 'ghp_abc123' },
    });

    fireEvent.click(screen.getByText(L['git.wizard.clone.submit']));
    await waitFor(() => expect(cloneRepoMock).toHaveBeenCalledTimes(1));

    // The mock already flipped mockedWizardError. Force a re-render so the
    // component picks up the new store state (our mock doesn't emit store
    // updates automatically).
    rerender(<GitPanelCloneForm />);

    // The inline banner for auth-failed must now be visible.
    await waitFor(() => {
      const alert = screen.getByRole('alert');
      expect(alert.textContent).toContain(L['git.wizard.clone.error.auth-failed']);
    });

    // CRITICAL regression check: the form inputs still hold the values the
    // user typed. Before the fix, the component would have unmounted during
    // the `initializing` phase and these would all be empty strings.
    expect((screen.getByLabelText(L['git.wizard.clone.urlLabel']) as HTMLInputElement).value).toBe(
      'https://github.com/foo/bar.git',
    );
    expect((screen.getByLabelText(L['git.wizard.clone.destLabel']) as HTMLInputElement).value).toBe(
      '/tmp/clone',
    );
    expect(
      (screen.getByLabelText(L['git.wizard.clone.usernameLabel']) as HTMLInputElement).value,
    ).toBe('alice');
    expect(
      (screen.getByLabelText(L['git.wizard.clone.tokenLabel']) as HTMLInputElement).value,
    ).toBe('ghp_abc123');
  });

  it('disables the submit button while busy is true', () => {
    mockedBusy = true;
    render(<GitPanelCloneForm />);
    const submit = screen.getByText(L['git.wizard.clone.submit']).closest('button')!;
    expect(submit.disabled).toBe(true);
  });
});
