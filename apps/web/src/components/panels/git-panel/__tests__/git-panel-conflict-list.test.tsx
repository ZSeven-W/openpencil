// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict-list.test.tsx
//
// Tests: interleaving, bulk ours/theirs, all-resolved state, null render
// when totalCount === 0.

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import type { GitConflictResolution } from '@/services/git-types';

// ---------------------------------------------------------------------------
// Shared mock state and actions
// ---------------------------------------------------------------------------

type NodeConflict = {
  id: string;
  pageId: null;
  nodeId: string;
  reason: 'both-modified-same-field';
  base: null;
  ours: null;
  theirs: null;
  resolution?: GitConflictResolution;
};

type FieldConflict = {
  id: string;
  field: string;
  path: string;
  base: null;
  ours: null;
  theirs: null;
  resolution?: GitConflictResolution;
};

const mocks = vi.hoisted(() => ({
  state: {
    kind: 'conflict' as const,
    repo: {
      repoId: 'r1',
      currentBranch: 'main',
      mode: 'single-file' as const,
      rootPath: '/tmp',
      gitdir: '/tmp/.git',
      engineKind: 'iso' as const,
      trackedFilePath: '/tmp/a.op',
      candidateFiles: [] as never[],
      branches: [] as never[],
      workingDirty: false,
      otherFilesDirty: 0,
      otherFilesPaths: [] as never[],
      ahead: 0,
      behind: 0,
      remote: null as null,
    },
    conflicts: {
      nodeConflicts: new Map<string, NodeConflict>(),
      docFieldConflicts: new Map<string, FieldConflict>(),
    },
    unresolvedFiles: [] as string[],
    finalizeError: null as string | null,
  } as {
    kind: 'conflict';
    repo: {
      repoId: string;
      currentBranch: string;
      mode: 'single-file';
      rootPath: string;
      gitdir: string;
      engineKind: 'iso';
      trackedFilePath: string;
      candidateFiles: never[];
      branches: never[];
      workingDirty: boolean;
      otherFilesDirty: number;
      otherFilesPaths: never[];
      ahead: number;
      behind: number;
      remote: null;
    };
    conflicts: {
      nodeConflicts: Map<string, NodeConflict>;
      docFieldConflicts: Map<string, FieldConflict>;
    };
    unresolvedFiles: string[];
    finalizeError: string | null;
  },
  resolveConflict: vi.fn(async (_id: string, _choice: GitConflictResolution) => {}),
}));

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

vi.mock('react-i18next', () => ({
  useTranslation: () => ({
    t: (k: string, opts?: Record<string, unknown>) => (opts ? `${k}:${JSON.stringify(opts)}` : k),
  }),
}));

// Stub shadcn primitives
vi.mock('@/components/ui/button', () => ({
  Button: ({
    children,
    onClick,
    ...props
  }: {
    children: React.ReactNode;
    onClick?: () => void;
    [k: string]: unknown;
  }) => (
    <button onClick={onClick} data-testid={props['data-testid'] as string}>
      {children}
    </button>
  ),
}));

vi.mock('@/components/ui/separator', () => ({
  Separator: () => <hr />,
}));

// Stub conflict item so we can observe what ids are rendered
vi.mock('@/components/panels/git-panel/git-panel-conflict-item', () => ({
  GitPanelConflictItem: ({ item }: { item: { id: string } }) => (
    <div data-testid={`conflict-item-${item.id}`}>{item.id}</div>
  ),
}));

import { GitPanelConflictList } from '@/components/panels/git-panel/git-panel-conflict-list';

function makeNodeConflict(id: string, resolved = false): NodeConflict {
  return {
    id,
    pageId: null,
    nodeId: id.replace('node:_:', ''),
    reason: 'both-modified-same-field',
    base: null,
    ours: null,
    theirs: null,
    ...(resolved ? { resolution: { kind: 'ours' as const } } : {}),
  };
}

function makeFieldConflict(id: string, resolved = false): FieldConflict {
  return {
    id,
    field: id.replace('docField:', ''),
    path: id,
    base: null,
    ours: null,
    theirs: null,
    ...(resolved ? { resolution: { kind: 'theirs' as const } } : {}),
  };
}

describe('GitPanelConflictList', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mocks.state.conflicts.nodeConflicts = new Map();
    mocks.state.conflicts.docFieldConflicts = new Map();
  });

  afterEach(() => {
    cleanup();
  });

  it('renders nothing when there are no conflicts', () => {
    const { container } = render(<GitPanelConflictList />);
    expect(container.firstChild).toBeNull();
  });

  it('renders nothing when state kind is not conflict', () => {
    // Temporarily override state kind
    const originalState = mocks.state;
    // @ts-expect-error — test override
    mocks.state = { kind: 'ready' };
    const { container } = render(<GitPanelConflictList />);
    expect(container.firstChild).toBeNull();
    mocks.state = originalState;
  });

  it('renders node conflict items', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A'));
    render(<GitPanelConflictList />);
    expect(screen.getByTestId('conflict-item-node:_:A')).toBeTruthy();
  });

  it('renders field conflict items', () => {
    mocks.state.conflicts.docFieldConflicts.set(
      'docField:name',
      makeFieldConflict('docField:name'),
    );
    render(<GitPanelConflictList />);
    expect(screen.getByTestId('conflict-item-docField:name')).toBeTruthy();
  });

  it('interleaves node and field conflicts in order (nodes first)', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A'));
    mocks.state.conflicts.nodeConflicts.set('node:_:B', makeNodeConflict('node:_:B'));
    mocks.state.conflicts.docFieldConflicts.set(
      'docField:name',
      makeFieldConflict('docField:name'),
    );
    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-/);
    expect(items).toHaveLength(3);
    // Nodes come first, then fields
    expect(items[0].getAttribute('data-testid')).toMatch(/node/);
    expect(items[2].getAttribute('data-testid')).toMatch(/docField/);
  });

  it('shows progress summary when not all resolved', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', true));
    mocks.state.conflicts.nodeConflicts.set('node:_:B', makeNodeConflict('node:_:B', false));
    render(<GitPanelConflictList />);
    const progress = screen.getByTestId('conflict-list-progress');
    expect(progress).toBeTruthy();
    // 1 of 2 resolved
    expect(progress.textContent).toContain('1');
  });

  it('shows all-resolved indicator when all are resolved', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', true));
    render(<GitPanelConflictList />);
    expect(screen.getByTestId('conflict-list-all-resolved')).toBeTruthy();
  });

  it('shows bulk action buttons when there are unresolved items', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', false));
    render(<GitPanelConflictList />);
    expect(screen.getByTestId('bulk-actions')).toBeTruthy();
    expect(screen.getByTestId('bulk-ours')).toBeTruthy();
    expect(screen.getByTestId('bulk-theirs')).toBeTruthy();
  });

  it('hides bulk action buttons when all items are resolved', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', true));
    render(<GitPanelConflictList />);
    expect(screen.queryByTestId('bulk-actions')).toBeNull();
  });

  it('bulk ours button calls resolveConflict for each unresolved item', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', false));
    mocks.state.conflicts.nodeConflicts.set('node:_:B', makeNodeConflict('node:_:B', false));
    mocks.state.conflicts.docFieldConflicts.set(
      'docField:name',
      makeFieldConflict('docField:name', false),
    );
    render(<GitPanelConflictList />);
    fireEvent.click(screen.getByTestId('bulk-ours'));
    // Called once per unresolved item (3 total)
    expect(mocks.resolveConflict).toHaveBeenCalledTimes(3);
    expect(mocks.resolveConflict).toHaveBeenCalledWith('node:_:A', { kind: 'ours' });
    expect(mocks.resolveConflict).toHaveBeenCalledWith('node:_:B', { kind: 'ours' });
    expect(mocks.resolveConflict).toHaveBeenCalledWith('docField:name', { kind: 'ours' });
  });

  it('bulk theirs button calls resolveConflict(theirs) for each unresolved item', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', false));
    render(<GitPanelConflictList />);
    fireEvent.click(screen.getByTestId('bulk-theirs'));
    expect(mocks.resolveConflict).toHaveBeenCalledWith('node:_:A', { kind: 'theirs' });
  });

  it('bulk ours skips already-resolved items', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A', true)); // already resolved
    mocks.state.conflicts.nodeConflicts.set('node:_:B', makeNodeConflict('node:_:B', false));
    render(<GitPanelConflictList />);
    fireEvent.click(screen.getByTestId('bulk-ours'));
    // Only node:_:B (unresolved)
    expect(mocks.resolveConflict).toHaveBeenCalledTimes(1);
    expect(mocks.resolveConflict).toHaveBeenCalledWith('node:_:B', { kind: 'ours' });
  });
});
