import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  Check,
  FolderOpen,
  GitCommitHorizontal,
  GitCompare,
  Save,
  Upload,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';
import { GitPanelErrorCard } from './git-panel/git-panel-error-card';
import type { Framework } from '@zseven-w/pen-types';
import type { SaveCodegenFileInput } from '@/types/cloud';
import {
  commitCodegenOutput,
  getCodegenOutputGitStatus,
  isCodegenLocalOutputAvailable,
  pushCodegenOutput,
  revealCodegenOutputPath,
  selectCodegenOutputDirectory,
  writeCodegenOutputFiles,
  type LocalCodegenOutputGitStatus,
  type LocalWriteCodegenOutputResult,
} from '@/services/codegen-local-output';

interface CodeLocalOutputActionsProps {
  framework: Framework;
  files: SaveCodegenFileInput[];
}

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  return `${(bytes / 1024).toFixed(1)} KB`;
}

function buildDefaultCommitMessage(framework: Framework): string {
  return `feat(codegen): add ${framework} generated code`;
}

export default function CodeLocalOutputActions({ framework, files }: CodeLocalOutputActionsProps) {
  const { t } = useTranslation();
  const [isAvailable] = useState(() => isCodegenLocalOutputAvailable());
  const [outputDir, setOutputDir] = useState('');
  const [isWriting, setIsWriting] = useState(false);
  const [isGitLoading, setIsGitLoading] = useState(false);
  const [isCommitting, setIsCommitting] = useState(false);
  const [isPushing, setIsPushing] = useState(false);
  const [writeResult, setWriteResult] = useState<LocalWriteCodegenOutputResult | null>(null);
  const [gitStatus, setGitStatus] = useState<LocalCodegenOutputGitStatus | null>(null);
  const [commitMessage, setCommitMessage] = useState(() => buildDefaultCommitMessage(framework));
  const [authorName, setAuthorName] = useState('');
  const [authorEmail, setAuthorEmail] = useState('');
  const [commitHash, setCommitHash] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [retryAction, setRetryAction] = useState<(() => void) | null>(null);
  const [pushDone, setPushDone] = useState(false);

  const fileCount = files.length;
  const totalBytes = useMemo(
    () => files.reduce((sum, file) => sum + new TextEncoder().encode(file.content).length, 0),
    [files],
  );

  useEffect(() => {
    setCommitMessage(buildDefaultCommitMessage(framework));
  }, [framework]);

  useEffect(() => {
    if (!isAvailable || typeof window === 'undefined') return;
    let mounted = true;
    window.electronAPI?.git
      ?.getSystemAuthor()
      .then((author) => {
        if (!mounted || !author) return;
        setAuthorName((current) => current || author.name);
        setAuthorEmail((current) => current || author.email);
      })
      .catch(() => {});
    return () => {
      mounted = false;
    };
  }, [isAvailable]);

  const refreshGitStatus = useCallback(
    async (rootDir: string) => {
      setIsGitLoading(true);
      try {
        const status = await getCodegenOutputGitStatus({ rootDir, files });
        setGitStatus(status);
      } finally {
        setIsGitLoading(false);
      }
    },
    [files],
  );

  const handleSelectDirectory = useCallback(async () => {
    setError(null);
    setRetryAction(null);
    const selected = await selectCodegenOutputDirectory();
    if (!selected) return;
    setOutputDir(selected);
    setWriteResult(null);
    setCommitHash('');
    setPushDone(false);
    await refreshGitStatus(selected);
  }, [refreshGitStatus]);

  const handleWrite = useCallback(async () => {
    setError(null);
    setRetryAction(null);
    setPushDone(false);
    let rootDir = outputDir;
    if (!rootDir) {
      const selected = await selectCodegenOutputDirectory();
      if (!selected) return;
      rootDir = selected;
      setOutputDir(selected);
    }

    setIsWriting(true);
    try {
      const result = await writeCodegenOutputFiles({ rootDir, files });
      setWriteResult(result);
      setCommitHash('');
      await refreshGitStatus(result.rootDir);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('codePanel.localOutput.errorWrite'));
      setRetryAction(() => handleWrite);
    } finally {
      setIsWriting(false);
    }
  }, [files, outputDir, refreshGitStatus]);

  const handleReveal = useCallback(() => {
    const target = writeResult?.rootDir || outputDir;
    if (!target) return;
    void revealCodegenOutputPath(target);
  }, [outputDir, writeResult]);

  const handleCommit = useCallback(async () => {
    const rootDir = writeResult?.rootDir || outputDir;
    if (!rootDir) return;
    if (!authorName.trim() || !authorEmail.trim()) {
      setError(t('codePanel.localOutput.errorAuthor'));
      setRetryAction(null);
      return;
    }

    setError(null);
    setRetryAction(null);
    setIsCommitting(true);
    try {
      const result = await commitCodegenOutput({
        rootDir,
        files,
        message: commitMessage.trim() || buildDefaultCommitMessage(framework),
        author: { name: authorName.trim(), email: authorEmail.trim() },
      });
      setCommitHash(result.hash);
      await refreshGitStatus(rootDir);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('codePanel.localOutput.errorCommit'));
      setRetryAction(() => handleCommit);
    } finally {
      setIsCommitting(false);
    }
  }, [
    authorEmail,
    authorName,
    commitMessage,
    files,
    framework,
    outputDir,
    refreshGitStatus,
    writeResult,
  ]);

  const handlePush = useCallback(async () => {
    const rootDir = writeResult?.rootDir || outputDir;
    if (!rootDir) return;

    setError(null);
    setRetryAction(null);
    setIsPushing(true);
    try {
      await pushCodegenOutput(rootDir);
      setPushDone(true);
      await refreshGitStatus(rootDir);
    } catch (err) {
      setError(err instanceof Error ? err.message : t('codePanel.localOutput.errorPush'));
      setRetryAction(() => handlePush);
    } finally {
      setIsPushing(false);
    }
  }, [outputDir, refreshGitStatus, writeResult]);

  if (!isAvailable || fileCount === 0) return null;

  const canCommit = gitStatus?.mode === 'repo' && gitStatus.changedFiles.length > 0;
  const hasDiff = gitStatus?.mode === 'repo' && gitStatus.diff.trim().length > 0;

  return (
    <div className="border-b border-border/50 bg-card px-3 py-2 text-[11px] shrink-0">
      <div className="flex items-center justify-between gap-2">
        <div className="min-w-0">
          <div className="flex items-center gap-1.5 text-xs font-medium text-foreground">
            <Save className="h-3.5 w-3.5 shrink-0" />
            {t('codePanel.localOutput.title')}
          </div>
          <div className="mt-0.5 truncate text-muted-foreground">
            {outputDir || t('codePanel.localOutput.chooseFolderHint')}
          </div>
        </div>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[11px]"
          onClick={handleSelectDirectory}
        >
          <FolderOpen className="mr-1 h-3 w-3" />
          {t('codePanel.localOutput.choose')}
        </Button>
      </div>

      <div className="mt-2 flex items-center gap-1">
        <Button
          variant="secondary"
          size="sm"
          className="h-7 flex-1 px-2 text-[11px]"
          disabled={isWriting}
          onClick={handleWrite}
        >
          <Save className="mr-1 h-3 w-3" />
          {isWriting ? t('codePanel.localOutput.writing') : t('codePanel.localOutput.writeLocal')}
        </Button>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 px-2 text-[11px]"
          disabled={!outputDir && !writeResult}
          onClick={handleReveal}
        >
          <FolderOpen className="h-3 w-3" />
        </Button>
      </div>

      <div className="mt-2 text-muted-foreground">
        {t('codePanel.localOutput.fileCount', {
          count: fileCount,
          size: formatBytes(totalBytes),
        })}
      </div>

      {writeResult && (
        <div className="mt-2 rounded-md border border-border bg-muted/25 px-2 py-1.5 text-muted-foreground">
          <div className="flex items-center gap-1.5 text-foreground">
            <Check className="h-3 w-3 text-green-500" />
            {t('codePanel.localOutput.wroteFiles', {
              count: writeResult.writtenFiles.length,
            })}
          </div>
          <div className="mt-1 truncate">
            {writeResult.writtenFiles
              .slice(0, 3)
              .map((file) => file.path)
              .join(', ')}
          </div>
        </div>
      )}

      {gitStatus?.mode === 'repo' && (
        <div className="mt-2 rounded-md border border-border bg-muted/25 p-2">
          <div className="flex items-center justify-between gap-2 text-muted-foreground">
            <span className="flex items-center gap-1.5">
              <GitCompare className="h-3 w-3" />
              {isGitLoading
                ? t('codePanel.localOutput.checkingGit')
                : t('codePanel.localOutput.gitChanges', {
                    branch: gitStatus.branch,
                    count: gitStatus.changedFiles.length,
                  })}
            </span>
            {commitHash && <span className="font-mono text-[10px]">{commitHash.slice(0, 7)}</span>}
          </div>
          {hasDiff && (
            <pre className="mt-2 max-h-28 overflow-auto rounded bg-background p-2 font-mono text-[10px] leading-relaxed text-foreground/75 whitespace-pre-wrap">
              {gitStatus.diff}
            </pre>
          )}
          <div className="mt-2 grid grid-cols-2 gap-1">
            <input
              className="h-7 rounded border border-border bg-background px-2 text-[11px] text-foreground outline-none"
              value={authorName}
              onChange={(event) => setAuthorName(event.target.value)}
              placeholder={t('codePanel.localOutput.authorName')}
            />
            <input
              className="h-7 rounded border border-border bg-background px-2 text-[11px] text-foreground outline-none"
              value={authorEmail}
              onChange={(event) => setAuthorEmail(event.target.value)}
              placeholder={t('codePanel.localOutput.authorEmail')}
            />
          </div>
          <input
            className="mt-1 h-7 w-full rounded border border-border bg-background px-2 text-[11px] text-foreground outline-none"
            value={commitMessage}
            onChange={(event) => setCommitMessage(event.target.value)}
            placeholder={t('codePanel.localOutput.commitMessage')}
          />
          <div className="mt-2 flex items-center gap-1">
            <Button
              variant="ghost"
              size="sm"
              className="h-7 flex-1 px-2 text-[11px]"
              disabled={!canCommit || isCommitting}
              onClick={handleCommit}
            >
              <GitCommitHorizontal className="mr-1 h-3 w-3" />
              {isCommitting ? t('codePanel.localOutput.committing') : t('codePanel.localOutput.commit')}
            </Button>
            <Button
              variant="ghost"
              size="sm"
              className={cn(
                'h-7 flex-1 px-2 text-[11px]',
                pushDone && 'text-green-500 hover:text-green-500',
              )}
              disabled={!gitStatus.hasRemote || isPushing}
              onClick={handlePush}
            >
              <Upload className="mr-1 h-3 w-3" />
              {isPushing
                ? t('codePanel.localOutput.pushing')
                : pushDone
                  ? t('codePanel.localOutput.pushed')
                  : t('codePanel.localOutput.push')}
            </Button>
          </div>
        </div>
      )}

      {gitStatus?.mode === 'none' && outputDir && (
        <div className="mt-2 rounded-md border border-border bg-muted/25 px-2 py-1.5 text-muted-foreground">
          {t('codePanel.localOutput.notGitRepository')}
        </div>
      )}

      {error && (
        <GitPanelErrorCard
          message={error}
          recoverable={!!retryAction}
          onRetry={retryAction ?? undefined}
          onDismiss={() => {
            setError(null);
            setRetryAction(null);
          }}
          className="mt-2 rounded-md border border-destructive/20 bg-destructive/8 p-3"
        />
      )}
    </div>
  );
}
