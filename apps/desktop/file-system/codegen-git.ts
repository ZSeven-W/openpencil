import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { resolve } from 'node:path';

import {
  type CodegenOutputFile,
  toNativeRelativePath,
  validateRelativeOutputPath,
} from './codegen-output';

const execFileAsync = promisify(execFile);
const DEFAULT_TIMEOUT_MS = 60_000;

interface RunGitResult {
  stdout: string;
  stderr: string;
}

interface CodegenGitFilesInput {
  rootDir: string;
  files: CodegenOutputFile[];
}

export interface CodegenOutputGitStatusRepo {
  mode: 'repo';
  rootDir: string;
  repoRoot: string;
  branch: string;
  changedFiles: string[];
  diff: string;
  hasRemote: boolean;
}

export type CodegenOutputGitStatus =
  | CodegenOutputGitStatusRepo
  | { mode: 'none'; rootDir: string };

export interface CommitCodegenOutputInput extends CodegenGitFilesInput {
  message: string;
  author: { name: string; email: string };
}

export interface CommitCodegenOutputResult {
  hash: string;
  changedFiles: string[];
}

export interface PushCodegenOutputResult {
  result: 'ok';
}

async function runGit(
  cwd: string,
  args: string[],
  opts: { reject?: boolean; env?: Record<string, string> } = {},
): Promise<RunGitResult> {
  try {
    const { stdout, stderr } = await execFileAsync('git', args, {
      cwd,
      env: { ...process.env, ...opts.env },
      timeout: DEFAULT_TIMEOUT_MS,
      maxBuffer: 32 * 1024 * 1024,
    });
    return { stdout, stderr };
  } catch (err) {
    if (opts.reject === false) {
      const failure = err as NodeJS.ErrnoException & { stdout?: string; stderr?: string };
      return { stdout: failure.stdout ?? '', stderr: failure.stderr ?? failure.message };
    }
    const failure = err as NodeJS.ErrnoException & { stderr?: string };
    throw new Error(failure.stderr?.trim() || failure.message);
  }
}

async function getRepoRoot(rootDir: string): Promise<string | null> {
  const { stdout } = await runGit(rootDir, ['rev-parse', '--show-toplevel'], { reject: false });
  const repoRoot = stdout.trim();
  return repoRoot ? resolve(repoRoot) : null;
}

function normalizeFilePaths(files: CodegenOutputFile[]): string[] {
  const paths = files.map((file) => {
    const normalized = validateRelativeOutputPath(file.path);
    if (!normalized) throw new Error(`Invalid output file path: ${file.path}`);
    return normalized;
  });
  return Array.from(new Set(paths)).sort((a, b) => a.localeCompare(b));
}

async function getCurrentBranch(rootDir: string): Promise<string> {
  const { stdout } = await runGit(rootDir, ['branch', '--show-current']);
  return stdout.trim() || 'HEAD';
}

async function hasOrigin(rootDir: string): Promise<boolean> {
  const { stdout } = await runGit(rootDir, ['remote'], { reject: false });
  return stdout
    .split(/\r?\n/)
    .map((remote) => remote.trim())
    .includes('origin');
}

async function getChangedGeneratedFiles(rootDir: string, filePaths: string[]): Promise<string[]> {
  if (filePaths.length === 0) return [];

  const args = ['status', '--porcelain', '--', ...filePaths.map(toNativeRelativePath)];
  const { stdout } = await runGit(rootDir, args);
  const changed = stdout
    .split(/\r?\n/)
    .map((line) => line.trimEnd())
    .filter(Boolean)
    .map((line) => line.slice(3).replace(/\\/g, '/').replace(/^"|"$/g, ''));
  return Array.from(new Set(changed)).sort((a, b) => a.localeCompare(b));
}

async function getDiff(rootDir: string, filePaths: string[]): Promise<string> {
  if (filePaths.length === 0) return '';
  const nativePaths = filePaths.map(toNativeRelativePath);
  const [staged, untracked] = await Promise.all([
    runGit(rootDir, ['diff', '--cached', '--', ...nativePaths]),
    runGit(rootDir, ['ls-files', '--others', '--exclude-standard', '--', ...nativePaths]),
  ]);
  const untrackedPaths = untracked.stdout
    .split(/\r?\n/)
    .map((path) => path.trim())
    .filter(Boolean);

  if (untrackedPaths.length > 0) {
    await runGit(rootDir, [
      'add',
      '--intent-to-add',
      '--',
      ...untrackedPaths.map(toNativeRelativePath),
    ]);
  }

  try {
    const unstaged = await runGit(rootDir, ['diff', '--', ...nativePaths]);
    return [unstaged.stdout, staged.stdout].filter(Boolean).join('\n');
  } finally {
    if (untrackedPaths.length > 0) {
      await runGit(rootDir, ['reset', '--', ...untrackedPaths.map(toNativeRelativePath)], {
        reject: false,
      });
    }
  }
}

export async function getCodegenOutputGitStatus(
  input: CodegenGitFilesInput,
): Promise<CodegenOutputGitStatus> {
  const rootDir = resolve(input.rootDir);
  const repoRoot = await getRepoRoot(rootDir);
  if (!repoRoot) return { mode: 'none', rootDir };

  const filePaths = normalizeFilePaths(input.files);
  const [branch, changedFiles, diff, remote] = await Promise.all([
    getCurrentBranch(rootDir),
    getChangedGeneratedFiles(rootDir, filePaths),
    getDiff(rootDir, filePaths),
    hasOrigin(rootDir),
  ]);

  return {
    mode: 'repo',
    rootDir,
    repoRoot,
    branch,
    changedFiles,
    diff,
    hasRemote: remote,
  };
}

export async function commitCodegenOutput(
  input: CommitCodegenOutputInput,
): Promise<CommitCodegenOutputResult> {
  const rootDir = resolve(input.rootDir);
  const repoRoot = await getRepoRoot(rootDir);
  if (!repoRoot) throw new Error('Output directory is not inside a git repository');

  const filePaths = normalizeFilePaths(input.files);
  const changedFiles = await getChangedGeneratedFiles(rootDir, filePaths);
  if (changedFiles.length === 0) {
    throw new Error('No generated file changes to commit');
  }

  await runGit(rootDir, ['add', '--', ...changedFiles.map(toNativeRelativePath)]);
  await runGit(rootDir, ['commit', '-m', input.message], {
    env: {
      GIT_AUTHOR_NAME: input.author.name,
      GIT_AUTHOR_EMAIL: input.author.email,
      GIT_COMMITTER_NAME: input.author.name,
      GIT_COMMITTER_EMAIL: input.author.email,
    },
  });
  const { stdout } = await runGit(rootDir, ['rev-parse', 'HEAD']);
  return { hash: stdout.trim(), changedFiles };
}

export async function pushCodegenOutput(input: {
  rootDir: string;
}): Promise<PushCodegenOutputResult> {
  const rootDir = resolve(input.rootDir);
  const repoRoot = await getRepoRoot(rootDir);
  if (!repoRoot) throw new Error('Output directory is not inside a git repository');
  if (!(await hasOrigin(rootDir))) throw new Error('No git remote named origin is configured');

  await runGit(rootDir, ['push', 'origin', 'HEAD']);
  return { result: 'ok' };
}
