// apps/web/src/services/git-client.ts
//
// 这是对 `window.electronAPI.git.*` 的一层轻量封装。
// 每个方法都只是把调用转发给 preload bridge，
// 并把 Electron 侧序列化后的 `GitError` 恢复成真正的错误实例，
// 这样上层调用方仍然可以可靠地使用 `instanceof GitError`。
//
// 这个模块本身不保存状态。
// `withCleanWorkingTree` 这类“是否允许继续操作”的门控逻辑放在 store 层，
// 这样被拦下来的 `PendingAction` 才能在用户保存后继续重试。

import { GitError, rehydrateGitError } from './git-error';
import type {
  GitAPI,
  GitAuthCreds,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitCommitMeta,
  GitConflictBag,
  GitConflictResolution,
  GitPublicSshKeyInfo,
  GitRemoteInfo,
  GitRepoOpenInfo,
  GitStatusInfo,
} from './git-types';

/**
 * 惰性获取 `window.electronAPI.git`。
 * 如果当前不在 Electron 环境中就直接抛错，
 * 调用方应该先用 `isElectron()` 做环境判断。
 * 这里故意抛得比较早，是为了尽快暴露误用场景，
 * 例如某个只该在桌面端出现的按钮被错误地渲染到了浏览器里。
 */
function getApi(): GitAPI {
  if (typeof window === 'undefined' || !window.electronAPI?.git) {
    throw new GitError(
      'engine-crash',
      'git-client: window.electronAPI.git is unavailable (not running in Electron)',
      { recoverable: false },
    );
  }
  return window.electronAPI.git;
}

/**
 * 执行一次 IPC 调用，并把抛出的 `GitError` 恢复出来。
 * 非 `GitError` 类型的异常（例如网络超时、Electron 内部错误、
 * 或格式错误的返回值）会原样向上抛出，
 * 方便 store 区分“后端返回了已知故障”与“真的出现了意外错误”。
 */
async function invoke<T>(fn: () => Promise<T>): Promise<T> {
  try {
    return await fn();
  } catch (err) {
    const gitErr = rehydrateGitError(err);
    if (gitErr) throw gitErr;
    throw err;
  }
}

/**
 * 对外暴露：检查 `window.electronAPI.git` 是否可用。
 * 顶栏按钮会用它决定是否渲染，
 * store 的启动逻辑也会用它在浏览器模式下尽早短路成 no-op。
 */
export function isGitApiAvailable(): boolean {
  return typeof window !== 'undefined' && !!window.electronAPI?.git;
}

// ---------------------------------------------------------------------------
// 对外接口：每个 IPC 通道对应一个方法。
// 每个方法都很薄：先拿 api，再通过 `invoke()` 转发。
// ---------------------------------------------------------------------------

export const gitClient = {
  // ---- 检测 / 打开 / 初始化 / 克隆 ---------------------------------------
  detect: (filePath: string) => invoke(() => getApi().detect(filePath)),
  init: (filePath: string) => invoke(() => getApi().init(filePath)),
  open: (repoPath: string, currentFilePath?: string) =>
    invoke(() => getApi().open(repoPath, currentFilePath)),
  clone: (opts: { url: string; dest: string; auth?: GitAuthCreds }) =>
    invoke(() => getApi().clone(opts)),
  bindTrackedFile: (repoId: string, filePath: string) =>
    invoke(() => getApi().bindTrackedFile(repoId, filePath)),
  listCandidates: (repoId: string) => invoke(() => getApi().listCandidates(repoId)),
  close: (repoId: string) => invoke(() => getApi().close(repoId)),

  // ---- 状态 / 日志 / 差异 -----------------------------------------------
  status: (repoId: string) => invoke(() => getApi().status(repoId)),
  log: (repoId: string, opts: { ref: 'main' | 'autosaves' | string; limit: number }) =>
    invoke(() => getApi().log(repoId, opts)),
  diff: (repoId: string, fromCommit: string, toCommit: string) =>
    invoke(() => getApi().diff(repoId, fromCommit, toCommit)),

  // ---- 提交 / 恢复 / 提升 ----------------------------------------
  commit: (
    repoId: string,
    opts: {
      kind: 'milestone' | 'autosave';
      message: string;
      author: { name: string; email: string };
    },
  ) => invoke(() => getApi().commit(repoId, opts)),
  restore: (repoId: string, commitHash: string) =>
    invoke(() => getApi().restore(repoId, commitHash)),
  promote: (
    repoId: string,
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => invoke(() => getApi().promote(repoId, autosaveHash, message, author)),

  // ---- 分支 ----------------------------------------------------------
  branchList: (repoId: string) => invoke(() => getApi().branchList(repoId)),
  branchCreate: (repoId: string, opts: { name: string; fromCommit?: string }) =>
    invoke(() => getApi().branchCreate(repoId, opts)),
  branchSwitch: (repoId: string, name: string) => invoke(() => getApi().branchSwitch(repoId, name)),
  branchDelete: (repoId: string, name: string, opts?: { force?: boolean }) =>
    invoke(() => getApi().branchDelete(repoId, name, opts)),
  branchMerge: (repoId: string, fromBranch: string) =>
    invoke(() => getApi().branchMerge(repoId, fromBranch)),

  // ---- 合并编排 -----------------------------------------------
  resolveConflict: (repoId: string, conflictId: string, choice: GitConflictResolution) =>
    invoke(() => getApi().resolveConflict(repoId, conflictId, choice)),
  applyMerge: (repoId: string) => invoke(() => getApi().applyMerge(repoId)),
  abortMerge: (repoId: string) => invoke(() => getApi().abortMerge(repoId)),

  // ---- Phase 4a：探测作者身份 -----------------------------------
  getSystemAuthor: () => invoke(() => getApi().getSystemAuthor()),

  // ---- 远程仓库 ------------------------------------------------------------
  fetch: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().fetch(repoId, auth)),
  pull: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().pull(repoId, auth)),
  push: (repoId: string, auth?: GitAuthCreds) => invoke(() => getApi().push(repoId, auth)),

  // ---- Phase 6a：远程元数据 + 配置（无网络） -------------------
  remoteGet: (repoId: string) => invoke(() => getApi().remoteGet(repoId)),
  remoteSet: (repoId: string, url: string | null) => invoke(() => getApi().remoteSet(repoId, url)),

  // ---- 认证 --------------------------------------------------------------
  authStore: (host: string, creds: GitAuthCreds) => invoke(() => getApi().authStore(host, creds)),
  authGet: (host: string) => invoke(() => getApi().authGet(host)),
  authClear: (host: string) => invoke(() => getApi().authClear(host)),

  // ---- SSH 密钥 ----------------------------------------------------------
  sshListKeys: () => invoke(() => getApi().sshListKeys()),
  sshGenerateKey: (opts: { host: string; comment: string }) =>
    invoke(() => getApi().sshGenerateKey(opts)),
  sshImportKey: (opts: { privateKeyPath: string; host: string }) =>
    invoke(() => getApi().sshImportKey(opts)),
  sshDeleteKey: (keyId: string) => invoke(() => getApi().sshDeleteKey(keyId)),
};

// 重新导出大多数消费者会用到的类型，避免导入点再去单独引用 `git-types`。
export type {
  GitRepoOpenInfo,
  GitStatusInfo,
  GitCommitMeta,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitConflictBag,
  GitConflictResolution,
  GitAuthCreds,
  GitPublicSshKeyInfo,
  GitRemoteInfo,
};
