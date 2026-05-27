// Parser
export { parseFigFile } from './fig-parser.js';

// Document 转换
export {
  figmaToPenDocument,
  figmaAllPagesToPenDocument,
  getFigmaPages,
  getFigmaPageLayers,
  figmaNodeChangesToPenNodes,
} from './figma-node-mapper.js';

// Clipboard
export {
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
} from './figma-clipboard.js';

// Image 分辨率
export { resolveImageBlobs } from './figma-image-resolver.js';

// Icon 查找注入
export { setIconLookup } from './figma-node-converters.js';

// Types
export type { FigmaDecodedFile, FigmaImportLayoutMode } from './figma-types.js';
export type { FigmaLayerSummary, FigmaToPenDocumentOptions } from './figma-node-mapper.js';
