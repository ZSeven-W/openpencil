/** Output CLI 的格式化和进程退出帮助程序。 */

import { readFile } from 'node:fs/promises';

let prettyMode = false;

export function setPretty(v: boolean): void {
  prettyMode = v;
}

export function output(data: unknown): void {
  const json = prettyMode ? JSON.stringify(data, null, 2) : JSON.stringify(data);
  process.stdout.write(json + '\n');
}

export function outputSuccess(data: Record<string, unknown> = {}): void {
  output({ ok: true, ...data });
}

export function outputError(message: string, code = 1): never {
  process.stderr.write(JSON.stringify({ error: message }) + '\n');
  process.exit(code);
}

/** Read 当不是 TTY （管道输入）时，所有标准输入。 */
export async function readStdin(): Promise<string> {
  if (process.stdin.isTTY) return '';
  const chunks: Buffer[] = [];
  for await (const chunk of process.stdin) chunks.push(chunk as Buffer);
  return Buffer.concat(chunks).toString('utf-8');
}

/**
 * Resolve 一个
 * CLI 参数，可能是： - `@filepath` → 读取文件内容
 * - `-` → 从标准输入读取 - 原始字符串 → 按原样使用 -
 * 未定义 → 回退到标准输入（如果通过管道传输）
 *
 */
export async function resolveArg(arg: string | undefined): Promise<string> {
  if (arg === '-') {
    const stdin = await readStdin();
    if (!stdin.trim()) outputError('No data received from stdin.');
    return stdin.trim();
  }
  if (arg && arg.startsWith('@')) {
    const filePath = arg.slice(1);
    try {
      return (await readFile(filePath, 'utf-8')).trim();
    } catch {
      outputError(`Cannot read file: ${filePath}`);
    }
  }
  if (arg) return arg;
  // No explicit arg — try stdin if piped
  const stdin = await readStdin();
  if (stdin.trim()) return stdin.trim();
  outputError('No data provided. Pass as argument, @filepath, or pipe via stdin.');
}

/** Parse JSON from a CLI positional arg, @filepath, or stdin. */
export async function parseJsonArg(arg: string | undefined): Promise<unknown> {
  const raw = await resolveArg(arg);
  try {
    return JSON.parse(raw);
  } catch {
    outputError(`Invalid JSON: ${raw.slice(0, 200)}...`);
  }
}
