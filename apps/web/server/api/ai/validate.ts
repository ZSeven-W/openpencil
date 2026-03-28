import { defineEventHandler, readBody, setResponseHeaders } from 'h3';
import { resolveClaudeCli } from '../../utils/resolve-claude-cli';
import {
  buildClaudeAgentEnv,
  buildSpawnClaudeCodeProcess,
  getClaudeAgentDebugFilePath,
  resolveAgentModel,
} from '../../utils/resolve-claude-agent-env';
import { writeFile, mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { runCodexExec } from '../../utils/codex-client';

interface ValidateBody {
  system: string;
  message: string;
  imageBase64: string;
  model?: string;
  provider?: 'anthropic' | 'openai' | 'opencode' | 'gemini';
}

/**
 * Vision-based validation endpoint.
 * Accepts a base64 PNG screenshot and a text prompt, sends multimodal
 * content blocks for analysis via Agent SDK.
 *
 * Saves screenshot to temp file, asks Claude Code to read it via its
 * built-in Read tool.
 */
export default defineEventHandler(async (event) => {
  const body = await readBody<ValidateBody>(event);

  if (!body?.system || !body?.message || !body?.imageBase64) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing required fields: system, message, imageBase64' };
  }

  if (!body.model?.trim()) {
    setResponseHeaders(event, { 'Content-Type': 'application/json' });
    return { error: 'Missing model. Model fallback is disabled.' };
  }

  try {
    if (body.provider === 'anthropic') {
      return await validateViaAgentSDK(body, body.model);
    }
    if (body.provider === 'openai') {
      return await validateViaCodex(body, body.model);
    }
    if (body.provider === 'opencode') {
      return await validateViaOpenCode(body, body.model);
    }
    if (body.provider === 'gemini') {
      return await validateViaGemini(body, body.model);
    }
    return { error: 'Missing or unsupported provider. Provider fallback is disabled.' };
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    return { error: message };
  }
});

function toImageBase64(data: string): string {
  const dataUrlPrefix = 'data:image/png;base64,';
  return data.startsWith(dataUrlPrefix) ? data.slice(dataUrlPrefix.length) : data;
}

async function withTempImageFile<T>(
  imageBase64: string,
  run: (tempPath: string) => Promise<T>,
  insideProject = false,
): Promise<T> {
  let tempDir: string;
  if (insideProject) {
    // Save inside the project directory so Claude Code Agent SDK (plan mode)
    // can read the file — it restricts reads to the project directory.
    const { mkdirSync, chmodSync } = await import('node:fs');
    const baseDir = join(process.cwd(), '.openpencil-tmp');
    mkdirSync(baseDir, { recursive: true });
    chmodSync(baseDir, 0o700);
    tempDir = await mkdtemp(join(baseDir, 'validate-'));
  } else {
    tempDir = await mkdtemp(join(tmpdir(), 'openpencil-validate-'));
  }
  const tempPath = join(tempDir, 'screenshot.png');
  try {
    await writeFile(tempPath, Buffer.from(toImageBase64(imageBase64), 'base64'));
    return await run(tempPath);
  } finally {
    await rm(tempDir, { recursive: true, force: true }).catch(() => {});
  }
}

/**
 * Agent SDK: save screenshot to a temp PNG file inside the project directory,
 * then ask Claude Code to read it (Claude Code's Read tool supports images
 * natively). Must use insideProject=true because plan mode restricts reads
 * to the project directory.
 */
async function validateViaAgentSDK(
  body: ValidateBody,
  requestedModel?: string,
): Promise<{ text: string; skipped?: boolean; error?: string }> {
  return await withTempImageFile(
    body.imageBase64,
    async (tempPath) => {
      const { query } = await import('@anthropic-ai/claude-agent-sdk');

      const env = buildClaudeAgentEnv();
      const debugFile = getClaudeAgentDebugFilePath();
      const claudePath = resolveClaudeCli();
      const model = resolveAgentModel(requestedModel, env);

      const prompt = `IMPORTANT: First, use the Read tool to read the image file at "${tempPath}". This is a PNG screenshot of a UI design.

After viewing the image, analyze it according to these instructions:

${body.system}

${body.message}

CRITICAL: Your ENTIRE response must be a single JSON object. No markdown, no explanation, no tool calls after reading the image. Just the JSON.`;

      const q = query({
        prompt,
        options: {
          ...(model ? { model } : {}),
          maxTurns: 3,
          tools: [],
          plugins: [],
          permissionMode: 'plan',
          persistSession: false,
          env,
          ...(debugFile ? { debugFile } : {}),
          ...(claudePath ? { pathToClaudeCodeExecutable: claudePath } : {}),
          ...(buildSpawnClaudeCodeProcess()
            ? { spawnClaudeCodeProcess: buildSpawnClaudeCodeProcess() }
            : {}),
        },
      });

      try {
        for await (const message of q) {
          if (message.type === 'result') {
            const isErrorResult =
              'is_error' in message && Boolean((message as { is_error?: boolean }).is_error);
            if (message.subtype === 'success' && !isErrorResult) {
              return { text: message.result };
            }
            const errors = 'errors' in message ? (message.errors as string[]) : [];
            const resultText = 'result' in message ? String(message.result ?? '') : '';
            return {
              error: errors.join('; ') || resultText || `Query ended with: ${message.subtype}`,
              text: '',
            };
          }
        }
      } finally {
        q.close();
      }

      return { text: '', skipped: true };
    },
    true,
  );
}

async function validateViaCodex(
  body: ValidateBody,
  model?: string,
): Promise<{ text: string; skipped?: boolean; error?: string }> {
  return await withTempImageFile(body.imageBase64, async (tempPath) => {
    const result = await runCodexExec(
      `${body.message}\n\nOutput ONLY the JSON object, no markdown fences, no explanation.`,
      {
        model,
        systemPrompt: body.system,
        imageFiles: [tempPath],
      },
    );
    if (result.error) {
      return { text: '', error: result.error };
    }
    return { text: result.text ?? '' };
  });
}

function parseOpenCodeModel(model?: string): { providerID: string; modelID: string } | undefined {
  if (!model || !model.includes('/')) return undefined;
  const idx = model.indexOf('/');
  return { providerID: model.slice(0, idx), modelID: model.slice(idx + 1) };
}

async function validateViaOpenCode(
  body: ValidateBody,
  model?: string,
): Promise<{ text: string; skipped?: boolean; error?: string }> {
  let ocServer: { close(): void } | undefined;
  try {
    const { getOpencodeClient } = await import('../../utils/opencode-client');
    const oc = await getOpencodeClient();
    const ocClient: any = oc.client;
    ocServer = oc.server;

    const { data: session, error: sessionError } = await ocClient.session.create({
      title: 'OpenPencil Validate',
    });
    if (sessionError || !session) {
      return { text: '', error: 'Failed to create OpenCode session' };
    }

    await ocClient.session.prompt({
      sessionID: session.id,
      noReply: true,
      parts: [{ type: 'text', text: body.system }],
    });

    const parsed = parseOpenCodeModel(model);
    if (!parsed) {
      return { text: '', error: 'Invalid OpenCode model format. Expected "provider/model".' };
    }

    const base64 = toImageBase64(body.imageBase64);
    const promptPayload = {
      sessionID: session.id,
      model: parsed,
      parts: [
        { type: 'image', url: `data:image/png;base64,${base64}` },
        {
          type: 'text',
          text: `${body.message}\n\nOutput ONLY the JSON object, no markdown fences, no explanation.`,
        },
      ],
    };

    const { data: result, error: promptError } = await ocClient.session.prompt(promptPayload);
    if (promptError) {
      return { text: '', error: 'OpenCode validation failed' };
    }

    const texts: string[] = [];
    if (result?.parts) {
      for (const part of result.parts) {
        if (part.type === 'text' && part.text) {
          texts.push(part.text);
        }
      }
    }
    return { text: texts.join('') };
  } catch (error) {
    const message = error instanceof Error ? error.message : 'Unknown error';
    return { text: '', error: message };
  } finally {
    const { releaseOpencodeServer } = await import('../../utils/opencode-client');
    releaseOpencodeServer(ocServer);
  }
}

/** Validate via Gemini CLI — saves screenshot to temp file, asks CLI to read it */
async function validateViaGemini(
  body: ValidateBody,
  model?: string,
): Promise<{ text: string; skipped?: boolean; error?: string }> {
  return await withTempImageFile(body.imageBase64, async (tempPath) => {
    const { runGeminiExec } = await import('../../utils/gemini-client');
    const prompt = `Read the image file at "${tempPath}". This is a PNG screenshot of a UI design.\n\n${body.message}\n\nOutput ONLY the JSON object, no markdown fences, no explanation.`;
    const result = await runGeminiExec(prompt, {
      model,
      systemPrompt: body.system,
    });
    if (result.error) {
      return { text: '', error: result.error };
    }
    return { text: result.text ?? '' };
  });
}
