import { getSkillByName } from '@zseven-w/pen-ai-skills';
import type { Framework, ChunkContract, CodePlanFromAI } from '@zseven-w/pen-types';
import type { PenNode } from '@zseven-w/pen-types';
import { nodeTreeToSummary } from '@zseven-w/pen-core';
import type { CodegenAssetHint } from './codegen-assets';
import type { CodegenDesignIR } from './codegen-design-ir';
import type { CodegenQualityIssue } from '@zseven-w/pen-types';

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
  designIR?: CodegenDesignIR,
): {
  system: string;
  user: string;
} {
  const planningSkill = loadSkill('codegen-planning');
  const summary = nodeTreeToSummary(nodes);
  const designIRSection = designIR
    ? [
        '',
        'Design IR:',
        JSON.stringify(designIR),
        '',
        'Use the Design IR as the source of truth for layout, text, assets, semantic hints, and responsive intent.',
      ].join('\n')
    : '';

  return {
    system: [
      planningSkill,
      '',
      'Production codegen target: preserve all visible text and exported assets, keep layout faithful to the design, and plan components that can ship with minimal manual rework.',
    ].join('\n'),
    user: [
      `Target framework: ${framework}`,
      '',
      'Node tree:',
      summary,
      designIRSection,
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
  designIR?: CodegenDesignIR,
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
  const frameworkProductionInstruction =
    framework === 'html'
      ? [
          'HTML requirements: output semantic, production-ready HTML/CSS for this chunk. Preserve exact text, image paths, colors, spacing, and responsive behavior. Do not emit placeholder copy.',
        ].join('\n')
      : framework === 'vue'
        ? [
            'Vue requirements: output Vue 3 SFC-compatible code with a clear template and styles. Preserve exact text, image paths, colors, spacing, and responsive behavior. Do not emit placeholder copy.',
          ].join('\n')
        : framework === 'uniapp'
          ? [
              'UniApp requirements: output Vue-compatible UniApp code using view/text/image semantics where appropriate. Preserve exact text, image paths, mobile layout, and rpx/px intent. Do not emit placeholder copy.',
            ].join('\n')
          : 'Preserve exact visual structure, text, assets, and spacing.';
  const designIRSection = designIR
    ? [
        '',
        '## Design IR Context',
        JSON.stringify(designIR),
        '',
        'Use semanticHints only as weak hints. If a hint conflicts with visual facts, follow the visual facts.',
      ].join('\n')
    : '';

  return {
    system: [
      chunkSkill,
      '',
      'Production delivery rules:',
      '- Preserve every visible design text string in the generated UI.',
      '- Preserve exported asset paths exactly; never replace them with stock placeholders.',
      '- Keep layout, spacing, colors, typography, radius, and shadows as close to the design as possible.',
      '- Do not generalize the design into a generic template.',
      '- Do not output explanations or markdown fences.',
      '',
      frameworkProductionInstruction,
      '',
      '---',
      '',
      frameworkSkill,
    ].join('\n'),
    user: [
      `生成一个名为 "${suggestedComponentName}" 的 ${framework} component。`,
      '',
      'Nodes (JSON):',
      compactNodes(nodes),
      designIRSection,
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
  designIR?: CodegenDesignIR,
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
  const productionInstruction =
    framework === 'html'
      ? [
          'HTML final output requirements:',
          '- Return a complete runnable HTML document with <!doctype html>, <html>, <head>, <body>, and CSS.',
          '- Use CSS variables where design tokens are available.',
          '- Preserve all visible text and asset references.',
          '- Include responsive rules for mobile and desktop when layout facts imply them.',
        ].join('\n')
      : framework === 'vue'
        ? [
            'Vue final output requirements:',
            '- Return a production-ready Vue 3 SFC unless multiple files are explicitly required.',
            '- Include <template> and <style>; include <script setup> only when useful.',
            '- Preserve all visible text and asset references.',
            '- Keep component structure maintainable and directly usable in a Vue app.',
          ].join('\n')
        : framework === 'uniapp'
          ? [
              'UniApp final output requirements:',
              '- Return a complete production-ready UniApp multi-file bundle.',
              '- Use valid pages.json routes and matching pages/**/*.vue files.',
              '- Preserve all visible text and asset references.',
              exportedAssetPaths.length > 0
                ? `- Preserve these asset paths exactly wherever the corresponding image appears: ${exportedAssetPaths.join(', ')}.`
                : '- Preserve all exported asset paths exactly where images appear.',
              '- For UniApp image nodes, use <image src="./assets/hero.png" mode="aspectFill" /> style references with the exact relative path. Do not replace local assets with remote URLs, base64, empty strings, or generated placeholder names.',
              '- For CSS background images, use url("./assets/hero.png") with the exact relative path.',
              '- Keep mobile layout faithful and avoid web-only DOM APIs.',
            ].join('\n')
          : 'Return production-ready code that preserves the design.';
  const designIRSection = designIR
    ? ['## Design IR', JSON.stringify(designIR), ''].join('\n')
    : '';

  return {
    system: [
      assemblySkill,
      '',
      'Production delivery rules:',
      '- Final code must be directly usable, not a sketch.',
      '- Do not drop design text, images, or key sections.',
      '- Do not output markdown fences or explanations.',
      productionInstruction,
      '',
      '---',
      '',
      frameworkSkill,
    ].join('\n'),
    user: [
      assemblyIntro,
      '',
      designIRSection,
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

export function buildDirectGenerationPrompt(input: {
  nodes: PenNode[];
  framework: Framework;
  variables?: Record<string, unknown>;
  exportedAssetPaths?: string[];
  designIR: CodegenDesignIR;
}): { system: string; user: string } {
  const frameworkSkill = loadSkill(`codegen-${input.framework}`);
  const assetRules = buildAssetPreservationRules(input.framework, input.exportedAssetPaths ?? []);
  const outputInstruction =
    input.framework === 'uniapp'
      ? [
          'Output a complete UniApp multi-file bundle using this delimiter format:',
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
        ].join('\n')
      : input.framework === 'vue'
        ? 'Output one complete Vue 3 SFC with <template> and <style>.'
        : 'Output one complete runnable HTML document with <!doctype html>, <head>, <body>, and CSS.';

  return {
    system: [
      'You are OpenPencil production direct generation worker.',
      'Use direct generation for simple single-page designs: no planning JSON, no chunk contracts, no markdown fences.',
      'Generate complete production-ready UI code that can be previewed or exported directly.',
      'Preserve all visible text, assets, layout structure, spacing, colors, typography, radius, and shadows.',
      'Do not generalize the design into a generic template and do not invent placeholder copy.',
      assetRules,
      outputInstruction,
      '',
      '---',
      '',
      frameworkSkill,
    ].join('\n'),
    user: [
      `Target framework: ${input.framework}`,
      '',
      'Design IR:',
      JSON.stringify(input.designIR),
      '',
      input.variables ? `Design variables: ${JSON.stringify(input.variables)}` : '',
      input.exportedAssetPaths?.length
        ? `Exported asset paths: ${input.exportedAssetPaths.join(', ')}`
        : '',
      '',
      'Nodes (JSON):',
      compactNodes(input.nodes),
      '',
      'Return only the final code or multi-file bundle.',
    ]
      .filter(Boolean)
      .join('\n'),
  };
}

export function buildRepairPrompt(input: {
  framework: Framework;
  code: string;
  files: Array<{ path: string; content: string }>;
  issues: CodegenQualityIssue[];
  designIR: CodegenDesignIR;
  exportedAssetPaths?: string[];
}): { system: string; user: string } {
  const frameworkSkill = loadSkill(`codegen-${input.framework}`);
  const multiFile = input.files.length > 1 || input.framework === 'uniapp';
  const outputInstruction = multiFile
    ? [
        'Return the complete repaired multi-file bundle.',
        'Use this exact delimiter format for every file:',
        '---FILE: path/to/file.ext---',
        '<complete file content>',
      ].join('\n')
    : 'Return the complete repaired source file only.';
  const assetRules = buildAssetPreservationRules(input.framework, input.exportedAssetPaths ?? []);

  return {
    system: [
      'You are OpenPencil production code repair worker.',
      'Repair generated UI code so it passes the quality gate and remains faithful to the design.',
      'Preserve working code unrelated to the listed issues.',
      'Preserve all visible design text and exported asset paths.',
      assetRules,
      'Do not return a diff. Do not explain the change. Do not output markdown fences.',
      outputInstruction,
      '',
      '---',
      '',
      frameworkSkill,
    ].join('\n'),
    user: [
      `Framework: ${input.framework}`,
      '',
      'Design IR:',
      JSON.stringify(input.designIR),
      '',
      input.exportedAssetPaths?.length
        ? `Exported asset paths: ${input.exportedAssetPaths.join(', ')}`
        : '',
      '',
      'Quality issues to fix:',
      JSON.stringify(input.issues),
      '',
      'Current generated files:',
      input.files.length > 0
        ? input.files.map((file) => `---FILE: ${file.path}---\n${file.content}`).join('\n\n')
        : input.code,
    ]
      .filter(Boolean)
      .join('\n'),
  };
}

function buildAssetPreservationRules(framework: Framework, assetPaths: string[]): string {
  const exactPaths = assetPaths.length > 0 ? assetPaths.join(', ') : './assets/...';
  if (framework === 'uniapp') {
    return [
      'Asset preservation rules:',
      `- You must preserve ${exactPaths} exactly; never rename, omit, normalize away "./", or replace local assets.`,
      '- Every image asset from Design IR/exported assets must appear in the final bundle where that image is visible.',
      '- Use UniApp <image> tags for image nodes, for example <image src="./assets/hero.png" mode="aspectFill" />.',
      '- Use CSS background references like url("./assets/hero.png") only when the design image is a background.',
      '- Do not use remote placeholder images, base64 data URLs, empty src values, or generated asset names.',
      assetPaths.length > 0
        ? `- Quality gate missing_asset issues must be fixed by adding references that preserve ${exactPaths} exactly.`
        : '- Quality gate missing_asset issues must be fixed by restoring the exact exported asset path.',
    ].join('\n');
  }
  return [
    'Asset preservation rules:',
    `- Preserve exported asset paths exactly: ${exactPaths}.`,
    '- Do not replace design assets with stock, remote placeholder, base64, or empty references.',
  ].join('\n');
}
