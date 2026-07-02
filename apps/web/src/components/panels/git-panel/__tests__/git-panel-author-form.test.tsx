// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-author-form.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

const setAuthorIdentityMock = vi.fn(async () => {});
const hideAuthorPromptMock = vi.fn();

vi.mock('@/stores/git-store', () => ({
  useGitStore: (
    selector: (s: {
      setAuthorIdentity: typeof setAuthorIdentityMock;
      hideAuthorPrompt: typeof hideAuthorPromptMock;
    }) => unknown,
  ) =>
    selector({ setAuthorIdentity: setAuthorIdentityMock, hideAuthorPrompt: hideAuthorPromptMock }),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

import { GitPanelAuthorForm } from '@/components/panels/git-panel/git-panel-author-form';

describe('GitPanelAuthorForm', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('submits valid name + email and calls setAuthorIdentity then hideAuthorPrompt', async () => {
    render(<GitPanelAuthorForm />);

    const nameInput = screen.getByLabelText('git.author.nameLabel');
    const emailInput = screen.getByLabelText('git.author.emailLabel');
    fireEvent.change(nameInput, { target: { value: 'Alice' } });
    fireEvent.change(emailInput, { target: { value: 'alice@example.com' } });

    const submitButton = screen.getByText('git.author.submit');
    fireEvent.click(submitButton);

    // Allow async handler to settle.
    await Promise.resolve();
    await Promise.resolve();

    expect(setAuthorIdentityMock).toHaveBeenCalledTimes(1);
    expect(setAuthorIdentityMock).toHaveBeenCalledWith('Alice', 'alice@example.com');
    expect(hideAuthorPromptMock).toHaveBeenCalledTimes(1);
  });

  it('rejects empty name or invalid email and does not call setAuthorIdentity', async () => {
    render(<GitPanelAuthorForm />);

    // Submit with empty fields.
    fireEvent.click(screen.getByText('git.author.submit'));
    await Promise.resolve();
    expect(setAuthorIdentityMock).not.toHaveBeenCalled();
    expect(hideAuthorPromptMock).not.toHaveBeenCalled();

    // Validation errors visible.
    expect(screen.getByText('git.author.validationName')).toBeTruthy();
    expect(screen.getByText('git.author.validationEmail')).toBeTruthy();

    // Fill name only, leave email invalid.
    fireEvent.change(screen.getByLabelText('git.author.nameLabel'), { target: { value: 'Bob' } });
    fireEvent.change(screen.getByLabelText('git.author.emailLabel'), {
      target: { value: 'no-at-sign' },
    });
    fireEvent.click(screen.getByText('git.author.submit'));
    await Promise.resolve();
    expect(setAuthorIdentityMock).not.toHaveBeenCalled();
  });

  it('cancel calls hideAuthorPrompt without persisting', () => {
    render(<GitPanelAuthorForm />);
    fireEvent.click(screen.getByText('git.author.cancel'));
    expect(hideAuthorPromptMock).toHaveBeenCalledTimes(1);
    expect(setAuthorIdentityMock).not.toHaveBeenCalled();
  });
});
