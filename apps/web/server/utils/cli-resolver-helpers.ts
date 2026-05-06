/**
 * Shared 帮助程序，
 * 用于跨内置代理提供程序解析本地 CLI 二进制文件。 macOS 上的 Electron 确实 NOT 继承了用户的登录 shell
 * PATH，因此，即使 CLI 在用户终端中工作，来自 Nitro 的裸 `which <cli>` 也经常会失败。 Common 安装位置如
 * ~/.bun/bin、~/.nvm/versions/node/<ver>/bin、~/Library/pnpm、~/.volta/bin、
 * ~/.asdf/shims、~/.local/share/mise/shims 对服务器不可见。 Two 原语在这里： -
 * posixUserBinDirs()：枚举标准用户本地/包管理器 bin 目录，以便解析器可以将它们扫描为具体候选者。 -
 *
 * probeViaLoginShell(binary)：向用户配置的 shell 询问二进制文件的解析路径，选择 nvm / pnpm /
 * Bun / mise /
 * asdf 垫片，而无需枚举每个可能的安装
 * 布局。 Everything 通过 serverLog 记录日志，因此 server-YYYY-MM-DD.log
 * 文件获取用于远程诊断的完整分辨率跟踪。
 *
 *
 *
 *
 */

import { execSync } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import { homedir, platform } from 'node:os';
import { join } from 'node:path';
import { serverLog } from './server-logger';

const isWindows = platform() === 'win32';

/** Enumerate 标准 macOS/Linux 用户本地安装目录。 */
export function posixUserBinDirs(): string[] {
  const home = homedir();
  const dirs = [
    join(home, '.bun', 'bin'),
    join(home, '.volta', 'bin'),
    join(home, '.local', 'bin'),
    join(home, '.local', 'share', 'mise', 'shims'),
    join(home, '.asdf', 'shims'),
    join(home, 'Library', 'pnpm'),
    join(home, '.pnpm-global', 'bin'),
    join(home, '.cargo', 'bin'),
    '/usr/local/bin',
    '/opt/homebrew/bin',
  ];

  // nvm：尽力枚举已安装的节点版本（只需 readdir）
  try {
    const nvmNodeRoot = join(home, '.nvm', 'versions', 'node');
    if (existsSync(nvmNodeRoot)) {
      for (const ver of readdirSync(nvmNodeRoot)) {
        dirs.push(join(nvmNodeRoot, ver, 'bin'));
      }
    }
  } catch {
    /* 尽最大努力 */
  }

  // 法奈米
  try {
    const fnmRoot = join(home, '.fnm', 'node-versions');
    if (existsSync(fnmRoot)) {
      for (const ver of readdirSync(fnmRoot)) {
        dirs.push(join(fnmRoot, ver, 'installation', 'bin'));
      }
    }
  } catch {
    /* 尽最大努力 */
  }

  return dirs;
}

/**
 * Standard
 * Fish 通常安装在 macOS/Linux 上的位置。 Homebrew Apple Silicon 使用
 * /opt/homebrew、Homebrew Intel + 大多数发行版运送到 /usr/local、MacPorts 使用
 /opt/local。
 */
const FISH_FALLBACK_PATHS = [
  '/opt/homebrew/bin/fish',
  '/usr/local/bin/fish',
  '/opt/local/bin/fish',
  '/usr/bin/fish',
];

/**
 * Probe 二进制文件解
 * 析路径的用户登录 shell。 Runs `<shell> -ilc 'command -v <binary>'`
 * 来获取用户的 rc + 配置文件，以便 nvm / pnpm / Bun / mise / asdf 垫片连线可见。 `prefix`
 * 仅用于日志行以匹配调用者的现有命名空间。 Supports zsh、bash 和 Fish — Fish 用户在
 *
 * `~/.config/fish/config.fish`（通常通过 nvm.fish / mise / bass）中配置他们的 PATH，zsh
 * 和 bash 都不会获取
 * 这些资源。 `command -v` 和 `2>/dev/null` 都可以在 Fish 3.x 中工作，因此跨 shell
 * 的调用是相同的。
 */
export function probeViaLoginShell(binary: string, prefix: string): string | undefined {
  if (isWindows) return undefined;

  const userShell = process.env.SHELL;
  const shells: string[] = [];

  // User 声明的 shell 获胜——源出其真实 rc/profile 的 shell。
  if (userShell && existsSync(userShell)) shells.push(userShell);

  // Fish 后备：许多用户运行 Fish，但 Electron 可能会在 SHELL 未设置的情况下启动（例如 Dock
  // 启动的应用程序继承清理环境）。 Without 这个，仅限鱼的 PATH 条目（nvm.fish / mise / bass）保持不可见。
  if (!shells.some((s) => s.endsWith('/fish'))) {
    for (const p of FISH_FALLBACK_PATHS) {
      if (existsSync(p)) {
        shells.push(p);
        break;
      }
    }
  }

  // zsh / bash 包罗万象 — 大多数 POSIX CLIs 至少通过其中一个连接他们的垫片，即使在 Fish-primary
  // 系统上也是如此。
  if (!shells.some((s) => s.endsWith('/zsh')) && existsSync('/bin/zsh')) {
    shells.push('/bin/zsh');
  }
  if (!shells.some((s) => s.endsWith('/bash')) && existsSync('/bin/bash')) {
    shells.push('/bin/bash');
  }

  for (const shell of shells) {
    try {
      const cmd = `${shell} -ilc 'command -v ${binary} 2>/dev/null' 2>/dev/null`;
      serverLog.info(`[${prefix}] login-shell probe: ${cmd}`);
      const raw = execSync(cmd, {
        encoding: 'utf-8',
        timeout: 6000,
        // Start from a minimal env — inheriting Electron's env can suppress
        // the login-shell side effects we rely on (nvm's __NVM_DIR etc).
        env: { HOME: homedir(), USER: process.env.USER ?? '' },
      }).trim();
      const path = raw.split(/\r?\n/).filter(Boolean).pop();
      if (path && existsSync(path)) {
        serverLog.info(`[${prefix}] login-shell probe hit via ${shell}: "${path}"`);
        return path;
      }
      if (path) {
        serverLog.info(
          `[${prefix}] login-shell (${shell}) returned "${path}" but file does not exist`,
        );
      }
    } catch (err) {
      serverLog.info(
        `[${prefix}] login-shell probe via ${shell} failed: ${err instanceof Error ? err.message : err}`,
      );
    }
  }
  return undefined;
}
