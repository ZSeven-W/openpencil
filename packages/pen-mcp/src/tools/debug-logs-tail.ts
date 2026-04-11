import { homedir } from 'node:os';
import { join } from 'node:path';
import { readdir } from 'node:fs/promises';
import { readLogTail } from '../utils/log-utils';

export interface DebugLogsTailParams {
  tailLines?: number;
  sinceMs?: number;
  grep?: string;
}

interface LogsTailResponse {
  source: 'server';
  path: string | null;
  totalLines: number;
  lines: string[];
}

// Override hook for tests.
let _logDirOverride: string | null = null;
export function __setLogDirForTests(dir: string | null): void {
  _logDirOverride = dir;
}

function logDir(): string {
  return _logDirOverride ?? join(homedir(), '.openpencil', 'logs');
}

function todayStamp(): string {
  return new Date().toISOString().slice(0, 10);
}

async function findLatestLog(): Promise<string | null> {
  const dir = logDir();
  let files: string[];
  try {
    files = await readdir(dir);
  } catch {
    return null;
  }
  // Prefer today's file, else fall back to the newest server-*.log within 7 days.
  const today = `server-${todayStamp()}.log`;
  if (files.includes(today)) return join(dir, today);

  const candidates = files
    .filter((f) => /^server-\d{4}-\d{2}-\d{2}\.log$/.test(f))
    .sort()
    .reverse();
  if (candidates.length === 0) return null;

  return join(dir, candidates[0]);
}

export async function handleLogsTail(params: DebugLogsTailParams): Promise<string> {
  const path = await findLatestLog();
  if (!path) {
    const resp: LogsTailResponse = {
      source: 'server',
      path: null,
      totalLines: 0,
      lines: [],
    };
    return JSON.stringify(resp, null, 2);
  }

  const lines = await readLogTail({
    path,
    tailLines: Math.min(params.tailLines ?? 100, 500),
    sinceMs: params.sinceMs,
    grep: params.grep,
  });

  const resp: LogsTailResponse = {
    source: 'server',
    path,
    totalLines: lines.length,
    lines,
  };
  return JSON.stringify(resp, null, 2);
}
