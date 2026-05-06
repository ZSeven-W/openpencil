/**
 * Nitro 插件 —
 * 在服务器启动时写入 ~/.openpencil/.port，以便 MCP 服务器可以发现正在运行的实例（开发服务器或 Electron）。
 *
 * In Electron 生产模式主进程也会写入此文件，但此插件确保开发服务器（`bun --bun run dev`）也可发现。
 *
 *
 */

import { writeFile, mkdir, unlink, readFile } from 'node:fs/promises';
import { randomUUID } from 'node:crypto';
import { join } from 'node:path';
import { homedir } from 'node:os';
const PORT_FILE_DIR = join(homedir(), '.openpencil');
const PORT_FILE_PATH = join(PORT_FILE_DIR, '.port');
const PORT_FILE_TOKEN = randomUUID();

function getOwnerPid(): number {
  return process.ppid > 1 ? process.ppid : process.pid;
}

async function writePortFile(port: number): Promise<void> {
  try {
    await mkdir(PORT_FILE_DIR, { recursive: true });
    await writeFile(
      PORT_FILE_PATH,
      JSON.stringify({
        port,
        pid: getOwnerPid(),
        writerPid: process.pid,
        token: PORT_FILE_TOKEN,
        timestamp: Date.now(),
      }),
      'utf-8',
    );
  } catch {
    // Non-关键 — MCP 同步将回退到文件 I/O
  }
}

async function cleanupPortFile(): Promise<void> {
  try {
    const raw = await readFile(PORT_FILE_PATH, 'utf-8');
    const current = JSON.parse(raw) as { token?: string };
    if (current.token !== PORT_FILE_TOKEN) return;
    await unlink(PORT_FILE_PATH);
  } catch {
    // Ignore（如果已删除）
  }
}

export default () => {
  const port = parseInt(process.env.PORT || '3000', 10);
  writePortFile(port);

  const cleanup = () => {
    cleanupPortFile();
  };
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
};
