// @vitest-environment jsdom
import { describe, it, expect, vi, beforeAll, afterEach } from 'vitest';
import { render, screen, cleanup, waitFor } from '@testing-library/react';
import { DesignEngineContext } from '../context';
import { DesignCanvas } from '../components/design-canvas';

// Mock via the resolved file path so vitest can intercept it regardless of
// how the package sub-path export is resolved through symlinks.
vi.mock('/Users/kayshen/Workspace/ZSeven-W/openpencil/packages/pen-engine/src/browser.ts', () => ({
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

// jsdom doesn't implement ResizeObserver — provide a stub
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

    // Initially shows loader
    expect(screen.getByTestId('loader')).toBeTruthy();

    // After attachCanvas resolves (mocked as instant), loading state clears
    await waitFor(() => {
      expect(screen.queryByTestId('loader')).toBeNull();
    });
  });
});
