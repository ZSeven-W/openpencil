import type {
  ChunkContract,
  ChunkResult,
  CodePlanFromAI,
  CodegenQualityIssue,
  Framework,
  PenNode,
} from '@zseven-w/pen-types';
import type { CodegenAssetHint } from './codegen-assets';
import type { buildCodegenFiles } from './codegen-files';
import type { CodegenDesignIR } from './codegen-design-ir';
import type { CodegenTextCollector } from './codegen-types';
import {
  buildAssemblyPrompt,
  buildChunkPrompt,
  buildDirectGenerationPrompt,
  buildPlanningPrompt,
  buildRepairPrompt,
} from './codegen-prompts';
import { streamChat } from './ai-service';
import { cleanCode, parseChunkResponse } from './codegen-response-parser';

export async function collectStreamText(
  systemPrompt: string,
  userMessage: string,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
): Promise<string> {
  let fullResponse = '';
  for await (const chunk of streamChat(
    systemPrompt,
    [{ role: 'user', content: userMessage }],
    model,
    undefined,
    provider,
    abortSignal,
  )) {
    if (chunk.type === 'text') {
      fullResponse += chunk.content;
    } else if (chunk.type === 'error') {
      throw new Error(chunk.content);
    }
  }
  return fullResponse;
}

export async function runPlanning(
  nodes: PenNode[],
  framework: Framework,
  designIR: CodegenDesignIR | undefined,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  strict?: boolean,
  collectText: CodegenTextCollector = collectStreamText,
): Promise<CodePlanFromAI> {
  const { system, user } = buildPlanningPrompt(nodes, framework, designIR);
  const systemPrompt = strict
    ? `${system}\n\nCRITICAL: Respond with ONLY valid JSON. No markdown, no explanation.`
    : system;

  const fullResponse = await collectText(systemPrompt, user, model, provider, abortSignal);
  const jsonMatch = fullResponse.match(/\{[\s\S]*\}/);
  if (!jsonMatch) throw new Error('No JSON found in planning response');

  const plan = JSON.parse(jsonMatch[0]) as CodePlanFromAI;
  if (!plan.chunks || !plan.rootLayout) {
    throw new Error('Planning response missing required fields (chunks, rootLayout)');
  }
  plan.sharedStyles = plan.sharedStyles ?? [];
  return plan;
}

export async function runChunkGeneration(
  nodes: PenNode[],
  framework: Framework,
  suggestedComponentName: string,
  depContracts: ChunkContract[],
  chunkId: string,
  designIR: CodegenDesignIR | undefined,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  assetHints: CodegenAssetHint[] = [],
  collectText: CodegenTextCollector = collectStreamText,
): Promise<ChunkResult> {
  const { system, user } = buildChunkPrompt(
    nodes,
    framework,
    suggestedComponentName,
    depContracts,
    assetHints,
    designIR,
  );
  const fullResponse = await collectText(system, user, model, provider, abortSignal);
  return parseChunkResponse(fullResponse, chunkId);
}

export async function runAssembly(
  chunkResults: {
    chunkId: string;
    name: string;
    code: string;
    contract?: ChunkContract;
    status: 'successful' | 'degraded' | 'failed';
  }[],
  plan: CodePlanFromAI,
  framework: Framework,
  variables: Record<string, unknown> | undefined,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  exportedAssetPaths: string[] = [],
  designIR?: CodegenDesignIR,
  collectText: CodegenTextCollector = collectStreamText,
): Promise<string> {
  const { system, user } = buildAssemblyPrompt(
    chunkResults,
    plan,
    framework,
    variables,
    exportedAssetPaths,
    designIR,
  );
  const fullResponse = await collectText(system, user, model, provider, abortSignal);
  return cleanCode(fullResponse);
}

export async function runDirectGeneration(
  input: {
    nodes: PenNode[];
    framework: Framework;
    variables?: Record<string, unknown>;
    exportedAssetPaths: string[];
    designIR: CodegenDesignIR;
  },
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  collectText: CodegenTextCollector = collectStreamText,
): Promise<string> {
  const { system, user } = buildDirectGenerationPrompt(input);
  const fullResponse = await collectText(system, user, model, provider, abortSignal);
  return cleanCode(fullResponse);
}

export async function runRepair(
  input: {
    framework: Framework;
    code: string;
    files: ReturnType<typeof buildCodegenFiles>;
    issues: CodegenQualityIssue[];
    designIR: CodegenDesignIR;
    exportedAssetPaths: string[];
  },
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  collectText: CodegenTextCollector = collectStreamText,
): Promise<string> {
  const { system, user } = buildRepairPrompt(input);
  const fullResponse = await collectText(system, user, model, provider, abortSignal);
  return cleanCode(fullResponse);
}
