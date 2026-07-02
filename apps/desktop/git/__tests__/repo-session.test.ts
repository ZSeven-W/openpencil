// apps/desktop/git/__tests__/repo-session.test.ts
import { describe, it, expect, beforeEach } from 'vitest';
import {
  registerSession,
  getSession,
  updateTrackedFile,
  updateCandidates,
  unregisterSession,
  clearAllSessions,
  sessionCount,
  setInflightMerge,
  clearInflightMerge,
} from '../repo-session';
import type { IsoRepoHandle } from '../git-iso';

const stubHandle: IsoRepoHandle = {
  dir: '/tmp/stub',
  gitdir: '/tmp/stub/.git',
  mode: 'folder',
};

describe('repo-session', () => {
  beforeEach(() => {
    clearAllSessions();
  });

  it('registerSession allocates a unique repoId and getSession round-trips', () => {
    const a = registerSession({
      handle: stubHandle,
      trackedFilePath: '/tmp/stub/a.op',
      candidateFiles: [],
      engineKind: 'iso',
    });
    const b = registerSession({
      handle: stubHandle,
      trackedFilePath: '/tmp/stub/b.op',
      candidateFiles: [],
      engineKind: 'iso',
    });
    expect(a.repoId).not.toBe(b.repoId);
    expect(getSession(a.repoId)?.trackedFilePath).toBe('/tmp/stub/a.op');
    expect(getSession(b.repoId)?.trackedFilePath).toBe('/tmp/stub/b.op');
    expect(sessionCount()).toBe(2);
  });

  it('updateTrackedFile mutates the session and returns true; unknown id returns false', () => {
    const s = registerSession({
      handle: stubHandle,
      trackedFilePath: null,
      candidateFiles: [],
      engineKind: 'iso',
    });
    expect(updateTrackedFile(s.repoId, '/tmp/stub/picked.op')).toBe(true);
    expect(getSession(s.repoId)?.trackedFilePath).toBe('/tmp/stub/picked.op');
    expect(updateTrackedFile('not-a-real-id', '/tmp/x.op')).toBe(false);
  });

  it('updateCandidates replaces the cached candidate list', () => {
    const s = registerSession({
      handle: stubHandle,
      trackedFilePath: null,
      candidateFiles: [],
      engineKind: 'iso',
    });
    expect(
      updateCandidates(s.repoId, [
        {
          path: '/tmp/stub/a.op',
          relativePath: 'a.op',
          milestoneCount: 0,
          autosaveCount: 0,
          lastCommitAt: null,
          lastCommitMessage: null,
        },
      ]),
    ).toBe(true);
    expect(getSession(s.repoId)?.candidateFiles).toHaveLength(1);
  });

  it('unregisterSession removes the entry and getSession returns undefined', () => {
    const s = registerSession({
      handle: stubHandle,
      trackedFilePath: null,
      candidateFiles: [],
      engineKind: 'iso',
    });
    expect(unregisterSession(s.repoId)).toBe(true);
    expect(getSession(s.repoId)).toBeUndefined();
    expect(unregisterSession(s.repoId)).toBe(false); // already gone
  });

  it('setInflightMerge and clearInflightMerge mutate the session', () => {
    const s = registerSession({
      handle: stubHandle,
      trackedFilePath: '/tmp/stub/a.op',
      candidateFiles: [],
      engineKind: 'iso',
    });
    expect(getSession(s.repoId)?.inflightMerge).toBeNull();

    // Minimal InflightMerge stub (types cast bypasses full shape — the test
    // just exercises the registry mutators, not the merge logic).
    const merge = {
      oursCommit: 'a'.repeat(40),
      theirsCommit: 'b'.repeat(40),
      baseCommit: 'c'.repeat(40),
      mergeResult: {
        merged: { version: '1.0.0', name: 'd', children: [] },
        nodeConflicts: [],
        docFieldConflicts: [],
      },
      conflictMap: new Map(),
      resolutions: new Map(),
      defaultMessage: 'Merge',
    };
    expect(setInflightMerge(s.repoId, merge as never)).toBe(true);
    expect(getSession(s.repoId)?.inflightMerge).toBe(merge);

    expect(clearInflightMerge(s.repoId)).toBe(true);
    expect(getSession(s.repoId)?.inflightMerge).toBeNull();
  });
});
