// apps/web/src/components/panels/git-panel/git-panel-ssh-keys.tsx
//
// Phase 6c：SSH 溢出菜单的密钥管理器子视图。 Responsibilities:
//   - refreshSshKeys() 打开
//   - 列出键，当前远程主机的键浮动到顶部
//   - 生成密钥（主持人、评论）
//   - 通过 window.electronAPI.openFile()导入密钥，将 filePath 传递到 importSshKey
//   - 删除键（带内联确认）
//   - 复制公钥并显示短暂的成功提示
//   - github.com / gitlab.com 的提供商链接，其他通用复制指南
//   - 当 iso 引擎遇到 SSH-ish URL 时，SSH 传输门控横幅
//
// Local 状态机：
// 视图='列表'| '生成' | '导入' | '删除-确认'
// View 状态是该子视图的本地状态，因为它纯粹是
// 溢出弹出窗口，不需要在打开时生存。

import { ArrowLeft, Copy, ExternalLink, Loader2, Plus, Trash2, Upload } from 'lucide-react';
import { useEffect, useMemo, useRef, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import { isGitError } from '@/services/git-error';
import type { GitPublicSshKeyInfo } from '@/services/git-types';
import { useGitStore } from '@/stores/git-store';
import { getProviderSshSettingsUrl, isSshRemoteUrl } from './git-remote-utils';

const INPUT_CLASS =
  'h-8 w-full rounded-md border border-border/70 bg-card px-2.5 text-xs text-foreground transition-[border-color,box-shadow] placeholder:text-muted-foreground/60 focus:border-primary/50 focus:outline-none focus:ring-[3px] focus:ring-primary/10';

type SshKeysView = 'list' | 'generate' | 'import' | 'delete-confirm';

interface GitPanelSshKeysProps {
  onBack: () => void;
}

export function GitPanelSshKeys({ onBack }: GitPanelSshKeysProps) {
  const { t } = useTranslation();
  const state = useGitStore((s) => s.state);
  const sshKeys = useGitStore((s) => s.sshKeys);
  const refreshSshKeys = useGitStore((s) => s.refreshSshKeys);
  const generateSshKey = useGitStore((s) => s.generateSshKey);
  const importSshKey = useGitStore((s) => s.importSshKey);
  const deleteSshKey = useGitStore((s) => s.deleteSshKey);

  const repo = state.kind === 'ready' || state.kind === 'conflict' ? state.repo : null;
  const currentHost = repo?.remote?.host ?? null;
  const remoteUrl = repo?.remote?.url ?? null;
  const engineKind = repo?.engineKind ?? null;
  const isoSshBlocked = engineKind === 'iso' && isSshRemoteUrl(remoteUrl);

  const [view, setView] = useState<SshKeysView>('list');
  const [busy, setBusy] = useState(false);
  const [inlineError, setInlineError] = useState<string | null>(null);
  const [copiedKeyId, setCopiedKeyId] = useState<string | null>(null);
  // Track 挂起的“复制”闪存计时器，以便可以在第二个副本上清除它（使闪存去抖动），并且最重要的是，在卸载时清除 -
  // 否则在弹出窗口关闭后会触发超时，并且 setCopiedKeyId 在陈旧组件上运行。
  const copyTimerRef = useRef<number | null>(null);

  // Generate 表单字段
  const [genHost, setGenHost] = useState<string>(currentHost ?? '');
  const [genComment, setGenComment] = useState<string>('');

  // Import 表单字段
  const [importHost, setImportHost] = useState<string>(currentHost ?? '');
  const [importPath, setImportPath] = useState<string>('');

  // Delete 确认目标
  const [deleteTarget, setDeleteTarget] = useState<GitPublicSshKeyInfo | null>(null);

  // On mount：刷新键列表，以便子视图永远不会针对陈旧的缓存打开。 Fire-然后忘记；完成后，商店将填充 `sshKeys`。
  useEffect(() => {
    void refreshSshKeys();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  // Clean 在卸载时启动任何正在运行的“复制”闪存计时器，以便在子视图消失后它无法调用 setCopiedKeyId。
  useEffect(() => {
    return () => {
      if (copyTimerRef.current !== null) {
        window.clearTimeout(copyTimerRef.current);
        copyTimerRef.current = null;
      }
    };
  }, []);

  // Float 将当前远程主机绑定的键添加到列表顶部。
  const sortedKeys = useMemo(() => {
    if (!currentHost) return sshKeys;
    const matching: GitPublicSshKeyInfo[] = [];
    const others: GitPublicSshKeyInfo[] = [];
    for (const k of sshKeys) {
      if (k.host === currentHost) matching.push(k);
      else others.push(k);
    }
    return [...matching, ...others];
  }, [sshKeys, currentHost]);

  const resetForms = () => {
    setGenHost(currentHost ?? '');
    setGenComment('');
    setImportHost(currentHost ?? '');
    setImportPath('');
    setInlineError(null);
  };

  async function handleGenerate() {
    const host = genHost.trim();
    const comment = genComment.trim();
    if (!host) {
      setInlineError(t('git.ssh.validationHost'));
      return;
    }
    if (!comment) {
      setInlineError(t('git.ssh.validationComment'));
      return;
    }
    setInlineError(null);
    setBusy(true);
    try {
      await generateSshKey({ host, comment });
      setView('list');
      resetForms();
    } catch (err) {
      // Symmetric 与 git-panel-remote-settings：将 iso SSH
      // 传输门转换为其本地化提示，以便用户不会看到原始引擎级错误字符串。
      if (isGitError(err) && err.code === 'ssh-not-supported-iso') {
        setInlineError(t('git.ssh.isoUnsupported'));
      } else {
        setInlineError(extractMessage(err));
      }
    } finally {
      setBusy(false);
    }
  }

  async function handlePickImportPath() {
    if (typeof window === 'undefined' || !window.electronAPI) return;
    const picked = await window.electronAPI.openFile();
    if (picked === null) return;
    // openFile() 返回 { filePath, 内容 }; importSshKey 采用路径，以便桌面端可以在带外复制 +
    // chmod 文件。
    setImportPath(picked.filePath);
  }

  async function handleImport() {
    const host = importHost.trim();
    const path = importPath.trim();
    if (!host) {
      setInlineError(t('git.ssh.validationHost'));
      return;
    }
    if (!path) {
      setInlineError(t('git.ssh.validationImportPath'));
      return;
    }
    setInlineError(null);
    setBusy(true);
    try {
      await importSshKey({ privateKeyPath: path, host });
      setView('list');
      resetForms();
    } catch (err) {
      if (isGitError(err) && err.code === 'ssh-not-supported-iso') {
        setInlineError(t('git.ssh.isoUnsupported'));
      } else {
        setInlineError(extractMessage(err));
      }
    } finally {
      setBusy(false);
    }
  }

  async function handleConfirmDelete() {
    if (!deleteTarget) return;
    setInlineError(null);
    setBusy(true);
    try {
      await deleteSshKey(deleteTarget.id);
      setDeleteTarget(null);
      setView('list');
    } catch (err) {
      setInlineError(extractMessage(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleCopyPublicKey(key: GitPublicSshKeyInfo) {
    // Feature-gate：navigator.clipboard 在非安全上下文中未定义（http://, file://，某些
    // jsdom 测试环境）。 Blindly 读取 `.writeText` 会抛出原始 TypeError 并带有令人困惑的消息。
    if (typeof navigator === 'undefined' || !navigator.clipboard?.writeText) {
      setInlineError(t('git.ssh.copyUnsupported'));
      return;
    }
    try {
      await navigator.clipboard.writeText(key.publicKey);
      setCopiedKeyId(key.id);
      // Clear 稍后复制提示，因此稍后的副本仍然闪烁。 Cancel 任何先前挂起的清除如此快速的连续副本在不同的行上不会相互竞争，并存储
      // 句柄，以便在子视图首先关闭时卸载效果可以取消它。
      if (copyTimerRef.current !== null) {
        window.clearTimeout(copyTimerRef.current);
      }
      copyTimerRef.current = window.setTimeout(() => {
        copyTimerRef.current = null;
        setCopiedKeyId((prev) => (prev === key.id ? null : prev));
      }, 1600);
    } catch (err) {
      setInlineError(extractMessage(err));
    }
  }

  if (!repo) return null;

  return (
    <div className="flex flex-col p-1" role="group" aria-label={t('git.ssh.label')}>
      <div className="flex items-center justify-between gap-2 px-1 pb-1.5 pt-0.5">
        <button
          type="button"
          onClick={() => {
            if (view === 'list') {
              onBack();
              return;
            }
            setView('list');
            resetForms();
            setDeleteTarget(null);
          }}
          aria-label={t('git.ssh.back')}
          className="flex items-center gap-1 rounded-md px-1.5 py-1 text-[11px] text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground"
        >
          <ArrowLeft size={12} strokeWidth={1.75} aria-hidden />
          {t('git.ssh.back')}
        </button>
      </div>

      <p className="px-2 pb-1 pt-0.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground/80">
        {t('git.ssh.heading')}
      </p>

      {isoSshBlocked && (
        <div
          role="note"
          className="mx-2 mb-2 rounded-md border border-border/60 bg-muted/40 px-2.5 py-2 text-[11px] text-muted-foreground"
        >
          {t('git.ssh.isoUnsupported')}
        </div>
      )}

      {view === 'list' && (
        <ListView
          keys={sortedKeys}
          currentHost={currentHost}
          copiedKeyId={copiedKeyId}
          onGenerate={() => {
            resetForms();
            setView('generate');
          }}
          onImport={() => {
            resetForms();
            setView('import');
          }}
          onCopy={(k) => void handleCopyPublicKey(k)}
          onDelete={(k) => {
            setDeleteTarget(k);
            setView('delete-confirm');
            setInlineError(null);
          }}
        />
      )}

      {view === 'generate' && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.hostLabel')}</span>
            <input
              type="text"
              value={genHost}
              onChange={(e) => setGenHost(e.target.value)}
              placeholder={t('git.ssh.hostPlaceholder')}
              aria-label={t('git.ssh.hostLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.commentLabel')}</span>
            <input
              type="text"
              value={genComment}
              onChange={(e) => setGenComment(e.target.value)}
              placeholder={t('git.ssh.commentPlaceholder')}
              aria-label={t('git.ssh.commentLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setView('list');
                resetForms();
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button type="button" size="sm" disabled={busy} onClick={() => void handleGenerate()}>
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.generateSubmit')}
            </Button>
          </div>
        </div>
      )}

      {view === 'import' && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <label className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">{t('git.ssh.hostLabel')}</span>
            <input
              type="text"
              value={importHost}
              onChange={(e) => setImportHost(e.target.value)}
              placeholder={t('git.ssh.hostPlaceholder')}
              aria-label={t('git.ssh.hostLabel')}
              className={INPUT_CLASS}
              autoComplete="off"
              spellCheck={false}
            />
          </label>
          <div className="flex flex-col gap-1">
            <span className="text-[11px] text-muted-foreground">
              {t('git.ssh.importPathLabel')}
            </span>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={importPath}
                onChange={(e) => setImportPath(e.target.value)}
                placeholder={t('git.ssh.importPathPlaceholder')}
                aria-label={t('git.ssh.importPathLabel')}
                className={INPUT_CLASS}
                autoComplete="off"
                spellCheck={false}
              />
              <Button
                type="button"
                variant="ghost"
                size="sm"
                disabled={busy}
                onClick={() => void handlePickImportPath()}
              >
                {t('git.ssh.importBrowse')}
              </Button>
            </div>
          </div>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setView('list');
                resetForms();
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button type="button" size="sm" disabled={busy} onClick={() => void handleImport()}>
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.importSubmit')}
            </Button>
          </div>
        </div>
      )}

      {view === 'delete-confirm' && deleteTarget && (
        <div className="flex flex-col gap-2 px-2 py-2">
          <p className="text-xs text-foreground">
            {t('git.ssh.deletePrompt', {
              name: deleteTarget.comment || deleteTarget.fingerprint,
            })}
          </p>
          <div className="flex items-center justify-end gap-2">
            <Button
              type="button"
              variant="ghost"
              size="sm"
              disabled={busy}
              onClick={() => {
                setDeleteTarget(null);
                setView('list');
              }}
            >
              {t('git.ssh.cancel')}
            </Button>
            <Button
              type="button"
              variant="destructive"
              size="sm"
              disabled={busy}
              onClick={() => void handleConfirmDelete()}
            >
              {busy && <Loader2 size={12} className="mr-1 animate-spin" aria-hidden />}
              {t('git.ssh.deleteConfirm')}
            </Button>
          </div>
        </div>
      )}

      {inlineError && (
        <div
          role="alert"
          className="mx-2 mb-2 rounded border border-destructive/40 bg-destructive/10 px-2 py-1.5 text-[11px] text-destructive"
        >
          {inlineError}
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// List 子视图 — 提取，以便键行映射保持可读，无需
// 父级中有条件的巨型级联。
// ---------------------------------------------------------------------------

function ListView({
  keys,
  currentHost,
  copiedKeyId,
  onGenerate,
  onImport,
  onCopy,
  onDelete,
}: {
  keys: GitPublicSshKeyInfo[];
  currentHost: string | null;
  copiedKeyId: string | null;
  onGenerate: () => void;
  onImport: () => void;
  onCopy: (k: GitPublicSshKeyInfo) => void;
  onDelete: (k: GitPublicSshKeyInfo) => void;
}) {
  const { t } = useTranslation();
  const providerUrl = getProviderSshSettingsUrl(currentHost);

  if (keys.length === 0) {
    return (
      <div className="flex flex-col gap-2 px-2 py-2" data-testid="ssh-keys-empty">
        <p className="text-[11px] text-muted-foreground">{t('git.ssh.emptyList')}</p>
        <div className="flex items-center justify-end gap-2">
          <Button type="button" variant="ghost" size="sm" onClick={onImport}>
            <Upload size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
            {t('git.ssh.importAction')}
          </Button>
          <Button type="button" size="sm" onClick={onGenerate}>
            <Plus size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
            {t('git.ssh.generateAction')}
          </Button>
        </div>
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-1 px-1 py-1">
      {keys.map((k) => {
        const isForCurrent = currentHost !== null && k.host === currentHost;
        return (
          <div
            key={k.id}
            data-testid={`ssh-key-row-${k.id}`}
            className="flex flex-col gap-1 rounded-sm border border-transparent px-2 py-1.5 hover:border-border"
          >
            <div className="flex items-center justify-between gap-2">
              <div className="flex min-w-0 flex-col">
                <span className="truncate text-xs text-foreground">
                  {k.comment || k.fingerprint}
                </span>
                <span className="truncate text-[10px] text-muted-foreground">
                  {k.host}
                  {isForCurrent && ` · ${t('git.ssh.currentHostBadge')}`}
                </span>
              </div>
              <div className="flex items-center gap-1">
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t('git.ssh.copyPublicKey')}
                  onClick={() => onCopy(k)}
                >
                  <Copy size={12} strokeWidth={1.5} aria-hidden />
                </Button>
                <Button
                  type="button"
                  variant="ghost"
                  size="icon-sm"
                  aria-label={t('git.ssh.deleteKey', { name: k.comment || k.fingerprint })}
                  onClick={() => onDelete(k)}
                >
                  <Trash2 size={12} strokeWidth={1.5} aria-hidden />
                </Button>
              </div>
            </div>
            {copiedKeyId === k.id && (
              <span className="text-[10px] text-muted-foreground" role="status" aria-live="polite">
                {t('git.ssh.copiedHint')}
              </span>
            )}
          </div>
        );
      })}

      <Separator className="my-1" />

      {providerUrl ? (
        <a
          href={providerUrl}
          target="_blank"
          rel="noopener noreferrer"
          data-testid="ssh-provider-link"
          className="mx-1 flex items-center gap-1 rounded-sm px-2 py-1 text-[11px] text-muted-foreground hover:bg-accent"
        >
          <ExternalLink size={12} strokeWidth={1.5} aria-hidden />
          {t('git.ssh.providerLink', { host: currentHost ?? '' })}
        </a>
      ) : (
        <p className="mx-2 py-1 text-[11px] text-muted-foreground">
          {t('git.ssh.genericGuidance')}
        </p>
      )}

      <div className="flex items-center justify-end gap-2 px-2 py-1">
        <Button type="button" variant="ghost" size="sm" onClick={onImport}>
          <Upload size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
          {t('git.ssh.importAction')}
        </Button>
        <Button type="button" size="sm" onClick={onGenerate}>
          <Plus size={12} strokeWidth={1.5} className="mr-1" aria-hidden />
          {t('git.ssh.generateAction')}
        </Button>
      </div>
    </div>
  );
}

function extractMessage(err: unknown): string {
  if (isGitError(err)) return err.message;
  if (err instanceof Error) return err.message;
  return String(err);
}
