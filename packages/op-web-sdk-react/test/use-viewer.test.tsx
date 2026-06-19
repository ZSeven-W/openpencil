import { describe, it, expect } from 'vitest';
import { renderHook } from '@testing-library/react';
import { makeMockViewer } from './mock-core.js';
import { DesignProvider, useViewer } from '../src/index.js';
import type { ReactNode } from 'react';

describe('useViewer', () => {
  it('returns the provided viewer', () => {
    const { viewer } = makeMockViewer();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignProvider viewer={viewer as never}>{children}</DesignProvider>
    );
    const { result } = renderHook(() => useViewer(), { wrapper });
    expect(result.current).toBe(viewer);
  });
  it('throws without a provider', () => {
    expect(() => renderHook(() => useViewer())).toThrow();
  });
});
