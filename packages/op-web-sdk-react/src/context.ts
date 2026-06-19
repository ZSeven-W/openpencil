import { createContext } from 'react';
import type { OpViewer } from '@zseven-w/op-web-sdk';

// React context holding the current OpViewer instance; null when no provider is mounted.
export const ViewerContext = createContext<OpViewer | null>(null);
