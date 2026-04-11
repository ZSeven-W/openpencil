// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-commit-input.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

const mocks = vi.hoisted(() => ({
  commitMessage: '',
  authorIdentity: null as { name: string; email: string } | null,
  authorPromptVisible: false,
  setCommitMessage: vi.fn(),
  showAuthorPrompt: vi.fn(),
  commitMilestone: vi.fn(async () => {}),
  // Fields needed by GitPanelAuthorForm
  setAuthorIdentity: vi.fn(async () => {}),
  hideAuthorPrompt: vi.fn(),
}));

vi.mock('@/stores/git-store', () => {
  const state = mocks;
  return {
    useGitStore: (
      selector: (s: {
        commitMessage: string;
        authorIdentity: { name: string; email: string } | null;
        authorPromptVisible: boolean;
        setCommitMessage: typeof mocks.setCommitMessage;
        showAuthorPrompt: typeof mocks.showAuthorPrompt;
        commitMilestone: typeof mocks.commitMilestone;
        setAuthorIdentity: typeof mocks.setAuthorIdentity;
        hideAuthorPrompt: typeof mocks.hideAuthorPrompt;
      }) => unknown,
    ) =>
      selector({
        commitMessage: state.commitMessage,
        authorIdentity: state.authorIdentity,
        authorPromptVisible: state.authorPromptVisible,
        setCommitMessage: state.setCommitMessage,
        showAuthorPrompt: state.showAuthorPrompt,
        commitMilestone: state.commitMilestone,
        setAuthorIdentity: state.setAuthorIdentity,
        hideAuthorPrompt: state.hideAuthorPrompt,
      }),
  };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

import { GitPanelCommitInput } from '@/components/panels/git-panel/git-panel-commit-input';

describe('GitPanelCommitInput', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.commitMessage = '';
    mocks.authorIdentity = { name: 'Alice', email: 'alice@example.com' };
    mocks.authorPromptVisible = false;
  });

  afterEach(() => {
    cleanup();
  });

  it('submit button is disabled when commitMessage is empty', () => {
    mocks.commitMessage = '';
    render(<GitPanelCommitInput />);
    const button = screen.getByText('git.commit.submitButton').closest('button');
    expect(button?.hasAttribute('disabled')).toBe(true);
  });

  it('submit calls commitMilestone with trimmed message and current author', async () => {
    mocks.commitMessage = '  first milestone  ';
    render(<GitPanelCommitInput />);

    const button = screen.getByText('git.commit.submitButton').closest('button');
    expect(button?.hasAttribute('disabled')).toBe(false);
    fireEvent.click(button!);
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.commitMilestone).toHaveBeenCalledTimes(1);
    expect(mocks.commitMilestone).toHaveBeenCalledWith('first milestone', {
      name: 'Alice',
      email: 'alice@example.com',
    });
  });

  it('submit with null authorIdentity triggers showAuthorPrompt and does NOT call commitMilestone', async () => {
    mocks.commitMessage = 'first milestone';
    mocks.authorIdentity = null;
    render(<GitPanelCommitInput />);

    const button = screen.getByText('git.commit.submitButton').closest('button');
    fireEvent.click(button!);
    await Promise.resolve();

    expect(mocks.showAuthorPrompt).toHaveBeenCalledTimes(1);
    expect(mocks.commitMilestone).not.toHaveBeenCalled();
  });

  it('re-fires commitMilestone after authorIdentity is set and form closes', async () => {
    // Start with no identity — user clicks submit, author form is shown,
    // but commitMilestone is NOT called yet.
    mocks.commitMessage = 'pending milestone';
    mocks.authorIdentity = null;
    mocks.authorPromptVisible = false;
    const { rerender } = render(<GitPanelCommitInput />);

    fireEvent.click(screen.getByText('git.commit.submitButton').closest('button')!);
    await Promise.resolve();

    expect(mocks.showAuthorPrompt).toHaveBeenCalledTimes(1);
    expect(mocks.commitMilestone).not.toHaveBeenCalled();

    // Simulate the author form succeeding: identity is now set and the
    // prompt is hidden. The useEffect should re-fire the commit using
    // the captured commitMessage from when the submit was first clicked.
    mocks.authorIdentity = { name: 'Bob', email: 'bob@example.com' };
    mocks.authorPromptVisible = false;
    rerender(<GitPanelCommitInput />);
    await Promise.resolve();
    await Promise.resolve();

    expect(mocks.commitMilestone).toHaveBeenCalledTimes(1);
    expect(mocks.commitMilestone).toHaveBeenCalledWith('pending milestone', {
      name: 'Bob',
      email: 'bob@example.com',
    });
  });

  it('renders the author form when authorPromptVisible is true instead of the textarea', () => {
    mocks.authorPromptVisible = true;
    mocks.commitMessage = 'pending message';
    render(<GitPanelCommitInput />);

    // The textarea should NOT be in the DOM.
    expect(screen.queryByPlaceholderText('git.commit.placeholder')).toBeNull();
    // The author form renders its heading key.
    expect(screen.getByText('git.author.heading')).toBeTruthy();
  });

  it('typing in the textarea calls setCommitMessage', () => {
    mocks.commitMessage = '';
    render(<GitPanelCommitInput />);

    const textarea = screen.getByPlaceholderText('git.commit.placeholder');
    fireEvent.change(textarea, { target: { value: 'new text' } });

    expect(mocks.setCommitMessage).toHaveBeenCalledWith('new text');
  });
});
