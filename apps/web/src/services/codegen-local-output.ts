import type { SaveCodegenFileInput } from '@/types/cloud';

export type LocalCodegenOutputGitStatus = CodegenOutputGitStatus;
export type LocalWriteCodegenOutputResult = WriteCodegenOutputResult;

export interface LocalCodegenOutputFile {
  path: string;
  content: string;
}

function toOutputFiles(files: SaveCodegenFileInput[]): LocalCodegenOutputFile[] {
  return files.map((file) => ({
    path: file.path,
    content: file.content,
  }));
}

function getCodegenApi(): CodegenAPI {
  if (typeof window === 'undefined' || !window.electronAPI?.codegen) {
    throw new Error('Codegen local output is not available outside the desktop app');
  }
  return window.electronAPI.codegen;
}

export function isCodegenLocalOutputAvailable(): boolean {
  return typeof window !== 'undefined' && !!window.electronAPI?.codegen;
}

export function selectCodegenOutputDirectory(): Promise<string | null> {
  return getCodegenApi().selectOutputDirectory();
}

export function writeCodegenOutputFiles(input: {
  rootDir: string;
  files: SaveCodegenFileInput[];
}): Promise<LocalWriteCodegenOutputResult> {
  return getCodegenApi().writeFiles({
    rootDir: input.rootDir,
    files: toOutputFiles(input.files),
  });
}

export function revealCodegenOutputPath(path: string): Promise<void> {
  return getCodegenApi().revealPath(path);
}

export function getCodegenOutputGitStatus(input: {
  rootDir: string;
  files: SaveCodegenFileInput[];
}): Promise<LocalCodegenOutputGitStatus> {
  return getCodegenApi().gitStatus({
    rootDir: input.rootDir,
    files: toOutputFiles(input.files),
  });
}

export function commitCodegenOutput(input: {
  rootDir: string;
  files: SaveCodegenFileInput[];
  message: string;
  author: { name: string; email: string };
}): Promise<{ hash: string; changedFiles: string[] }> {
  return getCodegenApi().gitCommit({
    rootDir: input.rootDir,
    files: toOutputFiles(input.files),
    message: input.message,
    author: input.author,
  });
}

export function pushCodegenOutput(rootDir: string): Promise<{ result: 'ok' }> {
  return getCodegenApi().gitPush({ rootDir });
}
