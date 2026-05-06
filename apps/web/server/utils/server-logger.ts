/**
 * Simple 服务器进程
 *
 * 的 Simple 文件记录器。 Writes 至 `~/.openpencil/logs/ser
 * ver-{YYYY-MM
 * -DD}.log`。 Also 转发到控制台以获得开发模式可见性。 Keeps 最近 7
 天的日志，首次写入时自动清除。
 */

import { appendFileSync, existsSync, mkdirSync, readdirSync, statSync, unlinkSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';

const MAX_LOG_DAYS = 7;

const logDir = join(homedir(), '.openpencil', 'logs');
let dirEnsured = false;
let cleanedUp = false;

function ensureDir(): void {
  if (dirEnsured) return;
  try {
    if (!existsSync(logDir)) {
      mkdirSync(logDir, { recursive: true });
    }
    dirEnsured = true;
  } catch {
    // Silently 失败 — 尽力记录
  }
}

function todayStamp(): string {
  return new Date().toISOString().slice(0, 10);
}

function timestamp(): string {
  return new Date().toISOString();
}

function getLogFilePath(): string {
  return join(logDir, `server-${todayStamp()}.log`);
}

function cleanOldLogs(): void {
  if (cleanedUp) return;
  cleanedUp = true;
  try {
    const files = readdirSync(logDir);
    const cutoff = Date.now() - MAX_LOG_DAYS * 24 * 60 * 60 * 1000;
    for (const file of files) {
      if (!file.endsWith('.log')) continue;
      const filePath = join(logDir, file);
      try {
        const s = statSync(filePath);
        if (s.mtimeMs < cutoff) {
          unlinkSync(filePath);
        }
      } catch {
        // ignore individual file errors
      }
    }
  } catch {
    // ignore
  }
}

function writeLine(level: string, msg: string): void {
  const line = `${timestamp()} [${level}] ${msg}\n`;
  // Forward to console
  if (level === 'ERROR') {
    process.stderr.write(line);
  } else {
    process.stdout.write(line);
  }
  // Write to file
  try {
    ensureDir();
    cleanOldLogs();
    appendFileSync(getLogFilePath(), line, 'utf-8');
  } catch {
    // Disk full or permission error — silently drop
  }
}

export const serverLog = {
  info: (msg: string) => writeLine('INFO', msg),
  warn: (msg: string) => writeLine('WARN', msg),
  error: (msg: string) => writeLine('ERROR', msg),
};
