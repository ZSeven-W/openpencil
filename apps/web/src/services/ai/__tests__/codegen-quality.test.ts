import { describe, expect, it } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { buildCodegenDesignIR } from '../codegen-design-ir';
import { buildCodegenFiles } from '../codegen-files';
import { buildCodegenQualityReport } from '../codegen-quality';

describe('buildCodegenQualityReport', () => {
  it('reports missing design text and missing asset references', () => {
    const designIR = buildCodegenDesignIR([
      {
        id: 'page',
        type: 'frame',
        width: 1200,
        height: 800,
        children: [
          { id: 'heading', type: 'text', content: 'Launch dashboard', width: 200, height: 32 },
          { id: 'hero', type: 'image', src: './assets/hero.png', width: 320, height: 180 },
        ],
      } as PenNode,
    ]);
    const files = buildCodegenFiles({
      framework: 'html',
      code: '<!doctype html><html><head><style>body{}</style></head><body><main></main></body></html>',
    });

    const report = buildCodegenQualityReport({ framework: 'html', files, designIR });

    expect(report.status).toBe('failed');
    expect(report.issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code: 'missing_text' }),
        expect.objectContaining({ code: 'missing_asset' }),
      ]),
    );
    expect(report.summary.missingTextCount).toBe(1);
    expect(report.summary.missingAssetCount).toBe(1);
  });

  it('passes production-ready Vue output with preserved text', () => {
    const designIR = buildCodegenDesignIR([
      { id: 'title', type: 'text', content: 'Revenue overview', width: 240, height: 32 } as PenNode,
    ]);
    const files = buildCodegenFiles({
      framework: 'vue',
      code: [
        '<template><section class="panel"><h1>Revenue overview</h1></section></template>',
        '<style scoped>.panel{padding:24px}</style>',
      ].join('\n'),
    });

    const report = buildCodegenQualityReport({ framework: 'vue', files, designIR });

    expect(report.status).toBe('passed');
    expect(report.summary.errorCount).toBe(0);
  });

  it('treats multi-item Figma text as preserved when every item appears separately', () => {
    const designIR = buildCodegenDesignIR([
      {
        id: 'nav',
        type: 'text',
        content: 'Features   Pricing   Docs',
        width: 320,
        height: 24,
      } as PenNode,
    ]);
    const files = buildCodegenFiles({
      framework: 'html',
      code: [
        '<!doctype html><html><head><style>body{}</style></head><body>',
        '<nav><a>Features</a><a>Pricing</a><a>Docs</a></nav>',
        '</body></html>',
      ].join(''),
    });

    const report = buildCodegenQualityReport({ framework: 'html', files, designIR });

    expect(report.status).toBe('passed');
    expect(report.summary.missingTextCount).toBe(0);
  });

  it('accepts asset references with or without a leading dot slash', () => {
    const designIR = buildCodegenDesignIR([
      {
        id: 'hero',
        type: 'image',
        src: './assets/landing-dashboard.png',
        width: 320,
        height: 180,
      } as PenNode,
    ]);
    const files = buildCodegenFiles({
      framework: 'html',
      code: [
        '<!doctype html><html><head><style>body{}</style></head><body>',
        '<img src="assets/landing-dashboard.png" alt="Dashboard" />',
        '</body></html>',
      ].join(''),
    });

    const report = buildCodegenQualityReport({ framework: 'html', files, designIR });

    expect(report.status).toBe('passed');
    expect(report.summary.missingAssetCount).toBe(0);
  });

  it('fails malformed HTML output with missing required document sections', () => {
    const files = buildCodegenFiles({
      framework: 'html',
      code: '<html><head><style>.page{}</style></head><main>Broken page</main>',
    });

    const report = buildCodegenQualityReport({ framework: 'html', files });

    expect(report.status).toBe('failed');
    expect(report.issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code: 'html_missing_body' }),
        expect.objectContaining({ code: 'html_unbalanced_tag' }),
      ]),
    );
  });

  it('fails malformed Vue SFC output with empty template and unbalanced blocks', () => {
    const files = buildCodegenFiles({
      framework: 'vue',
      code: '<template>   </template><style scoped>.page{color:#111}',
    });

    const report = buildCodegenQualityReport({ framework: 'vue', files });

    expect(report.status).toBe('failed');
    expect(report.issues).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ code: 'vue_empty_template' }),
        expect.objectContaining({ code: 'vue_unbalanced_block' }),
      ]),
    );
  });
});
