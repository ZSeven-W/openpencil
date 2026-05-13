// @vitest-environment jsdom

import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import PropertyPanel from './property-panel';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import type { PenDocument } from '@/types/pen';
import '@/i18n';

const documentWithLayers: PenDocument = {
  version: '1.0.0',
  pages: [
    {
      id: 'page-1',
      name: 'Page 1',
      children: [
        {
          id: 'layer-a',
          type: 'rectangle',
          name: 'Layer A',
          x: 10,
          y: 20,
          width: 111,
          height: 82,
          fill: [{ type: 'solid', color: '#ff0000' }],
        },
        {
          id: 'layer-b',
          type: 'rectangle',
          name: 'Layer B',
          x: 30,
          y: 40,
          width: 241,
          height: 121,
          fill: [{ type: 'solid', color: '#00ff00' }],
        },
      ],
    },
  ],
  children: [],
};

beforeEach(() => {
  useDocumentStore.setState({
    document: documentWithLayers,
    isDirty: false,
  });
  useCanvasStore.setState({
    activePageId: 'page-1',
    selection: {
      ...useCanvasStore.getState().selection,
      selectedIds: [],
      activeId: null,
      hoveredId: null,
    },
  });
});

afterEach(() => {
  cleanup();
});

describe('PropertyPanel active layer display', () => {
  it('refreshes key property values when the active layer changes', () => {
    const { rerender } = render(<PropertyPanel embedded />);

    useCanvasStore.getState().setSelection(['layer-a'], 'layer-a');
    rerender(<PropertyPanel embedded />);

    expect(screen.getByText('Layer A')).toBeTruthy();
    expect(screen.getByDisplayValue('10')).toBeTruthy();
    expect(screen.getByDisplayValue('20')).toBeTruthy();
    expect(screen.getByDisplayValue('111')).toBeTruthy();
    expect(screen.getByDisplayValue('82')).toBeTruthy();

    useCanvasStore.getState().setSelection(['layer-a', 'layer-b'], 'layer-b');
    rerender(<PropertyPanel embedded />);

    expect(screen.getByText('Layer B')).toBeTruthy();
    expect(screen.getByDisplayValue('30')).toBeTruthy();
    expect(screen.getByDisplayValue('40')).toBeTruthy();
    expect(screen.getByDisplayValue('241')).toBeTruthy();
    expect(screen.getByDisplayValue('121')).toBeTruthy();
    expect(screen.queryByText('Layer A')).toBeNull();
  });
});
