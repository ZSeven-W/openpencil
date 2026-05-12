import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, relative, resolve, sep } from 'node:path';

export interface CodegenOutputFile {
  path: string;
  content: string;
}

export interface WriteCodegenOutputInput {
  rootDir: string;
  files: CodegenOutputFile[];
}

export interface WrittenCodegenOutputFile {
  path: string;
  absolutePath: string;
  bytes: number;
}

export interface WriteCodegenOutputResult {
  rootDir: string;
  writtenFiles: WrittenCodegenOutputFile[];
}

export function validateRelativeOutputPath(path: string): string | null {
  if (path.includes('\0')) return null;

  const normalized = path.trim().replace(/\\/g, '/').replace(/^\.\/+/, '');
  if (!normalized || normalized.startsWith('/')) return null;

  const segments = normalized.split('/');
  if (segments.some((segment) => !segment || segment === '.' || segment === '..')) {
    return null;
  }

  return normalized;
}

export function resolveOutputFilePath(rootDir: string, relativePath: string): string {
  const normalizedPath = validateRelativeOutputPath(relativePath);
  if (!normalizedPath) {
    throw new Error(`Invalid output file path: ${relativePath}`);
  }

  const resolvedRoot = resolve(rootDir);
  const absolutePath = resolve(resolvedRoot, normalizedPath);
  const rel = relative(resolvedRoot, absolutePath);
  if (rel === '' || rel.startsWith('..') || resolve(rel) === rel) {
    throw new Error(`Invalid output file path: ${relativePath}`);
  }

  return absolutePath;
}

export function toNativeRelativePath(path: string): string {
  return validateRelativeOutputPath(path)?.split('/').join(sep) ?? path;
}

export async function writeCodegenFilesToDirectory(
  input: WriteCodegenOutputInput,
): Promise<WriteCodegenOutputResult> {
  if (!input.rootDir || input.rootDir.includes('\0')) {
    throw new Error('Invalid output directory');
  }
  if (input.files.length === 0) {
    throw new Error('No generated files to write');
  }

  const rootDir = resolve(input.rootDir);
  await mkdir(rootDir, { recursive: true });

  const writtenFiles: WrittenCodegenOutputFile[] = [];
  for (const file of input.files) {
    const normalizedPath = validateRelativeOutputPath(file.path);
    if (!normalizedPath) {
      throw new Error(`Invalid output file path: ${file.path}`);
    }

    const absolutePath = resolveOutputFilePath(rootDir, normalizedPath);
    await mkdir(dirname(absolutePath), { recursive: true });
    await writeFile(absolutePath, file.content, 'utf-8');
    writtenFiles.push({
      path: normalizedPath,
      absolutePath,
      bytes: Buffer.byteLength(file.content, 'utf-8'),
    });
  }

  return { rootDir, writtenFiles };
}
