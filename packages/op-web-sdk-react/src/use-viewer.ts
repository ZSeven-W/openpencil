import { createElement, useContext, type ReactNode } from 'react';
import type { OpViewer } from '@zseven-w/op-web-sdk';
import { ViewerContext } from './context.js';

// Provider component that injects an OpViewer into React context.
export function DesignProvider(props: { viewer: OpViewer; children: ReactNode }) {
  return createElement(ViewerContext.Provider, { value: props.viewer }, props.children);
}

// Returns the OpViewer from the nearest DesignProvider/DesignView ancestor.
// Throws if called outside a provider tree.
export function useViewer(): OpViewer {
  const v = useContext(ViewerContext);
  if (!v) throw new Error('op-web-sdk-react: useViewer must be used inside <DesignProvider>/<DesignView>');
  return v;
}
