/** Start/stop OpenPencil 应用程序来自 CLI。 */

import { spawn, fork, execSync } from 'node:child_process';
import { createServer } from 'node:net';
import { writeFile, unlink, mkdir } from 'node:fs/promises';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { existsSync } from 'node:fs';
import { getAppInfo } from './connection';

const IS_WIN = process.platform === 'win32';
const PORT_FILE_DIR = join(homedir(), '.openpencil');
const PORT_FILE_PATH = join(PORT_FILE_DIR, '.port');

function getFreePort(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer();
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address();
      if (addr && typeof addr === 'object') {
        const { port } = addr;
        server.close(() => resolve(port));
      } else {
        reject(new Error('Failed to get free port'));
      }
    });
    server.on('error', reject);
  });
}

async function waitForPortFile(timeoutMs = 15_000): Promise<{ port: number; pid: number }> {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const info = await getAppInfo();
    if (info) return { port: info.port, pid: info.pid };
    await new Promise((r) => setTimeout(r, 300));
  }
  throw new Error('Timeout waiting for OpenPencil to start');
}

/** Find 安装的桌面应用程序二进制文件。 */
function findDesktopBinary(): string | null {
  const candidates: string[] = [];

  if (process.platform === 'darwin') {
    candidates.push('/Applications/OpenPencil.app/Contents/MacOS/OpenPencil');
    candidates.push(
      join(homedir(), 'Applications', 'OpenPencil.app', 'Contents', 'MacOS', 'OpenPencil'),
    );
  } else if (process.platform === 'win32') {
    const localAppData = process.env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local');
    const programFiles = process.env.PROGRAMFILES ?? 'C:\\Program Files';
    const programFilesX86 = process.env['PROGRAMFILES(X86)'] ?? 'C:\\Program Files (x86)';
    // NSIS 每用户安装（默认）
    candidates.push(join(localAppData, 'Programs', 'openpencil', 'OpenPencil.exe'));
    // NSIS 每台机器安装
    candidates.push(join(programFiles, 'OpenPencil', 'OpenPencil.exe'));
    candidates.push(join(programFilesX86, 'OpenPencil', 'OpenPencil.exe'));
    // Portable — 与 CLI 相同的目录
    candidates.push(join(__dirname, '..', 'OpenPencil.exe'));
  } else {
    // Linux — AppImage、deb、snap、flatpak、手册
    candidates.push('/usr/bin/openpencil');
    candidates.push('/usr/local/bin/openpencil');
    candidates.push(join(homedir(), '.local', 'bin', 'openpencil'));
    // AppImage 位于常见下载位置
    const appImageDirs = [
      join(homedir(), 'Applications'),
      join(homedir(), 'Downloads'),
      join(homedir(), '.local', 'share', 'applications'),
    ];
    for (const dir of appImageDirs) {
      // Match OpenPencil*.AppImage（版本可能有所不同）
      try {
        if (existsSync(dir)) {
          const files = require('node:fs').readdirSync(dir) as string[];
          const appImage = files.find(
            (f: string) => f.startsWith('OpenPencil') && f.endsWith('.AppImage'),
          );
          if (appImage) candidates.push(join(dir, appImage));
        }
      } catch {
        /* 跳过 */
      }
    }
    // Snap
    candidates.push('/snap/bin/openpencil');
    // Flatpak
    candidates.push('/var/lib/flatpak/exports/bin/dev.openpencil.app');
    candidates.push(
      join(homedir(), '.local', 'share', 'flatpak', 'exports', 'bin', 'dev.openpencil.app'),
    );
  }

  for (const path of candidates) {
    if (existsSync(path)) return path;
  }
  return null;
}

/** Find 相对于 CLI 位置的 Nitro 服务器条目。 */
function findServerEntry(): string | null {
  // When 已编译，__dirname 指向 dist/ Server 位于
  // ../../out/web/server/index.mjs 或相对于 monorepo 根
  const candidates = [
    join(__dirname, '..', '..', '..', 'out', 'web', 'server', 'index.mjs'),
    join(__dirname, '..', '..', 'out', 'web', 'server', 'index.mjs'),
    join(__dirname, '..', 'server', 'index.mjs'), // 当捆绑在 Electron 资源中时
  ];
  for (const path of candidates) {
    if (existsSync(path)) return path;
  }
  return null;
}

export async function startDesktop(): Promise<{ port: number; pid: number }> {
  const info = await getAppInfo();
  if (info) return { port: info.port, pid: info.pid };

  const binary = findDesktopBinary();
  if (!binary) {
    throw new Error(
      'OpenPencil desktop app not found. Install it or use `op start --web` for the web server.',
    );
  }

  const child = spawn(binary, [], {
    detached: true,
    stdio: 'ignore',
    ...(IS_WIN ? { windowsHide: true, shell: false } : {}),
  });
  child.unref();

  return waitForPortFile();
}

export async function startWeb(): Promise<{ port: number; pid: number }> {
  const info = await getAppInfo();
  if (info) return { port: info.port, pid: info.pid };

  const entry = findServerEntry();
  if (!entry) {
    throw new Error(
      'Nitro server not found. Run `bun run build` first, or use `op start --desktop`.',
    );
  }

  const port = await getFreePort();

  const child = fork(entry, [], {
    detached: true,
    stdio: 'ignore',
    ...(IS_WIN ? { windowsHide: true } : {}),
    env: {
      ...process.env,
      HOST: '127.0.0.1',
      PORT: String(port),
      NITRO_HOST: '127.0.0.1',
      NITRO_PORT: String(port),
    },
  });
  child.unref();

  // Write 端口文件（Nitro 插件也会写入它，但尽早写入以便更快发现）
  await mkdir(PORT_FILE_DIR, { recursive: true });
  await writeFile(
    PORT_FILE_PATH,
    JSON.stringify({ port, pid: child.pid, timestamp: Date.now() }),
    'utf-8',
  );

  return { port, pid: child.pid! };
}

export async function stopApp(): Promise<boolean> {
  const info = await getAppInfo();
  if (!info) return false;

  try {
    if (IS_WIN) {
      // Windows：不支持 SIGTERM，使用 taskkill 正常关闭
      execSync(`taskkill /PID ${info.pid}`, { stdio: 'ignore' });
    } else {
      process.kill(info.pid, 'SIGTERM');
    }
  } catch {
    // already dead — try force kill on Windows
    if (IS_WIN) {
      try {
        execSync(`taskkill /F /PID ${info.pid}`, { stdio: 'ignore' });
      } catch {
        /* ignore */
      }
    }
  }

  try {
    await unlink(PORT_FILE_PATH);
  } catch {
    // ignore
  }

  return true;
}
