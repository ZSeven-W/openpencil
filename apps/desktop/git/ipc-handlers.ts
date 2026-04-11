// apps/desktop/git/ipc-handlers.ts
//
// Thin shim: each handler is a one-line forward to a git-engine function.
// Tests call gitIpcHandlers directly. setupGitIPC wires them onto ipcMain.
//
// Error serialization: Electron's structured-clone drops custom Error
// subclasses (only `message` survives reliably). We tag GitError instances
// by stuffing { __gitError, code, message, recoverable } into the message
// field as JSON, prefixed with a marker the renderer detects on receive.

import { ipcMain } from 'electron';
import { GitError, isGitError } from './error';
import {
  engineDetect,
  engineInit,
  engineOpen,
  engineBindTrackedFile,
  engineListCandidates,
  engineClose,
  engineStatus,
  engineLog,
  engineCommit,
  engineRestore,
  enginePromote,
  engineBranchList,
  engineBranchCreate,
  engineBranchSwitch,
  engineBranchDelete,
  engineClone,
  engineFetch,
  enginePull,
  enginePush,
  engineDiff,
  engineBranchMerge,
  engineResolveConflict,
  engineApplyMerge,
  engineAbortMerge,
  engineRemoteGet,
  engineRemoteSet,
  setSshKeyManager,
  setAuthStore,
} from './git-engine';
import type { ConflictResolution } from './merge-session';
import { createDefaultAuthStore, type AuthCreds, type AuthStore } from './auth-store';
import { createDefaultSshKeyManager, type SshKeyInfo, type SshKeyManager } from './ssh-keys';
import { getSystemAuthor as sysGetSystemAuthor } from './git-sys';

// ---------------------------------------------------------------------------
// Module-level singletons assigned by setupGitIPC at boot. We require Electron
// to be ready (app.whenReady()) before instantiating because both auth-store
// and ssh-keys lazy-import electron's `app` and `safeStorage`.
// ---------------------------------------------------------------------------

let authStore: AuthStore | null = null;
let sshKeyManager: SshKeyManager | null = null;

function requireAuthStore(): AuthStore {
  if (!authStore) throw new Error('auth store not initialized; call setupGitIPC() first');
  return authStore;
}

function requireSshKeyManager(): SshKeyManager {
  if (!sshKeyManager) throw new Error('ssh key manager not initialized; call setupGitIPC() first');
  return sshKeyManager;
}

/**
 * Strip the on-disk private key path before returning SSH key info to the
 * renderer. The path is backend-only — the renderer never needs it because
 * SSH transport is invoked via git-sys with GIT_SSH_COMMAND, never via the
 * renderer-side filesystem.
 */
type PublicSshKeyInfo = Omit<SshKeyInfo, 'privateKeyPath'>;

function stripPrivatePath(info: SshKeyInfo): PublicSshKeyInfo {
  const { privateKeyPath: _omit, ...rest } = info;
  void _omit;
  return rest;
}

const GIT_ERROR_MARKER = '__GIT_ERROR__';

/**
 * Serialize a GitError into an Error whose message is JSON-encoded with the
 * GIT_ERROR_MARKER prefix. The renderer's git-client (Phase 3) detects the
 * marker and rehydrates a GitError on its side.
 */
export function serializeGitError(err: GitError): Error {
  const payload = {
    code: err.code,
    message: err.message,
    recoverable: err.recoverable,
  };
  return new Error(`${GIT_ERROR_MARKER}${JSON.stringify(payload)}`);
}

/**
 * Wrap a handler so any thrown GitError becomes a serialized Error suitable
 * for crossing the IPC boundary. Other errors propagate as-is (their
 * `message` survives clone but the stack/type does not).
 */
async function runHandler<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    if (isGitError(err)) {
      throw serializeGitError(err as GitError);
    }
    throw err;
  }
}

// ---------------------------------------------------------------------------
// Direct handler functions — exported so tests can call them without ipcMain.
// Each is a one-line forward to engineX(). Argument shapes mirror the IPC
// contract section of the spec.
// ---------------------------------------------------------------------------

export const gitIpcHandlers = {
  detect: (filePath: string) => engineDetect(filePath),
  init: (filePath: string) => engineInit(filePath),
  open: (repoPath: string, currentFilePath?: string) => engineOpen(repoPath, currentFilePath),
  bindTrackedFile: (repoId: string, filePath: string) => engineBindTrackedFile(repoId, filePath),
  listCandidates: (repoId: string) => engineListCandidates(repoId),
  close: (repoId: string) => {
    engineClose(repoId);
    return Promise.resolve();
  },
  status: (repoId: string) => engineStatus(repoId),
  log: (repoId: string, opts: { ref: 'main' | 'autosaves' | string; limit: number }) =>
    engineLog(repoId, opts),
  commit: (
    repoId: string,
    opts: {
      kind: 'milestone' | 'autosave';
      message: string;
      author: { name: string; email: string };
    },
  ) => engineCommit(repoId, opts),
  restore: (repoId: string, commitHash: string) => engineRestore(repoId, commitHash),
  promote: (
    repoId: string,
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => enginePromote(repoId, autosaveHash, message, author),
  branchList: (repoId: string) => engineBranchList(repoId),
  branchCreate: (repoId: string, opts: { name: string; fromCommit?: string }) =>
    engineBranchCreate(repoId, opts),
  branchSwitch: (repoId: string, name: string) => engineBranchSwitch(repoId, name),
  branchDelete: (repoId: string, name: string, opts?: { force?: boolean }) =>
    engineBranchDelete(repoId, name, opts),

  // Phase 2b: remote ops
  clone: (opts: { url: string; dest: string; auth?: AuthCreds }) => engineClone(opts),
  fetch: (repoId: string, auth?: AuthCreds) => engineFetch(repoId, auth),
  pull: (repoId: string, auth?: AuthCreds) => enginePull(repoId, auth),
  push: (repoId: string, auth?: AuthCreds) => enginePush(repoId, auth),

  // Phase 6a: remote metadata + config (no network)
  remoteGet: (repoId: string) => engineRemoteGet(repoId),
  remoteSet: (repoId: string, url: string | null) => engineRemoteSet(repoId, url),

  // Phase 2b: auth
  authStore: (host: string, creds: AuthCreds) => requireAuthStore().set(host, creds),
  authGet: (host: string) => requireAuthStore().get(host),
  authClear: (host: string) => requireAuthStore().clear(host),

  // Phase 2b: ssh keys (privateKeyPath stripped before crossing IPC)
  sshListKeys: async (): Promise<PublicSshKeyInfo[]> => {
    const all = await requireSshKeyManager().list();
    return all.map(stripPrivatePath);
  },
  sshGenerateKey: async (opts: { host: string; comment: string }): Promise<PublicSshKeyInfo> => {
    const info = await requireSshKeyManager().generate(opts);
    return stripPrivatePath(info);
  },
  sshImportKey: async (opts: {
    privateKeyPath: string;
    host: string;
  }): Promise<PublicSshKeyInfo> => {
    const info = await requireSshKeyManager().import(opts);
    return stripPrivatePath(info);
  },
  sshDeleteKey: (keyId: string) => requireSshKeyManager().delete(keyId),

  // Phase 2c: merge orchestration
  diff: (repoId: string, fromCommit: string, toCommit: string) =>
    engineDiff(repoId, fromCommit, toCommit),
  branchMerge: (repoId: string, fromBranch: string) => engineBranchMerge(repoId, fromBranch),
  resolveConflict: (repoId: string, conflictId: string, choice: ConflictResolution) =>
    engineResolveConflict(repoId, conflictId, choice),
  applyMerge: (repoId: string) => engineApplyMerge(repoId),
  abortMerge: (repoId: string) => engineAbortMerge(repoId),

  // Phase 4a: author identity probe (system git config)
  getSystemAuthor: () => sysGetSystemAuthor(),
};

// ---------------------------------------------------------------------------
// ipcMain registration. Each channel is registered exactly once. Calling
// setupGitIPC twice would throw on the second call (ipcMain.handle rejects
// duplicate channel names) — main.ts must ensure single invocation.
// ---------------------------------------------------------------------------

export function setupGitIPC(): void {
  // Lazy-instantiate the auth/ssh singletons. These pull in Electron, so they
  // must run AFTER app.whenReady() (which is when main.ts calls setupGitIPC).
  authStore = createDefaultAuthStore();
  sshKeyManager = createDefaultSshKeyManager();
  // Inject both into the engine so engineClone/Fetch/Pull/Push can:
  //   - look up stored host credentials when no explicit auth was passed
  //   - resolve SSH keyIds to private key file paths
  setAuthStore(authStore);
  setSshKeyManager(sshKeyManager);

  ipcMain.handle('git:detect', (_e, filePath: string) =>
    runHandler(() => gitIpcHandlers.detect(filePath)),
  );
  ipcMain.handle('git:init', (_e, filePath: string) =>
    runHandler(() => gitIpcHandlers.init(filePath)),
  );
  ipcMain.handle('git:open', (_e, repoPath: string, currentFilePath?: string) =>
    runHandler(() => gitIpcHandlers.open(repoPath, currentFilePath)),
  );
  ipcMain.handle('git:bindTrackedFile', (_e, repoId: string, filePath: string) =>
    runHandler(() => gitIpcHandlers.bindTrackedFile(repoId, filePath)),
  );
  ipcMain.handle('git:listCandidates', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.listCandidates(repoId)),
  );
  ipcMain.handle('git:close', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.close(repoId)),
  );
  ipcMain.handle('git:status', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.status(repoId)),
  );
  ipcMain.handle(
    'git:log',
    (_e, repoId: string, opts: { ref: 'main' | 'autosaves' | string; limit: number }) =>
      runHandler(() => gitIpcHandlers.log(repoId, opts)),
  );
  ipcMain.handle(
    'git:commit',
    (
      _e,
      repoId: string,
      opts: {
        kind: 'milestone' | 'autosave';
        message: string;
        author: { name: string; email: string };
      },
    ) => runHandler(() => gitIpcHandlers.commit(repoId, opts)),
  );
  ipcMain.handle('git:restore', (_e, repoId: string, commitHash: string) =>
    runHandler(() => gitIpcHandlers.restore(repoId, commitHash)),
  );
  ipcMain.handle(
    'git:promote',
    (
      _e,
      repoId: string,
      autosaveHash: string,
      message: string,
      author: { name: string; email: string },
    ) => runHandler(() => gitIpcHandlers.promote(repoId, autosaveHash, message, author)),
  );
  ipcMain.handle('git:branchList', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.branchList(repoId)),
  );
  ipcMain.handle(
    'git:branchCreate',
    (_e, repoId: string, opts: { name: string; fromCommit?: string }) =>
      runHandler(() => gitIpcHandlers.branchCreate(repoId, opts)),
  );
  ipcMain.handle('git:branchSwitch', (_e, repoId: string, name: string) =>
    runHandler(() => gitIpcHandlers.branchSwitch(repoId, name)),
  );
  ipcMain.handle(
    'git:branchDelete',
    (_e, repoId: string, name: string, opts?: { force?: boolean }) =>
      runHandler(() => gitIpcHandlers.branchDelete(repoId, name, opts)),
  );

  // ---- Phase 2b: remote ops ------------------------------------------------
  ipcMain.handle('git:clone', (_e, opts: { url: string; dest: string; auth?: AuthCreds }) =>
    runHandler(() => gitIpcHandlers.clone(opts)),
  );
  ipcMain.handle('git:fetch', (_e, repoId: string, auth?: AuthCreds) =>
    runHandler(() => gitIpcHandlers.fetch(repoId, auth)),
  );
  ipcMain.handle('git:pull', (_e, repoId: string, auth?: AuthCreds) =>
    runHandler(() => gitIpcHandlers.pull(repoId, auth)),
  );
  ipcMain.handle('git:push', (_e, repoId: string, auth?: AuthCreds) =>
    runHandler(() => gitIpcHandlers.push(repoId, auth)),
  );

  // ---- Phase 6a: remote metadata + config ---------------------------------
  ipcMain.handle('git:remoteGet', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.remoteGet(repoId)),
  );
  ipcMain.handle('git:remoteSet', (_e, repoId: string, url: string | null) =>
    runHandler(() => gitIpcHandlers.remoteSet(repoId, url)),
  );

  // ---- Phase 2b: auth ------------------------------------------------------
  ipcMain.handle('git:authStore', (_e, host: string, creds: AuthCreds) =>
    runHandler(() => gitIpcHandlers.authStore(host, creds)),
  );
  ipcMain.handle('git:authGet', (_e, host: string) =>
    runHandler(() => gitIpcHandlers.authGet(host)),
  );
  ipcMain.handle('git:authClear', (_e, host: string) =>
    runHandler(() => gitIpcHandlers.authClear(host)),
  );

  // ---- Phase 2b: ssh keys --------------------------------------------------
  ipcMain.handle('git:sshListKeys', () => runHandler(() => gitIpcHandlers.sshListKeys()));
  ipcMain.handle('git:sshGenerateKey', (_e, opts: { host: string; comment: string }) =>
    runHandler(() => gitIpcHandlers.sshGenerateKey(opts)),
  );
  ipcMain.handle('git:sshImportKey', (_e, opts: { privateKeyPath: string; host: string }) =>
    runHandler(() => gitIpcHandlers.sshImportKey(opts)),
  );
  ipcMain.handle('git:sshDeleteKey', (_e, keyId: string) =>
    runHandler(() => gitIpcHandlers.sshDeleteKey(keyId)),
  );

  // ---- Phase 2c: merge orchestration --------------------------------------
  ipcMain.handle('git:diff', (_e, repoId: string, fromCommit: string, toCommit: string) =>
    runHandler(() => gitIpcHandlers.diff(repoId, fromCommit, toCommit)),
  );
  ipcMain.handle('git:branchMerge', (_e, repoId: string, fromBranch: string) =>
    runHandler(() => gitIpcHandlers.branchMerge(repoId, fromBranch)),
  );
  ipcMain.handle(
    'git:resolveConflict',
    (_e, repoId: string, conflictId: string, choice: ConflictResolution) =>
      runHandler(() => gitIpcHandlers.resolveConflict(repoId, conflictId, choice)),
  );
  ipcMain.handle('git:applyMerge', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.applyMerge(repoId)),
  );
  ipcMain.handle('git:abortMerge', (_e, repoId: string) =>
    runHandler(() => gitIpcHandlers.abortMerge(repoId)),
  );

  // Phase 4a: author identity probe
  ipcMain.handle('git:getSystemAuthor', () => runHandler(() => gitIpcHandlers.getSystemAuthor()));
}

/** Exposed for the renderer-side rehydrator (Phase 3). Tests use it too. */
export { GIT_ERROR_MARKER };
