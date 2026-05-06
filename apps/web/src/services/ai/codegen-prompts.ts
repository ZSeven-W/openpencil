import { getSkillByName } from '@zseven-w/pen-ai-skills';
import type { Framework, ChunkContract, CodePlanFromAI } from '@zseven-w/pen-types';
import type { PenNode } from '@zseven-w/pen-types';
import { nodeTreeToSummary } from '@zseven-w/pen-core';
import type { CodegenAssetHint } from './codegen-assets';

function loadSkill(name: string): string {
  return getSkillByName(name)?.content ?? '';
}

/**
 * Strip 不影响代码生
 * 成的字段。 Keeps 请求主体足够小，代理不会使用 403/413 拒绝它们，并减少输入令牌。
 */
function stripNoise(input: unknown): unknown {
  if (Array.isArray(input)) return input.map(stripNoise);
  if (!input || typeof input !== 'object') return input;
  const obj = input as Record<string, unknown>;
  const out: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(obj)) {
    // Drop 生成器不需要的字段： - id （模型选择自己的名称） -
    // parentId / pageId （嵌套中隐含树结构） -
    // 自动布局子项上的 x/y （布局引擎定位它们） - 我们通常无法翻译的效果（跳过嘈杂的嵌套道具） - 默认时为
    // rotation/opacity/visible
    if (k === 'id' || k === 'parentId' || k === 'pageId' || k === '_meta') continue;
    if (k === 'rotation' && v === 0) continue;
    if (k === 'opacity' && v === 1) continue;
    if (k === 'visible' && v === true) continue;
    out[k] = stripNoise(v);
  }
  return out;
}

/** Compact JSON 用于 AI 提示（无缩进，消除噪音场）。 */
function compactNodes(nodes: PenNode[]): string {
  return JSON.stringify(stripNoise(nodes));
}

/**
 * Build 系统提示 Step 1：规划。
 */
export function buildPlanningPrompt(
  nodes: PenNode[],
  framework: Framework,
): {
  system: string;
  user: string;
} {
  const planningSkill = loadSkill('codegen-planning');
  const summary = nodeTreeToSummary(nodes);

  return {
    system: planningSkill,
    user: [
      `Target framework: ${framework}`,
      '',
      'Node tree:',
      summary,
      '',
      'Analyze this node tree and output a JSON code generation plan.',
    ].join('\n'),
  };
}

/**
 * Build system prompt for Step 2: chunk generation.
 */
export function buildChunkPrompt(
  nodes: PenNode[],
  framework: Framework,
  suggestedComponentName: string,
  depContracts: ChunkContract[],
  assetHints: CodegenAssetHint[] = [],
): { system: string; user: string } {
  const chunkSkill = loadSkill('codegen-chunk');
  const frameworkSkill = loadSkill(`codegen-${framework}`);

  const depSection =
    depContracts.length > 0
      ? [
          '',
          '## Dependency Contracts',
          'The following components are available from upstream chunks. Import and use them:',
          '',
          ...depContracts.map(
            (c) =>
              `- \`${c.componentName}\` (chunk: ${c.chunkId}): props=[${c.exportedProps.map((p) => `${p.name}: ${p.type}`).join(', ')}], slots=[${c.slots.map((s) => s.name).join(', ')}]`,
          ),
        ].join('\n')
      : '';

  const assetSection =
    assetHints.length > 0
      ? [
          '',
          '## Exported Image Assets',
          'The following image assets were exported from the design. Use these relative paths directly as src/background-image URLs. Do NOT inline base64.',
          '',
          ...assetHints.map(
            (asset) =>
              `- ${asset.relativePath} (${asset.sourceKind}, node: ${asset.sourceNodeName ?? asset.sourceNodeId})`,
          ),
        ].join('\n')
      : '';

  return {
    system: [chunkSkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      `Generate a ${framework} component named "${suggestedComponentName}".`,
      '',
      'Nodes (JSON):',
      compactNodes(nodes),
      depSection,
      assetSection,
      '',
      'Output the code followed by ---CONTRACT--- and the JSON contract.',
    ].join('\n'),
  };
}

/**
 * Build system prompt for Step 3: assembly.
 */
export function buildAssemblyPrompt(
  chunkResults: {
    chunkId: string;
    name: string;
    code: string;
    contract?: ChunkContract;
    status: 'successful' | 'degraded' | 'failed';
  }[],
  plan: CodePlanFromAI,
  framework: Framework,
  variables?: Record<string, unknown>,
  exportedAssetPaths: string[] = [],
): { system: string; user: string } {
  const assemblySkill = loadSkill('codegen-assembly');
  const frameworkSkill = loadSkill(`codegen-${framework}`);

  const chunksSection = chunkResults
    .map((r) => {
      if (r.status === 'failed') {
        return `### ${r.name} (FAILED)\nThis chunk failed to generate. Insert a placeholder comment.`;
      }
      const contractNote =
        r.status === 'degraded'
          ? '\n*NOTE: No contract available. Infer component name and imports from the code.*'
          : `\nContract: ${JSON.stringify(r.contract)}`;
      return `### ${r.name} (${r.status})\n\`\`\`\n${r.code}\n\`\`\`${contractNote}`;
    })
    .join('\n\n');

  const assetSection =
    exportedAssetPaths.length > 0
      ? [
          'Exported image assets are available under ./assets/.',
          'Keep any existing ./assets/... references unchanged in the final code.',
          `Assets: ${exportedAssetPaths.join(', ')}`,
          '',
        ].join('\n')
      : '';

  return {
    system: [assemblySkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      `Assemble the following ${framework} code chunks into a single production-ready file.`,
      '',
      `Root layout: ${JSON.stringify(plan.rootLayout)}`,
      `Shared styles: ${JSON.stringify(plan.sharedStyles)}`,
      variables ? `Design variables: ${JSON.stringify(variables)}` : '',
      '',
      assetSection,
      '## Chunks',
      '',
      chunksSection,
      '',
      'Output ONLY the final assembled source code.',
    ]
      .filter(Boolean)
      .join('\n'),
  };
}
