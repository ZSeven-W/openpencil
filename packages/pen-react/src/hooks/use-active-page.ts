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
 * Returns 活动页面
 * ID、页面列表和 setActivePage 操作。 Re-在 page:change 和
 document:change 事件上呈现。
 */
export function useActivePage(): ActivePageState {
  const engine = useDesignEngine();
  const activePageId = useEngineSubscribe(engine, 'page:change', (e) => e.getActivePage());
  const doc = useDocument();
  const pages = doc.pages ?? [];
  const setActivePage = useCallback((id: string) => engine.setActivePage(id), [engine]);
  return { activePageId, pages, setActivePage };
}
