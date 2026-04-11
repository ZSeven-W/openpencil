// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict-json-editor.test.tsx

import { describe, it, expect, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string) => k,
  }),
}));

// Stub Button to render a plain button
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
    <button
      onClick={onClick}
      disabled={disabled}
      data-testid={props['data-testid'] as string | undefined}
    >
      {children}
    </button>
  ),
}));

import { GitPanelConflictJsonEditor } from '@/components/panels/git-panel/git-panel-conflict-json-editor';

describe('GitPanelConflictJsonEditor — field mode', () => {
  afterEach(() => {
    cleanup();
  });

  it('renders the textarea with initial value', () => {
    const onSubmit = vi.fn();
    const onCancel = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue='"hello"'
        mode="field"
        onSubmit={onSubmit}
        onCancel={onCancel}
      />,
    );
    const textarea = screen.getByTestId('conflict-json-textarea') as HTMLTextAreaElement;
    expect(textarea.value).toBe('"hello"');
  });

  it('enables apply button for valid JSON', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue='"hello"'
        mode="field"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    const apply = screen.getByTestId('conflict-json-apply') as HTMLButtonElement;
    expect(apply.disabled).toBe(false);
  });

  it('disables apply button for invalid JSON', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue="{ broken"
        mode="field"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    const apply = screen.getByTestId('conflict-json-apply') as HTMLButtonElement;
    expect(apply.disabled).toBe(true);
  });

  it('shows error message for invalid JSON', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue="not-json"
        mode="field"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByTestId('conflict-json-error')).toBeTruthy();
  });

  it('calls onSubmit with parsed value when apply clicked', () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue="42"
        mode="field"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );
    fireEvent.click(screen.getByTestId('conflict-json-apply'));
    expect(onSubmit).toHaveBeenCalledWith(42);
  });

  it('calls onCancel when cancel clicked', () => {
    const onCancel = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue='"hello"'
        mode="field"
        onSubmit={vi.fn()}
        onCancel={onCancel}
      />,
    );
    fireEvent.click(screen.getByTestId('conflict-json-cancel'));
    expect(onCancel).toHaveBeenCalledTimes(1);
  });

  it('updates parse result as user types valid JSON', () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue="{ broken"
        mode="field"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );
    // Initially invalid
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(true);

    // User types valid JSON
    fireEvent.change(screen.getByTestId('conflict-json-textarea'), {
      target: { value: '"fixed"' },
    });
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(false);

    // Submit
    fireEvent.click(screen.getByTestId('conflict-json-apply'));
    expect(onSubmit).toHaveBeenCalledWith('fixed');
  });

  it('accepts array JSON values in field mode', () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue="[1,2,3]"
        mode="field"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByTestId('conflict-json-apply'));
    expect(onSubmit).toHaveBeenCalledWith([1, 2, 3]);
  });
});

describe('GitPanelConflictJsonEditor — node mode', () => {
  afterEach(() => {
    cleanup();
  });

  it('enables apply when JSON is a valid object with correct nodeId', () => {
    const onSubmit = vi.fn();
    render(
      <GitPanelConflictJsonEditor
        initialValue='{"id":"node-A","type":"rectangle"}'
        mode="node"
        nodeId="node-A"
        onSubmit={onSubmit}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(false);
    fireEvent.click(screen.getByTestId('conflict-json-apply'));
    expect(onSubmit).toHaveBeenCalledWith({ id: 'node-A', type: 'rectangle' });
  });

  it('disables apply when nodeId does not match', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue='{"id":"different-node","type":"rectangle"}'
        mode="node"
        nodeId="node-A"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(true);
    expect(screen.getByTestId('conflict-json-error')).toBeTruthy();
  });

  it('disables apply when node has no type field', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue='{"id":"node-A"}'
        mode="node"
        nodeId="node-A"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(true);
  });

  it('disables apply for a JSON array in node mode', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue="[]"
        mode="node"
        nodeId="node-A"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect((screen.getByTestId('conflict-json-apply') as HTMLButtonElement).disabled).toBe(true);
  });

  it('shows validation error for wrong nodeId', () => {
    render(
      <GitPanelConflictJsonEditor
        initialValue='{"id":"wrong","type":"rectangle"}'
        mode="node"
        nodeId="node-A"
        onSubmit={vi.fn()}
        onCancel={vi.fn()}
      />,
    );
    expect(screen.getByTestId('conflict-json-error')).toBeTruthy();
  });
});
