import type { Framework, PenNode } from '@zseven-w/pen-types';
import { buildCodegenDesignIR } from './codegen-design-ir';

export type CodegenBenchmarkCaseKind =
  | 'landing_page'
  | 'dashboard'
  | 'form_detail'
  | 'mobile_screen'
  | 'card_list';

export interface CodegenBenchmarkCase {
  id: string;
  name: string;
  kind: CodegenBenchmarkCaseKind;
  description: string;
  nodes: PenNode[];
}

export const BENCHMARK_FRAMEWORKS: Framework[] = ['html', 'vue', 'uniapp'];

export function getCodegenBenchmarkFrameworks(): Framework[] {
  return [...BENCHMARK_FRAMEWORKS];
}

function fill(color: string, opacity?: number) {
  return [{ type: 'solid' as const, color, opacity }];
}

function shadow(color = '#00000020') {
  return [{ type: 'shadow' as const, offsetX: 0, offsetY: 8, blur: 24, spread: 0, color }];
}

function textNode(input: {
  id: string;
  content: string;
  x?: number;
  y?: number;
  width?: number;
  fontSize?: number;
  fontWeight?: number;
  color?: string;
  name?: string;
}): PenNode {
  return {
    id: input.id,
    type: 'text',
    name: input.name,
    content: input.content,
    x: input.x,
    y: input.y,
    width: input.width ?? 240,
    fontSize: input.fontSize ?? 16,
    fontWeight: input.fontWeight ?? 400,
    fill: fill(input.color ?? '#111827'),
  };
}

function card(input: {
  id: string;
  name: string;
  x?: number;
  y?: number;
  width?: number;
  height?: number;
  children: PenNode[];
}): PenNode {
  return {
    id: input.id,
    type: 'frame',
    name: input.name,
    role: 'card',
    x: input.x,
    y: input.y,
    width: input.width ?? 280,
    height: input.height ?? 180,
    layout: 'vertical',
    gap: 12,
    padding: [20, 20, 20, 20],
    cornerRadius: 8,
    fill: fill('#ffffff'),
    stroke: { thickness: 1, fill: fill('#e5e7eb') },
    effects: shadow(),
    children: input.children,
  };
}

function button(id: string, label: string, color = '#2563eb'): PenNode {
  return {
    id,
    type: 'frame',
    name: `${label} Button`,
    role: 'button',
    width: 160,
    height: 48,
    layout: 'horizontal',
    justifyContent: 'center',
    alignItems: 'center',
    cornerRadius: 8,
    fill: fill(color),
    children: [
      textNode({
        id: `${id}-label`,
        content: label,
        width: 120,
        fontSize: 15,
        fontWeight: 700,
        color: '#ffffff',
      }),
    ],
  };
}

function page(input: {
  id: string;
  name: string;
  width: number;
  height: number;
  children: PenNode[];
  fillColor?: string;
}): PenNode {
  return {
    id: input.id,
    type: 'frame',
    name: input.name,
    width: input.width,
    height: input.height,
    layout: 'vertical',
    gap: 28,
    padding: input.width <= 480 ? [28, 20, 28, 20] : [36, 48, 36, 48],
    fill: fill(input.fillColor ?? '#f8fafc'),
    children: input.children,
  };
}

export function getCodegenBenchmarkCases(): CodegenBenchmarkCase[] {
  return [
    {
      id: 'landing-page',
      name: 'SaaS 落地页',
      kind: 'landing_page',
      description: '包含导航、hero、功能卡片、CTA 和真实图片资产的桌面落地页。',
      nodes: [
        page({
          id: 'landing-root',
          name: 'SaaS Landing Page',
          width: 1200,
          height: 900,
          children: [
            {
              id: 'landing-nav',
              type: 'frame',
              name: 'Navbar',
              role: 'navbar',
              width: 1104,
              height: 64,
              layout: 'horizontal',
              justifyContent: 'space_between',
              alignItems: 'center',
              children: [
                textNode({ id: 'landing-logo', content: 'OpenFlow', fontSize: 22, fontWeight: 800 }),
                textNode({ id: 'landing-links', content: 'Features   Pricing   Docs', width: 320 }),
                button('landing-nav-cta', 'Start free'),
              ],
            },
            {
              id: 'landing-hero',
              type: 'frame',
              name: 'Hero Section',
              width: 1104,
              height: 520,
              layout: 'horizontal',
              gap: 48,
              alignItems: 'center',
              children: [
                {
                  id: 'landing-hero-copy',
                  type: 'frame',
                  name: 'Hero Copy',
                  width: 520,
                  layout: 'vertical',
                  gap: 18,
                  children: [
                    textNode({
                      id: 'landing-eyebrow',
                      content: 'Production workflow for AI teams',
                      width: 420,
                      fontSize: 14,
                      fontWeight: 700,
                      color: '#0f766e',
                    }),
                    textNode({
                      id: 'landing-title',
                      content: 'Design, generate, repair, and ship UI code faster',
                      width: 520,
                      fontSize: 52,
                      fontWeight: 800,
                    }),
                    textNode({
                      id: 'landing-subtitle',
                      content:
                        'Turn imported design files into production-ready Vue, HTML, and UniApp bundles with quality gates.',
                      width: 500,
                      fontSize: 18,
                      color: '#475569',
                    }),
                    button('landing-hero-cta', 'Generate code'),
                  ],
                },
                {
                  id: 'landing-hero-image',
                  type: 'image',
                  name: 'Dashboard preview image',
                  src: './assets/landing-dashboard.png',
                  width: 500,
                  height: 320,
                  cornerRadius: 12,
                },
              ],
            },
            {
              id: 'landing-features',
              type: 'frame',
              name: 'Feature Cards',
              width: 1104,
              layout: 'horizontal',
              gap: 20,
              children: [
                card({
                  id: 'landing-card-1',
                  name: 'Quality Card',
                  children: [
                    textNode({ id: 'landing-card-1-title', content: 'Quality reports', fontWeight: 800 }),
                    textNode({
                      id: 'landing-card-1-body',
                      content: 'Detect missing text, assets, unsafe code, and incomplete file bundles.',
                      color: '#64748b',
                    }),
                  ],
                }),
                card({
                  id: 'landing-card-2',
                  name: 'Repair Card',
                  children: [
                    textNode({ id: 'landing-card-2-title', content: 'Automatic repair', fontWeight: 800 }),
                    textNode({
                      id: 'landing-card-2-body',
                      content: 'Run targeted repair passes before saving degraded output to history.',
                      color: '#64748b',
                    }),
                  ],
                }),
              ],
            },
          ],
        }),
      ],
    },
    {
      id: 'dashboard',
      name: '运营 Dashboard',
      kind: 'dashboard',
      description: '覆盖侧边栏、指标卡、图表占位和表格文本的后台页面。',
      nodes: [
        page({
          id: 'dashboard-root',
          name: 'Operations Dashboard',
          width: 1200,
          height: 860,
          fillColor: '#f1f5f9',
          children: [
            textNode({ id: 'dashboard-title', content: 'Operations overview', fontSize: 32, fontWeight: 800 }),
            {
              id: 'dashboard-metrics',
              type: 'frame',
              name: 'Metric Cards',
              width: 1104,
              layout: 'horizontal',
              gap: 16,
              children: [
                card({
                  id: 'dashboard-metric-1',
                  name: 'Revenue Metric',
                  children: [
                    textNode({ id: 'dashboard-metric-1-label', content: 'Revenue', color: '#64748b' }),
                    textNode({ id: 'dashboard-metric-1-value', content: '$128,420', fontSize: 32, fontWeight: 800 }),
                  ],
                }),
                card({
                  id: 'dashboard-metric-2',
                  name: 'Conversion Metric',
                  children: [
                    textNode({ id: 'dashboard-metric-2-label', content: 'Conversion rate', color: '#64748b' }),
                    textNode({ id: 'dashboard-metric-2-value', content: '12.8%', fontSize: 32, fontWeight: 800 }),
                  ],
                }),
              ],
            },
            card({
              id: 'dashboard-table',
              name: 'Recent Orders Table',
              width: 1104,
              height: 260,
              children: [
                textNode({ id: 'dashboard-table-title', content: 'Recent orders', fontSize: 22, fontWeight: 800 }),
                textNode({
                  id: 'dashboard-table-head',
                  content: 'Customer        Plan        Status        Amount',
                  width: 760,
                  fontWeight: 700,
                  color: '#334155',
                }),
                textNode({
                  id: 'dashboard-table-row',
                  content: 'Acme Corp       Pro         Paid          $2,400',
                  width: 760,
                  color: '#475569',
                }),
              ],
            }),
          ],
        }),
      ],
    },
    {
      id: 'form-detail',
      name: '表单详情页',
      kind: 'form_detail',
      description: '覆盖详情标题、表单字段、状态卡和保存按钮。',
      nodes: [
        page({
          id: 'form-root',
          name: 'Customer Detail Form',
          width: 1200,
          height: 860,
          children: [
            textNode({ id: 'form-title', content: 'Customer profile', fontSize: 34, fontWeight: 800 }),
            card({
              id: 'form-card',
              name: 'Profile Form',
              width: 720,
              height: 420,
              children: [
                textNode({ id: 'form-name-label', content: 'Full name', fontWeight: 700 }),
                textNode({ id: 'form-name-input', content: 'Jane Cooper', width: 420 }),
                textNode({ id: 'form-email-label', content: 'Email address', fontWeight: 700 }),
                textNode({ id: 'form-email-input', content: 'jane.cooper@example.com', width: 420 }),
                textNode({ id: 'form-notes-label', content: 'Internal notes', fontWeight: 700 }),
                textNode({
                  id: 'form-notes-input',
                  content: 'Prefers monthly billing and email receipts.',
                  width: 520,
                }),
                button('form-save', 'Save changes', '#16a34a'),
              ],
            }),
          ],
        }),
      ],
    },
    {
      id: 'mobile-screen',
      name: '移动端首页',
      kind: 'mobile_screen',
      description: '覆盖移动端头部、搜索、卡片、底部操作区域。',
      nodes: [
        page({
          id: 'mobile-root',
          name: 'Mobile Finance Home',
          width: 390,
          height: 844,
          fillColor: '#f8fafc',
          children: [
            textNode({ id: 'mobile-greeting', content: 'Good morning, Lin', fontSize: 26, fontWeight: 800 }),
            card({
              id: 'mobile-balance',
              name: 'Balance Card',
              width: 350,
              height: 170,
              children: [
                textNode({ id: 'mobile-balance-label', content: 'Available balance', color: '#64748b' }),
                textNode({ id: 'mobile-balance-value', content: '$24,680.50', fontSize: 34, fontWeight: 800 }),
                textNode({ id: 'mobile-balance-sub', content: '+8.4% from last month', color: '#16a34a' }),
              ],
            }),
            card({
              id: 'mobile-action',
              name: 'Quick Action Card',
              width: 350,
              height: 150,
              children: [
                textNode({ id: 'mobile-action-title', content: 'Quick transfer', fontSize: 20, fontWeight: 800 }),
                textNode({
                  id: 'mobile-action-body',
                  content: 'Send money to saved contacts in seconds.',
                  color: '#64748b',
                }),
              ],
            }),
          ],
        }),
      ],
    },
    {
      id: 'card-list',
      name: '卡片列表页',
      kind: 'card_list',
      description: '覆盖列表标题、筛选标签、重复卡片和图片资产。',
      nodes: [
        page({
          id: 'list-root',
          name: 'Course Card List',
          width: 1200,
          height: 900,
          children: [
            textNode({ id: 'list-title', content: 'Recommended courses', fontSize: 34, fontWeight: 800 }),
            textNode({
              id: 'list-filters',
              content: 'All   Design   Frontend   Product',
              width: 420,
              color: '#475569',
            }),
            {
              id: 'list-grid',
              type: 'frame',
              name: 'Course Grid',
              width: 1104,
              layout: 'horizontal',
              gap: 20,
              children: [
                card({
                  id: 'list-card-1',
                  name: 'Design Systems Course',
                  children: [
                    {
                      id: 'list-card-1-image',
                      type: 'image',
                      name: 'Design course cover',
                      src: './assets/course-design.png',
                      width: 240,
                      height: 120,
                    },
                    textNode({ id: 'list-card-1-title', content: 'Design systems in practice', fontWeight: 800 }),
                    textNode({ id: 'list-card-1-meta', content: '12 lessons · Advanced', color: '#64748b' }),
                  ],
                }),
                card({
                  id: 'list-card-2',
                  name: 'Frontend Course',
                  children: [
                    {
                      id: 'list-card-2-image',
                      type: 'image',
                      name: 'Frontend course cover',
                      src: './assets/course-frontend.png',
                      width: 240,
                      height: 120,
                    },
                    textNode({ id: 'list-card-2-title', content: 'Vue production patterns', fontWeight: 800 }),
                    textNode({ id: 'list-card-2-meta', content: '18 lessons · Intermediate', color: '#64748b' }),
                  ],
                }),
              ],
            },
          ],
        }),
      ],
    },
  ];
}

export function makeBenchmarkBaselineCode(framework: Framework, benchmark: CodegenBenchmarkCase): string {
  const ir = buildCodegenDesignIR(benchmark.nodes);
  const title = ir.summary.textContent[0] ?? benchmark.name;
  const textHtml = ir.summary.textContent.map((text) => `<p>${text}</p>`).join('\n');
  const imagesHtml = ir.nodes
    .flatMap(function flatten(node): string[] {
      return [...node.assetRefs, ...(node.children ?? []).flatMap(flatten)];
    })
    .map((asset) => `<img src="${asset}" alt="" />`)
    .join('\n');

  if (framework === 'vue') {
    return [
      '<template>',
      '  <main class="page">',
      `    <h1>${title}</h1>`,
      textHtml
        .split('\n')
        .filter(Boolean)
        .map((line) => `    ${line}`)
        .join('\n'),
      imagesHtml
        .split('\n')
        .filter(Boolean)
        .map((line) => `    ${line}`)
        .join('\n'),
      '  </main>',
      '</template>',
      '<style scoped>',
      '.page { padding: 32px; font-family: Inter, sans-serif; color: #111827; }',
      'img { max-width: 100%; border-radius: 8px; }',
      '</style>',
    ].join('\n');
  }

  if (framework === 'uniapp') {
    const textViews = ir.summary.textContent.map((text) => `<text>${text}</text>`).join('\n');
    const imageViews = ir.nodes
      .flatMap(function flatten(node): string[] {
        return [...node.assetRefs, ...(node.children ?? []).flatMap(flatten)];
      })
      .map((asset) => `<image src="${asset}" mode="aspectFill" />`)
      .join('\n');
    return [
      '---FILE: App.vue---',
      '<template><view><slot /></view></template>',
      '---FILE: main.ts---',
      "import { createSSRApp } from 'vue';",
      "import App from './App.vue';",
      'export function createApp() { return { app: createSSRApp(App) }; }',
      '---FILE: pages.json---',
      '{"pages":[{"path":"pages/index/index","style":{"navigationBarTitleText":"Benchmark"}}]}',
      '---FILE: manifest.json---',
      '{"name":"OpenPencil Benchmark","appid":"__UNI__BENCHMARK"}',
      '---FILE: uni.scss---',
      '$page-bg: #f8fafc;',
      '---FILE: pages/index/index.vue---',
      '<template>',
      '  <view class="page">',
      `    ${textViews}`,
      imageViews
        .split('\n')
        .filter(Boolean)
        .map((line) => `    ${line}`)
        .join('\n'),
      '  </view>',
      '</template>',
      '<style scoped lang="scss">',
      '.page { padding: 32rpx; background: $page-bg; }',
      'text { display: block; margin-bottom: 20rpx; color: #111827; }',
      'image { display: block; width: 100%; height: 320rpx; border-radius: 16rpx; margin-bottom: 20rpx; }',
      '</style>',
    ].join('\n');
  }

  return [
    '<!doctype html>',
    '<html>',
    '<head>',
    '  <meta charset="utf-8" />',
    '  <meta name="viewport" content="width=device-width, initial-scale=1" />',
    `  <title>${title}</title>`,
    '  <style>:root{font-family:Inter,sans-serif;color:#111827}body{margin:0;padding:32px;background:#f8fafc}img{max-width:100%;border-radius:8px}</style>',
    '</head>',
    '<body>',
    `  <main>${textHtml}${imagesHtml}</main>`,
    '</body>',
    '</html>',
  ].join('\n');
}
