/**
 * Re-从转换器/模块导出以实现向后兼容性。
 * @deprecated Import 直接来自新代码中的“./converters/index.js”。
 */
export type { ConversionContext, IconLookupResult } from './converters/index.js';
export {
  convertNode,
  convertChildren,
  collectImageBlobs,
  setIconLookup,
} from './converters/index.js';
