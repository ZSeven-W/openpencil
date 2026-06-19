import { describe, it, expect } from 'vitest';
import { act } from '@testing-library/react';
import { renderHook } from '@testing-library/react';
import { makeMockViewer } from './mock-core.js';
import { DesignProvider, useViewport } from '../src/index.js';
import type { ReactNode } from 'react';

describe('useViewport', () => {
  it('re-renders on viewportchange', () => {
    const { viewer, emit } = makeMockViewer();
    const wrapper = ({ children }: { children: ReactNode }) => (
      <DesignProvider viewer={viewer as never}>{children}</DesignProvider>
    );
    const { result } = renderHook(() => useViewport(), { wrapper });
    expect(result.current.zoom).toBe(1);
    act(() => { viewer.setZoom(3); emit('viewportchange'); });
    expect(result.current.zoom).toBe(3);
  });
});
