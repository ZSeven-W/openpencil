// @vitest-环境 jsdom
import { describe, it, expect, vi, beforeAll, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import { DesignEngineContext } from '../context';
import { DesignCanvas } from '../components/design-canvas';

// Mock DesignCanvas 从中导入的浏览器条目。 Use 包子路径导出（与源 `import`
// 语句匹配），因此模拟在任何签出位置下都以相同的方式解析 - 之前的测试固定了绝对 /Users/... 路径，该路径在 CI
// 中默默无操作。
vi.mock('@zseven-w/pen-engine/browser', () => ({
  attachCanvas: vi.fn(() =>
    Promise.resolve({
      render: vi.fn(),
      resize: vi.fn(),
      dispose: vi.fn(),
      renderToImageData: vi.fn(),
    }),
  ),
  attachInteraction: vi.fn(() => vi.fn()),
}));

// jsdom 没有实现 ResizeObserver — 提供一个存根
beforeAll(() => {
  if (typeof window !== 'undefined' && !window.ResizeObserver) {
    window.ResizeObserver = class ResizeObserver {
      observe() {}
      unobserve() {}
      disconnect() {}
    };
  }
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

function createMockEngine() {
  return {
    getDocument: vi.fn(() => ({ id: 'doc', children: [], pages: [] })),
    on: vi.fn(() => vi.fn()),
    off: vi.fn(),
    zoom: 1,
    panX: 0,
    panY: 0,
    setViewport: vi.fn(),
    screenToScene: vi.fn(() => ({ x: 0, y: 0 })),
    zoomToRect: vi.fn(),
    getContentBounds: vi.fn(() => null),
    dispose: vi.fn(),
  };
}

describe('DesignCanvas', () => {
  it('should render a canvas element inside a container div', () => {
    const engine = createMockEngine();

    const { container } = render(
      <DesignEngineContext.Provider value={engine as any}>
        <DesignCanvas />
      </DesignEngineContext.Provider>,
    );

    const canvas = container.querySelector('canvas');
    expect(canvas).toBeTruthy();
  });

  it('should show loading fallback initially', () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <DesignCanvas loadingFallback={<div data-testid="loader">Loading...</div>} />
      </DesignEngineContext.Provider>,
    );

    expect(screen.getByTestId('loader')).toBeTruthy();
  });

  it('should hide loading fallback after attachCanvas resolves', async () => {
    const engine = createMockEngine();

    render(
      <DesignEngineContext.Provider value={engine as any}>
        <DesignCanvas loadingFallback={<div data-testid="loader">Loading...</div>} />
      </DesignEngineContext.Provider>,
    );

    // Initially 显示加载程序
    expect(screen.getByTestId('loader')).toBeTruthy();

    // After attachCanvas 解析（嘲笑为即时），加载状态清除
    await waitFor(() => {
      expect(screen.queryByTestId('loader')).toBeNull();
    });
  });
});
