/**
 * Codex CLI client — drives `codex exec` as a subprocess to reach
 * GPT-5.4 (the CLI's default) via the user's Codex Pro subscription.
 *
 * Trade-offs:
 *   + No per-token API billing (counts against Codex subscription)
 *   + Works offline of API key management
 *   - Each call incurs Codex's own system-prompt overhead (~20k tokens
 *     of codex agent framing) which the model still sees alongside
 *     our design-generation system prompt. Interpret A/B deltas
 *     against GPT-5.4/codex with that caveat; they're not a clean
 *     "raw GPT-5.4 completion" measurement.
 *   - `codex exec` cold-start adds a few seconds per call → 48 runs
 *     at 10-20s each = 10-15 min wall time for full GPT-5.4 pass.
 *
 * Output shape: we pass `--output-last-message` to write just the
 * assistant's final message to disk, then read + delete. Skips the
 * CLI's pretty-printed session log (which is noisy and harder to
 * parse reliably).
 */

import { spawn } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { ChatCallResult } from './openai-compat';

export interface CallCodexArgs {
  model: string;
  system: string;
  user: string;
  /**
   * Timeout in ms. Default 10 minutes — composite ab-v3 prompts plus
   * Codex's ~20k-token agent framing plus thinking can blow the older
   * 5-minute cap (saw this on ab-v4 dashboard-search-filters-composite,
   * where gpt-5.4 was mid-stream when codex SIGKILL'd it). Override via
   * `AB_CORPUS_CODEX_TIMEOUT_MS` env var if you want to surface
   * single-call slowness as a hard fail instead of waiting it out.
   */
  timeoutMs?: number;
}

const DEFAULT_CODEX_TIMEOUT_MS = (() => {
  const env = process.env.AB_CORPUS_CODEX_TIMEOUT_MS;
  if (env && /^\d+$/.test(env)) {
    const n = Number(env);
    if (n > 0) return n;
  }
  return 10 * 60 * 1000;
})();

export async function callCodex(args: CallCodexArgs): Promise<ChatCallResult> {
  const timeoutMs = args.timeoutMs ?? DEFAULT_CODEX_TIMEOUT_MS;
  const dir = mkdtempSync(join(tmpdir(), 'ab-codex-'));
  const outPath = join(dir, 'last-message.txt');
  // Codex CLI has its own system prompt (coding agent framing) that
  // we can't override. Fold OUR system prompt into the user-visible
  // prompt so it at least reaches the model. Tag the boundary so the
  // model treats the system block as authoritative context, not as
  // part of the task to answer.
  const prompt = `SYSTEM CONTEXT (treat as authoritative system prompt):\n<<<\n${args.system}\n>>>\n\nUSER REQUEST:\n${args.user}`;
  try {
    const content = await new Promise<string>((resolve, reject) => {
      // Pin reasoning effort to `medium` so ab-corpus stays comparable
      // across model bumps. Codex CLI's per-model default isn't fixed:
      // gpt-5.4 historically ran ~medium; gpt-5.5 defaults to `xhigh`,
      // which sent ab-v5 wall time from ~15 min (ab-v4) to a projected
      // ~13 h. Override via `AB_CORPUS_CODEX_REASONING` if you ever
      // need to measure full-effort ceiling.
      const reasoningEffort = process.env.AB_CORPUS_CODEX_REASONING || 'medium';
      const proc = spawn(
        'codex',
        [
          'exec',
          '--skip-git-repo-check',
          '--ephemeral',
          '--sandbox',
          'read-only',
          '-c',
          `model_reasoning_effort=${reasoningEffort}`,
          '-m',
          args.model,
          '--output-last-message',
          outPath,
          prompt,
        ],
        { stdio: ['ignore', 'pipe', 'pipe'] },
      );
      let stderr = '';
      proc.stderr.on('data', (chunk) => {
        stderr += chunk.toString();
      });
      const killTimer = setTimeout(() => {
        proc.kill('SIGKILL');
        reject(new Error(`codex timed out after ${timeoutMs}ms`));
      }, timeoutMs);
      proc.on('error', (err) => {
        clearTimeout(killTimer);
        reject(err);
      });
      proc.on('close', (code) => {
        clearTimeout(killTimer);
        if (code !== 0) {
          reject(new Error(`codex exit ${code}: ${stderr.slice(0, 300)}`));
          return;
        }
        try {
          const out = readFileSync(outPath, 'utf-8');
          if (out.trim().length === 0) {
            reject(new Error('codex produced empty output-last-message'));
            return;
          }
          resolve(out);
        } catch (err) {
          reject(err instanceof Error ? err : new Error(String(err)));
        }
      });
    });
    // Codex CLI doesn't surface token usage in --output-last-message
    // mode (the streaming JSONL session log has it, but parsing that
    // adds fragility for marginal value). Report 0/0 — the harness
    // aggregator skips zero rows, so codex columns show '—' instead
    // of looking falsely cheap.
    return { content, usage: { promptTokens: 0, completionTokens: 0 } };
  } finally {
    try {
      rmSync(dir, { recursive: true, force: true });
    } catch {
      // Best-effort cleanup
    }
  }
}
