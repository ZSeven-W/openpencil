import { spawn } from 'node:child_process';
import { resolveGeminiCli } from './resolve-gemini-cli';

type ThinkingMode = 'adaptive' | 'disabled' | 'enabled';
type ThinkingEffort = 'low' | 'medium' | 'high' | 'max';

export interface GeminiExecOptions {
  model?: string;
  systemPrompt?: string;
  thinkingMode?: ThinkingMode;
  thinkingBudgetTokens?: number;
  effort?: ThinkingEffort;
  timeoutMs?: number;
}

interface GeminiCliResult {
  text?: string;
  error?: string;
}

const DEFAULT_GEMINI_TIMEOUT_MS = 15 * 60 * 1000;

/**
 * Allowlist-based env filter for Gemini CLI subprocess.
 * Passes through safe system vars and Google/Gemini-specific prefixes.
 */
const GEMINI_ENV_ALLOWLIST = new Set([
  'PATH',
  'HOME',
  'TERM',
  'LANG',
  'SHELL',
  'TMPDIR',
  // Windows-essential
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

function filterGeminiEnv(
  env: Record<string, string | undefined>,
): Record<string, string | undefined> {
  const result: Record<string, string | undefined> = {};
  for (const [k, v] of Object.entries(env)) {
    if (
      GEMINI_ENV_ALLOWLIST.has(k) ||
      k.startsWith('GOOGLE_') ||
      k.startsWith('GEMINI_') ||
      k.startsWith('GCLOUD_')
    ) {
      result[k] = v;
    }
  }
  return result;
}

/**
 * Run Gemini CLI in non-interactive mode with JSON output.
 * Passes prompt via stdin to avoid command-line length limits.
 * The CLI handles its own authentication (OAuth or API key).
 */
export async function runGeminiExec(
  userPrompt: string,
  options: GeminiExecOptions = {},
): Promise<GeminiCliResult> {
  const binPath = resolveGeminiCli();
  if (!binPath) {
    return { error: 'Gemini CLI not found. Install it first.' };
  }

  const prompt = buildPrompt(options.systemPrompt, userPrompt);

  const args = ['-o', 'json', '--approval-mode', 'plan'];

  if (options.model) {
    args.push('-m', options.model);
  }

  // Use -p with a minimal marker; full prompt piped via stdin.
  // Gemini CLI appends -p value after stdin content.
  args.push('-p', ' ');

  try {
    const result = await executeGeminiCommand(
      binPath,
      args,
      options.timeoutMs ?? DEFAULT_GEMINI_TIMEOUT_MS,
      prompt,
    );
    return result;
  } catch (error) {
    return { error: error instanceof Error ? error.message : 'Gemini execution failed' };
  }
}

/**
 * Stream Gemini CLI output in real-time using `stream-json` format.
 * Passes prompt via stdin. Yields text deltas as they arrive.
 */
export function streamGeminiExec(
  userPrompt: string,
  options: GeminiExecOptions = {},
): {
  stream: AsyncGenerator<{ type: 'text' | 'error' | 'done'; content: string }>;
  kill: () => void;
} {
  const binPath = resolveGeminiCli();
  if (!binPath) {
    return {
      stream: (async function* () {
        yield { type: 'error' as const, content: 'Gemini CLI not found.' };
      })(),
      kill: () => {},
    };
  }

  const prompt = buildPrompt(options.systemPrompt, userPrompt);

  const args = ['-o', 'stream-json', '--approval-mode', 'plan'];

  if (options.model) {
    args.push('-m', options.model);
  }

  // Use -p with minimal marker; full prompt piped via stdin.
  args.push('-p', ' ');

  const child = spawn(binPath, args, {
    env: filterGeminiEnv(process.env as Record<string, string | undefined>),
    stdio: ['pipe', 'pipe', 'pipe'],
    ...(process.platform === 'win32' && { shell: true }),
  });

  // Pipe prompt via stdin
  if (child.stdin) {
    child.stdin.write(prompt);
    child.stdin.end();
  }

  const timeoutMs = options.timeoutMs ?? DEFAULT_GEMINI_TIMEOUT_MS;
  const timer = setTimeout(() => {
    child.kill('SIGTERM');
  }, timeoutMs);

  async function* generateStream(): AsyncGenerator<{
    type: 'text' | 'error' | 'done';
    content: string;
  }> {
    let buffer = '';

    child.stderr?.on('data', () => {
      /* discard stderr */
    });

    try {
      for await (const chunk of child.stdout!) {
        buffer += chunk.toString('utf-8');
        let idx = buffer.indexOf('\n');
        while (idx >= 0) {
          const line = buffer.slice(0, idx).trim();
          buffer = buffer.slice(idx + 1);
          if (line) {
            const event = parseStreamJsonLine(line);
            if (event) yield event;
          }
          idx = buffer.indexOf('\n');
        }
      }

      // Flush remaining buffer
      const tail = buffer.trim();
      if (tail) {
        const event = parseStreamJsonLine(tail);
        if (event) yield event;
      }

      yield { type: 'done', content: '' };
    } catch (error) {
      const msg = error instanceof Error ? error.message : 'Stream error';
      yield { type: 'error', content: msg };
    } finally {
      clearTimeout(timer);
    }
  }

  return {
    stream: generateStream(),
    kill: () => {
      clearTimeout(timer);
      child.kill('SIGTERM');
    },
  };
}

function buildPrompt(systemPrompt: string | undefined, userPrompt: string): string {
  const userText = userPrompt.trim();
  if (!systemPrompt?.trim()) return userText;

  return [
    'You are a design generation assistant. Follow the guidelines below to produce the requested output.',
    '',
    '--- GUIDELINES ---',
    systemPrompt.trim(),
    '',
    '--- TASK ---',
    userText,
  ].join('\n');
}

async function executeGeminiCommand(
  binPath: string,
  args: string[],
  timeoutMs: number,
  stdinText?: string,
): Promise<GeminiCliResult> {
  return await new Promise((resolve, reject) => {
    const child = spawn(binPath, args, {
      env: filterGeminiEnv(process.env as Record<string, string | undefined>),
      stdio: [stdinText ? 'pipe' : 'ignore', 'pipe', 'pipe'],
      ...(process.platform === 'win32' && { shell: true }),
    });

    // Pipe prompt via stdin
    if (stdinText && child.stdin) {
      child.stdin.write(stdinText);
      child.stdin.end();
    }

    let stdoutBuffer = '';
    let stderrBuffer = '';

    const timer = setTimeout(() => {
      child.kill('SIGTERM');
      reject(new Error(`Gemini request timed out after ${Math.round(timeoutMs / 1000)}s.`));
    }, timeoutMs);

    child.stdout!.on('data', (chunk: Buffer) => {
      stdoutBuffer += chunk.toString('utf-8');
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

      // Parse JSON output — Gemini CLI always outputs a JSON object at the end of stdout.
      // Error text / stack traces may appear before it.
      const parsed = parseGeminiJsonOutput(stdoutBuffer);

      if (parsed) {
        if (parsed.response) {
          resolve({ text: parsed.response });
          return;
        }
        if (parsed.errorMessage) {
          resolve({ error: friendlyGeminiApiError(parsed.errorMessage) });
          return;
        }
      }

      if (code !== 0) {
        // Extract meaningful error from stderr or stdout
        const errorMsg = extractGeminiError(stdoutBuffer, stderrBuffer);
        resolve({ error: errorMsg || `Gemini exited with code ${code ?? 'unknown'}.` });
        return;
      }

      const raw = stdoutBuffer.trim();
      resolve(raw ? { text: raw } : { error: 'Gemini returned no output.' });
    });
  });
}

/**
 * Parse Gemini CLI JSON output.
 * The CLI may print error text before the final JSON object.
 * We search from the END of stdout for the last valid JSON block.
 */
function parseGeminiJsonOutput(raw: string): { response?: string; errorMessage?: string } | null {
  const trimmed = raw.trim();
  if (!trimmed) return null;

  // Search backwards for the last top-level JSON object (starts with `{` at line beginning)
  const lines = trimmed.split('\n');
  for (let i = lines.length - 1; i >= 0; i--) {
    const line = lines[i].trim();
    if (!line.startsWith('{')) continue;

    // Try to parse from this line to the end
    const candidate = lines.slice(i).join('\n').trim();
    try {
      const data = JSON.parse(candidate) as Record<string, unknown>;
      // Must have session_id to be a valid Gemini CLI response
      if (!data.session_id && !data.response && !data.error) continue;

      const response = typeof data.response === 'string' ? data.response : undefined;

      // error can be a string or an object { type, message, code }
      let errorMessage: string | undefined;
      if (data.error) {
        if (typeof data.error === 'string') {
          errorMessage = data.error;
        } else if (typeof data.error === 'object' && data.error !== null) {
          const errObj = data.error as Record<string, unknown>;
          errorMessage =
            typeof errObj.message === 'string' ? errObj.message : JSON.stringify(data.error);
        }
      }

      return { response, errorMessage };
    } catch {
      /* not valid JSON from this point */
    }
  }

  return null;
}

/** Extract a human-readable error from Gemini CLI stdout/stderr */
function extractGeminiError(stdout: string, stderr: string): string | null {
  // Look for quota errors
  const quotaMatch =
    stdout.match(/quota will reset after (\S+)/i) || stderr.match(/quota will reset after (\S+)/i);
  if (quotaMatch) {
    return `Gemini quota exhausted. Resets after ${quotaMatch[1]}.`;
  }

  // Look for TerminalQuotaError or other named errors
  const namedError = stdout.match(/(Terminal\w+Error|ApiError|AuthError):\s*(.+)/m);
  if (namedError) {
    return namedError[2].trim();
  }

  // Stderr fallback
  const stderrTrimmed = stderr.trim();
  if (stderrTrimmed) return stderrTrimmed;

  return null;
}

/** Map raw Gemini API errors to user-friendly messages */
function friendlyGeminiApiError(raw: string): string {
  if (/quota|exhausted|429|capacity/i.test(raw)) {
    const resetMatch = raw.match(/reset after (\S+)/i);
    return resetMatch
      ? `Gemini quota exhausted. Resets after ${resetMatch[1]}.`
      : 'Gemini quota exhausted. Please wait and try again.';
  }
  if (/401|unauthenticated|auth/i.test(raw)) {
    return 'Gemini auth expired. Run "gemini" in your terminal to re-authenticate.';
  }
  if (/\[object Object\]/.test(raw)) {
    return 'Gemini API error. Check your quota or try a different model.';
  }
  return raw;
}

function parseStreamJsonLine(
  line: string,
): { type: 'text' | 'error' | 'done'; content: string } | null {
  // Skip non-JSON lines (e.g. "Loaded cached credentials.")
  if (!line.startsWith('{')) return null;

  let parsed: Record<string, unknown>;
  try {
    parsed = JSON.parse(line) as Record<string, unknown>;
  } catch {
    return null;
  }

  const type = typeof parsed.type === 'string' ? parsed.type : '';

  if (type === 'message' && parsed.role === 'assistant') {
    const content = typeof parsed.content === 'string' ? parsed.content : '';
    if (content) return { type: 'text', content };
  }

  if (type === 'result') {
    // Check for error in result event
    if (parsed.status === 'error' && parsed.error) {
      const errObj = parsed.error as Record<string, unknown>;
      const msg = typeof errObj.message === 'string' ? errObj.message : 'Unknown error';
      return { type: 'error', content: friendlyGeminiApiError(msg) };
    }
    return null;
  }

  if (type === 'error') {
    const content = typeof parsed.message === 'string' ? parsed.message : 'Unknown error';
    return { type: 'error', content: friendlyGeminiApiError(content) };
  }

  return null;
}
