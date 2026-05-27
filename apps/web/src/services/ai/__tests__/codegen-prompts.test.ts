import { describe, expect, it } from 'vitest';
import type { ChunkContract, CodePlanFromAI } from '@zseven-w/pen-types';
import { buildAssemblyPrompt, buildChunkPrompt, buildRepairPrompt } from '../codegen-prompts';
import { buildCodegenDesignIR } from '../codegen-design-ir';
import type { PenNode } from '@zseven-w/pen-types';

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
      undefined,
      ['./assets/hero.png'],
    );

    expect(prompt.user).toContain('UniApp 多文件 bundle');
    expect(prompt.user).toContain('---FILE: App.vue---');
    expect(prompt.user).toContain('---FILE: pages/index/index.vue---');
    expect(prompt.user).toContain('必须包含 App.vue、main.ts、pages.json、manifest.json、uni.scss');
    expect(prompt.user).toContain('"outputFiles":["components/HeroSection.vue"]');
    expect(prompt.user).toContain('./assets/hero.png');
    expect(prompt.system).toContain('<image src="./assets/hero.png"');
    expect(prompt.user).not.toContain('production-ready 单文件');
  });

  it('adds production delivery constraints for HTML and preserves Design IR context', () => {
    const designIR = buildCodegenDesignIR([
      { id: 'title', type: 'text', content: 'Launch faster', width: 200, height: 32 } as PenNode,
    ]);
    const prompt = buildAssemblyPrompt(
      [
        {
          chunkId: 'chunk-1',
          name: 'Hero',
          code: '<section><h1>Launch faster</h1></section>',
          contract,
          status: 'successful',
        },
      ],
      plan,
      'html',
      undefined,
      [],
      designIR,
    );

    expect(prompt.system).toContain('complete runnable HTML document');
    expect(prompt.system).toContain('Preserve all visible text');
    expect(prompt.user).toContain('Design IR');
    expect(prompt.user).toContain('Launch faster');
  });

  it('asks chunk generation to keep production visual fidelity for Vue', () => {
    const prompt = buildChunkPrompt(
      [{ id: 'button', type: 'frame', name: 'Primary Button', width: 120, height: 44 } as PenNode],
      'vue',
      'PrimaryButton',
      [],
    );

    expect(prompt.system).toContain('Production delivery rules');
    expect(prompt.system).toContain('Preserve every visible design text');
    expect(prompt.system).toContain('Vue requirements');
  });

  it('builds a repair prompt that requires complete files instead of diffs', () => {
    const designIR = buildCodegenDesignIR([
      { id: 'title', type: 'text', content: 'Checkout', width: 120, height: 32 } as PenNode,
    ]);
    const prompt = buildRepairPrompt({
      framework: 'uniapp',
      code: '---FILE: App.vue---\n<template><view /></template>',
      files: [{ path: 'App.vue', content: '<template><view /></template>' }],
      designIR,
      issues: [
        {
          code: 'missing_asset',
          severity: 'error',
          message: 'Generated output is missing design asset: ./assets/checkout.png',
        },
      ],
      exportedAssetPaths: ['./assets/checkout.png'],
    });

    expect(prompt.system).toContain('Do not return a diff');
    expect(prompt.system).toContain('complete repaired multi-file bundle');
    expect(prompt.system).toContain('preserve ./assets/checkout.png exactly');
    expect(prompt.user).toContain('missing_asset');
    expect(prompt.user).toContain('./assets/checkout.png');
  });
});
