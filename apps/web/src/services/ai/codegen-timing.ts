import type {
  CodegenProviderCallStage,
  CodegenTimingBreakdown,
} from '@zseven-w/pen-types';
import type { CodegenTextCollector } from './codegen-types';

export function nowMs(): number {
  return typeof performance !== 'undefined' && typeof performance.now === 'function'
    ? performance.now()
    : Date.now();
}

export function elapsedSince(start: number): number {
  return Math.max(0, Math.round(nowMs() - start));
}

type CodegenTimingNumberKey = Exclude<keyof CodegenTimingBreakdown, 'providerCalls'>;

export function addTiming(
  timing: CodegenTimingBreakdown,
  key: CodegenTimingNumberKey,
  ms: number,
) {
  timing[key] = (timing[key] ?? 0) + ms;
}

export function mergeProviderCallTiming(
  target: CodegenTimingBreakdown,
  source: CodegenTimingBreakdown,
) {
  if (!source.providerCalls?.length) return;
  target.providerCalls = [...(target.providerCalls ?? []), ...source.providerCalls];
  target.providerCallCount = target.providerCalls.length;
}

export function finalizeTiming(timing: CodegenTimingBreakdown, totalMs: number) {
  timing.totalMs = totalMs;
  timing.providerMs =
    (timing.planningMs ?? 0) +
    (timing.chunkMs ?? 0) +
    (timing.assemblyMs ?? 0) +
    (timing.repairMs ?? 0);

  const providerCallTotalMs =
    timing.providerCalls?.reduce((total, call) => total + call.durationMs, 0) ?? 0;
  if (providerCallTotalMs > 0) {
    timing.providerCallTotalMs = providerCallTotalMs;
  }
}

export function withProviderTiming(
  timing: CodegenTimingBreakdown,
  collectText: CodegenTextCollector,
  input: {
    stage: CodegenProviderCallStage;
    attempt: number;
    provider?: string;
    model?: string;
    chunkId?: string;
  },
): CodegenTextCollector {
  return async (systemPrompt, userMessage, model, provider, abortSignal) => {
    const started = nowMs();
    try {
      const response = await collectText(systemPrompt, userMessage, model, provider, abortSignal);
      recordProviderCall(timing, {
        ...input,
        model: input.model ?? model,
        provider: input.provider ?? provider,
        durationMs: elapsedSince(started),
      });
      return response;
    } catch (error) {
      recordProviderCall(timing, {
        ...input,
        model: input.model ?? model,
        provider: input.provider ?? provider,
        durationMs: elapsedSince(started),
        error: error instanceof Error ? error.message : 'Provider call failed',
      });
      throw error;
    }
  };
}

function recordProviderCall(
  timing: CodegenTimingBreakdown,
  input: {
    stage: CodegenProviderCallStage;
    durationMs: number;
    attempt: number;
    provider?: string;
    model?: string;
    chunkId?: string;
    error?: string;
  },
) {
  const durationMs = Math.max(0, Math.round(input.durationMs));
  timing.providerCalls = [
    ...(timing.providerCalls ?? []),
    {
      stage: input.stage,
      durationMs,
      attempt: input.attempt,
      provider: input.provider,
      model: input.model,
      chunkId: input.chunkId,
      error: input.error,
    },
  ];
  timing.providerCallCount = timing.providerCalls.length;
}
