/**
 * Simple file logger for the Electron main process.
 *
 * Writes to `{userData}/logs/main.log` with daily rotation.
 * Keeps the last 7 days of logs, auto-cleans on init.
 *
 * Usage:
 *   import { initLogger, log } from './logger'
 *   await initLogger()    // call once at startup
 *   log.info('message')
 *   log.error('message')
 *   log.warn('message')
 */

import { appendFile, readdir, unlink, mkdir, stat } from 'node:fs/promises';
import { join } from 'node:path';

let logDir = '';
let logFilePath = '';
let initialized = false;

const MAX_LOG_DAYS = 7;

function timestamp(): string {
  return new Date().toISOString();
}

function todayStamp(): string {
  return new Date().toISOString().slice(0, 10); // YYYY-MM-DD
}

async function writeLine(level: string, msg: string): Promise<void> {
  if (!initialized) return;
  const line = `${timestamp()} [${level}] ${msg}\n`;
  // Also forward to console for dev mode
  if (level === 'ERROR') {
    process.stderr.write(line);
  } else {
    process.stdout.write(line);
  }
  try {
    await appendFile(logFilePath, line, 'utf-8');
  } catch {
    // Disk full or permission error — silently drop
  }
}

async function cleanOldLogs(): Promise<void> {
  try {
    const files = await readdir(logDir);
    const cutoff = Date.now() - MAX_LOG_DAYS * 24 * 60 * 60 * 1000;
    for (const file of files) {
      if (!file.endsWith('.log')) continue;
      const filePath = join(logDir, file);
      try {
        const s = await stat(filePath);
        if (s.mtimeMs < cutoff) {
          await unlink(filePath);
        }
      } catch {
        // ignore individual file errors
      }
    }
  } catch {
    // ignore
  }
}

/**
 * Initialize the logger. Must be called after `app.getPath('userData')` is available.
 */
export async function initLogger(userDataPath: string): Promise<void> {
  logDir = join(userDataPath, 'logs');
  logFilePath = join(logDir, `main-${todayStamp()}.log`);
  try {
    await mkdir(logDir, { recursive: true });
  } catch {
    // ignore
  }
  initialized = true;
  await writeLine('INFO', '--- OpenPencil started ---');
  // Clean old logs in background
  cleanOldLogs();
}

/** Get the log directory path (for displaying to users). */
export function getLogDir(): string {
  return logDir;
}

export const log = {
  info: (msg: string) => {
    writeLine('INFO', msg);
  },
  warn: (msg: string) => {
    writeLine('WARN', msg);
  },
  error: (msg: string) => {
    writeLine('ERROR', msg);
  },
};
