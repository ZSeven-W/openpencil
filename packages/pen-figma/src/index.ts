// Parser
export { parseFigFile } from './fig-parser.js';

// Document conversion
export {
  figmaToPenDocument,
  figmaAllPagesToPenDocument,
  getFigmaPages,
  figmaNodeChangesToPenNodes,
} from './figma-node-mapper.js';

// Clipboard
export {
  isFigmaClipboardHtml,
  extractFigmaClipboardData,
  figmaClipboardToNodes,
} from './figma-clipboard.js';

// Image resolution
export { resolveImageBlobs } from './figma-image-resolver.js';

// Icon lookup injection
export { setIconLookup } from './figma-node-converters.js';

// Types
export type { FigmaDecodedFile, FigmaImportLayoutMode } from './figma-types.js';
