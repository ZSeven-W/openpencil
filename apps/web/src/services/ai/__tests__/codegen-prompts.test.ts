import { describe, expect, it } from 'vitest';
import type { ChunkContract, CodePlanFromAI } from '@zseven-w/pen-types';
import { buildAssemblyPrompt } from '../codegen-prompts';

const plan: CodePlanFromAI = {
  chunks: [],
  sharedStyles: [],
  rootLayout: { direction: 'vertical', gap: 16, responsive: true },
};

const contract: ChunkContract = {
  chunkId: 'chunk-1',
  componentName: 'HeroSection',
  exportedProps: [],
  slots: [],
  cssClasses: [],
  cssVariables: [],
  imports: [],
  outputFiles: ['components/HeroSection.vue'],
};

describe('buildAssemblyPrompt', () => {
  it('asks UniApp assembly to return a multi-file bundle', () => {
    const prompt = buildAssemblyPrompt(
      [
        {
          chunkId: 'chunk-1',
          name: 'Hero',
          code: '<template><view>Hero</view></template>',
          contract,
          status: 'successful',
        },
      ],
      plan,
      'uniapp',
    );

    expect(prompt.user).toContain('UniApp 多文件 bundle');
    expect(prompt.user).toContain('---FILE: App.vue---');
    expect(prompt.user).toContain('---FILE: pages/index/index.vue---');
    expect(prompt.user).toContain('必须包含 App.vue、main.ts、pages.json、manifest.json、uni.scss');
    expect(prompt.user).toContain('"outputFiles":["components/HeroSection.vue"]');
    expect(prompt.user).not.toContain('production-ready 单文件');
  });
});
