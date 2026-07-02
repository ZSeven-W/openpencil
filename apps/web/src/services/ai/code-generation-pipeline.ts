// apps/web/src/services/ai/code-generation-pipeline.ts

import type { PenNode } from '@zseven-w/pen-types';
import type {
  Framework,
  CodePlanFromAI,
  CodeExecutionPlan,
  ExecutableChunk,
  PlannedChunk,
  ChunkResult,
  ChunkContract,
  ChunkStatus,
  CodeGenProgress,
  ContractValidationResult,
} from '@zseven-w/pen-types';
import { sanitizeName } from '@zseven-w/pen-core';
import { buildPlanningPrompt, buildChunkPrompt, buildAssemblyPrompt } from './codegen-prompts';
import { streamChat } from './ai-service';
import {
  collectChunkAssetHints,
  extractCodegenAssets,
  type CodegenAssetFile,
  type CodegenAssetHint,
} from './codegen-assets';

// Inlined to avoid importing from @zseven-w/pen-mcp, which transitively pulls
// node:fs/promises via document-manager and breaks Vite browser builds.
function validateContract(result: ChunkResult): ContractValidationResult {
  const issues: string[] = [];
  const { contract, code } = result;
  if (contract.componentName && !/^[A-Z][a-zA-Z0-9]*$/.test(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" is not a valid PascalCase identifier`);
  }
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');
  if (contract.componentName && !isSFC && !code.includes(contract.componentName)) {
    issues.push(`componentName "${contract.componentName}" not found in generated code`);
  }
  return { valid: issues.length === 0, issues };
}

// ── Exported helpers (tested independently) ──

/**
 * Hydrate a CodePlanFromAI with actual node data.
 * Strips chunks whose nodeIds don't match any input nodes.
 */
export function hydratePlan(plan: CodePlanFromAI, nodes: PenNode[]): CodeExecutionPlan {
  const nodeMap = new Map<string, PenNode>();
  function indexNodes(list: PenNode[]) {
    for (const n of list) {
      nodeMap.set(n.id, n);
      const children = (n as { children?: PenNode[] }).children;
      if (children) indexNodes(children);
    }
  }
  indexNodes(nodes);

  const orders = computeExecutionOrder(plan.chunks);

  const chunks: ExecutableChunk[] = plan.chunks
    .map((chunk) => {
      const resolved = chunk.nodeIds
        .map((id) => nodeMap.get(id))
        .filter((n): n is PenNode => n !== undefined);
      if (resolved.length === 0) return null;
      return {
        ...chunk,
        nodes: resolved,
        order: orders.get(chunk.id) ?? 0,
        depContracts: [] as ChunkContract[],
      };
    })
    .filter((c): c is ExecutableChunk => c !== null);

  return {
    chunks,
    sharedStyles: plan.sharedStyles,
    rootLayout: plan.rootLayout,
  };
}

/**
 * Compute execution order from dependency graph.
 * Chunks with no deps get order 0. Dependent chunks get max(dep orders) + 1.
 */
export function computeExecutionOrder(chunks: PlannedChunk[]): Map<string, number> {
  const orders = new Map<string, number>();

  function resolve(id: string, visited: Set<string>): number {
    if (orders.has(id)) return orders.get(id)!;
    if (visited.has(id)) return 0; // cycle guard
    visited.add(id);

    const chunk = chunks.find((c) => c.id === id);
    if (!chunk || chunk.dependencies.length === 0) {
      orders.set(id, 0);
      return 0;
    }

    const maxDep = Math.max(...chunk.dependencies.map((depId) => resolve(depId, visited)));
    const order = maxDep + 1;
    orders.set(id, order);
    return order;
  }

  for (const chunk of chunks) {
    resolve(chunk.id, new Set());
  }

  return orders;
}

/**
 * Parse a chunk generation response into code + contract.
 * Looks for ---CONTRACT--- separator.
 */
export function parseChunkResponse(response: string, chunkId: string): ChunkResult {
  // Strategy 1: explicit ---CONTRACT--- separator
  const separator = '---CONTRACT---';
  const sepIdx = response.indexOf(separator);
  if (sepIdx !== -1) {
    const code = cleanCode(response.slice(0, sepIdx));
    const contractStr = response.slice(sepIdx + separator.length).trim();
    const contract = tryParseContract(contractStr, chunkId);
    if (contract) return { chunkId, code, contract };
  }

  // Strategy 2: find a JSON block containing "componentName" (AI often wraps in ```json)
  const contractJsonMatch = response.match(/```json\s*\n([\s\S]*?)\n\s*```/);
  if (contractJsonMatch) {
    const jsonStr = contractJsonMatch[1].trim();
    if (jsonStr.includes('"componentName"')) {
      const contract = tryParseContract(jsonStr, chunkId);
      if (contract) {
        // Everything before the JSON block is code
        const jsonBlockStart = response.indexOf(contractJsonMatch[0]);
        const code = cleanCode(response.slice(0, jsonBlockStart));
        return { chunkId, code, contract };
      }
    }
  }

  // Strategy 3: find last JSON object with "componentName" in the response
  const lastJsonMatch = response.match(/(\{[^{}]*"componentName"[^{}]*\})\s*$/);
  if (lastJsonMatch) {
    const contract = tryParseContract(lastJsonMatch[1], chunkId);
    if (contract) {
      const jsonStart = response.lastIndexOf(lastJsonMatch[1]);
      const code = cleanCode(response.slice(0, jsonStart));
      return { chunkId, code, contract };
    }
  }

  // Strategy 4: infer contract from code (extract component name from export)
  const code = cleanCode(response);
  const inferredContract = inferContractFromCode(code, chunkId);
  return { chunkId, code, contract: inferredContract };
}

function tryParseContract(str: string, chunkId: string): ChunkContract | null {
  try {
    // Strip markdown fences if present
    const cleaned = str
      .replace(/^```\w*\n?/gm, '')
      .replace(/```\s*$/gm, '')
      .trim();
    const parsed = JSON.parse(cleaned) as ChunkContract;
    if (parsed.componentName) {
      parsed.chunkId = chunkId;
      parsed.exportedProps = parsed.exportedProps ?? [];
      parsed.slots = parsed.slots ?? [];
      parsed.cssClasses = parsed.cssClasses ?? [];
      parsed.cssVariables = parsed.cssVariables ?? [];
      parsed.imports = parsed.imports ?? [];
      return parsed;
    }
  } catch {
    /* not valid JSON */
  }
  return null;
}

function inferContractFromCode(code: string, chunkId: string): ChunkContract {
  const isSFC = code.includes('<script') || code.includes('<template') || code.includes('<style');

  // Try to extract component name from export statements
  // Skip export let/const for SFC (Svelte uses them for props, not component names)
  const exportMatch =
    code.match(/export\s+default\s+function\s+(\w+)/) ??
    code.match(/export\s+function\s+([A-Z]\w*)/) ?? // only PascalCase functions
    (!isSFC ? code.match(/export\s+default\s+class\s+(\w+)/) : null) ??
    code.match(/fun\s+([A-Z]\w*)\s*\(/) ?? // Kotlin (PascalCase only)
    code.match(/struct\s+(\w+)\s*:\s*View/) ?? // SwiftUI
    code.match(/class\s+(\w+)\s+extends/); // Dart/Flutter
  const componentName = exportMatch?.[1] ?? '';

  // Extract imports
  const importMatches = [...code.matchAll(/import\s+.*?from\s+['"](.+?)['"]/g)];
  const imports = importMatches.map((m) => ({
    source: m[1],
    specifiers: [] as string[],
  }));

  return {
    chunkId,
    componentName,
    exportedProps: [],
    slots: [],
    cssClasses: [],
    cssVariables: [],
    imports,
  };
}

function cleanCode(raw: string): string {
  return raw
    .replace(/^```\w*\n?/gm, '')
    .replace(/```\s*$/gm, '')
    .trim();
}

// ── Main pipeline ──

export async function generateCode(
  nodes: PenNode[],
  framework: Framework,
  variables: Record<string, unknown> | undefined,
  onProgress: (event: CodeGenProgress) => void,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
): Promise<{ code: string; degraded: boolean; assets: CodegenAssetFile[] }> {
  const { nodes: sanitizedNodes, assets } = extractCodegenAssets(nodes);
  // ── Step 1: Planning ──
  onProgress({ step: 'planning', status: 'running' });

  let planFromAI: CodePlanFromAI;
  try {
    planFromAI = await runPlanning(sanitizedNodes, framework, model, provider, abortSignal);
    onProgress({ step: 'planning', status: 'done', plan: planFromAI });
  } catch (err) {
    if (abortSignal?.aborted) throw err;
    // Retry once with stricter prompt
    try {
      planFromAI = await runPlanning(sanitizedNodes, framework, model, provider, abortSignal, true);
      onProgress({ step: 'planning', status: 'done', plan: planFromAI });
    } catch (retryErr) {
      const msg = retryErr instanceof Error ? retryErr.message : 'Planning failed';
      onProgress({ step: 'planning', status: 'failed', error: msg });
      onProgress({ step: 'error', message: msg });
      throw retryErr;
    }
  }

  // Hydrate plan with actual node data
  const execPlan = hydratePlan(planFromAI, sanitizedNodes);
  if (execPlan.chunks.length === 0) {
    const msg = 'Planning produced no valid chunks';
    onProgress({ step: 'planning', status: 'failed', error: msg });
    onProgress({ step: 'error', message: msg });
    throw new Error(msg);
  }

  // Initialize all chunks as pending
  for (const chunk of execPlan.chunks) {
    onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'pending' });
  }

  // ── Step 2: Parallel Chunk Generation ──
  const results = new Map<string, ChunkResult>();
  const statuses = new Map<string, ChunkStatus>();

  // Group by execution order
  const maxOrder = Math.max(...execPlan.chunks.map((c) => c.order));

  for (let order = 0; order <= maxOrder; order++) {
    if (abortSignal?.aborted) throw new Error('Aborted');

    const batch = execPlan.chunks.filter((c) => c.order === order);
    const batchPromises = batch.map(async (chunk) => {
      // Check if dependencies failed
      const depsFailed = chunk.dependencies.some((depId) => statuses.get(depId) === 'failed');
      if (depsFailed) {
        statuses.set(chunk.id, 'skipped');
        onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'skipped' });
        return;
      }

      // Collect dependency contracts
      const depContracts: ChunkContract[] = chunk.dependencies
        .map((depId) => results.get(depId)?.contract)
        .filter((c): c is ChunkContract => c !== undefined && c.componentName !== '');

      onProgress({ step: 'chunk', chunkId: chunk.id, name: chunk.name, status: 'running' });

      try {
        const assetHints = collectChunkAssetHints(chunk.nodes, assets);
        const result = await runChunkGeneration(
          chunk.nodes,
          framework,
          chunk.suggestedComponentName,
          depContracts,
          chunk.id,
          model,
          provider,
          abortSignal,
          assetHints,
        );

        // Ensure componentName is valid PascalCase — AI may return kebab-case or empty
        if (
          !result.contract.componentName ||
          !/^[A-Z][a-zA-Z0-9]*$/.test(result.contract.componentName)
        ) {
          result.contract.componentName = sanitizeName(chunk.suggestedComponentName);
        }

        const validation = validateContract(result);
        if (validation.valid) {
          results.set(chunk.id, result);
          statuses.set(chunk.id, 'done');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'done',
            result,
          });
        } else {
          // Contract invalid — mark degraded
          results.set(chunk.id, result);
          statuses.set(chunk.id, 'degraded');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'degraded',
            result,
            error: validation.issues.join('; ') || 'Chunk contract validation failed',
          });
        }
      } catch {
        // Retry once
        try {
          const assetHints = collectChunkAssetHints(chunk.nodes, assets);
          const result = await runChunkGeneration(
            chunk.nodes,
            framework,
            chunk.suggestedComponentName,
            depContracts,
            chunk.id,
            model,
            provider,
            abortSignal,
            assetHints,
          );
          if (
            !result.contract.componentName ||
            !/^[A-Z][a-zA-Z0-9]*$/.test(result.contract.componentName)
          ) {
            result.contract.componentName = sanitizeName(chunk.suggestedComponentName);
          }
          results.set(chunk.id, result);
          const validation = validateContract(result);
          statuses.set(chunk.id, validation.valid ? 'done' : 'degraded');
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: statuses.get(chunk.id)!,
            result,
            ...(validation.valid
              ? {}
              : { error: validation.issues.join('; ') || 'Chunk contract validation failed' }),
          });
        } catch (retryErr) {
          statuses.set(chunk.id, 'failed');
          const msg = retryErr instanceof Error ? retryErr.message : 'Chunk generation failed';
          onProgress({
            step: 'chunk',
            chunkId: chunk.id,
            name: chunk.name,
            status: 'failed',
            error: msg,
          });
        }
      }
    });

    await Promise.all(batchPromises);
  }

  // ── Step 3: Assembly ──
  onProgress({ step: 'assembly', status: 'running' });

  const chunkInputs = execPlan.chunks.map((chunk) => {
    const status = statuses.get(chunk.id);
    const result = results.get(chunk.id);
    return {
      chunkId: chunk.id,
      name: chunk.name,
      code: result?.code ?? '',
      contract: result?.contract,
      status: (status === 'done' ? 'successful' : status === 'degraded' ? 'degraded' : 'failed') as
        | 'successful'
        | 'degraded'
        | 'failed',
    };
  });

  const hasAnyCode = chunkInputs.some((c) => c.code.length > 0);
  if (!hasAnyCode) {
    const msg = 'All chunks failed — no code to assemble';
    onProgress({ step: 'assembly', status: 'failed', error: msg });
    onProgress({ step: 'error', message: msg });
    throw new Error(msg);
  }

  let finalCode: string;
  let degraded = chunkInputs.some((c) => c.status !== 'successful');
  const exportedAssetPaths = assets.map((asset) => asset.relativePath);

  try {
    finalCode = await runAssembly(
      chunkInputs,
      planFromAI,
      framework,
      variables,
      model,
      provider,
      abortSignal,
      exportedAssetPaths,
    );
    onProgress({ step: 'assembly', status: 'done' });
  } catch {
    // Retry once
    try {
      finalCode = await runAssembly(
        chunkInputs,
        planFromAI,
        framework,
        variables,
        model,
        provider,
        abortSignal,
        exportedAssetPaths,
      );
      onProgress({ step: 'assembly', status: 'done' });
    } catch {
      // Best-effort fallback: concatenate chunk codes
      finalCode = chunkInputs
        .filter((c) => c.code)
        .map((c) => `// ── ${c.name} (${c.status}) ──\n\n${c.code}`)
        .join('\n\n');
      degraded = true;
      onProgress({
        step: 'assembly',
        status: 'failed',
        error: 'Assembly failed — showing concatenated chunks',
      });
    }
  }

  onProgress({ step: 'complete', finalCode, degraded });
  return { code: finalCode, degraded, assets };
}

// ── Internal AI call wrappers ──

/**
 * Collect all text content from a streamChat call.
 * Adapts the AIStreamChunk-based generator to return a plain string.
 * Throws on error chunks from the stream.
 */
async function collectStreamText(
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
    undefined, // options
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

async function runPlanning(
  nodes: PenNode[],
  framework: Framework,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  strict?: boolean,
): Promise<CodePlanFromAI> {
  const { system, user } = buildPlanningPrompt(nodes, framework);
  const systemPrompt = strict
    ? system + '\n\nCRITICAL: Respond with ONLY valid JSON. No markdown, no explanation.'
    : system;

  const fullResponse = await collectStreamText(systemPrompt, user, model, provider, abortSignal);

  // Extract JSON from response
  const jsonMatch = fullResponse.match(/\{[\s\S]*\}/);
  if (!jsonMatch) throw new Error('No JSON found in planning response');

  const plan = JSON.parse(jsonMatch[0]) as CodePlanFromAI;
  if (!plan.chunks || !plan.rootLayout) {
    throw new Error('Planning response missing required fields (chunks, rootLayout)');
  }
  plan.sharedStyles = plan.sharedStyles ?? [];

  return plan;
}

async function runChunkGeneration(
  nodes: PenNode[],
  framework: Framework,
  suggestedComponentName: string,
  depContracts: ChunkContract[],
  chunkId: string,
  model: string,
  provider: string | undefined,
  abortSignal?: AbortSignal,
  assetHints: CodegenAssetHint[] = [],
): Promise<ChunkResult> {
  const { system, user } = buildChunkPrompt(
    nodes,
    framework,
    suggestedComponentName,
    depContracts,
    assetHints,
  );

  const fullResponse = await collectStreamText(system, user, model, provider, abortSignal);

  return parseChunkResponse(fullResponse, chunkId);
}

async function runAssembly(
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
): Promise<string> {
  const { system, user } = buildAssemblyPrompt(
    chunkResults,
    plan,
    framework,
    variables,
    exportedAssetPaths,
  );

  const fullResponse = await collectStreamText(system, user, model, provider, abortSignal);

  return cleanCode(fullResponse);
}
