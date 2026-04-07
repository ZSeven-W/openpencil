import { readFile } from 'node:fs/promises';

/** Pattern for detecting sensitive data in debug log output */
export const SENSITIVE_LOG_PATTERN =
  /ANTHROPIC_API_KEY=|Authorization:\s*Bearer|api[_-]?key\s*[:=]/i;

/**
 * Read the tail of a debug log file with sensitive lines removed.
 * Returns undefined if the file cannot be read.
 *
 * Historical source: apps/web/server/api/ai/chat.ts (lines 53-63). This is
 * the canonical copy now; chat.ts imports from here.
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
  /** Max number of lines to return (after filtering). Default 100. */
  tailLines?: number;
  /**
   * Only return lines whose timestamp prefix is newer than this epoch ms.
   * Lines are expected to start with an ISO-8601 timestamp; unparseable
   * timestamps are always included (fail-open).
   */
  sinceMs?: number;
  /** Optional regex applied to line body (after sensitive redaction). */
  grep?: string;
}

/**
 * Read a log file, redact sensitive lines, optionally filter by time and
 * regex, and return the last N lines.
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
