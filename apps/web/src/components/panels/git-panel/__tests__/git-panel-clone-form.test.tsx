// @vitest-环境 jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-clone-form.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup, waitFor } from '@testing-library/react';
import en from '@/i18n/locales/en';
import { CLONE_INLINE_ERROR_CODES } from '@/stores/git-store-types';

const cloneRepoMock = vi.fn(async () => {});
const cancelCloneWizardMock = vi.fn(() => {});

// The 向导 error/busy 是可变的，因此各个测试可以在渲染之前翻转它们。下面的 The
// 存储模拟读取每个选择器调用的实时值，因此测试可以在 `cloneRepo` 副作用中改变这些值（例如，模拟存储在克隆失败后转换回
// `wizard-clone` 并出现错误）。
let mockedWizardError: { code: string; message: string } | null = null;
let mockedBusy = false;

vi.mock('@/stores/git-store', () => {
  // The 选择器接收部分类似 GitStore 的形状。 We 将其构造为与 GitPanelCloneForm
  // 读取的内容完全匹配：cloneRepo、cancelCloneWizard 和 `state`（因此 wizardError + busy
  // 选择器可以读取 state.error / state.busy）。
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
  // We 尊重不关心特定本地化字符串的测试的 defaultValue 回退，并尊重捆绑 `en` 区域设置中的真实键查找，以进行 DO
  // 关心的测试（下面的参数化区域设置测试）。
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

// Shortcut 到我们重复断言的本地化字符串。
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
    // The setState 在异步 openDirectory 解析后发生； waitFor 进行轮询，直到受控输入反映所选路径。
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

    // Token 字段在 SSH 模式下消失。
    expect(screen.queryByLabelText(L['git.wizard.clone.tokenLabel'])).toBeNull();
    expect(screen.queryByLabelText(L['git.wizard.clone.usernameLabel'])).toBeNull();
    // 显示 SSH 提示。
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

  // ----- Issue 3：每个内联错误代码的参数化区域覆盖范围 -
//
  // CLONE_INLINE_ERROR_CODES 枚举商店中所有可恢复的代码
  // 表面内联。 The i18n 层下的每个代码都有一个本地化字符串
  // `git.wizard.clone.error.<code>` 在所有 15 个区域设置中。 If 任何区域设置键是
  // 输入错误（例如 `auth_token_invalid` 而不是 `auth-token-invalid`）
  // t() defaultValue 后备隐藏了失误。 This 测试证明渲染效果
  // 每个代码的横幅文本与捆绑的 `en` 区域设置字符串 NOT 匹配
  // 原始 GitError 消息 — 因此任何语言环境键拼写错误都会显示为
  // 大声的测试失败。
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
