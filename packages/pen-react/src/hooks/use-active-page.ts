import { useCallback } from 'react';
import type { PenPage } from '@zseven-w/pen-types';
import { useDesignEngine } from './use-design-engine.js';
import { useEngineSubscribe } from '../utils/use-engine-subscribe.js';
import { useDocument } from './use-document.js';

interface ActivePageState {
  activePageId: string;
  pages: PenPage[];
  setActivePage: (pageId: string) => void;
}

/**
 * Returns active page ID, page list, and setActivePage action.
 * Re-renders on page:change and document:change events.
 */
export function useActivePage(): ActivePageState {
  const engine = useDesignEngine();
  const activePageId = useEngineSubscribe(engine, 'page:change', (e) => e.getActivePage());
  const doc = useDocument();
  const pages = doc.pages ?? [];
  const setActivePage = useCallback((id: string) => engine.setActivePage(id), [engine]);
  return { activePageId, pages, setActivePage };
}
