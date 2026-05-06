import { readFile } from 'node:fs/promises';

/** Pattern 用于检测调试日志输出中的敏感数据 */
export const SENSITIVE_LOG_PATTERN =
  /ANTHROPIC_API_KEY=|Authorization:\s*Bearer|api[_-]?key\s*[:=]/i;

/**
 * Read 删除了敏感行的
 * 调试日志文件的尾部。如果无法读取文件，则 Returns 未定义。 Historical
 *
 * 来源：apps/web/server/api/ai/chat.ts（第 53-63 行）。
 * This 现在是规范副本
 ； chat.ts 从这里导入。
 */
export async function readDebugTail(
  path: string | undefined,
  maxLines = 40,
): Promise<string[] | undefined> {
  if (!path) return undefined;
  try {
    const raw = await readFile(path, 'utf-8');
    const lines = raw.split('\n').filter((l) => l.trim().length > 0);
    const sanitized = lines.filter((l) => !SENSITIVE_LOG_PATTERN.test(l));
    return sanitized.slice(-maxLines);
  } catch {
    return undefined;
  }
}

export interface ReadLogTailOptions {
  path: string;
  /** Max 返回的行数（过滤后）。 Default 100。 */
  tailLines?: number;
  /**
   * Only 返回时间戳前缀
   * 比该纪元 ms 新的行。 Lines 预计以 ISO-8601 时间戳开始；始终包含不可解析的时间戳（失败打开）。
   *
   */
  sinceMs?: number;
  /** Optional 正则表达式应用于行正文（敏感编辑后）。 */
  grep?: string;
}

/**
 * Read 日志文件，编辑
 * 敏感行，可以选择按时间和正则表达式进行过滤，并返回最后 n 行。
 */
export async function readLogTail(opts: ReadLogTailOptions): Promise<string[]> {
  const tailLines = opts.tailLines ?? 100;
  let raw: string;
  try {
    raw = await readFile(opts.path, 'utf-8');
  } catch {
    return [];
  }
  const grepRe = opts.grep ? new RegExp(opts.grep) : null;

  const lines = raw.split('\n').filter((l) => l.trim().length > 0);

  const filtered: string[] = [];
  for (const line of lines) {
    if (SENSITIVE_LOG_PATTERN.test(line)) continue;
    if (opts.sinceMs !== undefined) {
      const match = line.match(/^(\d{4}-\d{2}-\d{2}T[\d:.]+Z)/);
      if (match) {
        const ts = Date.parse(match[1]);
        if (!Number.isNaN(ts) && ts < opts.sinceMs) continue;
      }
    }
    if (grepRe && !grepRe.test(line)) continue;
    filtered.push(line);
  }

  return filtered.slice(-tailLines);
}
