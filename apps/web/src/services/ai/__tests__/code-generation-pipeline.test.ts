// apps/web/src/services/ai/__tests__/code-generation-pipeline.test.ts

import { describe, it, expect } from 'vitest';
import {
  generateCode,
  hydratePlan,
  computeExecutionOrder,
  parseChunkResponse,
} from '../code-generation-pipeline';
import type { CodePlanFromAI, PlannedChunk } from '@zseven-w/pen-types';
import type { PenNode } from '@zseven-w/pen-types';

describe('hydratePlan', () => {
  it('maps nodeIds to actual PenNode objects', () => {
    const nodes: PenNode[] = [
      { id: 'n1', type: 'frame', name: 'Header', x: 0, y: 0, width: 100, height: 50 } as PenNode,
      { id: 'n2', type: 'text', name: 'Title', x: 0, y: 0, width: 80, height: 20 } as PenNode,
    ];
    const plan: CodePlanFromAI = {
      chunks: [
        {
          id: 'c1',
          name: 'header',
          nodeIds: ['n1', 'n2'],
          role: 'navbar',
          suggestedComponentName: 'Header',
          dependencies: [],
        },
      ],
      sharedStyles: [],
      rootLayout: { direction: 'vertical', gap: 0, responsive: true },
    };

    const result = hydratePlan(plan, nodes);
    expect(result.chunks[0].nodes).toHaveLength(2);
    expect(result.chunks[0].nodes[0].id).toBe('n1');
  });

  it('strips chunks with no valid nodeIds', () => {
    const nodes: PenNode[] = [
      { id: 'n1', type: 'frame', name: 'Header', x: 0, y: 0, width: 100, height: 50 } as PenNode,
    ];
    const plan: CodePlanFromAI = {
      chunks: [
        {
          id: 'c1',
          name: 'header',
          nodeIds: ['n1'],
          role: 'navbar',
          suggestedComponentName: 'Header',
          dependencies: [],
        },
        {
          id: 'c2',
          name: 'ghost',
          nodeIds: ['nonexistent'],
          role: 'card',
          suggestedComponentName: 'Ghost',
          dependencies: [],
        },
      ],
      sharedStyles: [],
      rootLayout: { direction: 'vertical', gap: 0, responsive: true },
    };

    const result = hydratePlan(plan, nodes);
    expect(result.chunks).toHaveLength(1);
    expect(result.chunks[0].id).toBe('c1');
  });
});

describe('computeExecutionOrder', () => {
  it('assigns order 0 to chunks with no dependencies', () => {
    const chunks: PlannedChunk[] = [
      { id: 'c1', name: 'a', nodeIds: [], role: '', suggestedComponentName: 'A', dependencies: [] },
      { id: 'c2', name: 'b', nodeIds: [], role: '', suggestedComponentName: 'B', dependencies: [] },
    ];
    const orders = computeExecutionOrder(chunks);
    expect(orders.get('c1')).toBe(0);
    expect(orders.get('c2')).toBe(0);
  });

  it('assigns higher order to dependent chunks', () => {
    const chunks: PlannedChunk[] = [
      { id: 'c1', name: 'a', nodeIds: [], role: '', suggestedComponentName: 'A', dependencies: [] },
      {
        id: 'c2',
        name: 'b',
        nodeIds: [],
        role: '',
        suggestedComponentName: 'B',
        dependencies: ['c1'],
      },
      {
        id: 'c3',
        name: 'c',
        nodeIds: [],
        role: '',
        suggestedComponentName: 'C',
        dependencies: ['c2'],
      },
    ];
    const orders = computeExecutionOrder(chunks);
    expect(orders.get('c1')).toBe(0);
    expect(orders.get('c2')).toBe(1);
    expect(orders.get('c3')).toBe(2);
  });
});

describe('parseChunkResponse', () => {
  it('splits code and contract from ---CONTRACT--- separator', () => {
    const response = `export function NavBar() { return <nav /> }
---CONTRACT---
{"chunkId":"c1","componentName":"NavBar","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}`;

    const result = parseChunkResponse(response, 'c1');
    expect(result.code).toContain('NavBar');
    expect(result.contract.componentName).toBe('NavBar');
    expect(result.contract.chunkId).toBe('c1');
  });

  it('infers component name when no separator found', () => {
    const response = 'export function NavBar() { return <nav /> }';
    const result = parseChunkResponse(response, 'c1');
    expect(result.code).toContain('NavBar');
    expect(result.contract.componentName).toBe('NavBar');
  });

  it('extracts contract from markdown json block', () => {
    const response = `\`\`\`tsx
export function HeroSection() { return <div /> }
\`\`\`

\`\`\`json
{"componentName":"HeroSection","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}
\`\`\``;
    const result = parseChunkResponse(response, 'c2');
    expect(result.code).toContain('HeroSection');
    expect(result.contract.componentName).toBe('HeroSection');
    expect(result.contract.chunkId).toBe('c2');
  });

  it('preserves outputFiles in parsed chunk contracts', () => {
    const response = `<template><view>Home</view></template>
---CONTRACT---
{"chunkId":"c1","componentName":"HomePage","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[],"outputFiles":["pages/index/index.vue"]}`;

    const result = parseChunkResponse(response, 'c1');

    expect(result.contract.outputFiles).toEqual(['pages/index/index.vue']);
  });
});

describe('generateCode', () => {
  const node = {
    id: 'n1',
    type: 'frame',
    name: 'Home',
    x: 0,
    y: 0,
    width: 320,
    height: 640,
  } as PenNode;

  it('marks incomplete UniApp assembly as degraded', async () => {
    const events: unknown[] = [];
    const responses = [
      JSON.stringify({
        chunks: [
          {
            id: 'chunk-1',
            name: 'Home',
            nodeIds: ['n1'],
            role: 'page',
            suggestedComponentName: 'HomePage',
            dependencies: [],
          },
        ],
        sharedStyles: [],
        rootLayout: { direction: 'vertical', gap: 0, responsive: true },
      }),
      [
        '<template><view>Home</view></template>',
        '---CONTRACT---',
        '{"chunkId":"chunk-1","componentName":"HomePage","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}',
      ].join('\n'),
      [
        '---FILE: App.vue---',
        '<template><view /></template>',
        '---FILE: pages.json---',
        '{"pages":[{"path":"pages/index/index"}]}',
      ].join('\n'),
    ];
    const collectText = async () => responses.shift() ?? '';

    const result = await generateCode(
      [node],
      'uniapp',
      undefined,
      (event) => events.push(event),
      'test-model',
      'builtin',
      undefined,
      { collectText, fastPath: 'never' },
    );

    expect(result.degraded).toBe(true);
    expect(events).toContainEqual(
      expect.objectContaining({
        step: 'quality_check',
        status: 'failed',
        error: expect.stringContaining('UniApp output missing main.ts'),
      }),
    );
    expect(events).toContainEqual(expect.objectContaining({ step: 'complete', degraded: true }));
    expect(events).toContainEqual(
      expect.objectContaining({
        step: 'planning',
        status: 'done',
        designIR: expect.objectContaining({
          summary: expect.objectContaining({ nodeCount: expect.any(Number) }),
        }),
      }),
    );
    expect(events).toContainEqual(
      expect.objectContaining({
        step: 'assembly',
        status: 'done',
        code: expect.stringContaining('---FILE: App.vue---'),
        files: expect.arrayContaining([
          expect.objectContaining({ path: 'App.vue' }),
          expect.objectContaining({ path: 'pages.json' }),
        ]),
      }),
    );
  });

  it('runs quality check and auto-repairs missing design text before completing', async () => {
    const events: unknown[] = [];
    const responses = [
      JSON.stringify({
        chunks: [
          {
            id: 'chunk-1',
            name: 'Hero',
            nodeIds: ['n1'],
            role: 'section',
            suggestedComponentName: 'HeroSection',
            dependencies: [],
          },
        ],
        sharedStyles: [],
        rootLayout: { direction: 'vertical', gap: 0, responsive: true },
      }),
      [
        'function HeroSection(){return `<section class="hero"></section>`}',
        '---CONTRACT---',
        '{"chunkId":"chunk-1","componentName":"HeroSection","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}',
      ].join('\n'),
      '<!doctype html><html><head><style>.hero{padding:32px}</style></head><body><section class="hero"></section></body></html>',
      '<!doctype html><html><head><style>.hero{padding:32px}</style></head><body><section class="hero"><h1>Ship faster</h1></section></body></html>',
    ];
    const collectText = async () => responses.shift() ?? '';

    const result = await generateCode(
      [
        {
          ...node,
          children: [
            {
              id: 'title',
              type: 'text',
              content: 'Ship faster',
              width: 160,
              height: 32,
            } as PenNode,
          ],
        } as PenNode,
      ],
      'html',
      undefined,
      (event) => events.push(event),
      'test-model',
      'builtin',
      undefined,
      { collectText, fastPath: 'never' },
    );

    expect(result.degraded).toBe(false);
    expect(result.pipelineMode).toBe('direct_generation');
    expect(result.code).toContain('Ship faster');
    expect(result.qualityReport?.status).toBe('repaired');
    expect(result.repairAttempts).toHaveLength(1);
    expect(result.timing?.totalMs).toBeGreaterThanOrEqual(0);
    expect(events).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ step: 'quality_check', status: 'failed' }),
        expect.objectContaining({ step: 'repair', status: 'done', attempt: 1 }),
        expect.objectContaining({ step: 'final_validation', status: 'done' }),
        expect.objectContaining({ step: 'complete', qualityReport: expect.any(Object) }),
      ]),
    );
  });

  it('records provider call count and stage-level provider call timings', async () => {
    const responses = [
      JSON.stringify({
        chunks: [
          {
            id: 'chunk-1',
            name: 'Hero',
            nodeIds: ['n1'],
            role: 'section',
            suggestedComponentName: 'HeroSection',
            dependencies: [],
          },
        ],
        sharedStyles: [],
        rootLayout: { direction: 'vertical', gap: 0, responsive: true },
      }),
      [
        'function HeroSection(){return `<section><h1>Ship faster</h1></section>`}',
        '---CONTRACT---',
        '{"chunkId":"chunk-1","componentName":"HeroSection","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}',
      ].join('\n'),
      '<!doctype html><html><head><style>body{}</style></head><body><main><h1>Ship faster</h1></main></body></html>',
    ];
    const collectText = async () => responses.shift() ?? '';

    const result = await generateCode(
      [
        {
          ...node,
          children: [
            {
              id: 'title',
              type: 'text',
              content: 'Ship faster',
              width: 160,
              height: 32,
            } as PenNode,
          ],
        } as PenNode,
      ],
      'html',
      undefined,
      () => undefined,
      'test-model',
      'builtin',
      undefined,
      { collectText, fastPath: 'never', maxRepairAttempts: 0 },
    );

    expect(result.timing?.providerCallCount).toBe(3);
    expect(result.pipelineMode).toBe('full_pipeline');
    expect(result.timing?.providerCalls?.map((call) => call.stage)).toEqual([
      'planning',
      'chunk',
      'assembly',
    ]);
    expect(result.timing?.providerCalls?.[1]).toMatchObject({
      stage: 'chunk',
      chunkId: 'chunk-1',
      attempt: 1,
      provider: 'builtin',
      model: 'test-model',
    });
  });

  it('uses direct fast path for simple single-page codegen', async () => {
    const events: unknown[] = [];
    const calls: Array<{ system: string; user: string }> = [];
    const collectText = async (system: string, user: string) => {
      calls.push({ system, user });
      return '<!doctype html><html><head><style>body{}</style></head><body><main><h1>Ship faster</h1></main></body></html>';
    };

    const result = await generateCode(
      [
        {
          ...node,
          children: [
            {
              id: 'title',
              type: 'text',
              content: 'Ship faster',
              width: 160,
              height: 32,
            } as PenNode,
          ],
        } as PenNode,
      ],
      'html',
      undefined,
      (event) => events.push(event),
      'test-model',
      'builtin',
      undefined,
      { collectText, maxRepairAttempts: 0 },
    );

    expect(result.degraded).toBe(false);
    expect(result.code).toContain('Ship faster');
    expect(calls).toHaveLength(1);
    expect(calls[0].system).toContain('direct generation');
    expect(result.timing?.providerCallCount).toBe(1);
    expect(result.timing?.providerCalls?.map((call) => call.stage)).toEqual(['direct_generation']);
    expect(events).not.toContainEqual(expect.objectContaining({ step: 'planning', status: 'running' }));
    expect(events).toEqual(
      expect.arrayContaining([
        expect.objectContaining({ step: 'assembly', status: 'done' }),
        expect.objectContaining({ step: 'quality_check', status: 'done' }),
        expect.objectContaining({ step: 'complete' }),
      ]),
    );
  });

  it('can force the full planning/chunk/assembly pipeline when fast path is disabled', async () => {
    const responses = [
      JSON.stringify({
        chunks: [
          {
            id: 'chunk-1',
            name: 'Hero',
            nodeIds: ['n1'],
            role: 'section',
            suggestedComponentName: 'HeroSection',
            dependencies: [],
          },
        ],
        sharedStyles: [],
        rootLayout: { direction: 'vertical', gap: 0, responsive: true },
      }),
      [
        'function HeroSection(){return `<section><h1>Ship faster</h1></section>`}',
        '---CONTRACT---',
        '{"chunkId":"chunk-1","componentName":"HeroSection","exportedProps":[],"slots":[],"cssClasses":[],"cssVariables":[],"imports":[]}',
      ].join('\n'),
      '<!doctype html><html><head><style>body{}</style></head><body><main><h1>Ship faster</h1></main></body></html>',
    ];
    const collectText = async () => responses.shift() ?? '';

    const result = await generateCode(
      [
        {
          ...node,
          children: [
            {
              id: 'title',
              type: 'text',
              content: 'Ship faster',
              width: 160,
              height: 32,
            } as PenNode,
          ],
        } as PenNode,
      ],
      'html',
      undefined,
      () => undefined,
      'test-model',
      'builtin',
      undefined,
      { collectText, fastPath: 'never', maxRepairAttempts: 0 },
    );

    expect(result.timing?.providerCallCount).toBe(3);
    expect(result.timing?.providerCalls?.map((call) => call.stage)).toEqual([
      'planning',
      'chunk',
      'assembly',
    ]);
  });
});
