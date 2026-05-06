// pen-engine 无头核心 — public API Engine
export { DesignEngine } from './core/design-engine.js';

// Managers（用于高级合成）
export { TypedEventEmitter } from './core/event-emitter.js';
export { HistoryManager, type HistoryManagerOptions } from './core/history-manager.js';
export { DocumentManager, type DocumentManagerOptions } from './core/document-manager.js';
export { SelectionManager, type SelectionManagerOptions } from './core/selection-manager.js';
export { PageManager, type PageManagerOptions } from './core/page-manager.js';
export { VariableManager, type VariableManagerOptions } from './core/variable-manager.js';
export { ViewportController, type ViewportControllerOptions } from './core/viewport-controller.js';
export { EngineSpatialIndex } from './core/spatial-index.js';

// Utilities
export { createNodeForTool, isDrawingTool } from './core/node-creator.js';
export { parseSvgToNodes } from './core/svg-parser.js';

// Re-从笔型中导出按键类型以方便使用
export type {
  DesignEngineOptions,
  DesignEngineEvents,
  CodePlatform,
  CodeResult,
} from '@zseven-w/pen-types';
