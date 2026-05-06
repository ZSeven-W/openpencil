import { describe, it, expect, vi, beforeEach } from 'vitest';

// 在导入 SkiaEngine 之前进行 Must 模拟，以防止 CanvasKit WASM 初始化。 SkiaEngine → SkiaRenderer →
// SkiaNodeRenderer → CanvasKit TypefaceFontProvider.Make()
vi.mock('@zseven-w/pen-renderer', async () => {
  // SkiaNodeRenderer 的 Minimal 存根及相关导出
  const SkiaNodeRenderer = vi.fn().mockImplementation(() => ({
    setIconLookup: vi.fn(),
    setRedrawCallback: vi.fn(),
    init: vi.fn(),
    dispose: vi.fn(),
    fontManager: {
      ensureFont: vi.fn().mockResolvedValue(undefined),
      pendingCount: () => 0,
      flushPending: async () => {},
    },
    imageLoader: {
      pendingCount: () => 0,
      flushPending: async () => {},
    },
  }));
  // Keep 被 skia-engine.ts 使用的命名导出
  return {
    SkiaNodeRenderer,
    SpatialIndex: vi.fn().mockImplementation(() => ({
      rebuild: vi.fn(),
      get: vi.fn(),
    })),
    parseColor: vi.fn(),
    viewportMatrix: vi.fn(),
    zoomToPoint: vi.fn(),
    flattenToRenderNodes: vi.fn().mockReturnValue([]),
    resolveRefs: vi.fn().mockReturnValue([]),
    premeasureTextHeights: vi.fn().mockReturnValue([]),
    collectReusableIds: vi.fn(),
    collectInstanceIds: vi.fn(),
    getViewportBounds: vi.fn().mockReturnValue({}),
    isRectInViewport: vi.fn().mockReturnValue(false),
    screenToScene: vi.fn(),
  };
});

vi.mock('@/stores/document-store', () => ({
  useDocumentStore: {
    getState: () => ({ document: { children: [], variables: {}, themes: undefined } }),
  },
  getActivePageChildren: () => [],
  getAllChildren: () => [],
}));

vi.mock('@/stores/canvas-store', () => ({
  useCanvasStore: { getState: () => ({ activePageId: null, selection: { selectedIds: [] } }) },
}));

vi.mock('@/canvas/canvas-layout-engine', () => ({
  setRootChildrenProvider: vi.fn(),
}));

vi.mock('@/variables/resolve-variables', () => ({
  resolveNodeForCanvas: vi.fn((n: unknown) => n),
  getDefaultTheme: vi.fn().mockReturnValue(null),
}));

vi.mock('@/services/ai/icon-resolver', () => ({
  lookupIconByName: vi.fn(),
}));

vi.mock('@/services/ai/design-animation', () => ({
  isNodeBorderReady: vi.fn().mockReturnValue(false),
  getNodeRevealTime: vi.fn().mockReturnValue(undefined),
}));

vi.mock('@/canvas/agent-indicator', () => ({
  getActiveAgentIndicators: vi.fn().mockReturnValue(new Map()),
  getActiveAgentFrames: vi.fn().mockReturnValue(new Map()),
  isPreviewNode: vi.fn().mockReturnValue(false),
}));

import { SkiaEngine } from '../skia-engine';

// Provide requestAnimationFrame 用于 Node.js 测试环境。
// waitForSettled() 使用它来产生一个渲染周期。 We 使用 setImmediate，因此回调会在下一个 tick
// 时触发，而无需真正的动画帧。
let _rafCounter = 0;
const _rafMap = new Map<number, NodeJS.Immediate>();
if (typeof globalThis.requestAnimationFrame === 'undefined') {
  globalThis.requestAnimationFrame = (cb: FrameRequestCallback): number => {
    const id = ++_rafCounter;
    _rafMap.set(
      id,
      setImmediate(() => {
        _rafMap.delete(id);
        cb(Date.now());
      }),
    );
    return id;
  };
  globalThis.cancelAnimationFrame = (id: number) => {
    const handle = _rafMap.get(id);
    if (handle) {
      clearImmediate(handle);
      _rafMap.delete(id);
    }
  };
}

// Minimal 存根渲染器以及我们需要的两个管理器 APIs。
function makeStubRenderer(opts: { fontPending?: number; imagePending?: number }): unknown {
  let fontCount = opts.fontPending ?? 0;
  let imageCount = opts.imagePending ?? 0;
  return {
    fontManager: {
      pendingCount: () => fontCount,
      flushPending: async () => {
        // Drain 刷新时的挂起计数 — 模拟真实的刷新行为
        fontCount = 0;
      },
    },
    imageLoader: {
      pendingCount: () => imageCount,
      flushPending: async () => {
        imageCount = 0;
      },
    },
  };
}

describe('SkiaEngine.waitForSettled', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('returns quickly when nothing is pending and dirty is false', async () => {
    const engine = new SkiaEngine({} as never);
    // Inject 最小渲染器存根 + 干净状态
    (engine as unknown as { renderer: unknown }).renderer = makeStubRenderer({});
    (engine as unknown as { dirty: boolean }).dirty = false;

    const start = Date.now();
    await engine.waitForSettled(1000);
    expect(Date.now() - start).toBeLessThan(500);
  });

  it('drains font/image pending and returns when stable', async () => {
    const engine = new SkiaEngine({} as never);
    (engine as unknown as { renderer: unknown }).renderer = makeStubRenderer({
      fontPending: 2,
      imagePending: 1,
    });
    (engine as unknown as { dirty: boolean }).dirty = false;

    await engine.waitForSettled(2000);
    // After 两个稳定的帧，都应该被排空
    const r = (
      engine as unknown as {
        renderer: {
          fontManager: { pendingCount(): number };
          imageLoader: { pendingCount(): number };
        };
      }
    ).renderer;
    expect(r.fontManager.pendingCount()).toBe(0);
    expect(r.imageLoader.pendingCount()).toBe(0);
  });

  it('logs warning on timeout when state cannot stabilize', async () => {
    const engine = new SkiaEngine({} as never);
    // Renderer 总是报告新的待处理（不稳定）
    (engine as unknown as { renderer: unknown }).renderer = {
      fontManager: {
        pendingCount: () => 1, // Never 零
        flushPending: async () => {},
      },
      imageLoader: {
        pendingCount: () => 0,
        flushPending: async () => {},
      },
    };
    (engine as unknown as { dirty: boolean }).dirty = false;

    const warnSpy = (() => {
      const original = console.warn;
      const calls: unknown[][] = [];
      console.warn = (...args: unknown[]) => calls.push(args);
      return { calls, restore: () => (console.warn = original) };
    })();

    try {
      await engine.waitForSettled(150);
      expect(warnSpy.calls.length).toBeGreaterThan(0);
      expect(String(warnSpy.calls[0][0])).toContain('Timed out');
    } finally {
      warnSpy.restore();
    }
  });
});

describe('SkiaEngine.captureRegion', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('rejects with a clear error when bounds contain non-numeric fields', async () => {
    const engine = new SkiaEngine({} as never);
    (engine as unknown as { renderer: unknown }).renderer = makeStubRenderer({});

    await expect(
      engine.captureRegion(
        { x: 0, y: 0, w: 'fill_container' as unknown as number, h: 80 },
        { waitForSettled: false },
      ),
    ).rejects.toThrow('bounds must have numeric');
  });
});
