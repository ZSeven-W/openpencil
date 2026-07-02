// @vitest-environment jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-conflict-list.test.tsx
//
// Tests: document-order interleaving, orphan handling, field-ordering, bulk
// ours/theirs, all-resolved state, null render when totalCount === 0.

import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, cleanup, fireEvent } from '@testing-library/react';
import type { GitConflictResolution } from '@/services/git-types';
import type { PenDocument } from '@/types/pen';

// ---------------------------------------------------------------------------
// Shared mock state and actions
// ---------------------------------------------------------------------------

type NodeConflict = {
  id: string;
  pageId: string | null;
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

// Document store mock — set currentDocument via the exported ref.
let mockDocument: PenDocument = {
  id: 'doc-1',
  name: 'Test',
  children: [],
} as unknown as PenDocument;

vi.mock('@/stores/git-store', () => ({
  useGitStore: (selector: (s: typeof mocks) => unknown) => selector(mocks),
}));

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: (selector: (s: { document: PenDocument }) => unknown) =>
    selector({ document: mockDocument }),
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

function makeNode(id: string) {
  return { id, type: 'rectangle', x: 0, y: 0, width: 10, height: 10, children: [] };
}

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

/** Build a node conflict scoped to a specific page. Key schema: `node:<pageId>:<nodeId>`. */
function makePagedNodeConflict(pageId: string, nodeId: string): NodeConflict {
  return {
    id: `node:${pageId}:${nodeId}`,
    pageId,
    nodeId,
    reason: 'both-modified-same-field',
    base: null,
    ours: null,
    theirs: null,
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
    mockDocument = {
      id: 'doc-1',
      name: 'Test',
      children: [],
    } as unknown as PenDocument;
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

  // ---------------------------------------------------------------------------
  // Document-order tests (Gap 1)
  // ---------------------------------------------------------------------------

  it('emits node conflicts in document tree order (A before C, B not emitted)', () => {
    // Document has nodes A, B, C in that order.
    mockDocument = {
      id: 'doc-1',
      name: 'Test',
      children: [makeNode('A'), makeNode('B'), makeNode('C')],
    } as unknown as PenDocument;

    // Conflicts for A and C only — B has no conflict.
    mocks.state.conflicts.nodeConflicts.set('node:_:C', makeNodeConflict('node:_:C'));
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A'));

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-/);
    expect(items).toHaveLength(2);
    // A appears before C because A precedes C in the document tree.
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:_:A');
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-node:_:C');
    // B is not emitted because it has no conflict.
    expect(screen.queryByTestId('conflict-item-node:_:B')).toBeNull();
  });

  it('respects depth-first document order for nested children', () => {
    // Tree: parent (children: childA, childB)
    mockDocument = {
      id: 'doc-1',
      name: 'Test',
      children: [
        {
          ...makeNode('parent'),
          children: [makeNode('childA'), makeNode('childB')],
        },
      ],
    } as unknown as PenDocument;

    // Register conflicts in reverse insertion order to prove we sort by tree.
    mocks.state.conflicts.nodeConflicts.set('node:_:childB', makeNodeConflict('node:_:childB'));
    mocks.state.conflicts.nodeConflicts.set('node:_:childA', makeNodeConflict('node:_:childA'));

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-node/);
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:_:childA');
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-node:_:childB');
  });

  it('appends orphan node conflicts (nodeId not in document tree) at the end', () => {
    // Document has only node A.
    mockDocument = {
      id: 'doc-1',
      name: 'Test',
      children: [makeNode('A')],
    } as unknown as PenDocument;

    // Conflict for A (in-tree) and Z (orphan — not in document).
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A'));
    mocks.state.conflicts.nodeConflicts.set('node:_:Z', makeNodeConflict('node:_:Z'));

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-node/);
    expect(items).toHaveLength(2);
    // In-tree A first, orphan Z last.
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:_:A');
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-node:_:Z');
  });

  it('emits doc-field conflicts after node conflicts, sorted alphabetically by path', () => {
    mocks.state.conflicts.nodeConflicts.set('node:_:A', makeNodeConflict('node:_:A'));
    // Fields in reverse alphabetical order to prove we sort.
    mocks.state.conflicts.docFieldConflicts.set(
      'docField:name',
      makeFieldConflict('docField:name'),
    );
    mocks.state.conflicts.docFieldConflicts.set(
      'docField:author',
      makeFieldConflict('docField:author'),
    );

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-/);
    expect(items).toHaveLength(3);
    // Node conflict first.
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:_:A');
    // Fields alphabetical: author < name.
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-docField:author');
    expect(items[2].getAttribute('data-testid')).toBe('conflict-item-docField:name');
  });

  // ---------------------------------------------------------------------------
  // Existing state / bulk-action tests
  // ---------------------------------------------------------------------------

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

  // ---------------------------------------------------------------------------
  // C2: Multi-page document ordering
  // ---------------------------------------------------------------------------

  it('orders conflicts across pages in document page order (p1:A before p2:C)', () => {
    // Multi-page document: page p1 has [A, B], page p2 has [C].
    mockDocument = {
      id: 'doc-multi',
      name: 'Multi-page',
      children: [],
      pages: [
        { id: 'p1', name: 'Page 1', children: [makeNode('A'), makeNode('B')] },
        { id: 'p2', name: 'Page 2', children: [makeNode('C')] },
      ],
    } as unknown as PenDocument;

    // Register conflicts for A (p1) and C (p2) in reverse Map insertion order.
    mocks.state.conflicts.nodeConflicts.set('node:p2:C', makePagedNodeConflict('p2', 'C'));
    mocks.state.conflicts.nodeConflicts.set('node:p1:A', makePagedNodeConflict('p1', 'A'));

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-node:/);
    expect(items).toHaveLength(2);
    // p1:A comes before p2:C because p1 precedes p2 in document.pages order.
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:p1:A');
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-node:p2:C');
  });

  it('matches node conflicts by pageId so same-named nodes on different pages are separate', () => {
    // Both pages have a node named 'A' (same id 'A', but on different pages).
    mockDocument = {
      id: 'doc-same-name',
      name: 'Same name multi-page',
      children: [],
      pages: [
        { id: 'p1', name: 'Page 1', children: [makeNode('A')] },
        { id: 'p2', name: 'Page 2', children: [makeNode('A')] },
      ],
    } as unknown as PenDocument;

    // Two distinct conflicts: one for p1:A and one for p2:A.
    mocks.state.conflicts.nodeConflicts.set('node:p1:A', makePagedNodeConflict('p1', 'A'));
    mocks.state.conflicts.nodeConflicts.set('node:p2:A', makePagedNodeConflict('p2', 'A'));

    render(<GitPanelConflictList />);
    const items = screen.getAllByTestId(/conflict-item-node:/);
    // Both conflicts appear — one per page.
    expect(items).toHaveLength(2);
    expect(items[0].getAttribute('data-testid')).toBe('conflict-item-node:p1:A');
    expect(items[1].getAttribute('data-testid')).toBe('conflict-item-node:p2:A');
  });
});
