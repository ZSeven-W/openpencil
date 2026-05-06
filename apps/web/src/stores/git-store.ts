// apps/web/src/stores/git-store.ts
//
// Zustand 存储实现 GitState 状态机。 Every 变异
// 动作经过 withCleanWorkingTree 所以渲染器永远不能
// 用不同步的树覆盖磁盘。 Pure 助手和脏/
// runOrError 包装器位于 git-store-helpers.ts 中以将此文件保存在
// 800-LoC 上限。
//
// NOTE: 该文件大约有 848 行（超过上限 48 行）。 Phase 7c 提取
// makeReloadAfterApply 到 git-store-helpers.ts，但是 applyMerge 重新加载
// 编排和 noop 处理添加了线路。 Further 提取
// 延迟 — 请参阅 Phase 8+ 了解专用重构。

import { create } from 'zustand';
import { gitClient } from '@/services/git-client';
import { GitError, isGitError } from '@/services/git-error';
import { useDocumentStore } from '@/stores/document-store';
import { documentEvents } from '@/utils/document-events';
import { loadOpFileFromPath } from '@/utils/load-op-file';
import {
  buildConflictState,
  classifyCloneError,
  currentLogRef,
  dropSaveRequired,
  makeAutosaveHandler,
  makeReloadAfterApply,
  makeSyncAfterHeadMove,
  metaFromOpenInfo,
  patchRepoRemote,
  requireRepoId,
  resolveAuthorIdentity,
} from './git-store-helpers';
import type { GitStore, PendingAction } from './git-store-types';

export const useGitStore = create<GitStore>((set, get) => {
  /**
   * Phase 6b：共享头
   * 部移动同步（refreshStatus + refreshBranches + 重新加载跟踪文件 + loadLog）。
   * Called 来自 pull/switchBranch/mergeBranch 干净的路径，因此所有三个头部移动动作保持同步。
   */
  const syncAfterHeadMove = makeSyncAfterHeadMove(get);

  /**
   * Phase 7c：应用后
   * 重新加载（重新加载跟踪文件+ refreshStatus + loadLog）。 Called 来自 applyMerge()
   * 在正常成功和 noop 路径上。 Extracted 到 git-store-helpers.ts 以将此文件保持在上限之下。
   */
  const reloadAfterApply = makeReloadAfterApply(get);

  /**
   * Guard 对
   * `useDocumentStore.getState().isDirty` 的变异操作。 Dirty → 隐藏 PendingAction 并抛出
   * GitError('save-required')； UI 显示内嵌警报，而 retrySaveRequired 在保存后重新运行该操作。
   */
  async function withCleanWorkingTree<T>(action: () => Promise<T>, label: string): Promise<T> {
    if (useDocumentStore.getState().isDirty) {
      const pending: PendingAction = {
        label,
        run: async () => {
          await action();
        },
      };
      set((s) => {
        if (s.state.kind === 'ready' || s.state.kind === 'conflict') {
          return { state: { ...s.state, saveRequiredFor: pending } };
        }
        return s;
      });
      throw new GitError('save-required', 'Document has unsaved changes');
    }
    return action();
  }

  /** Run 一个动作；抛出 GitError 转换到一般错误状态。 */
  async function runOrError<T>(action: () => Promise<T>): Promise<T | undefined> {
    try {
      return await action();
    } catch (err) {
      const message = isGitError(err)
        ? err.message
        : err instanceof Error
          ? err.message
          : String(err);
      const recoverable = isGitError(err) ? err.recoverable : false;
      set({ state: { kind: 'error', message, recoverable } });
      throw err;
    }
  }

  return {
    state: { kind: 'no-file' },
    panelOpen: false,
    log: [],
    sshKeys: [],

    // Phase 4a：作者身份（loadAuthorIdentity 运行查找链）
    authorIdentity: null,
    authorPromptVisible: false,

    // Phase 4b：自动绑定横幅标志（自动绑定单个候选者时由 openRepo/cloneRepo 设置；通过确认操作清除）
    lastAutoBindedPath: null,

    // Phase 4c：提交输入草稿（临时）
    commitMessage: '',

    // Phase 4c：自动保存错误显示（来自订户的最后一个错误）
    autosaveError: null,

    // Phase 4c：订阅者生命周期句柄（内部）
    __autosaveUnsub: null,

    // ---- Panel 生命周期 -------------------------------------------------
    togglePanel: () => set((s) => ({ panelOpen: !s.panelOpen })),
    openPanel: () => set({ panelOpen: true }),
    closePanel: () => set({ panelOpen: false }),

    // ---- Phase 4a：作者身份操作 ------------------------------
    loadAuthorIdentity: async () => {
      // The 解析链（首选项 → 系统 git → null）位于 git-store-helpers.ts
      // 中，以将此文件保持在 800-LoC 上限之下。
      const id = await resolveAuthorIdentity();
      set({ authorIdentity: id });
    },

    setAuthorIdentity: async (name, email) => {
      // 首先将 Persist 更改为 OpenPencil 首选项，以便面板在链的第 1 步中重新从它们重新水化。 If 首选项 IPC
      // 失败（例如浏览器模式），仍然更新内存缓存，以便当前会话正常工作。 SSR 防护：完全跳过 IPC（裸 `window` 将
      // ReferenceError），但仍更新内存中缓存。
      if (typeof window !== 'undefined') {
        try {
          await window.electronAPI?.setPreference('git.authorName', name);
          await window.electronAPI?.setPreference('git.authorEmail', email);
        } catch {
          /* 吞下去——下面的内存缓存仍然为会话提供服务 */
        }
      }
      set({ authorIdentity: { name, email } });
    },

    showAuthorPrompt: () => set({ authorPromptVisible: true }),
    hideAuthorPrompt: () => set({ authorPromptVisible: false }),

    // ---- Phase 4b：自动绑定横幅操作 ------------------------------
    acknowledgeAutoBind: () => set({ lastAutoBindedPath: null }),
    acknowledgeAutoBindAndOpen: async () => {
      const path = get().lastAutoBindedPath;
      if (!path) return;
      // Load 通过共享帮助程序将文件放入编辑器中。 Fire-and-forget——失败是无声的（助手返回
      // false，但横幅会清除，以避免烦人）。
      await loadOpFileFromPath(path);
      set({ lastAutoBindedPath: null });
    },

    // ---- Phase 4c：提交输入操作 ---------------------------------
    setCommitMessage: (text) => set({ commitMessage: text }),
    clearCommitMessage: () => set({ commitMessage: '' }),
    cancelSaveRequired: () => set((s) => ({ state: dropSaveRequired(s.state) })),

    // ---- Phase 4c：溢出菜单操作 --------------------------------
    enterTrackedFilePicker: () =>
      set((s) => {
        if (s.state.kind !== 'ready') return s;
        return { state: { kind: 'needs-tracked-file', repo: s.state.repo } };
      }),

    // ---- Phase 7b：退出跟踪文件选择器 -------------------------
    exitTrackedFilePicker: async () => {
      const state = get().state;
      if (state.kind !== 'needs-tracked-file') return;
      if (state.repo.trackedFilePath !== null) {
        // Entered 从就绪状态（重新绑定）：回到就绪状态。
        set({ state: { kind: 'ready', repo: state.repo } });
      } else {
        // Entered 作为第一个 post-open/clone 屏幕：关闭临时会话并返回到无文件状态，以便呈现空状态。
        try {
          await gitClient.close(state.repo.repoId);
        } catch {
          // Best-effort：即使关闭失败，也会重置状态以避免过时的 UI。
        }
        set({ state: { kind: 'no-file' } });
      }
    },

    clearAuthorIdentity: async () => {
      // Remove 首先是 OpenPencil 首选项，因此重新加载不会再水化。 We 必须 REMOVE 键（而不是
      // `setPreference(..., '')`），否则 resolveAuthorIdentity
      // 中的查找链将在磁盘上看到空字符串哨兵，并将它们视为设置但空白而不是不存在 - 偏离记录的“清除缓存 AND
      // 删除两个首选项键”合同。
      if (typeof window !== 'undefined') {
        try {
          await window.electronAPI?.removePreference('git.authorName');
          await window.electronAPI?.removePreference('git.authorEmail');
        } catch {
          /* 吞下 - 内存中清除在本次会议中仍然获胜 */
        }
      }
      set({ authorIdentity: null });
    },

    // ---- Phase 4c：自动保存订阅者生命周期 ------------------------
    initAutosaveSubscriber: () => {
      // Idempotent：如果已经连线，则返回。
      if (get().__autosaveUnsub !== null) return;
      const handler = makeAutosaveHandler(get, set);
      const unsub = documentEvents.on('saved', handler);
      set({ __autosaveUnsub: unsub });
    },

    disposeAutosaveSubscriber: () => {
      const unsub = get().__autosaveUnsub;
      if (unsub) {
        unsub();
        set({ __autosaveUnsub: null });
      }
    },

    clearAutosaveError: () => set({ autosaveError: null }),

    // ---- Repo 发现/创建 --------------------------------------
    detectRepo: async (filePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const result = await gitClient.detect(filePath);
        if (result.mode === 'none') {
          set({ state: { kind: 'no-repo' } });
          return;
        }
        set({ state: { kind: 'ready', repo: metaFromOpenInfo(result) } });
        // Hydrate 通过轮询状态、分支和远程元数据来获取 metaFromOpenInfo
        // 中的占位符字段（currentBranch、分支、workingDirty、ahead/behind、远程）。如果后端报告状态，
        // Also 会协调正在进行的合并状态。
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      });
    },

    initRepo: async (filePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.init(filePath);
        set({ state: { kind: 'ready', repo: metaFromOpenInfo(info) } });
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      });
    },

    openRepo: async (repoPath, currentFilePath) => {
      set({ state: { kind: 'initializing' } });
      await runOrError(async () => {
        const info = await gitClient.open(repoPath, currentFilePath);

        // Phase 4b 自动绑定：如果存储库只有一个候选者，并且 open() 尚未设置
        // trackedFilePath，则立即绑定它并完全跳过选择器。 Surface
        // 自动绑定横幅，因此用户也可以根据需要将文件加载到编辑器中。
        if (info.trackedFilePath === null && info.candidates.length === 1) {
          const only = info.candidates[0];
          await gitClient.bindTrackedFile(info.repoId, only.path);
          set({
            state: {
              kind: 'ready',
              repo: { ...metaFromOpenInfo(info), trackedFilePath: only.path },
            },
            lastAutoBindedPath: only.path,
          });
          await get().refreshStatus();
          await get().refreshBranches();
          await get().refreshRemote();
          return;
        }

        const meta = metaFromOpenInfo(info);
        if (info.trackedFilePath === null) {
          set({ state: { kind: 'needs-tracked-file', repo: meta } });
        } else {
          set({ state: { kind: 'ready', repo: meta } });
        }
        // refreshStatus + refreshBranches 都在需求跟踪文件中工作（requireRepoId 接受它）。
        // They 甚至在用户选择跟踪文件之前就填充 currentBranch / 分支 /
        // 脏计数，因此选择器可以显示“main·3leading”标题信息。
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      });
    },

    cloneRepo: async (opts) => {
      // Phase 6a：向导启动的克隆捕获可恢复的内联错误
      // （因此表单保持其状态以供重试）； CLI 驱动的克隆治疗
      // 每个代码都是致命的。 classifyCloneError() 对该策略进行编码。
//
      // CRITICAL：从向导进入时，我们必须 NOT 转换到
      // `initializing` 飞行中 — 这将卸载 <GitPanelCloneForm>
      // 并在可恢复重试时擦除 URL/dest/token 输入。 Instead 我们
      // 留在 `wizard-clone` 并翻转 `busy` 标志，表格读取为
      // 加载指示器。
      const prevWasWizard = get().state.kind === 'wizard-clone';
      if (prevWasWizard) {
        set({ state: { kind: 'wizard-clone', busy: true, error: null } });
      } else {
        set({ state: { kind: 'initializing' } });
      }
      try {
        const info = await gitClient.clone(opts);

        // Phase 4b 自动绑定：单个候选人 → 就绪 + 横幅。 Multi / 零个候选者按照规范第 109
        // 行进入需求跟踪文件。Both 分支自然会离开向导，因此表单会干净地卸载。
        if (info.candidates.length === 1) {
          const only = info.candidates[0];
          await gitClient.bindTrackedFile(info.repoId, only.path);
          set({
            state: {
              kind: 'ready',
              repo: { ...metaFromOpenInfo(info), trackedFilePath: only.path },
            },
            lastAutoBindedPath: only.path,
          });
        } else {
          set({ state: { kind: 'needs-tracked-file', repo: metaFromOpenInfo(info) } });
        }
        await get().refreshStatus();
        await get().refreshBranches();
        await get().refreshRemote();
      } catch (err) {
        const decision = classifyCloneError(err, prevWasWizard);
        if (decision.kind === 'inline') {
          // Keep 向导已安装，表单状态完好；关闭忙碌并显示内嵌横幅，以便用户可以重试。
          set({
            state: {
              kind: 'wizard-clone',
              busy: false,
              error: { code: decision.code, message: decision.message },
            },
          });
          return;
        }
        set({
          state: {
            kind: 'error',
            message: decision.message,
            recoverable: decision.recoverable,
          },
        });
        throw err;
      }
    },

    bindTrackedFile: async (filePath) => {
      const state = get().state;
      if (state.kind !== 'needs-tracked-file' && state.kind !== 'ready') {
        throw new GitError('no-file', 'No repo to bind tracked file to', {
          recoverable: false,
        });
      }
      const repoId = state.repo.repoId;
      await runOrError(async () => {
        await gitClient.bindTrackedFile(repoId, filePath);
        // Transition 需要跟踪文件 → 准备就绪。
        set((s) => {
          if (s.state.kind === 'needs-tracked-file') {
            return {
              state: {
                kind: 'ready',
                repo: { ...s.state.repo, trackedFilePath: filePath },
              },
            };
          }
          if (s.state.kind === 'ready') {
            return {
              state: { ...s.state, repo: { ...s.state.repo, trackedFilePath: filePath } },
            };
          }
          return s;
        });
        // After 绑定，status() 可以返回特定于文件的脏信息（后端的 engineStatus 使用
        // session.trackedFilePath 根据 autosave-ref blob 计算 workingDirty）。
        await get().refreshStatus();
      });
    },

    refreshCandidates: async () => {
      const repoId = requireRepoId(get().state);
      const candidates = await gitClient.listCandidates(repoId);
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return {
            state: { ...s.state, repo: { ...s.state.repo, candidateFiles: candidates } },
          };
        }
        return s;
      });
    },

    closeRepo: async () => {
      const state = get().state;
      // 持有 RepoMeta 的 Every 状态有一个活动的主进程会话 — 包括需求跟踪文件。所有这些上的 Calling close()
      // 都可以防止用户打开或克隆存储库，然后在绑定跟踪文件之前关闭面板时发生会话泄漏。
      if (
        state.kind === 'ready' ||
        state.kind === 'conflict' ||
        state.kind === 'needs-tracked-file'
      ) {
        try {
          await gitClient.close(state.repo.repoId);
        } catch {
          // Best-effort：即使关闭失败（例如后端已经清理了会话），我们仍然希望重置渲染器状态以避免过时的 UI。
          // Swallow 并继续。
        }
      }
      set({ state: { kind: 'no-file' }, log: [], lastAutoBindedPath: null });
    },

    // ---- Status / 日志 / 差异 --------------------------------------------
    refreshStatus: async () => {
      const repoId = requireRepoId(get().state);
      const status = await gitClient.status(repoId);

      // Step 1：将基本存储库字段复制到 RepoMeta 中。 Applies
      // 到持有存储库的所有状态（就绪/冲突/需求跟踪文件）。
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return {
            state: {
              ...s.state,
              repo: {
                ...s.state.repo,
                currentBranch: status.branch,
                workingDirty: status.workingDirty,
                otherFilesDirty: status.otherFilesDirty,
                otherFilesPaths: status.otherFilesPaths,
                ahead: status.ahead,
                behind: status.behind,
              },
            },
          };
        }
        return s;
      });

      // Step 2：协调冲突状态。 Phase 2c 的 engineStatus 填充
      // `mergeInProgress`、`conflicts` 和 (Phase 6b)
      // `unresolvedFiles`。 We 将所有三个镜像到渲染器状态机中，因此在合并过程中重新打开的面板会看到冲突视图
      // AND 和非 `.op` 文件横幅。
      const current = get().state;
      const unresolved = status.unresolvedFiles ?? [];
      const reopenedMidMerge = status.reopenedMidMerge ?? false;
      // I2：也进入面板重新打开降级模式的冲突状态 — 即使 unresolvedFiles 为空（跟踪的 .op 被过滤掉）且冲突为
      // null，mergeInProgress + reopenedMidMerge 也为 true。
      if (
        status.mergeInProgress &&
        (status.conflicts || unresolved.length > 0 || reopenedMidMerge)
      ) {
        // Backend 报告正在进行的合并。
        if (current.kind === 'conflict') {
          // Already 处于冲突状态：保留内存中分辨率和 finalizeError — .op
          // 冲突包在合并会话期间不会发生变化，并且用户的分辨率选择必须在 3 秒的轮询周期中幸存下来。当用户从外部解析 non-.op
          // 文件时，Only unresolvedFiles 可能会发生变化。
          set({
            state: {
              ...current,
              unresolvedFiles: unresolved,
              reopenedMidMerge,
            },
          });
        } else if (current.kind === 'ready') {
          // Promote 准备好 → 与新包冲突（新进入冲突状态）。
          set({
            state: buildConflictState(
              current.repo,
              status.conflicts ?? null,
              unresolved,
              null,
              reopenedMidMerge,
            ),
          });
        }
      } else if (!status.mergeInProgress && current.kind === 'conflict') {
        // Backend 表示没有进行中合并，但渲染器处于冲突状态 - 合并是在外部完成的（例如终端 git 或来自另一个窗口的
        // applyMerge）。 Transition 回到准备状态。
        set({ state: { kind: 'ready', repo: current.repo } });
      }
    },

    loadLog: async (opts) => {
      const repoId = requireRepoId(get().state);
      const commits = await gitClient.log(repoId, opts);
      set({ log: commits });
    },

    computeDiff: async (from, to) => {
      const repoId = requireRepoId(get().state);
      return gitClient.diff(repoId, from, to);
    },

    // ---- Commit / 恢复 / 提升（门控） -----------------------------
    commitMilestone: async (message, author) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.commit(repoId, { kind: 'milestone', message, author });
        // Phase 4c：刷新日志并在成功时清除草稿，以便历史列表显示新的提交并且输入为空。
        await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
        get().clearCommitMessage();
      }, 'commit milestone');
    },

    commitAutosave: async (message, author) => {
      const repoId = requireRepoId(get().state);
      // Autosave 不在按规范设置的 withCleanWorkingTree 中 — 自动保存订阅者 (Phase 4)
      // 运行 AFTER 成功保存，因此文档在构造上是干净的。
      await gitClient.commit(repoId, { kind: 'autosave', message, author });
    },

    restoreCommit: async (commitHash) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.restore(repoId, commitHash);
        // The IPC 覆盖磁盘上跟踪的 .op 文件。 Reload 将其放入文档存储中，以便内存中的文档与恢复的树相匹配 -
        // 否则下一个 Cmd+s / autosave 会将旧的内存内容写回磁盘，默默地撤消恢复。 HEAD
        // 本身在恢复过程中不会发生变化，因此日志不需要刷新。
        const state = get().state;
        if ((state.kind === 'ready' || state.kind === 'conflict') && state.repo.trackedFilePath) {
          await loadOpFileFromPath(state.repo.trackedFilePath);
        }
      }, 'restore');
    },

    promoteAutosave: async (autosaveHash, message, author) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.promote(repoId, autosaveHash, message, author);
        // Promote 在自动保存树中写入新的里程碑提交。 Reload 文档的原因与 restoreCommit 相同 -
        // 磁盘上的树可能与内存中的文档不同。
        const state = get().state;
        if ((state.kind === 'ready' || state.kind === 'conflict') && state.repo.trackedFilePath) {
          await loadOpFileFromPath(state.repo.trackedFilePath);
        }
        await get().loadLog({ ref: currentLogRef(get()), limit: 50 });
      }, 'promote autosave');
    },

    // ---- Branches -------------------------------------------------------
    refreshBranches: async () => {
      const repoId = requireRepoId(get().state);
      const branches = await gitClient.branchList(repoId);
      set((s) => {
        if (
          s.state.kind === 'ready' ||
          s.state.kind === 'conflict' ||
          s.state.kind === 'needs-tracked-file'
        ) {
          return { state: { ...s.state, repo: { ...s.state.repo, branches } } };
        }
        return s;
      });
    },

    createBranch: async (opts) => {
      const repoId = requireRepoId(get().state);
      await gitClient.branchCreate(repoId, opts);
      await get().refreshBranches();
    },

    switchBranch: async (name) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        await gitClient.branchSwitch(repoId, name);
        // HEAD 感动了。 syncAfterHeadMove 刷新 status/branches，将磁盘上的跟踪文件重新加载到文档存
        // 储中，并刷新当前活动分支的历史列表。
        await syncAfterHeadMove();
      }, 'switch branch');
    },

    deleteBranch: async (name, opts) => {
      const repoId = requireRepoId(get().state);
      await gitClient.branchDelete(repoId, name, opts);
      await get().refreshBranches();
    },

    mergeBranch: async (fromBranch) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        const result = await gitClient.branchMerge(repoId, fromBranch);
        if (result.result === 'conflict' && result.conflicts) {
          set((s) => {
            if (s.state.kind !== 'ready') return s;
            return { state: buildConflictState(s.state.repo, result.conflicts!, []) };
          });
          // Conflict 路径：状态完全水合 — 跳过同步级联。
          return;
        }
        if (result.result === 'conflict-non-op') {
          // I3：non-.op 冲突 — 合并正在进行，但引擎无法应用 .op 合并，因为非 `.op` 文件未解析。
          // refreshStatus 执行完整的 repo-meta 更新 AND 通过共享 mergeInProgress 分支促进与
          // unresolvedFiles 列表的就绪 → 冲突。
          await get().refreshStatus();
          return;
        }
        // Success 路径（快进、合并）：HEAD 已移动。 Delegate 级联到共享助手（有关详细信息，请参阅
        // switchBranch）。
        await syncAfterHeadMove();
      }, 'merge branch');
    },

    // ---- Merge 编排 --------------------------------------------
    resolveConflict: async (conflictId, choice) => {
      const state = get().state;
      if (state.kind !== 'conflict') {
        throw new GitError('engine-crash', 'resolveConflict called outside conflict state', {
          recoverable: false,
        });
      }
      await gitClient.resolveConflict(state.repo.repoId, conflictId, choice);
      // Update 具有记录分辨率的本地 Map。 Also 清除任何过时的 finalizeError，以便在用户修复另一个冲突后横幅
      // 不会显示旧错误。
      set((s) => {
        if (s.state.kind !== 'conflict') return s;
        const nodeConflicts = new Map(s.state.conflicts.nodeConflicts);
        const docFieldConflicts = new Map(s.state.conflicts.docFieldConflicts);
        if (nodeConflicts.has(conflictId)) {
          const c = nodeConflicts.get(conflictId)!;
          nodeConflicts.set(conflictId, { ...c, resolution: choice });
        } else if (docFieldConflicts.has(conflictId)) {
          const c = docFieldConflicts.get(conflictId)!;
          docFieldConflicts.set(conflictId, { ...c, resolution: choice });
        }
        return {
          state: {
            ...s.state,
            conflicts: { nodeConflicts, docFieldConflicts },
            finalizeError: null,
          },
        };
      });
    },

    applyMerge: async () => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        try {
          await gitClient.applyMerge(repoId);
        } catch (err) {
          // Phase 7b：`merge-still-conflicted` 在横幅上内联显示，而不是转换到通用错误卡。 The
          // 用户必须解决剩余冲突并重试 applyMerge。
          if (isGitError(err) && err.code === 'merge-still-conflicted') {
            set((s) => {
              if (s.state.kind === 'conflict') {
                return { state: { ...s.state, finalizeError: err.message } };
              }
              return s;
            });
            // Immediately 刷新状态，以便未解析的文件列表是最新的。
            await get().refreshStatus();
            return; // do NOT 重新抛出 — 横幅拥有错误显示
          }
          throw err;
        }
        // Phase 7c：成功（包括 noop：true）→转换冲突→准备并清除任何陈旧的 finalizeError，然后重新加载跟踪的.op
        // 文件并刷新历史日志。
        set((s) => {
          if (s.state.kind === 'conflict') {
            return { state: { kind: 'ready', repo: s.state.repo } };
          }
          return s;
        });
        // Reload 跟踪的文件和刷新日志。 reloadAfterApply 读取当前状态（现在“就绪”），以便它可以找到
        // trackedFilePath。
        await reloadAfterApply();
      }, 'apply merge');
    },

    abortMerge: async () => {
      const repoId = requireRepoId(get().state);
      await gitClient.abortMerge(repoId);
      set((s) => {
        if (s.state.kind === 'conflict') {
          return { state: { kind: 'ready', repo: s.state.repo } };
        }
        return s;
      });
    },

    // ---- Remote ---------------------------------------------------------
    fetchRemote: async (auth) => {
      const repoId = requireRepoId(get().state);
      await gitClient.fetch(repoId, auth);
      await get().refreshStatus();
    },

    pull: async (auth) => {
      const repoId = requireRepoId(get().state);
      await withCleanWorkingTree(async () => {
        const result = await gitClient.pull(repoId, auth);
        if (result.result === 'fast-forward' || result.result === 'merge') {
          // Clean 头部移动。 Delegate 级联 so pull 的行为类似于 switchBranch /
          // mergeBranch 成功路径 - 刷新状态 + 分支，重新加载跟踪的 .op 文件，刷新日志。
          await syncAfterHeadMove();
          return;
        }
        if (result.result === 'conflict') {
          // `.op` 冲突包。 Transition 就绪 → 与未解决的非操作文件冲突；手动分辨率 UI
          // 涵盖了这里的所有内容（登陆 Phase 7）。
          set((s) => {
            if (s.state.kind !== 'ready') return s;
            return { state: buildConflictState(s.state.repo, result.conflicts ?? null, []) };
          });
          return;
        }
        // 结果 === 'conflict-non-op'：合并正在进行中，但引擎无法应用 .op 合并，因为非 `.op`
        // 文件未解析。 refreshStatus 执行完整的 repo-meta 更新（分支/前面/后面/脏工作） AND
        // 通过共享的 mergeInProgress 分支促进就绪→与 unresolvedFiles 列表冲突 -
        // 不需要手动状态构建。
        await get().refreshStatus();
      }, 'pull');
    },

    push: async (auth) => {
      const repoId = requireRepoId(get().state);
      // Note：push IPC 当前在失败时抛出 GitError('push-rejected') 或
      // GitError('auth-failed')，而不是返回标记结果。 We 让那些人从这里逃脱（不是通过
      // runOrError），这样远程控制按钮就可以捕获并在 err.code 上分支：被拒绝的推送会打开“先拉”重试条；身份验证失败的
      // 推送将打开共享身份验证表单。 Anything else 作为正常抛出传播，按钮显示紧凑的内联错误。
      await withCleanWorkingTree(async () => {
        await gitClient.push(repoId, auth);
        // Success：刷新状态，使 ahead/behind 归零并且“无内容可推送”提示接管。 No 头部移动 → 没有
        // syncAfterHeadMove。
        await get().refreshStatus();
      }, 'push');
    },

    // ---- Auth -----------------------------------------------------------
    storeAuth: (host, creds) => gitClient.authStore(host, creds),
    getAuth: (host) => gitClient.authGet(host),
    clearAuth: (host) => gitClient.authClear(host),

    // ---- Phase 6a：克隆向导 + 远程元数据 -----------------------
    enterCloneWizard: () => set({ state: { kind: 'wizard-clone', busy: false, error: null } }),

    cancelCloneWizard: () => {
      // Always 进入无文件状态。 The git-panel.tsx 检测回购效果将在下一次渲染时立即从当前打开的文档路径重新水化正
      // 确的无回购/就绪状态。
      set({ state: { kind: 'no-file' } });
    },

    refreshRemote: async () => {
      const state = get().state;
      if (
        state.kind !== 'ready' &&
        state.kind !== 'conflict' &&
        state.kind !== 'needs-tracked-file'
      ) {
        return;
      }
      const remote = await gitClient.remoteGet(state.repo.repoId);
      set((s) => ({ state: patchRepoRemote(s.state, remote) }));
    },

    setRemoteUrl: async (url) => {
      const repoId = requireRepoId(get().state);
      // Normalize empty/whitespace-only 字符串为空，因此桌面端可以将空白输入视为“删除原点” -
      // 表单层不必强制。
      const normalized = url === null || url.trim() === '' ? null : url.trim();
      const remote = await gitClient.remoteSet(repoId, normalized);
      // Update 渲染器状态 IMMEDIATELY 来自 IPC 返回值。 Per 是 Phase 6a 合约，调用者 MUST
      // NOT 依靠后续 refreshRemote() 来查看新值。
      set((s) => ({ state: patchRepoRemote(s.state, remote) }));
    },

    // ---- SSH 键 -------------------------------------------------------
    refreshSshKeys: async () => {
      const keys = await gitClient.sshListKeys();
      set({ sshKeys: keys });
    },
    generateSshKey: async (opts) => {
      const key = await gitClient.sshGenerateKey(opts);
      await get().refreshSshKeys();
      return key;
    },
    importSshKey: async (opts) => {
      const key = await gitClient.sshImportKey(opts);
      await get().refreshSshKeys();
      return key;
    },
    deleteSshKey: async (keyId) => {
      await gitClient.sshDeleteKey(keyId);
      await get().refreshSshKeys();
    },

    // ---- Retry 排队操作 --------------------------------------------
    retrySaveRequired: async () => {
      const state = get().state;
      if (state.kind !== 'ready' && state.kind !== 'conflict') return;
      const pending = state.saveRequiredFor;
      if (!pending) return;
      // 首先通过文档存储 Save。
      const saved = await useDocumentStore.getState().save();
      if (!saved) return;
      // Clear 挂起标志，然后重新运行。
      set((s) => ({ state: dropSaveRequired(s.state) }));
      await pending.run();
    },
  };
});

// Test-仅帮助程序，用于在测试之间重置存储。
export function __resetGitStore(): void {
  useGitStore.setState({
    state: { kind: 'no-file' },
    panelOpen: false,
    log: [],
    sshKeys: [],
    authorIdentity: null,
    authorPromptVisible: false,
    lastAutoBindedPath: null,
    commitMessage: '',
    autosaveError: null,
    __autosaveUnsub: null,
  });
}
