// apps/web/src/services/ai/codegen-prompts.ts

import { getSkillByName } from '@zseven-w/pen-ai-skills';
import type { Framework, ChunkContract, CodePlanFromAI } from '@zseven-w/pen-types';
import type { PenNode } from '@zseven-w/pen-types';
import { nodeTreeToSummary } from '@zseven-w/pen-core';

function loadSkill(name: string): string {
  return getSkillByName(name)?.content ?? '';
}

/**
 * Build system prompt for Step 1: planning.
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

  return {
    system: [chunkSkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      `Generate a ${framework} component named "${suggestedComponentName}".`,
      '',
      'Nodes (JSON):',
      JSON.stringify(nodes, null, 2),
      depSection,
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

  return {
    system: [assemblySkill, '', '---', '', frameworkSkill].join('\n'),
    user: [
      `Assemble the following ${framework} code chunks into a single production-ready file.`,
      '',
      `Root layout: ${JSON.stringify(plan.rootLayout)}`,
      `Shared styles: ${JSON.stringify(plan.sharedStyles)}`,
      variables ? `Design variables: ${JSON.stringify(variables)}` : '',
      '',
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
