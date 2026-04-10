// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict-banner.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

const mocks = vi.hoisted(() => ({
  abortMerge: vi.fn(async () => {}),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string) => k,
  }),
}));

import { GitPanelConflictBanner } from '@/components/panels/git-panel/git-panel-conflict-banner';

describe('GitPanelConflictBanner', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders title, description, and abort button', () => {
    render(<GitPanelConflictBanner />);
    expect(screen.getByText('git.conflict.title')).toBeTruthy();
    expect(screen.getByText('git.conflict.description')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'git.conflict.abort' })).toBeTruthy();
  });

  it('clicking the abort button calls abortMerge', () => {
    render(<GitPanelConflictBanner />);
    fireEvent.click(screen.getByRole('button', { name: 'git.conflict.abort' }));
    expect(mocks.abortMerge).toHaveBeenCalledTimes(1);
  });

  it('renders with role="alert" so screen readers announce the conflict', () => {
    const { container } = render(<GitPanelConflictBanner />);
    expect(container.querySelector('[role="alert"]')).toBeTruthy();
  });
});
