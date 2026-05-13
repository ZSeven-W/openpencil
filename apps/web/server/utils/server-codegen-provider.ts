import { runCodexExec } from './codex-client';
import { runGeminiExec } from './gemini-client';
import { resolveClaudeCli } from './resolve-claude-cli';
import {
  buildClaudeAgentEnv,
  buildSpawnClaudeCodeProcess,
  getClaudeAgentDebugFilePath,
} from './resolve-claude-agent-env';

export async function createChatCompletionText(input: {
  system: string;
  message: string;
  model: string;
  provider?: string;
  abortSignal?: AbortSignal;
}): Promise<string> {
  if (input.abortSignal?.aborted) throw new Error('Aborted');

  if (input.provider === 'anthropic') {
    const { query } = await import('@anthropic-ai/claude-agent-sdk');
    const env = buildClaudeAgentEnv();
    const debugFile = getClaudeAgentDebugFilePath();
    const claudePath = resolveClaudeCli();
    const spawnClaudeCodeProcess = buildSpawnClaudeCodeProcess();
    const q = query({
      prompt: input.message,
      options: {
        systemPrompt: input.system,
        model: input.model,
        maxTurns: 1,
        tools: [],
        plugins: [],
        permissionMode: 'plan',
        persistSession: false,
        env,
        ...(debugFile ? { debugFile } : {}),
        ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
        ...(spawnClaudeCodeProcess ? { spawnClaudeCodeProcess } : {}),
      },
    });

    try {
      for await (const message of q) {
        if (message.type !== 'result') continue;
        const isErrorResult =
          'is_error' in message && Boolean((message as { is_error?: boolean }).is_error);
        if (message.subtype === 'success' && !isErrorResult) {
          return message.result;
        }
        const errors = 'errors' in message ? (message.errors as string[]) : [];
        const resultText = 'result' in message ? String(message.result ?? '') : '';
        throw new Error(errors.join('; ') || resultText || `Query ended with: ${message.subtype}`);
      }
    } finally {
      q.close();
    }

    throw new Error('No result received from Claude Agent SDK');
  }

  if (input.provider === 'openai') {
    const result = await runCodexExec(input.message, {
      model: input.model,
      systemPrompt: input.system,
    });
    if (result.error) throw new Error(result.error);
    return result.text ?? '';
  }

  if (input.provider === 'gemini') {
    const result = await runGeminiExec(input.message, {
      model: input.model,
      systemPrompt: input.system,
    });
    if (result.error) throw new Error(result.error);
    return result.text ?? '';
  }

  throw new Error(`Background provider ${input.provider ?? 'unknown'} is not available yet.`);
}
