// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-node-conflict-card.test.tsx

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

// Stub the JSON editor so we can control its behaviour in tests
vi.mock('@/components/panels/git-panel/git-panel-conflict-json-editor', () => ({
  GitPanelConflictJsonEditor: ({
    onSubmit,
    onCancel,
    nodeId,
  }: {
    onSubmit: (v: unknown) => void;
    onCancel: () => void;
    nodeId?: string;
  }) => (
    <div data-testid="json-editor">
      <button
        data-testid="json-editor-submit"
        onClick={() => onSubmit({ id: nodeId ?? 'n', type: 'rectangle' })}
      >
        Submit
      </button>
      <button data-testid="json-editor-cancel" onClick={onCancel}>
        Cancel
      </button>
    </div>
  ),
}));

// Stub pen-renderer to avoid WASM loading
vi.mock('@zseven-w/pen-renderer', () => ({
  renderNodeThumbnail: vi.fn(async () => null),
}));

import { GitPanelNodeConflictCard } from '@/components/panels/git-panel/git-panel-node-conflict-card';

function makeConflict(id: string, resolution?: GitConflictResolution) {
  return {
    id,
    pageId: null,
    nodeId: id.replace('node:_:', ''),
    reason: 'both-modified-same-field' as const,
    base: null,
    ours: { id: id.replace('node:_:', ''), type: 'rectangle' },
    theirs: { id: id.replace('node:_:', ''), type: 'ellipse' },
    resolution,
  };
}

describe('GitPanelNodeConflictCard', () => {
  afterEach(() => cleanup());

  it('renders the card with nodeId', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByText('rect-1')).toBeTruthy();
  });

  it('renders ours and theirs choose buttons', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByTestId('node-choose-ours-node:_:rect-1')).toBeTruthy();
    expect(screen.getByTestId('node-choose-theirs-node:_:rect-1')).toBeTruthy();
  });

  it('calls onResolve with {kind: ours} when clicking choose ours', () => {
    const onResolve = vi.fn();
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('node-choose-ours-node:_:rect-1'));
    expect(onResolve).toHaveBeenCalledWith({ kind: 'ours' });
  });

  it('calls onResolve with {kind: theirs} when clicking choose theirs', () => {
    const onResolve = vi.fn();
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('node-choose-theirs-node:_:rect-1'));
    expect(onResolve).toHaveBeenCalledWith({ kind: 'theirs' });
  });

  it('shows edit manually button initially (no editor)', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByTestId('node-edit-manual-node:_:rect-1')).toBeTruthy();
    expect(screen.queryByTestId('json-editor')).toBeNull();
  });

  it('shows JSON editor when edit manually is clicked', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('node-edit-manual-node:_:rect-1'));
    expect(screen.getByTestId('json-editor')).toBeTruthy();
    expect(screen.queryByTestId('node-edit-manual-node:_:rect-1')).toBeNull();
  });

  it('hides JSON editor when cancel is clicked', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('node-edit-manual-node:_:rect-1'));
    fireEvent.click(screen.getByTestId('json-editor-cancel'));
    expect(screen.queryByTestId('json-editor')).toBeNull();
    expect(screen.getByTestId('node-edit-manual-node:_:rect-1')).toBeTruthy();
  });

  it('calls onResolve with manual-node choice when editor submits', () => {
    const onResolve = vi.fn();
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={onResolve} />);
    fireEvent.click(screen.getByTestId('node-edit-manual-node:_:rect-1'));
    fireEvent.click(screen.getByTestId('json-editor-submit'));
    expect(onResolve).toHaveBeenCalledWith({
      kind: 'manual-node',
      node: { id: 'rect-1', type: 'rectangle' },
    });
  });

  it('closes editor after manual-node submit', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    fireEvent.click(screen.getByTestId('node-edit-manual-node:_:rect-1'));
    fireEvent.click(screen.getByTestId('json-editor-submit'));
    expect(screen.queryByTestId('json-editor')).toBeNull();
  });

  it('shows resolved badge when conflict has a resolution', () => {
    const conflict = makeConflict('node:_:rect-1', { kind: 'ours' });
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.getByText('git.conflict.item.resolved')).toBeTruthy();
  });

  it('does not show resolved badge when conflict has no resolution', () => {
    const conflict = makeConflict('node:_:rect-1');
    render(<GitPanelNodeConflictCard conflict={conflict} onResolve={() => {}} />);
    expect(screen.queryByText('git.conflict.item.resolved')).toBeNull();
  });
});
