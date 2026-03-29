// pen-engine headless core — public API
export { TypedEventEmitter } from './core/event-emitter.js';
export { HistoryManager, type HistoryManagerOptions } from './core/history-manager.js';
export { SelectionManager, type SelectionManagerOptions } from './core/selection-manager.js';
export { DocumentManager, type DocumentManagerOptions } from './core/document-manager.js';
export { PageManager, type PageManagerOptions } from './core/page-manager.js';
export { VariableManager, type VariableManagerOptions } from './core/variable-manager.js';
export { ViewportController, type ViewportControllerOptions } from './core/viewport-controller.js';
export { EngineSpatialIndex } from './core/spatial-index.js';
export { createNodeForTool, isDrawingTool } from './core/node-creator.js';
export { parseSvgToNodes } from './core/svg-parser.js';
