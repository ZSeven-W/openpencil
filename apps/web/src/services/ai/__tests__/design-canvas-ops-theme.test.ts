import { describe, it, expect, beforeEach, vi } from 'vitest';

// Mock canvas-text-measure 以避免测试中 CanvasKit WASM 依赖
vi.mock('@/canvas/canvas-text-measure', () => ({
  estimateLineWidth: () => 0,
  estimateTextHeight: () => 0,
  defaultLineHeight: () => 1.2,
  hasCjkText: () => false,
}));

// Mock image-search-pipeline 因此 upsertNodesToCanvas 不会尝试访问网络
vi.mock('../image-search-pipeline', () => ({
  scanAndFillImages: vi.fn(() => Promise.resolve()),
  enqueueImageForSearch: vi.fn(),
  resetImageSearchQueue: vi.fn(),
}));

import { useDocumentStore, DEFAULT_FRAME_ID } from '@/stores/document-store';
import {
  upsertNodesToCanvas,
  insertStreamingNode,
  resetGenerationRemapping,
} from '../design-canvas-ops';
import '../role-definitions/index';
import type { PenDocument, PenNode } from '@/types/pen';

/**
 * Regression
 *
 * 测试为主题检测路径 Codex 捕获。 The bug：`sanitizeNodesForUpsert` 的帮助程序
 * `detectActiv
 * eDocumentTheme()` 用于读取缓存的 `generationRootFrameId` 模块变量，该变量仅在协调器生成流程开始时由
 * `resetGenerationRemapping()` 填充。 For 直接 MCP/upsert 绕过该 init
 * 的调用路径，缓存的值是陈旧的或默认的 - 因此主题检测即使在黑暗的页面上也会默默地变成“浅色”，并且插入的导航栏将白色角色默认标记在顶部
 * 。 The 修复是在每次调用时读取 LIVE 活动页面主框架。 This 测试加载深色页面，立即调用
 * upsertNodesToCanvas （使用 NO 生成 init），并断言插入的导航栏已填充深色主题。
 *
 *
 *
 *
 *
 */

function loadDocument(doc: PenDocument): void {
  useDocumentStore.getState().applyExternalDocument(doc);
}

/**
 * Build 一个最小的深
 * 色主题文档。 The 根框架具有 ONE 占位符子级，因此 `isCanvasOnlyEmptyFrame()` 返回
 * false，并且 `upsertNodesToCanvas` 执行正常的 addNode
 * 路径（具有主题感知清理的路径），而不是空框架替换快捷方式。
 *
 */
function makeDarkPageDoc(): PenDocument {
  return {
    version: '1.0.0',
    variables: {},
    pages: [
      {
        id: 'page-1',
        name: 'Page 1',
        children: [
          {
            id: DEFAULT_FRAME_ID,
            type: 'frame',
            name: 'Root',
            x: 0,
            y: 0,
            width: 375,
            height: 800,
            fill: [{ type: 'solid', color: '#111111' }],
            children: [
              {
                id: 'placeholder-content',
                type: 'frame',
                name: 'Placeholder',
                width: 100,
                height: 40,
              } as unknown as PenNode,
            ],
          } as unknown as PenNode,
        ],
      },
    ],
    children: [],
  } as unknown as PenDocument;
}

describe('design-canvas-ops theme detection (regression)', () => {
  beforeEach(() => {
    useDocumentStore.getState().newDocument();
  });

  it('upsertNodesToCanvas detects DARK theme from the live active page even without resetGenerationRemapping()', () => {
    // Load 直接黑页。 We 故意执行 NOT 调用 resetGenerationRemapping() —
    // 这是 Codex 标记的路径（绕过 Orchestrator init 的直接 MCP 更新插入）。
    loadDocument(makeDarkPageDoc());

    // 带有 NO 填充的导航栏 — LLM 期望深色页面背景显示出来。 Without
    // 主题检测，角色默认会在顶部标记#FFFFFF。
    const navbar = {
      id: 'inserted-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 375,
      height: 56,
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('inserted-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#111111');
  });

  it('upsertNodesToCanvas detects LIGHT theme on a white page (baseline preserved)', () => {
    // Counter-test：相同的路径，但白色页面仍必须生成原始的浅色主题导航栏默认值。 Locks 因为暗修复纯粹是附加的。
    const lightDoc: PenDocument = {
      version: '1.0.0',
      variables: {},
      pages: [
        {
          id: 'page-1',
          name: 'Page 1',
          children: [
            {
              id: DEFAULT_FRAME_ID,
              type: 'frame',
              name: 'Root',
              x: 0,
              y: 0,
              width: 1200,
              height: 800,
              fill: [{ type: 'solid', color: '#FFFFFF' }],
              children: [
                {
                  id: 'placeholder-content',
                  type: 'frame',
                  name: 'Placeholder',
                  width: 100,
                  height: 40,
                } as unknown as PenNode,
              ],
            } as unknown as PenNode,
          ],
        },
      ],
      children: [],
    } as unknown as PenDocument;
    loadDocument(lightDoc);

    const navbar = {
      id: 'light-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 1200,
      height: 72,
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('light-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#FFFFFF');
  });

  it('does NOT overwrite an LLM-supplied navbar fill on either theme', () => {
    // 即使有主题意识，applyDefaults 覆盖保护也必须保持不变：无论页面背景如何，#FF00AA
    // 的填充仍保持#FF00AA。
    loadDocument(makeDarkPageDoc());

    const navbar = {
      id: 'magenta-navbar',
      type: 'frame',
      name: 'Header',
      role: 'navbar',
      width: 375,
      height: 56,
      fill: [{ type: 'solid', color: '#FF00AA' }],
      children: [],
    } as unknown as PenNode;

    upsertNodesToCanvas([navbar]);

    const stored = useDocumentStore.getState().getNodeById('magenta-navbar') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored?.fill?.[0]?.color).toBe('#FF00AA');
  });

  it('detects DARK theme from INPUT NODES even when store still has light default root', () => {
    // The bug Claude 在健身应用重新运行中暴露：新鲜
    // Generation 发出新的暗根帧 INSIDE upsert 输入
    // 节点，但存储仍然保留空的默认根
    // 来自全新的文档。 The 先前的主题检测器读取
    // ONLY 直播店，发现一根光根，盖上白色印记
    // cardFill 默认为新黑暗的每个卡牌家族子代
    // 在到达商店之前先 root。
//
    // The 修复首先遍历输入节点 BFS 并使用最外层
    // 框架的填充作为主题源，回落到现场
    // 仅当输入没有可读填充时才存储。 This 测试练习
    // 该路径：全新文档（默认值），然后更新插入
    // 带有 NO 填充的子卡的暗根。 The
    // 子卡必须出来默认是深色 cardFill
    // (#1a1a1a)，不是浅色默认值(#FFFFFF)。
    // newDocument() 离开商店时带有默认的空根
    // 没有明确的填充——这是“商店说光”状态。
    const darkRoot = {
      id: 'fresh-dark-root',
      type: 'frame',
      name: 'Page',
      x: 0,
      y: 0,
      width: 375,
      height: 1166,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [
        {
          id: 'fresh-dark-card',
          type: 'frame',
          name: 'Heart Rate',
          role: 'card',
          width: 343,
          height: 200,
          children: [],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    upsertNodesToCanvas([darkRoot]);

    const card = useDocumentStore.getState().getNodeById('fresh-dark-card') as
      | (PenNode & { fill?: Array<{ color?: string }>; effects?: unknown[] })
      | undefined;
    expect(card).toBeDefined();
    // The 卡必须获得 DARK 卡填充 (#1a1a1a)，而不是默认灯光 (#FFFFFF)。 This
    // 是一个断言中的整个错误修复。
    expect(card?.fill?.[0]?.color).toBe('#1A1A1A');
    // And 卡片阴影仍然应该应用（角色本身没有被剥离，只是填充选择了正确的主题）。
    expect(card?.effects).toBeDefined();
    expect(Array.isArray(card?.effects) && (card?.effects?.length ?? 0) > 0).toBe(true);
  });

  // -------------------------------------------------------------------------
  // Streaming 路径覆盖 (insertStreamingNode)
// -------------------------------------------------------------------------
  // Codex stop-hook 捕获：流路径绕过
  // 完全是 sanitizeNodesForInsert/Upsert。 Every 每节点流式传输
  // insert 直接使用新构建的 RoleContext 调用 resolveNodeRole
  // 之前不包括 `theme`。 Result：卡系列角色
  // 即使在深色页面上，默认值也始终使用 cardFill('light')=#FFFFFF。
  // The 修复将 `detectActiveDocumentTheme([node])` 传递到流中
  // ctx，首先读取输入节点，然后回退到实时节点
  // 商店。 These 测试锁定两种形状。

  it('streaming: root node with dark fill propagates DARK theme to its own role defaults', () => {
    // First 流节点是页面根框架本身。 Store 仍然具有 newDocument() 的空默认根。 Input-first
    // 检测会看到传入根节点上的深色填充并将其用于主题。 The 根框架在这里有作用：卡行使主题感知默认路径；
    // cardFill('dark') = 如果填充缺失，#1a1a1a 将被注入。 （Author
    // 已经提供了#0a0a0a，覆盖保护保留。）
    resetGenerationRemapping();
    const root = {
      id: 'stream-dark-root',
      type: 'frame',
      name: 'Page',
      x: 0,
      y: 0,
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [],
    } as unknown as PenNode;

    insertStreamingNode(root, null);
    // Now 根已提交给商店。 Stream 一个无填充卡片子项 — 它应该选择默认的深色主题 cardFill。
    const card = {
      id: 'stream-dark-card',
      type: 'frame',
      name: 'Heart Card',
      role: 'card',
      width: 343,
      height: 200,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(card, 'stream-dark-root');

    const stored = useDocumentStore.getState().getNodeById('stream-dark-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('streaming: card child under a dark page root gets DARK cardFill default (no white bar)', () => {
    // Same 如上所述，但使用预加载的暗文档而不是流式传输根。 This 更接近新一代，其中页面根已经在画布上（例如在同一活动
    // 页面上重新运行）。
    loadDocument(makeDarkPageDoc());
    resetGenerationRemapping();

    const card = {
      id: 'stream-card-on-dark',
      type: 'frame',
      name: 'Activity',
      role: 'card',
      width: 343,
      height: 180,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(card, DEFAULT_FRAME_ID);

    const stored = useDocumentStore.getState().getNodeById('stream-card-on-dark') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored).toBeDefined();
    expect(stored?.fill?.[0]?.color).toBe('#1A1A1A');
  });

  it('streaming: navbar inferred from name "Header" inside a dark page root gets DARK navbarFill', () => {
    // The 健身应用程序重新运行时的确切失败案例：名为“Header”的流式子项被推断为导航栏角色，并获得了默认的
    // #FFFFFF 填充灯。 After 修复它应该选择 navbarFill('dark') = #111111。
    loadDocument(makeDarkPageDoc());
    resetGenerationRemapping();

    const header = {
      id: 'stream-page-header',
      type: 'frame',
      name: 'Header', // 名称推断 → 导航栏
      width: 375,
      height: 56,
      children: [],
    } as unknown as PenNode;
    insertStreamingNode(header, DEFAULT_FRAME_ID);

    const stored = useDocumentStore.getState().getNodeById('stream-page-header') as
      | (PenNode & { role?: string; fill?: Array<{ color?: string }> })
      | undefined;
    expect(stored?.role).toBe('navbar');
    expect(stored?.fill?.[0]?.color).toBe('#111111');
  });

  it('does NOT let a small white nested card outvote the dark page root for theme detection', () => {
    // BFS 保证：当输入森林有多个填充帧时，OUTERMOST 获胜。包含小白卡的深色页面根仍必须检测为“深色”，因此
    // OTHER 卡（没有填充的卡片）将获得深色默认值。
    const darkRoot = {
      id: 'mixed-root',
      type: 'frame',
      name: 'Page',
      width: 375,
      height: 800,
      fill: [{ type: 'solid', color: '#0A0A0A' }],
      layout: 'vertical',
      children: [
        // An 显式白卡（LLM 作者意图 - 逐字保留）
        {
          id: 'mixed-white-card',
          type: 'frame',
          name: 'White Card',
          role: 'card',
          width: 343,
          height: 100,
          fill: [{ type: 'solid', color: '#FFFFFF' }],
          children: [],
        } as unknown as PenNode,
        // 一张无填充卡，应获得 DARK 默认值
        {
          id: 'mixed-default-card',
          type: 'frame',
          name: 'Default Card',
          role: 'card',
          width: 343,
          height: 100,
          children: [],
        } as unknown as PenNode,
      ],
    } as unknown as PenNode;

    upsertNodesToCanvas([darkRoot]);

    const whiteCard = useDocumentStore.getState().getNodeById('mixed-white-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;
    const defaultCard = useDocumentStore.getState().getNodeById('mixed-default-card') as
      | (PenNode & { fill?: Array<{ color?: string }> })
      | undefined;

    // Author 意图保留
    expect(whiteCard?.fill?.[0]?.color).toBe('#FFFFFF');
    // Outermost 深色根在主题检测中获胜→填充卡默认为深色，而不是浅色
    expect(defaultCard?.fill?.[0]?.color).toBe('#1A1A1A');
  });
});
