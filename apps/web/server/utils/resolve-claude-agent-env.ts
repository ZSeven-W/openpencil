import { spawn } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync, existsSync, appendFileSync } from 'node:fs';
import { homedir, tmpdir, platform } from 'node:os';
import { join } from 'node:path';

const IS_WIN = platform() === 'win32';

type EnvLike = Record<string, string | undefined>;

interface ClaudeSettings {
  env?: Record<string, unknown>;
}

function normalizeEnvValue(key: string, value: unknown): string | undefined {
  if (value == null) return undefined;
  if (typeof value === 'string') {
    // Filter 输出空字符串 - 它们会导致问题
    if (value.trim() === '') return undefined;
    return value;
  }
  if (typeof value === 'number' || typeof value === 'boolean') {
    return String(value);
  }
  // ANTHROPIC_CUSTOM_HEADERS 可以是 settings.json 中的对象 — 序列化它。跳过 Other
  // 对象值以防止“Invalid header name”错误。
  if (typeof value === 'object') {
    if (key === 'ANTHROPIC_CUSTOM_HEADERS') {
      try {
        return JSON.stringify(value);
      } catch {
        return undefined;
      }
    }
    return undefined;
  }
  return undefined;
}

function readSingleSettingsFile(filePath: string): EnvLike {
  try {
    const raw = readFileSync(filePath, 'utf-8');
    const parsed = JSON.parse(raw) as ClaudeSettings;
    if (!parsed.env || typeof parsed.env !== 'object') return {};

    const env: EnvLike = {};
    for (const [key, value] of Object.entries(parsed.env)) {
      const normalized = normalizeEnvValue(key, value);
      if (normalized !== undefined) {
        env[key] = normalized;
      }
    }
    return env;
  } catch {
    return {};
  }
}

/**
 * 来自 ~/.claude
 * /settings.json 和 ~/.claude/settings.local.json 的 Read 环境。 Local 设置优先（与
 Claude Code 自身的优先级相同）。
 */
function readClaudeSettingsEnv(): EnvLike {
  const claudeDir = join(homedir(), '.claude');
  const base = readSingleSettingsFile(join(claudeDir, 'settings.json'));
  const local = readSingleSettingsFile(join(claudeDir, 'settings.local.json'));
  return { ...base, ...local };
}

/**
 * Validate 如果字符串有效 JSON（对于 ANTHROPIC_CUSTOM_HEADERS）。
 */
function isValidJson(str: string): boolean {
  try {
    JSON.parse(str);
    return true;
  } catch {
    return false;
  }
}

/**
 * 写入 ~/.claude
 * .json 或 ~/.claude/ 配置文件时，On Windows、Claude Code SDK 可能会失败并出现 EPERM。 Ensure
 目录和配置文件存在并且可写。
 */
function ensureClaudeConfigWritable(): void {
  if (!IS_WIN) return;
  try {
    const claudeDir = join(homedir(), '.claude');
    mkdirSync(claudeDir, { recursive: true });
    // Ensure .claude.json 存在 — Claude SDK 如果不能 write/lock 就会崩溃
    const configFile = join(homedir(), '.claude.json');
    if (!existsSync(configFile)) {
      writeFileSync(configFile, '{}', 'utf-8');
    }
    // Ensure credentials.json 存在 — SDK 尝试 read/write 时可能会崩溃
    const credFile = join(claudeDir, 'credentials.json');
    if (!existsSync(credFile)) {
      writeFileSync(credFile, '{}', 'utf-8');
    }
    // Ensure statsig/ 缓存目录存在 — SDK 写入功能门缓存时崩溃
    const statsigDir = join(claudeDir, 'statsig');
    mkdirSync(statsigDir, { recursive: true });
  } catch {
    // Best 努力 — 如果我们无法修复它，SDK 错误提示将指导用户
  }
}

/**
 * Build env
 * 传递给 Claude Agent SDK。 Priority：当前进程环境
 > ~/.claude/settings.json 环境。
 */
export function buildClaudeAgentEnv(): EnvLike {
  // On Windows，预先创建配置文件以避免 EPERM 错误
  ensureClaudeConfigWritable();

  const fromSettings = readClaudeSettingsEnv();
  const fromProcess = process.env as EnvLike;

  const merged: EnvLike = {
    ...fromSettings,
    ...fromProcess,
  };

  // Validate ANTHROPIC_CUSTOM_HEADERS 如果存在 - 必须有效 JSON If
  // 无效，删除它以防止“Invalid header name”错误
  if (merged.ANTHROPIC_CUSTOM_HEADERS) {
    if (!isValidJson(merged.ANTHROPIC_CUSTOM_HEADERS)) {
      delete merged.ANTHROPIC_CUSTOM_HEADERS;
    }
  }

  // Compatibility：如果没有设置 API 键，则使用 ANTHROPIC_AUTH_TOKEN 作为 ANTHROPIC_API_KEY
  const authToken = merged.ANTHROPIC_AUTH_TOKEN;
  if (authToken && !merged.ANTHROPIC_API_KEY) {
    merged.ANTHROPIC_API_KEY = authToken;
  }

  // Claude 终端内的 Running 可以中断嵌套的 Claude 调用。
  delete merged.CLAUDECODE;

  // Remove Electron 特定的环境变量可能会混淆生成的 CLI 进程
  delete merged.ELECTRON_RUN_AS_NODE;
  delete merged.ELECTRON_RESOURCES_PATH;
  delete merged.CHROME_CRASHPAD_PIPE_NAME;

  // Enable Agent SDK 调试 stderr，以便我们可以捕获 CLI 崩溃诊断。 Without 这个，SDK 将 stderr
  // 设置为“忽略”并且崩溃输出丢失。
  if (!merged.DEBUG_CLAUDE_AGENT_SDK) {
    merged.DEBUG_CLAUDE_AGENT_SDK = '1';
  }

  if (IS_WIN) {
    // Redirect Claude 调试输出到临时以避免写权限问题
    if (!merged.CLAUDE_DEBUG_FILE) {
      const debugPath = getClaudeAgentDebugFilePath();
      if (debugPath) merged.CLAUDE_DEBUG_FILE = debugPath;
    }
    // Set CLAUDE_CONFIG_DIR 到可写临时位置作为后备
// if the default ~/.claude directory is not writable (common in Windows Electron)
    if (!merged.CLAUDE_CONFIG_DIR) {
      try {
        const fallbackDir = join(tmpdir(), 'openpencil-claude-config');
        mkdirSync(fallbackDir, { recursive: true });
        // Only 如果我们无法写入默认位置，则使用后备
        const defaultDir = join(homedir(), '.claude');
        const testFile = join(defaultDir, '.write-test');
        try {
          writeFileSync(testFile, '', 'utf-8');
          const { unlinkSync } = require('node:fs');
          unlinkSync(testFile);
        } catch {
          // Default 目录不可写 — 使用后备
          merged.CLAUDE_CONFIG_DIR = fallbackDir;
        }
      } catch {
        /* 忽略 */
      }
    }
  }

  return merged;
}

/**
 * Force Claude
 * CLI 调试输出到可写临时位置。 This 避免在 ~/.claude/debug 不可写的受限环境中崩溃。
 */
export function getClaudeAgentDebugFilePath(): string | undefined {
  try {
    const dir = join(tmpdir(), 'openpencil-claude-debug');
    mkdirSync(dir, { recursive: true });
    return join(dir, 'claude-agent.log');
  } catch {
    return undefined;
  }
}

/**
 * Custom spawn
 * ClaudeCodeProcess 为 Windows。 On
 * Windows、npm 安装的 CLIs 是 .cmd/.ps1 脚本，无法在没有 shell 的情况下直接生成。 - `.cmd` 文件：使用
 *
 * `cmd.exe /c`（PowerShell
 * 无法直接运行 .cmd）
 * - `.ps1` 文件：使用 `powershell.exe` - `.exe` 文件：不使用 shell 直接生成 -
 * Others：使用 `cmd.exe /c` 作为安全默认值 Also
 *
 * 将 stderr 捕获到调试文件 — 当 Claude 时 Code
 * 早期崩溃，调试文件可能为空，但 stderr 通常包含根本原因。
 */
export function buildSpawnClaudeCodeProcess() {
  if (process.platform !== 'win32') return undefined;
  return (options: {
    command: string;
    args: string[];
    cwd?: string;
    env: Record<string, string | undefined>;
    signal: AbortSignal;
  }) => {
    const cmd = options.command;
    const isPowerShell = cmd.endsWith('.ps1');

    let child;
    if (isPowerShell) {
      // For .ps1 脚本，通过 PowerShell 调用
      const psArgs = ['-ExecutionPolicy', 'Bypass', '-File', cmd, ...options.args];
      child = spawn('powershell.exe', psArgs, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      });
    } else if (cmd.endsWith('.exe')) {
      // 无需 shell 即可直接生成.exe 文件
      child = spawn(cmd, options.args, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        windowsHide: true,
      });
    } else {
      // For .cmd 或无扩展名二进制文件，使用 shell。 When shell：在
      // Windows 上为 true，空字符串参数被吞掉。 Filter out --setting-sources
      // 为空值，以防止下一个标志（例如 --permission-mode）被用作其值。
      const safeArgs: string[] = [];
      for (let i = 0; i < options.args.length; i++) {
        const arg = options.args[i];
        // Skip --setting-sources 后跟一个空字符串
        if (
          arg === '--setting-sources' &&
          i + 1 < options.args.length &&
          options.args[i + 1] === ''
        ) {
          i++; // 也跳过空值
          continue;
        }
        safeArgs.push(arg);
      }
      child = spawn(cmd, safeArgs, {
        cwd: options.cwd,
        env: options.env as NodeJS.ProcessEnv,
        signal: options.signal,
        stdio: ['pipe', 'pipe', 'pipe'],
        shell: true,
        windowsHide: true,
      });
    }

    // Capture stderr 调试文件 — 在将任何内容写入调试日志之前帮助诊断进程退出时的崩溃
    const stderrChunks: Buffer[] = [];
    child.stderr?.on('data', (chunk: Buffer) => {
      stderrChunks.push(chunk);
    });
    child.on('exit', (code) => {
      if (code !== 0 && stderrChunks.length > 0) {
        const stderr = Buffer.concat(stderrChunks).toString('utf-8').trim();
        if (stderr) {
          const debugPath = getClaudeAgentDebugFilePath();
          if (debugPath) {
            try {
              appendFileSync(debugPath, `\n[stderr exit=${code}] ${stderr}\n`);
            } catch {
              /* best effort */
            }
          }
        }
      }
    });

    return child;
  };
}
