// apps/web/src/stores/git-store-types.ts
//
// 从 git-store.ts 中提取的 Type 声明以将该文件保存在
// 800-LoC 上限。 Pure 类型 — 没有运行时代码，没有操作，没有帮助程序。
// Imported by git-store.ts 以及任何需要类型检查的消费者
// 反对 GitState / RepoMeta / GitStore。

import type {
  GitAuthCreds,
  GitBranchInfo,
  GitCandidateFileInfo,
  GitCommitMeta,
  GitConflictBag,
  GitConflictResolution,
  GitPublicSshKeyInfo,
  GitRemoteInfo,
} from '@/services/git-types';

// ---------------------------------------------------------------------------
// Types: GitState 联合 + RepoMeta + ConflictBagState + PendingAction
// ---------------------------------------------------------------------------

export interface RepoMeta {
  repoId: string;
  mode: 'single-file' | 'folder';
  rootPath: string;
  gitdir: string;
  engineKind: 'iso' | 'sys';
  trackedFilePath: string | null;
  candidateFiles: GitCandidateFileInfo[];
  currentBranch: string;
  branches: GitBranchInfo[];
  workingDirty: boolean;
  otherFilesDirty: number;
  otherFilesPaths: string[];
  ahead: number;
  behind: number;
  /**
   * Phase 6a：缓存单
   * 个“原始”远程的远程元数据，如果尚未发出探测，则为 null。 Hydrated by `refreshRemote()`，由
   * `setRemoteUrl()` 突变。 The 存储仅读取 `.git/config` — 无网络。 The Phase 6b
   * pull/push 控件和 Phase 6c 远程设置 UI 均在此字段上分支。
   *
   */
  remote: GitRemoteInfo | null;
}

/**
 * Renderer
 * 端包装器，跟踪由 conflictId 键控的 Map 中的每个冲突解决状态。当 branchMerge/pull 返回冲突时，Built 由
 * hydrateConflictBag() 来自有线格式 GitConflictBag。 Invariant：后端在不同的命名空间中发出
 *
 * conflictIds — `node:<pageId|_>:<nodeId>`
 * 表示节点冲突，`docF
 * ield:<field>` 表示文档字段冲突。因此，The 和 Maps 两个 Maps 不共享密钥，并且
 * resolveConflict 可以按顺序探测它们而不会产生歧义。 If 后端曾经改变过这一点，resolveConflict
 * 的分支逻辑变得不明确，必须更新以携带显式类型标签。
 *
 */
export interface ConflictBagState {
  nodeConflicts: Map<
    string,
    GitConflictBag['nodeConflicts'][number] & { resolution?: GitConflictResolution }
  >;
  docFieldConflicts: Map<
    string,
    GitConflictBag['docFieldConflicts'][number] & { resolution?: GitConflictResolution }
  >;
}

export interface PendingAction {
  label: string;
  run: () => Promise<void>;
}

/**
 * Recoverable
 * GitErrorCodes 克隆向导捕获内联而不是让它们转义到通用 `error` 状态。 Defined
 * 在这里，以便存储操作和向导组件在确切的设置上达成一致。
 */
export const CLONE_INLINE_ERROR_CODES = [
  'clone-network',
  'network',
  'timeout',
  'auth-required',
  'auth-failed',
  'auth-token-invalid',
  'clone-failed',
  'clone-target-exists',
] as const;

export type CloneInlineErrorCode = (typeof CLONE_INLINE_ERROR_CODES)[number];

/**
 * Phase 6b：与可恢
 * 复的身份验证相关的 GitErrorCodes 拉/推按钮捕获内联并显示共享身份验证表单。 `auth-required`
 * 涵盖“根本没有凭据”的情况； `auth-failed` 和 `auth-token-invalid` 涵盖“服务器拒绝的存储凭据”。
 * Anything 否则转义到一般错误状态。 Pull 和 Push 共享一个常量——两个流的 auth 形式是相同的，并且没有计划中的分歧。
 *
 *
 */
export const REMOTE_AUTH_ERROR_CODES = [
  'auth-required',
  'auth-failed',
  'auth-token-invalid',
] as const;

export type RemoteAuthErrorCode = (typeof REMOTE_AUTH_ERROR_CODES)[number];

export type GitState =
  | { kind: 'no-file' }
  | { kind: 'no-repo' }
  | {
      kind: 'wizard-clone';
      /**
       * True 而 `clon
       * eRepo()` 则从向导内部飞行。 The 向导在整个往返过程中保持安装状态（不会转换到
       * `initializing`），因此表单的 URL/dest/token 输入可以在可恢复的故障中幸存下来。 The
       * 克隆形式直接读取此内容，而不是保留在卸载时会丢失的本地 `useState`。
       *
       */
      busy: boolean;
      /**
       * Inline 错误出现在
       * 克隆表单下。 Set 当 cloneRepo() 捕获可恢复代码时（请参阅 CLONE_INLINE_ERROR_CODES）。
       * The 向导保持安装状态，以便用户可以修复 URL/auth 并重试，而不会丢失表单状态。 Cleared 在下一次尝试
       * cloneRepo() 或 cancelCloneWizard() 时。
       *
       */
      error: { code: CloneInlineErrorCode; message: string } | null;
    }
  | { kind: 'initializing' }
  | { kind: 'needs-tracked-file'; repo: RepoMeta }
  | { kind: 'ready'; repo: RepoMeta; saveRequiredFor?: PendingAction }
  | {
      kind: 'conflict';
      repo: RepoMeta;
      conflicts: ConflictBagState;
      /**
       * Phase 6b：后端报
       * 告为未解析的非 `.op` 文件的路径（相对于存储库根）。 Empty 数组意味着冲突纯粹是在 `.op`
       * node/field 数据上，现有的每节点分辨率 UI 涵盖了它。 Non-empty
       * 意味着用户必须在外部解析这些文件并点击“继续”，或者完全中止合并 - 当非空时，冲突横幅会呈现一条具有两种恢复功能的条
       * 带。
       *
       *
       */
      unresolvedFiles: string[];
      /**
       * Phase 7b：上次
       * applyMerge() 调用引发的内联错误，抛出 `merge-still-conflicted`。 Cleared
       * 当用户解决更多冲突并重试时，或者当 refreshStatus() 协调状态时。 Null 表示没有待处理的最终错误。
       *
       */
      finalizeError: string | null;
      /**
       * I2：当面板在合并过程中
       * 重新打开且内存中冲突状态丢失时为 true（session.inflightMerge === null，MERGE_HEAD
       * 在磁盘上）。当这是 true 时，The 横幅会呈现仅中止的 UI。 False （或不存在）在所有正常的合并流中。
       *
       */
      reopenedMidMerge: boolean;
      saveRequiredFor?: PendingAction;
    }
  | { kind: 'error'; message: string; recoverable: boolean };

// ---------------------------------------------------------------------------
// Store 接口
// ---------------------------------------------------------------------------

export interface GitStore {
  state: GitState;
  panelOpen: boolean;
  log: GitCommitMeta[];
  sshKeys: GitPublicSshKeyInfo[];

  // Phase 4a：作者身份（缓存+通过首选项保存）
  authorIdentity: { name: string; email: string } | null;
  authorPromptVisible: boolean;

  // Phase 4b：自动绑定横幅（openRepo/cloneRepo 自动绑定单个候选文件时设置的瞬态标志；通过确认操作或
  // closeRepo 清除）
  lastAutoBindedPath: string | null;

  // Phase 4c：提交输入草稿（短暂，不持久）
  commitMessage: string;

  // Phase 4c：自动保存错误显示（来自订户的最后一个错误）
  autosaveError: string | null;

  // Phase 4c：订阅者生命周期句柄（内部，不会被 UI 读取）
  __autosaveUnsub: (() => void) | null;

  // Panel 生命周期
  togglePanel: () => void;
  openPanel: () => void;
  closePanel: () => void;

  // Phase 4a：作者身份操作
  loadAuthorIdentity: () => Promise<void>;
  setAuthorIdentity: (name: string, email: string) => Promise<void>;
  showAuthorPrompt: () => void;
  hideAuthorPrompt: () => void;

  // Phase 4b：自动绑定横幅操作
  acknowledgeAutoBind: () => void;
  acknowledgeAutoBindAndOpen: () => Promise<void>;

  // Phase 4c：提交输入操作
  setCommitMessage: (text: string) => void;
  clearCommitMessage: () => void;
  cancelSaveRequired: () => void;

  // Phase 4c：溢出菜单操作
  enterTrackedFilePicker: () => void;
  /**
   * Phase 7b：退出跟
   * 踪文件选择器。 - If 从 `ready` 输入选择器（repo.track
   * edFilePath 非空）→ 使用相同的存储库转换回 `ready`。 - If 选择器是第一个 post-open/post-clone
   * 屏幕 (repo.trackedFilePath === null) →
   * 关闭临时存储库会话并返回到 `no-file`。
   *
   */
  exitTrackedFilePicker: () => Promise<void>;
  clearAuthorIdentity: () => Promise<void>;

  // Phase 4c：自动保存订阅者生命周期
  initAutosaveSubscriber: () => void;
  disposeAutosaveSubscriber: () => void;
  clearAutosaveError: () => void;

  // Repo 发现/创建
  detectRepo: (filePath: string) => Promise<void>;
  initRepo: (filePath: string) => Promise<void>;
  openRepo: (repoPath: string, currentFilePath?: string) => Promise<void>;
  cloneRepo: (opts: { url: string; dest: string; auth?: GitAuthCreds }) => Promise<void>;
  bindTrackedFile: (filePath: string) => Promise<void>;
  refreshCandidates: () => Promise<void>;
  closeRepo: () => Promise<void>;

  // Status / 日志 / 差异
  refreshStatus: () => Promise<void>;
  loadLog: (opts: { ref: 'main' | 'autosaves' | string; limit: number }) => Promise<void>;
  computeDiff: (
    from: string,
    to: string,
  ) => Promise<{
    summary: {
      framesChanged: number;
      nodesAdded: number;
      nodesRemoved: number;
      nodesModified: number;
    };
    patches: unknown[];
  }>;

  // Commit / 恢复 / 提升（所有 MUTATING，由 withCleanWorkingTree 门控）
  commitMilestone: (message: string, author: { name: string; email: string }) => Promise<void>;
  commitAutosave: (message: string, author: { name: string; email: string }) => Promise<void>;
  restoreCommit: (commitHash: string) => Promise<void>;
  promoteAutosave: (
    autosaveHash: string,
    message: string,
    author: { name: string; email: string },
  ) => Promise<void>;

  // Branches（switch/merge MUTATING，其他只读）
  refreshBranches: () => Promise<void>;
  createBranch: (opts: { name: string; fromCommit?: string }) => Promise<void>;
  switchBranch: (name: string) => Promise<void>;
  deleteBranch: (name: string, opts?: { force?: boolean }) => Promise<void>;
  mergeBranch: (fromBranch: string) => Promise<void>;

  // Merge 编排
  resolveConflict: (conflictId: string, choice: GitConflictResolution) => Promise<void>;
  applyMerge: () => Promise<void>;
  abortMerge: () => Promise<void>;

  // Remote（pull/push MUTATING，获取只读）
  fetchRemote: (auth?: GitAuthCreds) => Promise<void>;
  pull: (auth?: GitAuthCreds) => Promise<void>;
  push: (auth?: GitAuthCreds) => Promise<void>;

  // Phase 6a：克隆向导 + 远程 metadata/config
  /**
   * Transition
   * 任何状态进入 `wizard-clone`，没有内联错误。 The 空状态克隆卡是 6a 中唯一的入口点；后续阶段可能会添加来自
   * `ready` 的设置条目。
   */
  enterCloneWizard: () => void;
  /**
   * Always 转换回
   * `no-file`。 The git-panel.tsx detector-repo 效果会立即从当前打开的文档路径重新水化正确的
   * `no-repo` / `ready` 状态，因此我们在这里不需要更智能的取消目标。
   *
   */
  cancelCloneWizard: () => void;
  /**
   * Refresh 通过
   * remoteGet 从桌面端缓存 `repo.remote`。仅 Reads `.git/config` — 无网络。 No-op
   当状态没有存储库时。
   */
  refreshRemote: () => Promise<void>;
  /**
   * Set 或清除单个“来源
   * ”遥控器。 Pass 一个指向 add/update 的非空 URL；通过 `null` 来删除。 Updates
   * `repo.remote` 立即从 IPC 返回值，因此单次往返就足够了 - 调用者 MUST NOT 依赖后续
   * refreshRemote() 来查看新值。
   */
  setRemoteUrl: (url: string | null) => Promise<void>;

  // Auth
  storeAuth: (host: string, creds: GitAuthCreds) => Promise<void>;
  getAuth: (host: string) => Promise<GitAuthCreds | null>;
  clearAuth: (host: string) => Promise<void>;

  // SSH 键
  refreshSshKeys: () => Promise<void>;
  generateSshKey: (opts: { host: string; comment: string }) => Promise<GitPublicSshKeyInfo>;
  importSshKey: (opts: { privateKeyPath: string; host: string }) => Promise<GitPublicSshKeyInfo>;
  deleteSshKey: (keyId: string) => Promise<void>;

  // Retry 成功保存后排队的操作（Phase 4 线按钮）
  retrySaveRequired: () => Promise<void>;
}
