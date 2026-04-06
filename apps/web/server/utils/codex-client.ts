import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

type ThinkingMode = 'adaptive' | 'disabled' | 'enabled';
type ThinkingEffort = 'low' | 'medium' | 'high' | 'max';

interface CodexExecOptions {
  model?: string;
  systemPrompt?: string;
  thinkingMode?: ThinkingMode;
  thinkingBudgetTokens?: number;
  effort?: ThinkingEffort;
  timeoutMs?: number;
  /** Paths to temporary image files to reference in the prompt */
  imageFiles?: string[];
}

interface CodexCliResult {
  text?: string;
  error?: string;
}

const DEFAULT_CODEX_TIMEOUT_MS = 15 * 60 * 1000;

/**
 * Allowlist-based env filter for Codex CLI subprocess.
 * Only passes through safe system vars and provider-specific prefixes.
 * Prevents leaking secrets like ANTHROPIC_API_KEY, AWS_SECRET_KEY, GITHUB_TOKEN, etc.
 */
const CODEX_ENV_ALLOWLIST = new Set([
  'PATH',
  'HOME',
  'TERM',
  'LANG',
  'SHELL',
  'TMPDIR',
  // Windows-essential vars
  'SYSTEMROOT',
  'COMSPEC',
  'USERPROFILE',
  'APPDATA',
  'LOCALAPPDATA',
  'PATHEXT',
  'SYSTEMDRIVE',
  'TEMP',
  'TMP',
  'HOMEDRIVE',
  'HOMEPATH',
]);

export function filterCodexEnv(
  env: Record<string, string | undefined>,
): Record<string, string | undefined> {
  const result: Record<string, string | undefined> = {};
  for (const [k, v] of Object.entries(env)) {
    if (CODEX_ENV_ALLOWLIST.has(k) || k.startsWith('OPENAI_') || k.startsWith('CODEX_')) {
      result[k] = v;
    }
  }
  return result;
}

export async function runCodexExec(
  userPrompt: string,
  options: CodexExecOptions = {},
): Promise<CodexCliResult> {
  const tempDir = await mkdtemp(join(tmpdir(), 'openpencil-codex-'));
  const outputPath = join(tempDir, 'last-message.txt');
  const prompt = buildPrompt(options.systemPrompt, userPrompt, options.imageFiles);
  const codexEffort = resolveCodexEffort(options.thinkingMode, options.effort);

  const args = [
    'exec',
    '--json',
    '--skip-git-repo-check',
    '--sandbox',
    'read-only',
    '--output-last-message',
    outputPath,
  ];

  if (options.model) {
    args.push('--model', options.model);
  }

  if (codexEffort) {
    args.push('--config', `model_reasoning_effort=${codexEffort}`);
  }

  // On Windows, passing long prompts as command-line arguments causes
  // shell escaping issues (PowerShell MissingExpression, special chars).
  // Use codex's stdin mode (`-` as prompt arg) on all platforms — simpler
  // and avoids command-line length limits.
  args.push('-');

  try {
    const runResult = await executeCodexCommand(
      args,
      options.timeoutMs ?? DEFAULT_CODEX_TIMEOUT_MS,
      prompt,
    );
    const finalText = await readFile(outputPath, 'utf-8').catch(() => '');
    const normalizedText = finalText.trim() || runResult.text.trim();

    if (normalizedText) {
      return { text: normalizedText };
    }

    if (runResult.errors.length > 0) {
      return { error: runResult.errors.join('; ') };
    }

    return { error: 'Codex returned no output.' };
  } catch (error) {
    return { error: error instanceof Error ? error.message : 'Codex execution failed' };
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}

export async function* streamCodexExec(
  userPrompt: string,
  options: CodexExecOptions = {},
): AsyncGenerator<{ type: 'text'; content: string } | { type: 'error'; content: string }> {
  const tempDir = await mkdtemp(join(tmpdir(), 'openpencil-codex-'));
  const prompt = buildPrompt(options.systemPrompt, userPrompt, options.imageFiles);
  const codexEffort = resolveCodexEffort(options.thinkingMode, options.effort);

  const args = [
    'exec',
    '--json',
    '--skip-git-repo-check',
    '--sandbox',
    'read-only',
    '-',
    ...(options.model ? ['--model', options.model] : []),
    ...(codexEffort ? ['--config', `model_reasoning_effort=${codexEffort}`] : []),
  ];

  const child = spawn('codex', args, {
    env: filterCodexEnv(process.env as Record<string, string | undefined>),
    stdio: ['pipe', 'pipe', 'pipe'],
    ...(process.platform === 'win32' && { shell: true }),
  });

  if (child.stdin) {
    child.stdin.write(prompt);
    child.stdin.end();
  }

  try {
    let stdoutBuffer = '';
    for await (const chunk of child.stdout!) {
      stdoutBuffer += chunk.toString('utf-8');
      let idx = stdoutBuffer.indexOf('\n');
      while (idx >= 0) {
        const line = stdoutBuffer.slice(0, idx).trim();
        stdoutBuffer = stdoutBuffer.slice(idx + 1);
        if (line) {
          const event = parseCodexJsonLine(line);
          if (event?.text) yield { type: 'text' as const, content: event.text };
          if (event?.error) yield { type: 'error' as const, content: event.error };
        }
        idx = stdoutBuffer.indexOf('\n');
      }
    }
    if (stdoutBuffer.trim()) {
      const event = parseCodexJsonLine(stdoutBuffer.trim());
      if (event?.text) yield { type: 'text' as const, content: event.text };
      if (event?.error) yield { type: 'error' as const, content: event.error };
    }
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}

function buildPrompt(
  systemPrompt: string | undefined,
  userPrompt: string,
  imageFiles?: string[],
): string {
  const userText = userPrompt.trim();
  const imageSection =
    imageFiles && imageFiles.length > 0
      ? '\n' +
        imageFiles.map((f) => `[Attached image: ${f} — read this file to see the image]`).join('\n')
      : '';

  if (!systemPrompt?.trim()) {
    return userText + imageSection;
  }

  return [
    'You are a design generation assistant. Follow the guidelines below to produce the requested output.',
    '',
    '--- GUIDELINES ---',
    systemPrompt.trim(),
    '',
    '--- TASK ---',
    userText + imageSection,
  ].join('\n');
}

function resolveCodexEffort(
  thinkingMode: ThinkingMode | undefined,
  effort: ThinkingEffort | undefined,
): 'low' | 'medium' | 'high' | undefined {
  if (thinkingMode === 'disabled') {
    return 'low';
  }

  if (effort === 'max') {
    return 'high';
  }

  if (effort === 'low' || effort === 'medium' || effort === 'high') {
    return effort;
  }

  if (thinkingMode === 'enabled') {
    return 'medium';
  }

  return undefined;
}

async function executeCodexCommand(
  args: string[],
  timeoutMs: number,
  stdinText?: string,
): Promise<{ text: string; errors: string[] }> {
  return await new Promise((resolve, reject) => {
    const child = spawn('codex', args, {
      env: filterCodexEnv(process.env as Record<string, string | undefined>),
      stdio: [stdinText ? 'pipe' : 'ignore', 'pipe', 'pipe'],
      // On Windows, npm-installed CLIs are .cmd scripts — need shell to resolve.
      ...(process.platform === 'win32' && { shell: true }),
    });

    // Pipe prompt via stdin (codex reads from stdin when `-` is the prompt arg)
    if (stdinText && child.stdin) {
      child.stdin.write(stdinText);
      child.stdin.end();
    }

    let stdoutBuffer = '';
    let stderrBuffer = '';
    let textAccumulator = '';
    const errors: string[] = [];

    const flushStdoutLine = (line: string) => {
      const event = parseCodexJsonLine(line);
      if (!event) return;
      if (event.text) {
        textAccumulator += event.text;
      }
      if (event.error) {
        errors.push(event.error);
      }
    };

    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`Codex request timed out after ${Math.round(timeoutMs / 1000)}s.`));
    }, timeoutMs);

    child.stdout!.on('data', (chunk: Buffer) => {
      stdoutBuffer += chunk.toString('utf-8');
      let idx = stdoutBuffer.indexOf('\n');
      while (idx >= 0) {
        const line = stdoutBuffer.slice(0, idx).trim();
        stdoutBuffer = stdoutBuffer.slice(idx + 1);
        if (line) flushStdoutLine(line);
        idx = stdoutBuffer.indexOf('\n');
      }
    });

    child.stderr!.on('data', (chunk: Buffer) => {
      stderrBuffer += chunk.toString('utf-8');
    });

    child.on('error', (err) => {
      clearTimeout(timer);
      reject(err);
    });

    child.on('close', (code) => {
      clearTimeout(timer);

      const tail = stdoutBuffer.trim();
      if (tail) {
        flushStdoutLine(tail);
      }

      if (code === 0) {
        resolve({ text: textAccumulator, errors });
        return;
      }

      const stderrError = extractCodexCliError(stderrBuffer);
      const fallback = errors[errors.length - 1];
      reject(new Error(stderrError || fallback || `Codex exited with code ${code ?? 'unknown'}.`));
    });
  });
}

function parseCodexJsonLine(line: string): { text?: string; error?: string } | null {
  let parsed: Record<string, unknown>;
  try {
    parsed = JSON.parse(line) as Record<string, unknown>;
  } catch {
    return null;
  }

  const type = typeof parsed.type === 'string' ? parsed.type : '';
  if (type === 'error') {
    const message = getStringField(parsed, ['message']);
    return { error: message || 'Codex returned an unknown error.' };
  }

  // Common Codex JSONL stream events include deltas in "delta" or "text".
  const text =
    getStringField(parsed, ['delta']) ||
    getStringField(parsed, ['text']) ||
    getStringField(parsed, ['content']);

  if (!text) return null;
  return { text };
}

function getStringField(obj: Record<string, unknown>, keys: string[]): string | null {
  for (const key of keys) {
    const val = obj[key];
    if (typeof val === 'string' && val.length > 0) {
      return val;
    }
  }
  return null;
}

function extractCodexCliError(stderr: string): string | null {
  const trimmed = stderr.trim();
  if (!trimmed) return null;

  const lines = trimmed
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);

  // 1. Look for "error: ..." lines (simple CLI errors)
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i];
    if (line.toLowerCase().startsWith('error:')) {
      return line.replace(/^error:\s*/i, '').trim();
    }
  }

  // 2. Look for Codex structured log errors: "<timestamp> ERROR <module>: <message>"
  //    These contain the real error (auth failures, API errors, etc.)
  for (let i = lines.length - 1; i >= 0; i--) {
    const match = lines[i].match(/\bERROR\s+\S+:\s*(.+)/);
    if (match) {
      const msg = match[1].trim();
      // For auth errors, provide actionable guidance
      if (/refresh token|sign in again|token.*expired|401 Unauthorized/i.test(msg)) {
        return 'Codex authentication expired. Run "codex logout && codex login" to re-authenticate.';
      }
      return msg;
    }
  }

  // 3. Skip unhelpful "Warning: no last agent message" — surface it only as fallback
  const lastLine = lines[lines.length - 1] ?? null;
  if (lastLine && /^warning:\s*no last agent message/i.test(lastLine)) {
    return 'Codex returned no output. Check "codex login" status or try a different model.';
  }

  return lastLine;
}
