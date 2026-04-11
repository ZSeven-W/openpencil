// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-field-conflict-card.test.tsx

import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import type { GitConflictResolution } from '@/services/git-types';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    disabled,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    disabled?: boolean;
    [k: string]: unknown;
  }) => (
    <button onClick={onClick} disabled={disabled} data-testid={props['data-testid'] as string}>
      {children}
    </button>
  ),
}));

// Note: shadcn Badge is not in this project; the card uses an inline InlineBadge.
// No mock needed.

vi.mock('@/components/panels/git-panel/git-panel-conflict-json-editor', () => ({
  GitPanelConflictJsonEditor: ({
    onSubmit,
    onCancel,
  }: {
    onSubmit: (v: unknown) => void;
    onCancel: () => void;
  }) => (
    <div data-testid="json-editor">
      <button data-testid="json-editor-submit" onClick={() => onSubmit('custom-value')}>
        Submit
      </button>
      <button data-testid="json-editor-cancel" onClick={onCancel}>
        Cancel
      </button>
    </div>
  ),
}));

import { GitPanelFieldConflictCard } from '@/components/panels/git-panel/git-panel-field-conflict-card';

function makeFieldConflict(id: string, resolution?: GitConflictResolution) {
  return {
    id,
    field: 'name',
    path: 'document.name',
    base: 'old-name',
    ours: 'my-name',
    theirs: 'their-name',
    resolution,
  };
}

describe('GitPanelFieldConflictCard', () => {
  afterEach(() => cleanup());

  it('renders the card with field name', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByText('name')).toBeTruthy();
  });

  it('renders ours and theirs choose buttons', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByTestId('field-choose-ours-docField:name')).toBeTruthy();
    expect(screen.getByTestId('field-choose-theirs-docField:name')).toBeTruthy();
  });

  it('calls onResolve with {kind: ours}', () => {
    const onResolve = vi.fn();
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('field-choose-ours-docField:name'));
    expect(onResolve).toHaveBeenCalledWith({ kind: 'ours' });
  });

  it('calls onResolve with {kind: theirs}', () => {
    const onResolve = vi.fn();
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('field-choose-theirs-docField:name'));
    expect(onResolve).toHaveBeenCalledWith({ kind: 'theirs' });
  });

  it('shows edit manually button initially', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByTestId('field-edit-manual-docField:name')).toBeTruthy();
    expect(screen.queryByTestId('json-editor')).toBeNull();
  });

  it('shows JSON editor after clicking edit manually', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('field-edit-manual-docField:name'));
    expect(screen.getByTestId('json-editor')).toBeTruthy();
  });

  it('hides JSON editor when cancel clicked', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('field-edit-manual-docField:name'));
    fireEvent.click(screen.getByTestId('json-editor-cancel'));
    expect(screen.queryByTestId('json-editor')).toBeNull();
  });

  it('calls onResolve with manual-field choice when editor submits', () => {
    const onResolve = vi.fn();
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('field-edit-manual-docField:name'));
    fireEvent.click(screen.getByTestId('json-editor-submit'));
    expect(onResolve).toHaveBeenCalledWith({ kind: 'manual-field', value: 'custom-value' });
  });

  it('closes editor after manual-field submit', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('field-edit-manual-docField:name'));
    fireEvent.click(screen.getByTestId('json-editor-submit'));
    expect(screen.queryByTestId('json-editor')).toBeNull();
  });

  it('shows resolved badge when resolution is set', () => {
    const conflict = makeFieldConflict('docField:name', { kind: 'theirs' });
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByText('git.conflict.item.resolved')).toBeTruthy();
  });

  it('shows base, ours, and theirs values', () => {
    const conflict = makeFieldConflict('docField:name');
    render(<GitPanelFieldConflictCard conflict={conflict} onResolve={() => {}} />);
    // prettyJson renders strings with JSON quotes
    expect(screen.getByTestId('field-base-value-docField:name').textContent).toContain('old-name');
    expect(screen.getByTestId('field-ours-value-docField:name').textContent).toContain('my-name');
    expect(screen.getByTestId('field-theirs-value-docField:name').textContent).toContain(
      'their-name',
    );
  });
});
