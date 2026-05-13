import type { Framework } from '@zseven-w/pen-types';
import type { PenNode } from '../../src/types/pen';
import type { CloudCodeGenerationDetail, SaveCodegenFileInput } from '../../src/types/cloud';
import { buildCodegenFiles } from '../../src/services/ai/codegen-files';
import type { CodegenTextCollector } from '../../src/services/ai/code-generation-pipeline';

export interface CodegenPatchInput {
  framework: Framework;
  nodes: PenNode[];
  instruction: string;
  baseGeneration: CloudCodeGenerationDetail;
  model: string;
  provider?: string;
  collectText: CodegenTextCollector;
}

export interface CodegenPatchResult {
  code: string;
  files: SaveCodegenFileInput[];
  degraded: boolean;
}

function formatBaseFiles(baseGeneration: CloudCodeGenerationDetail): string {
  const files = baseGeneration.files?.length
    ? baseGeneration.files
    : buildCodegenFiles({
        framework: baseGeneration.framework,
        code: baseGeneration.finalCode ?? '',
      });

  return files
    .map((file) => [`---FILE: ${file.path}---`, file.content].join('\n'))
    .join('\n\n');
}

function cleanCode(raw: string): string {
  return raw
    .replace(/^```[a-zA-Z0-9_-]*\n?/gm, '')
    .replace(/```\s*$/gm, '')
    .trim();
}

function buildPatchedFiles(input: {
  framework: Framework;
  code: string;
  baseGeneration: CloudCodeGenerationDetail;
}): SaveCodegenFileInput[] {
  const parsed = buildCodegenFiles({ framework: input.framework, code: input.code });
  if (parsed.length > 1 || input.framework === 'uniapp') return parsed;
  if ((input.baseGeneration.files?.length ?? 0) <= 1) return parsed;

  return [
    {
      path: input.baseGeneration.entryFile ?? parsed[0]?.path ?? 'design.txt',
      language: parsed[0]?.language ?? 'text',
      role: parsed[0]?.role ?? 'entry',
      content: input.code,
      orderIndex: 0,
    },
  ];
}

export async function generateCodePatch(input: CodegenPatchInput): Promise<CodegenPatchResult> {
  const baseFiles = formatBaseFiles(input.baseGeneration);
  const selectedNodesJson = JSON.stringify(input.nodes);
  const isMultiFile =
    input.framework === 'uniapp' || (input.baseGeneration.files?.length ?? 0) > 1;
  const outputInstruction = isMultiFile
    ? [
        'Return the complete updated multi-file bundle.',
        'Use this exact delimiter format for every file:',
        '---FILE: path/to/file.ext---',
        '<complete file content>',
      ].join('\n')
    : 'Return the complete updated source file only. Do not return a diff.';

  const system = [
    'You are OpenPencil code patch worker.',
    'Patch an existing generated UI implementation using the selected design nodes and the user instruction.',
    'Preserve working code that is unrelated to the selected nodes.',
    'Do not explain the change. Do not output markdown fences.',
    outputInstruction,
  ].join('\n');

  const user = [
    `Framework: ${input.framework}`,
    '',
    'User patch instruction:',
    input.instruction,
    '',
    'Selected design nodes JSON:',
    selectedNodesJson,
    '',
    'Base generated code:',
    baseFiles,
  ].join('\n');

  const code = cleanCode(
    await input.collectText(system, user, input.model, input.provider),
  );
  const files = buildPatchedFiles({
    framework: input.framework,
    code,
    baseGeneration: input.baseGeneration,
  });
  return { code, files, degraded: false };
}
