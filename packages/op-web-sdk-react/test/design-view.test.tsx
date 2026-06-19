import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, waitFor } from '@testing-library/react';
import { makeMockViewer } from './mock-core.js';

const mock = makeMockViewer();
vi.mock('@zseven-w/op-web-sdk', () => ({ createViewer: vi.fn(async () => mock.viewer) }));
import { DesignView } from '../src/index.js';
import { createViewer } from '@zseven-w/op-web-sdk';

describe('DesignView', () => {
  beforeEach(() => vi.clearAllMocks());
  it('creates a viewer on mount and destroys on unmount', async () => {
    const onLoad = vi.fn();
    const { unmount, container } = render(<DesignView doc='{"version":"1.0"}' onLoad={onLoad} />);
    expect(container.querySelector('canvas')).toBeTruthy();
    await waitFor(() => expect(createViewer).toHaveBeenCalledTimes(1));
    await waitFor(() => expect(onLoad).toHaveBeenCalledWith(mock.viewer));
    unmount();
    expect(mock.viewer.destroy).toHaveBeenCalled();
  });
});
