import { spawn } from 'node:child_process';
import { existsSync, writeFileSync, unlinkSync, readFileSync } from 'node:fs';
import { networkInterfaces } from 'node:os';
import { join, resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { tmpdir } from 'node:os';

// ESM 兼容 __dirname polyfill
const __filename = fileURLToPath(import.meta.url);
const __dirname = dirname(__filename);

// PID/Port 文件，用于在重新启动时跟踪分离的 MCP 服务器进程
const MCP_PID_FILE = join(tmpdir(), 'openpencil-mcp-server.pid');
const MCP_PORT_FILE = join(tmpdir(), 'openpencil-mcp-server.port');

/** Resolve 跨开发、Web 构建和 Electron 生产的 MCP 服务器脚本路径。 */
function resolveMcpServerScript(): string {
  // Electron 制作：extraResources
  const electronResources = process.env.ELECTRON_RESOURCES_PATH;
  if (electronResources) {
    const p = join(electronResources, 'mcp-server.cjs');
    if (existsSync(p)) return p;
  }
  // dev + web build (来自 cwd) — monorepo 输出到 out/ In dev
  // 模式，CWD 是 apps/web/ （由于开发脚本中的“cd apps/web && ...”），因此还要检查 ../../out/ 以到达
  // monorepo 根目录。
  for (const base of [
    resolve(process.cwd(), 'out', 'mcp-server.cjs'),
    resolve(process.cwd(), '..', '..', 'out', 'mcp-server.cjs'),
  ]) {
    if (existsSync(base)) return base;
  }
  // Fallback：相对于此文件（Nitro 捆绑输出）
  const fromFile = resolve(__dirname, '..', '..', '..', 'out', 'mcp-server.cjs');
  if (existsSync(fromFile)) return fromFile;
  return resolve(process.cwd(), 'out', 'mcp-server.cjs');
}

/** Get 第一个非内部 IPv4 地址 (LAN IP)。 */
export function getLocalIp(): string | null {
  const nets = networkInterfaces();
  for (const name of Object.keys(nets)) {
    for (const net of nets[name] ?? []) {
      if (net.family === 'IPv4' && !net.internal) {
        return net.address;
      }
    }
  }
  return null;
}

/** Check 如果具有给定 PID 的进程正在运行。 */
function isProcessRunning(pid: number): boolean {
  try {
    // Signal 0 检查存在性而不实际发送信号
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

/** Read 来自文件的 PID（如果存在且进程仍在运行）。 */
function getRunningPid(): { pid: number; port: number } | null {
  try {
    if (!existsSync(MCP_PID_FILE)) return null;
    const pid = parseInt(readFileSync(MCP_PID_FILE, 'utf-8').trim(), 10);
    if (isNaN(pid) || !isProcessRunning(pid)) {
      // Stale PID 文件 - 清理
      try {
        unlinkSync(MCP_PID_FILE);
      } catch {
        /* 忽略 */
      }
      try {
        unlinkSync(MCP_PORT_FILE);
      } catch {
        /* 忽略 */
      }
      return null;
    }
    const port = existsSync(MCP_PORT_FILE)
      ? parseInt(readFileSync(MCP_PORT_FILE, 'utf-8').trim(), 10)
      : 3100;
    return { pid, port: isNaN(port) ? 3100 : port };
  } catch {
    return null;
  }
}

export function getMcpServerStatus(): {
  running: boolean;
  port: number | null;
  localIp: string | null;
} {
  const info = getRunningPid();
  if (!info) {
    return { running: false, port: null, localIp: null };
  }
  return { running: true, port: info.port, localIp: getLocalIp() };
}

export function startMcpHttpServer(port: number): {
  running: boolean;
  port: number;
  localIp: string | null;
  error?: string;
} {
  // Check 如果已经运行
  const existing = getRunningPid();
  if (existing) {
    return { running: true, port: existing.port, localIp: getLocalIp() };
  }

  const serverScript = resolveMcpServerScript();

  try {
    // CRITICAL：Use 与 unref() 分离模式，因此 MCP 服务器可以生存
    // 独立于父 Nitro 进程。 This 阻止服务器
    // 死亡时：
    // 1. The UI 设置对话框已关闭
    // 2. The 用户与编辑器画布交互
    // 3. The Nitro 服务器重新启动或热重载
    // 4. The Electron 应用程序在窗口关闭时将 SIGTERM 发送到 Nitro
//
    // The 进程在自己的会话中运行并将其 PID 写入文件
// for later tracking and graceful shutdown.
    const child = spawn(process.execPath, [serverScript, '--http', '--port', String(port)], {
      stdio: ['ignore', 'ignore', 'ignore'],
      env: {
        ...process.env,
        // In Electron，process.execPath 是 Electron 二进制文件。
        // ELECTRON_RUN_AS_NODE 使其行为与普通 Node.js 相同。
        ELECTRON_RUN_AS_NODE: '1',
      },
      detached: true,
      windowsHide: true,
    });

    // Allow 父级独立于子级退出
    child.unref();

    // Write PID 归档以供稍后跟踪（短暂延迟后以确保启动）
    const childPid = child.pid;
    if (childPid) {
      setTimeout(() => {
        try {
          if (isProcessRunning(childPid)) {
            writeFileSync(MCP_PID_FILE, String(childPid), 'utf-8');
            writeFileSync(MCP_PORT_FILE, String(port), 'utf-8');
          }
        } catch {
          /* 忽略写入错误 */
        }
      }, 100);
    }

    return { running: true, port, localIp: getLocalIp() };
  } catch (err) {
    return {
      running: false,
      port,
      localIp: null,
      error: err instanceof Error ? err.message : String(err),
    };
  }
}

export function stopMcpHttpServer(): { running: false } {
  const info = getRunningPid();
  if (info) {
    try {
      // Use process.kill 跨平台且安全
      process.kill(info.pid, 'SIGTERM');
    } catch {
      /* 进程可能已经退出 */
    }
    // Clean up PID/Port 文件
    try {
      unlinkSync(MCP_PID_FILE);
    } catch {
      /* 忽略 */
    }
    try {
      unlinkSync(MCP_PORT_FILE);
    } catch {
      /* 忽略 */
    }
  }
  return { running: false };
}
