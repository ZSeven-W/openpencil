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
      '分析这个 node tree，并输出 JSON code generation plan。',
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
          '以下 components 来自 upstream chunks，可用。请 import 并使用它们：',
          '',
          ...depContracts.map(
            (c) =>
              `- \`${c.componentName}\` (chunk: ${c.chunkId}): props=[${c.exportedProps.map((p) => `${p.name}: ${p.type}`).join(', ')}], slots=[${c.slots.map((s) => s.name).join(', ')}], outputFiles=[${(c.outputFiles ?? []).join(', ')}]`,
          ),
        ].join('\n')
      : '';

  const assetSection =
    assetHints.length > 0
      ? [
          '',
          '## Exported Image Assets',
          '以下 image assets 从 design 导出。请直接把这些 relative paths 用作 src/background-image URLs。不要 inline base64。',
          '',
          ...assetHints.map(
            (asset) =>
              `- ${asset.relativePath} (${asset.sourceKind}, node: ${asset.sourceNodeName ?? asset.sourceNodeId})`,
          ),
        ].join('\n')
      : '';

  const contractInstruction =
    framework === 'uniapp'
      ? [
          '',
          'UniApp contract 需要额外包含 `outputFiles`，列出这个 chunk 预期贡献的文件路径，例如 ["components/HeroCard.vue"] 或 ["pages/index/index.vue"]。',
        ].join('\n')
      : '';

  return {
    system: [chunkSkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      `生成一个名为 "${suggestedComponentName}" 的 ${framework} component。`,
      '',
      'Nodes (JSON):',
      compactNodes(nodes),
      depSection,
      assetSection,
      contractInstruction,
      '',
      '先输出 code，然后输出 ---CONTRACT---，再输出 JSON contract。',
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
        return `### ${r.name} (FAILED)\n这个 chunk 生成失败。请插入 placeholder comment。`;
      }
      const contractNote =
        r.status === 'degraded'
          ? '\n*NOTE: 没有可用 contract。请从 code 中推断 component name 和 imports。*'
          : `\nContract: ${JSON.stringify(r.contract)}`;
      return `### ${r.name} (${r.status})\n\`\`\`\n${r.code}\n\`\`\`${contractNote}`;
    })
    .join('\n\n');

  const assetSection =
    exportedAssetPaths.length > 0
      ? [
          'Exported image assets 位于 ./assets/ 下。',
          '最终 code 中请保留所有已有 ./assets/... references 不变。',
          `Assets: ${exportedAssetPaths.join(', ')}`,
          '',
        ].join('\n')
      : '';

  const assemblyIntro =
    framework === 'uniapp'
      ? '将以下 uniapp code chunks 组装成一个 production-ready UniApp 多文件 bundle。'
      : `将以下 ${framework} code chunks 组装成一个 production-ready 单文件。`;
  const outputInstruction =
    framework === 'uniapp'
      ? [
          '输出多个文件，使用以下严格格式：',
          '---FILE: App.vue---',
          '<App.vue source>',
          '---FILE: main.ts---',
          '<main.ts source>',
          '---FILE: pages.json---',
          '<pages.json source>',
          '---FILE: manifest.json---',
          '<manifest.json source>',
          '---FILE: uni.scss---',
          '<uni.scss source>',
          '---FILE: pages/index/index.vue---',
          '<page source>',
          '',
          '必须包含 App.vue、main.ts、pages.json、manifest.json、uni.scss、至少一个 pages/.../index.vue。',
          '只输出最终 UniApp 多文件 bundle，不要解释。',
        ].join('\n')
      : '只输出最终 assembled source code。';

  return {
    system: [assemblySkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      assemblyIntro,
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
      outputInstruction,
    ]
      .filter(Boolean)
      .join('\n'),
  };
}
