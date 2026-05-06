/**
 * Shared OpenC
 * ode 客户端管理器。 Reuses 端口 4096
 * 上的现有服务器；在随机端口上启动一个作为后备。 Tracks 生成了服务器，因此可以在进程退出时清理它们。
 */
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { homedir } from 'node:os';

const activeServers = new Set<{ close(): void }>();

// Clean up 在进程退出时生成 OpenCode 服务器
function cleanup() {
  for (const server of activeServers) {
    try {
      server.close();
    } catch {
      /* 忽略 */
    }
  }
  activeServers.clear();
}

process.on('beforeExit', cleanup);
process.on('SIGTERM', cleanup);
process.on('SIGINT', cleanup);

const isWindows = process.platform === 'win32';

/** Cached 解析的二进制路径 */
let _resolvedBinary: string | undefined | null = null;

/** Resolve opencode 二进制文件，带缓存。 */
function resolveOpencodeBinary(): string | undefined {
  if (_resolvedBinary !== null) return _resolvedBinary ?? undefined;

  // PATH 查找
  try {
    const cmd = isWindows ? 'where opencode 2>nul' : 'which opencode 2>/dev/null';
    const result = execSync(cmd, { encoding: 'utf-8', timeout: 5000 })
      .trim()
      .split(/\r?\n/)[0]
      ?.trim();
    if (result && existsSync(result)) {
      _resolvedBinary = result;
      return result;
    }
  } catch {
    /* 不在 PATH 上 */
  }

  // Common 安装位置
  const home = homedir();
  const candidates = isWindows
    ? [
        join(process.env.APPDATA || '', 'npm', 'opencode.cmd'),
        join(process.env.NVM_SYMLINK || '', 'opencode.cmd'),
        join(process.env.FNM_MULTISHELL_PATH || '', 'opencode.cmd'),
        join(home, 'scoop', 'shims', 'opencode.exe'),
      ]
    : [
        join(home, '.opencode', 'bin', 'opencode'),
        join(home, '.npm-global', 'bin', 'opencode'),
        '/usr/local/bin/opencode',
        '/opt/homebrew/bin/opencode',
        join(home, '.local', 'bin', 'opencode'),
      ];

  for (const c of candidates) {
    if (c && existsSync(c)) {
      _resolvedBinary = c;
      return c;
    }
  }

  _resolvedBinary = undefined;
  return undefined;
}

export async function getOpencodeClient(binaryPath?: string) {
  const { createOpencodeClient, createOpencode } = await import('../opencode/index');

  // Try 首先连接到现有服务器
  try {
    const client = createOpencodeClient();
    await client.config.providers(); // 探针
    return { client, server: undefined };
  } catch {
    // No running server — 在随机端口上启动一个临时服务器
    const resolvedPath = binaryPath ?? resolveOpencodeBinary();
    const timeout = isWindows ? 15_000 : 5000;
    const oc = await createOpencode({ port: 0, binaryPath: resolvedPath, timeout });
    activeServers.add(oc.server);
    return { client: oc.client, server: oc.server };
  }
}

export function releaseOpencodeServer(server: { close(): void } | undefined) {
  if (!server) return;
  try {
    server.close();
  } catch {
    /* 忽略 */
  }
  activeServers.delete(server);
}
