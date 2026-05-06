import { createContext } from 'react';
import type { DesignEngine } from '@zseven-w/pen-engine';

/**
 * React 上下文包含
 * DesignEngine 实例。 Provided by <DesignProvider>，被
 useDesignEngine() 消耗。
 */
export const DesignEngineContext = createContext<DesignEngine | null>(null);
